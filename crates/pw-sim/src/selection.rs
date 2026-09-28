//! Team selection (07 §3): score every candidate for every slot from the
//! manager's *perception*, then solve the exact assignment. The same function
//! picks AI line-ups and powers the protagonist's selection forecast.

use pw_core::rng::{Rng, hash_key, stream};
use pw_core::{Attr, CompId, Date, Hidden, Mentality, PlayerId, Pos, PosGroup, Slot, Tactics, TeamId};
use pw_match::{PlayerSheet, TeamSheet};
use pw_world::knowledge::{Observer, perceive, sigma};
use pw_world::player::{familiarity_factor, raw_ability};
use pw_world::{Archetype, Philosophy, SquadStatus, World};
use smallvec::SmallVec;

use crate::health;
use crate::hungarian;

#[derive(Clone, Debug)]
pub struct Selection {
    pub team: TeamId,
    pub formation: u8,
    pub slots: [Slot; 11],
    pub xi: [PlayerId; 11],
    pub bench: SmallVec<[PlayerId; 12]>,
    pub captain: PlayerId,
    pub tactics: Tactics,
}

impl Selection {
    pub fn role_of(&self, p: PlayerId) -> Option<Slot> {
        self.xi.iter().position(|&x| x == p).map(|i| self.slots[i])
    }
}

/// Why a candidate scored as they did; compared to explain omissions.
#[derive(Clone, Copy, Debug, Default)]
pub struct Factors {
    pub ability: f32,
    pub form: f32,
    pub fitness: f32,
    pub role: f32,
    pub trust: f32,
    pub youth: f32,
    pub rotation: f32,
}

#[derive(Clone, Copy)]
struct Weights {
    ability: f32,
    form: f32,
    fitness: f32,
    role: f32,
    trust: f32,
    youth: f32,
    rotation: f32,
}

fn weights(a: Archetype) -> Weights {
    // 17 §4 defaults.
    match a {
        Archetype::Pragmatist => Weights { ability: 0.40, form: 0.20, fitness: 0.15, role: 0.10, trust: 0.05, youth: 0.0, rotation: 0.05 },
        Archetype::Developer => Weights { ability: 0.30, form: 0.15, fitness: 0.15, role: 0.10, trust: 0.05, youth: 0.15, rotation: 0.05 },
        Archetype::Rotator => Weights { ability: 0.35, form: 0.20, fitness: 0.20, role: 0.10, trust: 0.05, youth: 0.03, rotation: 0.15 },
        Archetype::Loyalist => Weights { ability: 0.35, form: 0.10, fitness: 0.10, role: 0.10, trust: 0.20, youth: 0.0, rotation: 0.02 },
    }
}

fn status_trust(s: SquadStatus) -> f32 {
    match s {
        SquadStatus::Star => 1.0,
        SquadStatus::Important => 0.85,
        SquadStatus::Regular => 0.7,
        SquadStatus::Squad => 0.5,
        SquadStatus::ImpactSub => 0.45,
        SquadStatus::Fringe => 0.3,
        SquadStatus::Backup => 0.2,
        SquadStatus::Youngster => 0.35,
        SquadStatus::NotNeeded => 0.05,
    }
}

pub fn philosophy_of(w: &World, team: TeamId) -> Philosophy {
    let club = &w.clubs[w.teams[team].club];
    club.manager.get().map(|m| w.staff[m].philosophy).unwrap_or_default()
}

