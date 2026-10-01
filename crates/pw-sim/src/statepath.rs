//! The state pathway: state leagues feeding the national pyramid, club
//! licensing, and the state-team championship (Wave 5).
//!
//! Each state has a premier league and a second division running beside the
//! national tiers. At a season's end the state champions are nominated for the
//! national fourth tier, weighed on results and their association's standing,
//! and they must hold a licence. The bottom of the fourth tier drops back into
//! its state. Inside each state, second-division winners replace the bottom of
//! the premier league.
//!
//! In February the states field their squads, chosen by selectors from what
//! they have seen (never the true ability), and play a championship whose
//! performances feed the same records and scouting as everything else.

use pw_core::rng::{hash_key, stream};
use pw_core::{ClubId, CompId, Date, Mentality, NationId, PlayerId, Pos, RegionId, Slot, Tactics, TeamId};
use pw_match::{Lod, MatchInput, PlayerSheet, TeamSheet, simulate};
use pw_world::ecosystem::{StageKind, Tournament};
use pw_world::recog::{Learned, Org};
use pw_world::scenario::CalEvent;
use pw_world::event::{EventKind, Visibility};
use pw_world::knowledge::{Observer, perceive};
use pw_world::player::{familiarity_factor, raw_ability};
use pw_world::records::Scope;
use pw_world::ruling::{Outcome, Ruling, RulingKind};
use pw_world::{CompKind, PlayerStatus, TeamKind, World};

use crate::hungarian;
use pw_world::ecosystem::Basis;

/// What a club must show to take a place in the national pyramid (a first draft of licensing).
pub struct Licence {
    pub min_capacity: u32,
    pub min_training: u8,
    /// Debt as a share of season revenue above which a licence is refused.
    pub max_debt_share: f32,
}

pub const LICENCE: Licence = Licence { min_capacity: 2_000, min_training: 4, max_debt_share: 0.6 };

fn licence_ok(w: &World, club: ClubId) -> bool {
    let c = &w.clubs[club];
    let revenue = crate::finance::season_revenue(w, club).max(1);
    c.capacity >= LICENCE.min_capacity
        && c.facilities.training >= LICENCE.min_training
        && (c.finance.debt as f32 / revenue as f32) <= LICENCE.max_debt_share
        && c.manager.is_some()
}

fn table_of(w: &World, comp: CompId, year: i32) -> Vec<pw_core::TeamId> {
    w.history.tables.iter().rev().find(|t| t.comp == comp && t.season == year).map(|t| t.rows.iter().map(|r| r.team).collect()).unwrap_or_default()
}

fn swap_league(w: &mut World, team: TeamId, from: CompId, to: CompId) {
    w.comps[from].state.entrants.retain(|&t| t != team);
    if !w.comps[to].state.entrants.contains(&team) {
        w.comps[to].state.entrants.push(team);
    }
}

