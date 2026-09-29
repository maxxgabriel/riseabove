//! Team selection (07 §3): score every candidate for every slot from the
//! manager's *perception*, then solve the exact assignment. The same function
//! picks AI line-ups and powers the protagonist's selection forecast.

use pw_core::rng::{Rng, hash_key, stream};
use pw_core::{Attr, CompId, Date, Hidden, Mentality, PersonId, PlayerId, Pos, PosGroup, Slot, Tactics, TeamId};
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

/// What the manager knows about the match in front of him besides the squad.
#[derive(Clone, Copy, Debug)]
pub struct Context {
    /// How much the match matters, 0..1 (competition, stakes).
    pub importance: f32,
    /// The opposition against ours by club standing: −1 (far weaker) … +1 (far stronger).
    pub opposition: f32,
    /// Days until our next match and how much it matters, when it is within four days.
    pub next: Option<(i32, f32)>,
}

impl Context {
    /// A match known only by its importance (forecasts, cup-tie-agnostic callers).
    pub fn plain(importance: f32) -> Self {
        Self { importance, opposition: 0.0, next: None }
    }
}

/// The context of a scheduled fixture for one of its two sides.
pub fn context_for(w: &World, fx: &pw_world::Fixture, team: TeamId, importance: f32) -> Context {
    let other = if fx.home == team { fx.away } else { fx.home };
    let rep = |t: TeamId| f32::from(w.clubs[w.teams[t].club].reputation);
    let opposition = ((rep(other) - rep(team)) / 4000.0).clamp(-1.0, 1.0);
    let next = w.fixtures.next_for(team, w.date.add_days(1), 4).map(|id| {
        let n = w.fixtures.get(id);
        (w.date.days_until(n.date), (crate::matchday::importance(w, n.comp, n.decisive) + crate::culture::stakes(w, n)).min(1.0))
    });
    Context { importance, opposition, next }
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
    /// Injury exposure: hazard from workload, a managed condition, low condition.
    pub risk: f32,
    /// Reputation for trouble and what this manager remembers of disciplining him.
    pub indiscipline: f32,
    /// A promise the manager has made him and is behind on keeping.
    pub promise: f32,
    /// Temperament for big matches (important-matches and pressure traits), −0.5..0.5.
    pub big_match: f32,
    /// Composure, concentration and bravery against stronger opposition.
    pub edge: f32,
    pub leadership: f32,
    /// How much resting him now protects him for a bigger match coming soon.
    pub rest_need: f32,
    /// The manager's plan for bringing a newcomer in keeps him out of the eleven for now, 0..1.
    pub hold: f32,
    /// What the manager believes of the man's state today: a strained player he knows of is worth less, a confident one in form a little more.
    pub state: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Weights {
    pub ability: f32,
    pub form: f32,
    pub fitness: f32,
    pub role: f32,
    pub trust: f32,
    pub youth: f32,
    pub rotation: f32,
}

/// This manager's way of weighing things: the archetype's base weights, then his own traits. Two managers with the
/// same squad and the same fixture list can therefore pick different elevens for reasons that can be named.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub w: Weights,
    /// Weight on injury exposure (better sports-science knowledge, more attention to it).
    pub caution: f32,
    /// Weight on indiscipline (from his discipline attribute).
    pub strictness: f32,
    /// Weight on keeping promises he has made (from sportsmanship and professionalism).
    pub word: f32,
    /// Weight on temperament for big matches (managers who back youth lean on experience less).
    pub big_game: f32,
    pub leadership: f32,
    /// How strongly a bigger match soon makes him rest players now.
    pub rest: f32,
}

/// A team's manager's style as selection will use it.
pub fn style_of(w: &World, team: TeamId) -> Style {
    style(w, team, &philosophy_of(w, team))
}

/// The factors selection sees for one player of `team` today, for explanations and tests. `None` if he is not in today's pool.
pub fn factors_of(w: &World, team: TeamId, comp: CompId, date: Date, player: PlayerId, ctx: &Context) -> Option<Factors> {
    let phil = philosophy_of(w, team);
    let slots = w.data.formations.get(usize::from(phil.formations[0])).unwrap_or(&w.data.formations[0]).slots;
    candidates(w, team, comp, &slots, &phil, date, 0, ctx).into_iter().find(|c| c.id == player).map(|c| c.f)
}