/// The pool a team can pick from today: its own available players, topped up
/// from the club's other teams when short.
fn pool(w: &World, team: TeamId, comp: CompId) -> Vec<PlayerId> {
    let t = &w.teams[team];
    let ok = |p: &PlayerId| {
        w.players.hot[*p].available() && w.players.hot[*p].team == team && !w.intl.duty.contains(p) && !w.incidents.is_away(*p, w.date) && pw_world::rules::match_eligible(w, *p, comp, t.club)
    };
    let mut v: Vec<PlayerId> = t.squad.iter().copied().filter(ok).collect();
    if v.len() < 16 {
        let club = &w.clubs[t.club];
        let mut extra: Vec<PlayerId> = club
            .teams
            .iter()
            .filter(|&&o| o != team)
            .flat_map(|&o| w.teams[o].squad.iter().copied())
            .filter(|&p| w.players.hot[p].available() && w.age(p) >= 15 && pw_world::rules::match_eligible(w, p, comp, t.club))
            .collect();
        extra.sort_by(|&a, &b| w.players.cold[b].ca.cmp(&w.players.cold[a].ca).then(a.cmp(&b)));
        let need = 16 - v.len();
        let add: Vec<PlayerId> = extra.into_iter().filter(|p| !v.contains(p)).take(need).collect();
        v.extend(add);
    }
    if v.len() < 11 {
        // Emergency: field anyone registered with the team.
        for &p in &t.squad {
            if !v.contains(&p) && w.players.hot[p].status == pw_world::PlayerStatus::Active {
                v.push(p);
            }
        }
    }
    // Foreigners-on-the-pitch rules (02 §3): the manager only considers as many
    // foreigners as can take part (starters plus a few substitutes).
    let nation = w.clubs[t.club].nation;
    let prof = pw_world::rules::profile(w, nation);
    if prof.foreign_on_pitch_max > 0 {
        let mut foreign: Vec<PlayerId> = v.iter().copied().filter(|&p| pw_world::rules::is_foreign(w, p, nation)).collect();
        let allowed = usize::from(prof.foreign_on_pitch_max) + 2;
        if foreign.len() > allowed {
            foreign.sort_by(|&a, &b| w.players.cold[b].ca.cmp(&w.players.cold[a].ca).then(a.cmp(&b)));
            let drop: Vec<PlayerId> = foreign.split_off(allowed);
            v.retain(|p| !drop.contains(p));
        }
    }
    v.sort();
    v
}

struct Candidate {
    id: PlayerId,
    /// Perceived ability per slot, CA scale.
    ability: [f32; 11],
    f: Factors,
    keeper: bool,
    leadership: f32,
}

/// Score candidates for a formation from the manager's perspective.
fn candidates(w: &World, team: TeamId, comp: CompId, slots: &[Slot; 11], phil: &Philosophy, date: Date, noise_key: u64) -> Vec<Candidate> {
    let club = w.teams[team].club;
    let (judging, _) = w.club_manager_judging(club);
    let t = &w.data.tuning.perception;
    let famous = t.famous_reputation;
    let manager = crate::social::team_manager(w, team);
    pool(w, team, comp)
        .into_iter()
        .map(|p| {
            let h = &w.players.hot[p];
            let c = &w.players.cold[p];
            let person = &w.people[c.person];
            let sig = sigma(t, w.knowledge.seen(club, p), judging, date, c.rep.world >= famous);
            let mut ability = [0.0f32; 11];
            for (i, s) in slots.iter().enumerate() {
                let truth = raw_ability(&c.attrs, s.pos, &w.data.weights) * familiarity_factor(c.familiarity[s.pos.idx()]);
                ability[i] = perceive(truth, sig * 6.0, Observer::Club(club), p, 2000 + s.pos.idx() as u64);
            }
            let form = h.form_avg().map_or(f32::from(h.training) / 10.0, |f| 0.6 * f + 0.4 * f32::from(h.training) / 10.0);
            let age = person.dob.age_years(date);
            let consistency = person.hidden.f(Hidden::Consistency);
            let mut rng = Rng::keyed(&[noise_key, u64::from(p.0)]);
            let rested = h.last_match.days_until(date);
            Candidate {
                id: p,
                ability,
                f: Factors {
                    ability: 0.0,
                    form: ((form - 5.5) / 3.0).clamp(0.0, 1.0),
                    fitness: (f32::from(h.condition) / 100.0 * (0.7 + 0.3 * f32::from(h.sharpness) / 100.0)).clamp(0.0, 1.0) - if h.condition < 70 { 0.25 } else { 0.0 },
                    role: 0.0,
                    // Status says what the club expects; the manager's own trust,
                    // built from memories of training, promises and rows, moves it.
                    trust: manager.map_or(status_trust(c.status), |m| {
                        0.55 * status_trust(c.status) + 0.45 * crate::consider::trust(w, m, c.person) - 0.05 * crate::consider::memory(w, m, c.person, pw_world::MemoryKind::PoorAttitude)
                            + 0.04 * crate::consider::memory(w, m, c.person, pw_world::MemoryKind::ExtraWork)
                    }) + w.clubs[club].manager.get().map_or(0.0, |s| crate::managers::preference(w, s, p) * 0.06)
                        + rng.normal() * 0.02 * (1.3 - consistency / 20.0),
                    youth: if age <= 21.5 { f32::from(phil.youth_trust) / 100.0 * (1.0 - ((age - 17.0) / 5.0).clamp(0.0, 1.0)) } else { 0.0 },
                    // Players with a managed condition need longer between games.
                    rotation: if rested <= 3 || (rested <= 5 && w.medical.needs_managing(p)) { 1.0 } else { 0.0 },
                },
                keeper: c.familiarity[Pos::GK.idx()] >= 12,
                leadership: c.attrs.get(Attr::Leadership),
            }
        })
        .collect()
}