/// A nation's season has ended: move clubs between the state leagues and the pyramid.
pub fn season_end(w: &mut World, n: NationId, year: i32) {
    if !w.ext.ecosystem.is_configured() {
        return;
    }
    let Some(&fourth) = w.nations[n].leagues.last() else { return };
    let mut prem: Vec<(RegionId, CompId)> = Vec::new();
    let mut div2: Vec<(RegionId, CompId)> = Vec::new();
    for (id, c) in w.comps.iter_enumerated() {
        if c.nation != n || c.kind != CompKind::League || c.team_kind != TeamKind::First || c.tier < 50 {
            continue;
        }
        let Some(&t) = c.state.entrants.first() else { continue };
        let region = w.ext.ecosystem.state_of(w.ext.ecosystem.region_of_club(w.teams[t].club));
        if c.tier == 50 { prem.push((region, id)) } else { div2.push((region, id)) }
    }
    prem.sort();
    div2.sort();
    // Inside each state: the second division's best replace the premier league's worst.
    for &(state, p) in &prem {
        let Some(&(_, d)) = div2.iter().find(|x| x.0 == state) else { continue };
        let (up, down) = (table_of(w, d, year), table_of(w, p, year));
        let k = 2.min(up.len() / 3).min(down.len() / 3);
        for i in 0..k {
            let (promoted, relegated) = (up[i], down[down.len() - 1 - i]);
            swap_league(w, promoted, d, p);
            swap_league(w, relegated, p, d);
            w.events.push(w.date, Visibility::Public, EventKind::Promoted { comp: p, team: promoted });
            w.events.push(w.date, Visibility::Public, EventKind::Relegated { comp: p, team: relegated });
        }
    }
    // Into the pyramid: nominated champions, weighed on results and their association, and licensed.
    let slots = (usize::from(w.comps[fourth].size) / 6).max(2);
    let mut nominees: Vec<(f32, TeamId, RegionId, CompId)> = Vec::new();
    for &(state, p) in &prem {
        let table = w.history.tables.iter().rev().find(|t| t.comp == p && t.season == year).map(|t| t.rows.clone()).unwrap_or_default();
        let Some(top) = table.first() else { continue };
        let ppg = f32::from(top.points.max(0) as u16) / f32::from(top.played.max(1));
        let quality = w.ext.ecosystem.assoc.get(&state).map_or(50.0, |a| a.comp_quality);
        nominees.push((ppg + quality / 100.0, top.team, state, p));
    }
    nominees.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut promoted: Vec<(TeamId, CompId)> = Vec::new();
    for (_, t, _, p) in nominees {
        if promoted.len() >= slots {
            break;
        }
        let club = w.teams[t].club;
        if licence_ok(w, club) {
            promoted.push((t, p));
        } else {
            let today = w.date;
            let id = w.ext.decisions.add(Ruling {
                id: 0,
                kind: RulingKind::ClubLicence,
                date: today,
                club,
                subject: PlayerId::NONE,
                about: pw_core::PersonId::NONE,
                liability: 0,
                decider: pw_core::PersonId::NONE,
                stances: Default::default(),
                true_pct: 0,
                want: 0,
                outcome: Outcome::Vetoed,
                resolved: Some(today),
                event: pw_core::EventId::NONE,
            });
            let ev = w.events.push(today, Visibility::Public, EventKind::Ruling { ruling: id });
            if let Some(r) = w.ext.decisions.get_mut(id) {
                r.event = ev;
            }
        }
    }
    // The bottom of the fourth tier makes way, one for one, back into its own state.
    let bottom = table_of(w, fourth, year);
    for (i, &(t, p)) in promoted.iter().enumerate() {
        let Some(&drop) = bottom.get(bottom.len().wrapping_sub(1 + i)) else { break };
        let club = w.teams[drop].club;
        let state = w.ext.ecosystem.state_of(w.ext.ecosystem.region_of_club(club));
        let Some(&(_, home)) = prem.iter().find(|x| x.0 == state) else { continue };
        swap_league(w, t, p, fourth);
        swap_league(w, drop, fourth, home);
        // Everyone in the promoted squad goes up with the club: that is why each has a step at this level.
        let up_club = w.teams[t].club;
        for x in w.teams[t].squad.clone() {
            crate::ecosystem::why_only(w, x, StageKind::Professional, pw_world::pathway::Why::Promotion { club: up_club });
        }
        w.clubs[w.teams[t].club].reputation = w.clubs[w.teams[t].club].reputation.saturating_add(150);
        w.clubs[club].reputation = w.clubs[club].reputation.saturating_sub(150);
        w.events.push(w.date, Visibility::Public, EventKind::Promoted { comp: fourth, team: t });
        w.events.push(w.date, Visibility::Public, EventKind::Relegated { comp: fourth, team: drop });
        w.comps[fourth].state.last_moves.push((t, 1));
        w.comps[fourth].state.last_moves.push((drop, -1));
    }
}

// ------------------------------------------------------------------ the state championship

/// Every state a player may represent under the world's eligibility rules, with the ground, in the order of preference.
/// The rules are data (`Ecosystem::eligibility`); the judgement itself lives in `crate::eligibility`.
pub fn eligible_states(w: &World, p: PlayerId) -> Vec<(RegionId, Basis)> {
    crate::eligibility::state_grounds(w, p)
}

/// A club's tier in the national pyramid (1 top) or 9 outside it.
pub(crate) fn tier_of(w: &World, club: ClubId) -> u8 {
    let l = w.clubs[club].league;
    if l.is_some() { w.comps[l].tier } else { 9 }
}