fn style(w: &World, team: TeamId, phil: &Philosophy) -> Style {
    let mut base = weights(phil.archetype);
    let club = w.teams[team].club;
    let staff = w.clubs[club].manager;
    let Some(m) = staff.get() else {
        return Style { w: base, caution: 0.08, strictness: 0.05, word: 0.06, big_game: 0.05, leadership: 0.04, rest: 0.7 };
    };
    let person = &w.people[w.staff[m].person];
    let attrs = &w.staff[m].attrs;
    let tendency = w.careers.managers.get(&m).map_or(match phil.archetype {
        Archetype::Rotator => 0.75,
        Archetype::Loyalist => 0.25,
        _ => 0.45,
    }, |p| f32::from(p.rotation) / 100.0);
    base.rotation = 0.02 + 0.16 * tendency;
    base.trust *= 0.7 + 0.6 * person.hidden.f(Hidden::Loyalty) / 20.0;
    Style {
        w: base,
        caution: 0.03 + 0.12 * attrs.f(pw_core::StaffAttr::SportsScience) / 20.0,
        strictness: 0.10 * attrs.f(pw_core::StaffAttr::Discipline) / 20.0,
        word: 0.12 * (person.hidden.f(Hidden::Sportsmanship) + person.hidden.f(Hidden::Professionalism)) / 40.0,
        big_game: 0.03 + 0.07 * (1.0 - f32::from(phil.youth_trust) / 100.0),
        leadership: 0.04,
        rest: 0.4 + 0.6 * tendency,
    }
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
        let extra: Vec<PlayerId> = club
            .teams
            .iter()
            .filter(|&&o| o != team)
            .flat_map(|&o| w.teams[o].squad.iter().copied())
            .filter(|&p| w.players.hot[p].available() && w.age(p) >= 15 && pw_world::rules::match_eligible(w, p, comp, t.club))
            .collect();
        // Called up by how the manager rates them, not by their hidden ability.
        let mut rated: Vec<(f32, PlayerId)> = extra.into_iter().map(|p| (crate::scouting::view(w, t.club, p).0, p)).collect();
        rated.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let need = 16 - v.len();
        let add: Vec<PlayerId> = rated.into_iter().map(|x| x.1).filter(|p| !v.contains(p)).take(need).collect();
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
            let mut rated: Vec<(f32, PlayerId)> = foreign.iter().map(|&p| (crate::scouting::view(w, t.club, p).0, p)).collect();
            rated.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            foreign = rated.into_iter().map(|x| x.1).collect();
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
    /// How well the manager believes his attributes suit the slot's role, 0..1.
    role: [f32; 11],
    f: Factors,
    keeper: bool,
    leadership: f32,
}

/// Indiscipline as the manager can know it: what he believes of the player's reputation for trouble (a personality trait he can
/// only read through evidence, so uncertain), bookings on record, and what he remembers of disciplining him. 0..1.
fn indiscipline(w: &World, manager: Option<PersonId>, p: PlayerId, believed_repute: f32) -> f32 {
    let c = &w.players.cold[p];
    let bookings = (f32::from(w.players.hot[p].yellows) / 8.0).min(1.0);
    let recalled = manager.map_or(0.0, |m| {
        (crate::consider::memory(w, m, c.person, pw_world::MemoryKind::Fined) + crate::consider::memory(w, m, c.person, pw_world::MemoryKind::Dropped) + crate::consider::memory(w, m, c.person, pw_world::MemoryKind::PoorAttitude))
            .min(1.0)
    });
    (0.35 * believed_repute + 0.25 * bookings + 0.4 * recalled).clamp(0.0, 1.0)
}

/// Injury exposure as medical staff can see it: workload, fatigue, injuries on record, a managed condition and low condition.
/// Not proneness or body wear, which are hidden.
fn observed_risk(w: &World, p: PlayerId) -> f32 {
    let (h, c) = (&w.players.hot[p], &w.players.cold[p]);
    let t = &w.data.tuning.health;
    let overload = (h.acwr() - t.acwr_safe_high).max(0.0) / 0.6;
    let history = f32::from(c.injuries_career.min(10)) / 10.0 * 0.5;
    let managed = if w.medical.needs_managing(p) { 0.3 } else { 0.0 };
    (0.5 * overload + 0.4 * f32::from(h.fatigue) / 100.0 + history + managed + (80.0 - f32::from(h.condition)).max(0.0) / 100.0).clamp(0.0, 1.0)
}