fn slot_score(c: &Candidate, i: usize, s: Slot, max_ability: f32, wt: &Weights, importance: f32, attrs: &pw_core::Attrs) -> f32 {
    let keeper_slot = s.pos == Pos::GK;
    if keeper_slot != c.keeper && (keeper_slot || s.pos.group() != PosGroup::Gk) {
        if keeper_slot {
            return f32::NEG_INFINITY;
        }
        return -1.0 + c.ability[i] / max_ability.max(1.0) * 0.2;
    }
    let ability = c.ability[i] / max_ability.max(1.0);
    let role = attrs.weighted(s.role.key_attrs()) / 20.0;
    let rotation = c.f.rotation * (1.0 - importance);
    wt.ability * ability + wt.form * c.f.form + wt.fitness * c.f.fitness + wt.role * role + wt.trust * c.f.trust + wt.youth * c.f.youth - wt.rotation * rotation
}

/// Pick the line-up for `team`'s next match (competition unknown: no cup-tying).
pub fn select(w: &World, team: TeamId, date: Date, importance: f32, bench_size: u8, noise: u64) -> Option<Selection> {
    select_in(w, team, CompId::NONE, date, importance, bench_size, noise)
}

/// Pick the line-up for a match in `comp`, honouring its eligibility rules.
pub fn select_in(w: &World, team: TeamId, comp: CompId, date: Date, importance: f32, bench_size: u8, noise: u64) -> Option<Selection> {
    let phil = philosophy_of(w, team);
    let wt = weights(phil.archetype);
    let noise_key = hash_key(&[w.seed, stream::SELECTION, u64::from(team.0), date.0 as u64, noise]);
    let mut formations: SmallVec<[u8; 2]> = SmallVec::new();
    for f in phil.formations {
        if usize::from(f) < w.data.formations.len() && !formations.contains(&f) {
            formations.push(f);
        }
    }
    if formations.is_empty() {
        formations.push(0);
    }

    let mut best: Option<(f32, u8, [Slot; 11], Vec<Candidate>, Vec<usize>)> = None;
    for &f in &formations {
        let slots = w.data.formations[usize::from(f)].slots;
        let cands = candidates(w, team, comp, &slots, &phil, date, noise_key);
        if cands.len() < 11 {
            return None;
        }
        let max_ability = cands.iter().flat_map(|c| c.ability).fold(1.0f32, f32::max);
        let score = |r: usize, col: usize| {
            let p = cands[col].id;
            slot_score(&cands[col], r, slots[r], max_ability, &wt, importance, &w.players.cold[p].attrs)
        };
        let assign = hungarian::maximise(11, cands.len(), score);
        let total: f32 = assign.iter().enumerate().map(|(r, &c)| score(r, c)).filter(|s| s.is_finite()).sum();
        if best.as_ref().is_none_or(|b| total > b.0 + 0.02) {
            best = Some((total, f, slots, cands, assign));
        }
    }
    let (_, formation, slots, cands, assign) = best?;
    let xi: [PlayerId; 11] = std::array::from_fn(|r| cands[assign[r]].id);

    // Bench: best remaining, guaranteeing a goalkeeper.
    let max_ability = cands.iter().flat_map(|c| c.ability).fold(1.0f32, f32::max);
    let mut rest: Vec<(f32, &Candidate)> = cands
        .iter()
        .filter(|c| !xi.contains(&c.id))
        .map(|c| {
            let best_slot = (0..11).map(|r| slot_score(c, r, slots[r], max_ability, &wt, importance, &w.players.cold[c.id].attrs)).filter(|s| s.is_finite()).fold(-9.0f32, f32::max);
            (best_slot, c)
        })
        .collect();
    rest.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.id.cmp(&b.1.id)));
    let mut bench: SmallVec<[PlayerId; 12]> = SmallVec::new();
    if let Some((_, gk)) = rest.iter().find(|(_, c)| c.keeper) {
        bench.push(gk.id);
    }
    for (_, c) in &rest {
        if bench.len() >= usize::from(bench_size) {
            break;
        }
        if !bench.contains(&c.id) {
            bench.push(c.id);
        }
    }

    let team_captain = w.teams[team].captain;
    let captain = if xi.contains(&team_captain) { team_captain } else { cands.iter().filter(|c| xi.contains(&c.id)).max_by(|a, b| a.leadership.total_cmp(&b.leadership)).map_or(xi[0], |c| c.id) };

    let tactics = Tactics { formation, mentality: Mentality::from_level(i32::from(phil.mentality)), tempo: phil.tempo, width: 50, directness: phil.directness, line: 50, press: phil.press };
    Some(Selection { team, formation, slots, xi, bench, captain, tactics })
}