fn select_squad(w: &World, state: RegionId, india: NationId, year: i32, fill: bool, taken: &pw_world::FxHashSet<PlayerId>) -> Vec<PlayerId> {
    let a = w.ext.ecosystem.assoc.get(&state).copied();
    let scouting = a.map_or(40.0, |a| a.scouting);
    let sigma = (14.0 - scouting / 10.0).max(3.0);
    let mut seen: Vec<(f32, PlayerId, bool)> = Vec::new();
    for (p, h) in w.players.hot.iter_enumerated() {
        if !matches!(h.status, PlayerStatus::Active | PlayerStatus::Amateur | PlayerStatus::FreeAgent) || h.injury_days > 14 || taken.contains(&p) {
            continue;
        }
        if w.people[w.players.cold[p].person].nation != india {
            continue;
        }
        // Their own state picks first; a state short of players may then call anyone else who qualifies for it.
        if !crate::eligibility::judge_state(w, p, state, year, fill).eligible {
            continue;
        }
        // Selectors only pick who they know of. They follow their own state's league and its university teams, and remember whom
        // they watched at district trials and earlier championships; a player none of that reaches is heard of only by report.
        let looks = crate::recognition::looks_by(w, Org::State(state), p);
        let in_own_league = h.club.is_some() && w.ext.ecosystem.state_of(w.ext.ecosystem.region_of_club(h.club)) == state;
        let seen_p = if in_own_league {
            0.70 + 0.25 * scouting / 100.0
        } else if looks > 0 {
            0.45 + 0.05 * f32::from(looks.min(4)) + 0.25 * scouting / 100.0
        } else {
            0.08 + 0.25 * scouting / 100.0
        };
        if (hash_key(&[w.seed, stream::INTL, u64::from(state.0), u64::from(p.0), year as u64, 0x5a1]) % 1000) as f32 / 1000.0 > seen_p {
            continue;
        }
        let c = &w.players.cold[p];
        let form = h.form_avg().map_or(0.0, |f| (f - 6.6) * 3.0);
        let est = perceive(f32::from(c.ca), sigma * 2.0, Observer::Person(2_000_000 + state.0), p, 6000 + year as u64) + form; // truth-ok: a selector's noisy reading, with an observer-specific bias
        seen.push((est, p, c.familiarity[Pos::GK.idx()] >= 15));
    }
    seen.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut squad: Vec<PlayerId> = seen.iter().filter(|x| x.2).take(2).map(|x| x.1).collect();
    for x in &seen {
        if squad.len() >= 22 {
            break;
        }
        if !squad.contains(&x.1) && !x.2 {
            squad.push(x.1);
        }
    }
    squad
}

fn sheet(w: &World, squad: &[PlayerId], state: RegionId) -> Option<TeamSheet> {
    let avail: Vec<PlayerId> = squad.iter().copied().filter(|&p| w.players.hot[p].injury == 0).collect();
    if avail.len() < 11 {
        return None;
    }
    let slots = w.data.formations[0].slots;
    let keeper: Vec<bool> = avail.iter().map(|&p| w.players.cold[p].familiarity[Pos::GK.idx()] >= 12).collect();
    let est: Vec<[f32; 11]> = avail
        .iter()
        .map(|&p| {
            let c = &w.players.cold[p];
            std::array::from_fn(|i| {
                let pos = slots[i].pos;
                perceive(raw_ability(&c.attrs, pos, &w.data.weights) * familiarity_factor(c.familiarity[pos.idx()]), 3.0, Observer::Person(2_000_000 + state.0), p, 7000 + pos.idx() as u64)
            })
        })
        .collect();
    let score = |r: usize, c: usize| {
        let gk_slot: bool = slots[r].pos == Pos::GK;
        if gk_slot && !keeper[c] {
            return f32::NEG_INFINITY;
        }
        if !gk_slot && keeper[c] {
            return -50.0;
        }
        est[c][r]
    };
    let assign = hungarian::maximise(11, avail.len(), score);
    let xi: [PlayerId; 11] = std::array::from_fn(|r| avail[assign[r]]);
    let bench: smallvec::SmallVec<[PlayerId; 12]> = avail.iter().copied().filter(|p| !xi.contains(p)).take(9).collect();
    let ps = |p: PlayerId| -> PlayerSheet { crate::selection::player_sheet(w, p) };
    Some(TeamSheet {
        team: TeamId::NONE,
        tactics: Tactics { formation: 0, mentality: Mentality::from_level(0), tempo: 50, width: 50, directness: 50, line: 50, press: 50 },
        slots: slots as [Slot; 11],
        xi: std::array::from_fn(|i| ps(xi[i])),
        bench: bench.iter().map(|&p| ps(p)).collect(),
        manager_reactivity: 0.4,
    })
}