/// How pressing a promise the manager made to this player is: 1 when he is behind on minutes he promised, less when on track.
fn promise_pressure(w: &World, manager: Option<PersonId>, p: PlayerId) -> f32 {
    let Some(m) = manager else { return 0.0 };
    let person = w.players.cold[p].person;
    w.social
        .open_promises_between(m, person)
        .map(|pr| match pr.kind {
            pw_world::social::PromiseKind::Minutes { share } => {
                let behind = pr.team_minutes > 0 && (pr.player_minutes as f32 / pr.team_minutes as f32) < share;
                let closing = pr.due.days_until(w.date) >= -21;
                if behind { if closing { 1.0 } else { 0.8 } } else { 0.3 }
            }
            pw_world::social::PromiseKind::Status(_) | pw_world::social::PromiseKind::Captaincy => 0.3,
            _ => 0.0,
        })
        .fold(0.0f32, f32::max)
}

/// Score candidates for a formation from the manager's perspective.
fn candidates(w: &World, team: TeamId, comp: CompId, slots: &[Slot; 11], phil: &Philosophy, date: Date, noise_key: u64, ctx: &Context) -> Vec<Candidate> {
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
            // What the club's staff believe of things they cannot read: the truth seen through their evidence on him. Attributes
            // are read with the club's own uncertainty; personality traits are harder to read than skills.
            let believe = |truth: f32, spread: f32, field: u64| perceive(truth, sig * spread, Observer::Club(club), p, field);
            let mut role = [0.0f32; 11];
            for (i, s) in slots.iter().enumerate() {
                role[i] = (believe(c.attrs.weighted(s.role.key_attrs()), 1.0, 3000 + i as u64) / 20.0).clamp(0.0, 1.0);
            }
            let trait_of = |h: Hidden, field: u64| (believe(person.hidden.f(h), 1.6, field) / 20.0).clamp(0.0, 1.0);
            let repute = 0.5 * (trait_of(Hidden::Controversy, 4001) + trait_of(Hidden::Dirtiness, 4002));
            // A bigger match soon: stars and players who just played are rested first.
            let rest_need = ctx.next.filter(|&(_, imp)| imp > ctx.importance + 0.05).map_or(0.0, |(d, imp)| (imp - ctx.importance) * (1.0 - (d as f32 - 1.0).clamp(0.0, 3.0) / 4.0) * (0.35 + 0.65 * status_trust(c.status)));
            let a = |x: Attr, field: u64| believe(c.attrs.get(x), 1.0, 5000 + field);
            let leadership = a(Attr::Leadership, 1);
            Candidate {
                id: p,
                ability,
                role,
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
                    risk: observed_risk(w, p),
                    indiscipline: indiscipline(w, manager, p, repute),
                    promise: promise_pressure(w, manager, p),
                    big_match: 0.6 * trait_of(Hidden::ImportantMatches, 4003) + 0.4 * trait_of(Hidden::Pressure, 4004) - 0.5,
                    edge: ((a(Attr::Concentration, 2) + a(Attr::Composure, 3) + a(Attr::Bravery, 4)) / 60.0).clamp(0.0, 1.0),
                    leadership: (leadership / 20.0).clamp(0.0, 1.0),
                    rest_need,
                    hold: crate::adaptation::hold(w, p),
                    state: crate::lifestate::selection_term(w, manager, p, ctx.importance),
                },
                keeper: c.familiarity[Pos::GK.idx()] >= 12,
                leadership,
            }
        })
        .collect()
}

fn slot_score(c: &Candidate, i: usize, s: Slot, max_ability: f32, st: &Style, ctx: &Context) -> f32 {
    let keeper_slot = s.pos == Pos::GK;
    if keeper_slot != c.keeper && (keeper_slot || s.pos.group() != PosGroup::Gk) {
        if keeper_slot {
            return f32::NEG_INFINITY;
        }
        return -1.0 + c.ability[i] / max_ability.max(1.0) * 0.2;
    }
    let (wt, f, imp) = (&st.w, &c.f, ctx.importance);
    let ability = c.ability[i] / max_ability.max(1.0);
    let role = c.role[i];
    let rotation = f.rotation * (1.0 - imp);
    let (stronger, weaker) = (ctx.opposition.max(0.0), (-ctx.opposition).max(0.0));
    wt.ability * ability + wt.form * f.form + wt.fitness * f.fitness + wt.role * role + wt.trust * f.trust
        // Weaker opposition is the day to give young players a game.
        + wt.youth * f.youth * (1.0 + weaker)
        - wt.rotation * rotation
        // A bigger match soon rests the players it would cost most (stars first, then anyone who just played).
        - 0.5 * st.rest * f.rest_need
        // A newcomer being brought in gently: out of the eleven for now, less so in the games that matter most.
        - 0.9 * f.hold * (1.0 - 0.4 * imp)
        // What he believes of the man's life today, as the manager knows it.
        + 0.5 * f.state
        // Caution about injury exposure matters least in the matches that matter most.
        - st.caution * f.risk * (1.15 - imp)
        - st.strictness * f.indiscipline
        + st.word * f.promise * (1.0 - 0.7 * imp)
        + st.big_game * f.big_match * imp
        + 0.06 * f.edge * stronger
        + st.leadership * f.leadership * imp
}