/// Probability the player starts / makes the squad, from repeated selections
/// under the manager's uncertainty (07 §3 selection forecast).
pub fn forecast(w: &World, team: TeamId, player: PlayerId, date: Date, samples: u32) -> (f32, f32) {
    let (mut start, mut squad) = (0u32, 0u32);
    for k in 0..samples {
        if let Some(s) = select(w, team, date, 0.6, w.data.tuning.matches.bench_size, 1000 + u64::from(k)) {
            if s.xi.contains(&player) {
                start += 1;
                squad += 1;
            } else if s.bench.contains(&player) {
                squad += 1;
            }
        }
    }
    let n = samples.max(1) as f32;
    (start as f32 / n, squad as f32 / n)
}

/// Engine input for a selected side.
pub fn team_sheet(w: &World, sel: &Selection) -> TeamSheet {
    let sheet = |p: PlayerId| player_sheet(w, p);
    let phil = philosophy_of(w, sel.team);
    TeamSheet {
        team: sel.team,
        tactics: sel.tactics,
        slots: sel.slots,
        xi: std::array::from_fn(|i| sheet(sel.xi[i])),
        bench: sel.bench.iter().map(|&p| sheet(p)).collect(),
        manager_reactivity: match phil.archetype {
            Archetype::Rotator => 0.7,
            Archetype::Developer => 0.5,
            Archetype::Pragmatist => 0.4,
            Archetype::Loyalist => 0.25,
        },
    }
}

pub fn player_sheet(w: &World, p: PlayerId) -> PlayerSheet {
    let h = &w.players.hot[p];
    let c = &w.players.cold[p];
    let person = &w.people[c.person];
    PlayerSheet {
        id: p,
        attrs: c.attrs,
        hidden: person.hidden,
        traits: c.traits,
        familiarity: c.familiarity,
        left_foot: c.left_foot,
        right_foot: c.right_foot,
        height: c.height,
        condition: f32::from(h.condition),
        sharpness: f32::from(h.sharpness),
        morale: f32::from(h.morale),
        injury_risk: health::hazard_mult(w, p),
    }
}