/// Daily: open the championship on 1 February, play its matches, close it when the final is done.
pub fn daily(w: &mut World) {
    if !w.ext.ecosystem.is_configured() {
        return;
    }
    let today = w.date;
    if w.ext.scenario.due(CalEvent::StateChampionship, today.month(), today.day()) && w.ext.ecosystem.tournament.is_none() {
        open(w);
    }
    let Some(t) = w.ext.ecosystem.tournament.as_ref() else { return };
    if !t.fixtures.iter().any(|f| f.0 == today) {
        if t.fixtures.is_empty() {
            advance(w);
        }
        return;
    }
    play(w);
    if w.ext.ecosystem.tournament.as_ref().is_some_and(|t| t.fixtures.is_empty()) {
        advance(w);
    }
}

/// The nation whose states play the championship.
fn home_nation(w: &World) -> Option<NationId> {
    w.nations.iter_enumerated().find(|(id, n)| !n.leagues.is_empty() && w.ext.ecosystem.regions.iter().any(|r| r.nation == *id) && n.code == "IND").map(|x| x.0).or_else(|| w.ext.ecosystem.regions.iter().next().map(|r| r.nation))
}

/// The next day the state selectors name their squads (the championship opens on the scenario's calendar), and the state that may
/// pick this player then, if any. Built only from the published calendar and the eligibility rules as they apply to the player.
pub fn selection_ahead(w: &World, p: PlayerId) -> Option<(RegionId, Date)> {
    if !w.ext.ecosystem.is_configured() || Some(w.person_of(p).nation) != home_nation(w) {
        return None;
    }
    let day = w.ext.scenario.next_day(CalEvent::StateChampionship, w.date)?;
    Some((crate::eligibility::own_state_on(w, p, day)?, day))
}

fn open(w: &mut World) {
    let year = w.date.year();
    let Some(india) = home_nation(w) else { return };
    let mut states: Vec<RegionId> = w.ext.ecosystem.assoc.keys().copied().collect();
    states.sort();
    let mut squads: Vec<(RegionId, Vec<PlayerId>)> = Vec::new();
    let mut taken: pw_world::FxHashSet<PlayerId> = Default::default();
    let mut picks: Vec<(RegionId, Vec<PlayerId>)> = Vec::new();
    for &s in &states {
        let sq = select_squad(w, s, india, year, false, &taken);
        taken.extend(sq.iter().copied());
        picks.push((s, sq));
    }
    // Second call: states that could not raise a side fill it from anyone else who qualifies and is free.
    for (s, sq) in &mut picks {
        if sq.len() < 22 {
            let more = select_squad(w, *s, india, year, true, &taken);
            for p in more {
                if sq.len() >= 22 {
                    break;
                }
                taken.insert(p);
                sq.push(p);
            }
        }
    }
    for (s, sq) in picks {
        if sq.len() >= 14 {
            squads.push((s, sq));
        }
    }
    if w.ext.ecosystem.eligibility.one_state_per_year {
        for (s, sq) in &squads {
            for &p in sq {
                w.ext.ecosystem.represented.insert(p, (year, *s));
            }
        }
    }
    if squads.len() < 4 {
        return;
    }
    // A chronicled player the selectors already knew (a district side this season, or the state side before) who is eligible for
    // their own state and is not in its squad hears it as a squad named without them.
    let mut left_out: Vec<(PlayerId, RegionId)> = Vec::new();
    for who in w.ext.chronicle.lives.keys() {
        let p = w.people[*who].player;
        if p.is_none() || taken.contains(&p) || w.people[*who].nation != india {
            continue;
        }
        let known = w.ext.ecosystem.route(p).iter().any(|x| (x.kind == StageKind::District && x.date.days_until(w.date) <= 200) || x.kind == StageKind::StateTeam);
        let own = crate::eligibility::state_grounds(w, p).first().map(|x| x.0);
        if let Some(state) = own.filter(|&s| known && squads.iter().any(|x| x.0 == s) && crate::eligibility::judge_state(w, p, s, year, false).eligible) {
            left_out.push((p, state));
        }
    }
    left_out.sort();
    for (p, state) in left_out {
        crate::chronicle::left_out(w, p, StageKind::StateTeam, state);
    }
    // The chosen are away from their clubs and universities for the tournament, and it goes on their route.
    for (s, sq) in &squads {
        for &p in sq {
            w.intl.duty.insert(p);
            crate::ecosystem::note(w, p, StageKind::StateTeam, s.0);
            crate::recognition::sighted_by(w, Org::State(*s), pw_core::PersonId::NONE, p, Learned::Came);
        }
    }
    let per = 4usize;
    let groups: Vec<Vec<usize>> = (0..squads.len()).collect::<Vec<_>>().chunks(per).map(|c| c.to_vec()).collect();
    let mut fixtures = Vec::new();
    let start = w.date.add_days(2);
    for g in &groups {
        let mut md = 0;
        for i in 0..g.len() {
            for j in i + 1..g.len() {
                fixtures.push((start.add_days((md % 3) * 2), g[i], g[j], false));
                md += 1;
            }
        }
    }
    let n = squads.len();
    w.ext.ecosystem.tournament = Some(Tournament {
        year,
        nation: india,
        squads,
        groups,
        fixtures,
        table: vec![(0, 0, 0, 0); n],
        alive: Vec::new(),
        stage: 0,
        winner: RegionId::NONE,
        scorers: Default::default(),
    });
}