/// Pick the line-up for `team`'s next match (competition unknown: no cup-tying).
pub fn select(w: &World, team: TeamId, date: Date, importance: f32, bench_size: u8, noise: u64) -> Option<Selection> {
    select_in(w, team, CompId::NONE, date, importance, bench_size, noise)
}

/// Pick the line-up for a match in `comp`, honouring its eligibility rules, knowing only its importance.
pub fn select_in(w: &World, team: TeamId, comp: CompId, date: Date, importance: f32, bench_size: u8, noise: u64) -> Option<Selection> {
    select_ctx(w, team, comp, date, &Context::plain(importance), bench_size, noise)
}

/// The best starting XI found so far while trying formations.
type BestXi = (f32, u8, [Slot; 11], Vec<Candidate>, Vec<usize>);

/// Pick the line-up for a match in `comp` with everything the manager knows about it (opposition, what comes next).
pub fn select_ctx(w: &World, team: TeamId, comp: CompId, date: Date, ctx: &Context, bench_size: u8, noise: u64) -> Option<Selection> {
    let phil = philosophy_of(w, team);
    let wt = style(w, team, &phil);
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

    // (score, formation, slots, candidates, chosen candidate per slot)
    let mut best: Option<BestXi> = None;
    for &f in &formations {
        let slots = w.data.formations[usize::from(f)].slots;
        let cands = candidates(w, team, comp, &slots, &phil, date, noise_key, ctx);
        if cands.len() < 11 {
            return None;
        }
        let max_ability = cands.iter().flat_map(|c| c.ability).fold(1.0f32, f32::max);
        let score = |r: usize, col: usize| slot_score(&cands[col], r, slots[r], max_ability, &wt, ctx);
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
            let best_slot = (0..11).map(|r| slot_score(c, r, slots[r], max_ability, &wt, ctx)).filter(|s| s.is_finite()).fold(-9.0f32, f32::max);
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
    // A player still settling in brings less to the pitch than his attributes say: his body and clock, his grasp of the system, his head.
    let (body, execution, mind) = crate::adaptation::effect(w, p);
    let mut familiarity = c.familiarity;
    if execution < 1.0 {
        for f in &mut familiarity {
            *f = (f32::from(*f) * execution).round() as u8;
        }
    }
    PlayerSheet {
        id: p,
        attrs: c.attrs,
        hidden: person.hidden,
        traits: c.traits,
        familiarity,
        left_foot: c.left_foot,
        right_foot: c.right_foot,
        height: c.height,
        condition: f32::from(h.condition),
        sharpness: f32::from(h.sharpness) * body,
        morale: f32::from(h.morale) * mind,
        injury_risk: health::hazard_mult(w, p),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pw_core::Role;

    fn slot() -> Slot {
        Slot { pos: Pos::MC, role: Role::CentralMidfielder }
    }

    fn base_style() -> Style {
        Style { w: weights(Archetype::Pragmatist), caution: 0.10, strictness: 0.08, word: 0.10, big_game: 0.06, leadership: 0.04, rest: 0.7 }
    }

    /// A candidate whose factors are neutral except where a test changes them.
    fn cand(f: Factors) -> Candidate {
        Candidate { id: PlayerId(0), ability: [100.0; 11], role: [0.6; 11], f, keeper: false, leadership: 10.0 }
    }

    fn neutral() -> Factors {
        Factors { form: 0.5, fitness: 0.9, trust: 0.5, ..Default::default() }
    }

    fn score(c: &Candidate, st: &Style, ctx: &Context) -> f32 {
        slot_score(c, 0, slot(), 100.0, st, ctx)
    }

    fn ctx(importance: f32) -> Context {
        Context::plain(importance)
    }

    #[test]
    fn a_promise_the_manager_is_behind_on_lifts_a_player_more_for_a_man_of_his_word_and_less_in_big_matches() {
        let (kept, owed) = (cand(neutral()), cand(Factors { promise: 1.0, ..neutral() }));
        let honest = Style { word: 0.12, ..base_style() };
        let flaky = Style { word: 0.0, ..base_style() };
        let lift = |st: &Style, imp: f32| score(&owed, st, &ctx(imp)) - score(&kept, st, &ctx(imp));
        assert!(lift(&honest, 0.3) > 0.05);
        assert!(lift(&flaky, 0.3).abs() < 1e-6, "a manager who does not care about his word is not moved by a promise");
        assert!(lift(&honest, 0.3) > lift(&honest, 1.0), "promises count for less when the match matters most");
    }

    #[test]
    fn resting_before_a_bigger_match_depends_on_the_managers_rotation_habit_and_the_player() {
        let star = cand(Factors { rest_need: 0.5, ..neutral() });
        let ordinary = cand(neutral());
        let soon = Context { importance: 0.4, opposition: 0.0, next: Some((2, 0.95)) };
        let rotator = Style { rest: 1.0, ..base_style() };
        let stubborn = Style { rest: 0.4, ..base_style() };
        let cost = |st: &Style| score(&ordinary, st, &soon) - score(&star, st, &soon);
        assert!(cost(&rotator) > cost(&stubborn) && cost(&stubborn) > 0.0);
        // Nothing to rest for when nothing bigger is coming: rest_need is zero and so is the effect.
        assert!((score(&ordinary, &rotator, &ctx(0.4)) - score(&cand(Factors { rest_need: 0.0, ..neutral() }), &rotator, &ctx(0.4))).abs() < 1e-6);
    }

    #[test]
    fn caution_about_injury_risk_relaxes_in_the_matches_that_matter() {
        let fragile = cand(Factors { risk: 0.9, ..neutral() });
        let sound = cand(neutral());
        let careful = Style { caution: 0.15, ..base_style() };
        let reckless = Style { caution: 0.03, ..base_style() };
        let gap = |st: &Style, imp: f32| score(&sound, st, &ctx(imp)) - score(&fragile, st, &ctx(imp));
        assert!(gap(&careful, 0.25) > gap(&reckless, 0.25) && gap(&reckless, 0.25) > 0.0);
        assert!(gap(&careful, 0.25) > gap(&careful, 1.0), "the cup final is not the day to protect him");
    }

    #[test]
    fn strict_managers_mind_indiscipline_and_lenient_ones_do_not() {
        let trouble = cand(Factors { indiscipline: 0.9, ..neutral() });
        let clean = cand(neutral());
        let strict = Style { strictness: 0.10, ..base_style() };
        let lenient = Style { strictness: 0.0, ..base_style() };
        assert!(score(&clean, &strict, &ctx(0.5)) - score(&trouble, &strict, &ctx(0.5)) > 0.05);
        assert_eq!(score(&clean, &lenient, &ctx(0.5)), score(&trouble, &lenient, &ctx(0.5)));
    }

    #[test]
    fn big_match_temperament_counts_only_when_the_match_matters() {
        let bottler = cand(Factors { big_match: -0.5, ..neutral() });
        let bigger = cand(Factors { big_match: 0.5, ..neutral() });
        let st = base_style();
        let edge = |imp: f32| score(&bigger, &st, &ctx(imp)) - score(&bottler, &st, &ctx(imp));
        assert!(edge(1.0) > edge(0.5) && edge(0.5) > edge(0.1));
        assert!(edge(0.0).abs() < 1e-6, "in a friendly it does not matter");
    }

    #[test]
    fn composure_helps_against_stronger_opposition_and_youth_against_weaker() {
        let cool = cand(Factors { edge: 1.0, ..neutral() });
        let raw = cand(Factors { edge: 0.3, ..neutral() });
        let st = base_style();
        let strong = Context { importance: 0.5, opposition: 0.8, next: None };
        let weak = Context { importance: 0.5, opposition: -0.8, next: None };
        assert!(score(&cool, &st, &strong) - score(&raw, &st, &strong) > score(&cool, &st, &weak) - score(&raw, &st, &weak));
        let youngster = cand(Factors { youth: 0.8, ..neutral() });
        let vet = cand(neutral());
        let developer = Style { w: weights(Archetype::Developer), ..base_style() };
        let gain = |c: &Context| score(&youngster, &developer, c) - score(&vet, &developer, c);
        assert!(gain(&weak) > gain(&strong), "a weaker opponent is the day to play the kid");
    }

    #[test]
    fn a_missing_goalkeeper_can_never_be_fielded_outfield_or_vice_versa() {
        let outfield = cand(neutral());
        let keeper_slot = Slot { pos: Pos::GK, role: Role::Goalkeeper };
        assert_eq!(slot_score(&outfield, 0, keeper_slot, 100.0, &base_style(), &ctx(0.5)), f32::NEG_INFINITY);
    }
}