fn play(w: &mut World) {
    let today = w.date;
    let t = w.ext.ecosystem.tournament.as_ref().unwrap();
    let todo: Vec<(usize, usize, bool)> = t.fixtures.iter().filter(|f| f.0 == today).map(|f| (f.1, f.2, f.3)).collect();
    let nation = t.nation;
    let year = t.year;
    let played: Vec<(usize, usize, bool, Option<pw_match::MatchResult>)> = {
        let world: &World = w;
        let t = world.ext.ecosystem.tournament.as_ref().unwrap();
        todo.iter()
            .map(|&(a, b, ko)| {
                let (sa, sb) = (&t.squads[a], &t.squads[b]);
                let (Some(ha), Some(hb)) = (sheet(world, &sa.1, sa.0), sheet(world, &sb.1, sb.0)) else { return (a, b, ko, None) };
                let uid = hash_key(&[u64::from(sa.0.0), u64::from(sb.0.0), year as u64, 0x5a17]);
                let input = MatchInput {
                    seed: hash_key(&[world.seed, stream::INTL, uid]),
                    home: ha,
                    away: hb,
                    neutral: true,
                    decisive: ko,
                    first_leg: None,
                    away_goals_rule: false,
                    // Between states that care about the result it matters more to the players.
                    importance: 0.75 + 0.2 * world.ext.ecosystem.rivalry_of(sa.0, sb.0) / 100.0,
                    referee_strictness: 1.0,
                    max_subs: 5,
                    lod: Lod::Standard,
                    tuning: &world.data.tuning.matches,
                };
                (a, b, ko, Some(simulate(&input)))
            })
            .collect()
    };
    if let Some(t) = w.ext.ecosystem.tournament.as_mut() {
        t.fixtures.retain(|f| f.0 != today);
    }
    for (a, b, ko, r) in played {
        let Some(r) = r else {
            // A side that cannot raise eleven forfeits.
            continue;
        };
        let (states, squads): ([RegionId; 2], [Vec<PlayerId>; 2]) = {
            let t = w.ext.ecosystem.tournament.as_ref().unwrap();
            ([t.squads[a].0, t.squads[b].0], [t.squads[a].1.clone(), t.squads[b].1.clone()])
        };
        let mut rng = pw_core::Rng::keyed(&[w.seed, stream::HEALTH, u64::from(states[0].0), u64::from(states[1].0), today.0 as u64]);
        for line in r.lines.iter().filter(|l| l.minutes > 0) {
            let p = line.player;
            let h = &mut w.players.hot[p];
            h.condition = line.condition_end;
            h.minutes_4w = h.minutes_4w.saturating_add(u16::from(line.minutes));
            h.push_rating(line.rating);
            h.sharpness = (f32::from(h.sharpness) + f32::from(line.minutes) / 90.0 * 12.0).min(100.0) as u8;
            // State football is the level that carries a name: it is real evidence, and it is watched.
            crate::recognition::credit(w, p, pw_world::ecosystem::Tier::State, line.rating, 1.0);
            if line.goals > 0
                && let Some(t) = w.ext.ecosystem.tournament.as_mut()
            {
                *t.scorers.entry(p).or_default() += u16::from(line.goals);
            }
            if line.injured {
                crate::health::match_injury(w, p, &mut rng, line.injury_noncontact);
            }
            // A big performance is seen: by clubs that are looking at this level of football.
            if line.rating >= 7.8 || line.goals >= 2 {
                notice(w, p, line.minutes);
            }
        }
        let ctx = crate::almanac::Ctx { comp: None, shared: vec![Scope::Event(nation, 1)], level: pw_world::minor::Level::State, pro: false };
        crate::almanac::report(w, &ctx, &r, [Scope::Region(states[0]), Scope::Region(states[1])], [&squads[0], &squads[1]], hash_key(&[u64::from(states[0].0), u64::from(states[1].0), today.0 as u64]));
        let t = w.ext.ecosystem.tournament.as_mut().unwrap();
        let (hg, ag) = (r.home_goals, r.away_goals);
        {
            let (sa, sb) = (t.squads[a].0, t.squads[b].0);
            let close = if hg.abs_diff(ag) <= 1 { 3.0 } else { 0.0 };
            // Neighbours' meetings weigh more: the memory of a derby is local. Nothing exists before the first meeting.
            let near = (1.0 - w.ext.ecosystem.travel_burden(sa, sb) / 0.25).max(0.0);
            let by = if ko { 4.0 } else { 2.0 } + close + 3.0 * near;
            w.ext.ecosystem.bump_rivalry(sa, sb, by);
        }
        let t = w.ext.ecosystem.tournament.as_mut().unwrap();
        if ko {
            let win = match r.winner() {
                Some(0) => a,
                Some(_) => b,
                None => if hash_key(&[u64::from(a as u32), u64::from(b as u32), today.0 as u64]) & 1 == 0 { a } else { b },
            };
            t.alive.retain(|&x| x == win || (x != a && x != b));
        } else {
            let mut row = |i: usize, f: u8, ag_: u8, pts: u8| {
                let e = &mut t.table[i];
                e.0 += 1;
                e.1 += pts;
                e.2 += u16::from(f);
                e.3 += u16::from(ag_);
            };
            let (pa, pb) = match hg.cmp(&ag) {
                std::cmp::Ordering::Greater => (3, 0),
                std::cmp::Ordering::Less => (0, 3),
                std::cmp::Ordering::Equal => (1, 1),
            };
            row(a, hg, ag, pa);
            row(b, ag, hg, pb);
        }
    }
}

/// Clubs that watch state football see a standout: each notices with a chance that grows with its scouting.
pub(crate) fn notice(w: &mut World, p: PlayerId, minutes: u8) {
    let today = w.date;
    let mut clubs: Vec<ClubId> = w.clubs.ids().collect();
    clubs.retain(|&c| {
        let t = tier_of(w, c);
        (2..=4).contains(&t) || t >= 5
    });
    for c in clubs {
        let scouts = w.clubs[c].staff.iter().filter(|&&s| w.staff[s].role == pw_world::StaffRole::Scout).count() as f32;
        // Standing out once at state level counts for less than doing it across a tournament.
        let sample = crate::recognition::games(w, p, pw_world::ecosystem::Tier::State);
        let prob = ((0.10 + 0.05 * scouts).min(0.5)) * (0.45 + 0.55 * sample / (sample + 2.0));
        if (hash_key(&[w.seed, u64::from(c.0), u64::from(p.0), today.0 as u64, 0x5a2]) % 1000) as f32 / 1000.0 < prob {
            w.knowledge.observe(c, p, u16::from(minutes), today);
            let by = crate::ecosystem::scout_of(w, c).map_or(pw_core::PersonId::NONE, |s| w.staff[s].person);
            crate::recognition::sighted_by(w, Org::Club(c), by, p, Learned::Watched);
        }
    }
}

/// A stage of the championship is done: seed the knockout, or crown the winner.
fn advance(w: &mut World) {
    let today = w.date;
    let t = w.ext.ecosystem.tournament.as_mut().unwrap();
    match t.stage {
        0 => {
            // Group winners and runners-up go through, in order of points, goal difference, goals.
            let mut through: Vec<usize> = Vec::new();
            for g in &t.groups {
                let mut order = g.clone();
                order.sort_by(|&x, &y| {
                    let (a, b) = (t.table[x], t.table[y]);
                    b.1.cmp(&a.1).then((i32::from(b.2) - i32::from(b.3)).cmp(&(i32::from(a.2) - i32::from(a.3)))).then(b.2.cmp(&a.2)).then(x.cmp(&y))
                });
                through.extend(order.into_iter().take(2));
            }
            t.alive = through;
            t.stage = 1;
            schedule_round(t, today.add_days(3));
        }
        _ => {
            if t.alive.len() <= 1 {
                let winner = t.alive.first().map_or(RegionId::NONE, |&i| t.squads[i].0);
                t.winner = winner;
                let (nation, year) = (t.nation, t.year);
                let squads = std::mem::take(&mut t.squads);
                let scorers = std::mem::take(&mut t.scorers);
                w.ext.ecosystem.tournament = None;
                for (_, sq) in &squads {
                    for p in sq {
                        w.intl.duty.remove(p);
                    }
                }
                finish(w, nation, year, winner, &scorers);
            } else {
                schedule_round(t, today.add_days(3));
            }
        }
    }
}

fn schedule_round(t: &mut Tournament, date: Date) {
    let alive = t.alive.clone();
    let mut i = 0;
    while i + 1 < alive.len() {
        t.fixtures.push((date, alive[i], alive[i + 1], true));
        i += 2;
    }
    // An odd side out goes straight through (`alive` keeps it).
    if alive.len() % 2 == 1 {
        // nothing to play for the last one
    }
}

fn finish(w: &mut World, nation: NationId, year: i32, winner: RegionId, scorers: &pw_world::FxHashMap<PlayerId, u16>) {
    // Foreign clubs watch the tournament's leading scorers and the winners' stars.
    {
        let mut top: Vec<(u16, PlayerId)> = scorers.iter().map(|(&p, &g)| (g, p)).collect();
        top.sort_by(|a, b| b.cmp(a));
        let pool: Vec<PlayerId> = top.into_iter().take(30).map(|x| x.1).collect();
        crate::export::eyes(w, pw_world::recog::Segment::League, pw_world::ecosystem::Tier::State, &pool);
    }
    let today = w.date;
    let event = 1u16;
    // Top scorer of the tournament.
    if let Some((&p, &g)) = scorers.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))) {
        let who = w.players.cold[p].person;
        let m = pw_world::records::Mark { holder: pw_world::records::Holder::Person(who), value: i64::from(g), date: today, against: None };
        crate::records::note(w, pw_world::records::RecordKey { scope: Scope::Event(nation, event), stat: pw_world::records::Stat::GoalsInSeason, level: pw_world::minor::Level::State }, m, 4, true);
        w.ext.almanac.offer(Scope::Event(nation, event), pw_world::records::Stat::GoalsInSeason, who, i64::from(g));
    }
    if winner.is_some() {
        let titles = {
            let k = (winner, year);
            w.ext.ecosystem.tournament_titles.push(k);
            w.ext.ecosystem.tournament_titles.iter().filter(|x| x.0 == winner).count() as i64
        };
        let m = pw_world::records::Mark { holder: pw_world::records::Holder::Region(winner), value: titles, date: today, against: None };
        crate::records::note(w, pw_world::records::RecordKey { scope: Scope::Event(nation, event), stat: pw_world::records::Stat::Titles, level: pw_world::minor::Level::State }, m, 2, true);
    }
}
