//! International football (07 §13).
//!
//! Every nation with enough eligible players fields a senior side and youth
//! sides. Each side has a manager — a real staff member hired by the
//! federation from the same pool clubs hire from — who names squads from his
//! own imperfect view: what he can see of a player depends on the stage the
//! player performs on, how often he has had him in camp, and his own
//! judgement; who he trusts depends on his memories of them. Squads meet in
//! the calendar's international windows for friendlies and qualifiers, and in
//! the summers for continental championships and the world tournament.
//!
//! Consequences flow outwards through the ordinary systems: caps and results
//! move reputations (player, nation, manager); matches are watched by club
//! scouts with the right briefs; travel and minutes load the body; injuries on
//! duty sour club managers on national managers; dual nationals choose (with
//! their own mind, or at the keyboard); veterans retire from international
//! football; federations sack managers who fail at tournaments.
//!
//! Assumptions (documented in docs/WORLD_SYSTEMS.md):
//! - One confederation per nation, taken from the nation data.
//! - Continental finals in years ≡ 0 (mod 4), the world tournament in years
//!   ≡ 2 (mod 4); qualifying in the Sep–Nov windows of the year before.
//! - Youth sides play friendlies only (no youth tournaments yet).
//! - A competitive senior cap locks a player to that nation.

use pw_core::rng::{Rng, hash_key, stream};
use pw_core::{Attr, ClubId, Date, Mentality, NationId, PersonId, PlayerId, Pos, PosGroup, Slot, StaffAttr, StaffId, Tactics, TeamId};
use pw_match::{Lod, MatchInput, MatchResult, PlayerSheet, TeamSheet, simulate};
use pw_world::calendar::span;
use pw_world::decision::{Decision, DecisionKind, MindKind};
use pw_world::event::{EventKind, Visibility};
use pw_world::intl::{Cap, IntlFixture, IntlLine, IntlMatch, Level, MatchKind, NationalSide, Stage, Standing, Tournament, TournamentKind};
use pw_world::knowledge::{Observer, Seen, perceive, sigma};
use pw_world::nation::Confed;
use pw_world::player::{familiarity_factor, raw_ability};
use pw_world::scouting::Brief;
use pw_world::staff::ManagerRecord;
use pw_world::{Archetype, FxHashMap, FxHashSet, Intent, MemoryKind, Person, Philosophy, PlayerStatus, Staff, StaffRole, World};
use rayon::prelude::*;
use smallvec::SmallVec;

use crate::consider;
use crate::generate as gen_;
use crate::hungarian;

/// Eligible players a nation needs before it fields a side at a level.
const MIN_POOL: usize = 18;

// ---------------------------------------------------------------------------
// Eligibility
// ---------------------------------------------------------------------------

/// Nations a player can currently be picked by.
pub fn eligible_nations(w: &World, p: PlayerId) -> SmallVec<[NationId; 2]> {
    let mut v = SmallVec::new();
    if let Some(n) = w.intl.locked_to(p).or_else(|| w.intl.declared.get(&p).copied()) {
        v.push(n);
        return v;
    }
    let person = &w.people[w.players.cold[p].person];
    if person.nation.is_some() {
        v.push(person.nation);
    }
    if person.nation2.is_some() && person.nation2 != person.nation {
        v.push(person.nation2);
    }
    v
}

fn age_ok(w: &World, p: PlayerId, level: Level) -> bool {
    let dob = w.people[w.players.cold[p].person].dob;
    let jan1 = Date::from_ymd(w.date.year(), 1, 1);
    let age = dob.age_on(jan1);
    match level.max_age() {
        None => w.age(p) >= 16,
        Some(max) => age < max && w.age(p) >= max.saturating_sub(4),
    }
}

/// Players each nation could call on, built once per window.
fn pools(w: &World) -> FxHashMap<NationId, Vec<PlayerId>> {
    let mut m: FxHashMap<NationId, Vec<PlayerId>> = FxHashMap::default();
    for p in w.players.ids() {
        if !matches!(w.players.hot[p].status, PlayerStatus::Active | PlayerStatus::FreeAgent) {
            continue;
        }
        for n in eligible_nations(w, p) {
            m.entry(n).or_default().push(p);
        }
    }
    m
}

// ---------------------------------------------------------------------------
// Federations and their managers
// ---------------------------------------------------------------------------

/// Create sides for nations that can field them; replace managers who have
/// retired or been hired by clubs. Runs at start and monthly.
pub fn ensure(w: &mut World) {
    let today = w.date;
    let pools = pools(w);
    let nations: Vec<NationId> = w.nations.ids().collect();
    for n in nations {
        for level in Level::ALL {
            let key = (n, level);
            let count = pools.get(&n).map_or(0, |v| v.iter().filter(|&&p| age_ok(w, p, level)).count());
            if count < MIN_POOL {
                continue;
            }
            match w.intl.sides.get(&key).map(|s| s.manager) {
                None => {
                    let m = appoint(w, n, level);
                    w.intl.sides.insert(
                        key,
                        NationalSide { nation: n, level, manager: m, squad: Vec::new(), captain: PlayerId::NONE, selected: Date(0), since: today, streak: FxHashMap::default(), record: (0, 0, 0) },
                    );
                }
                Some(m) if manager_gone(w, m) => {
                    w.intl.managers.remove(&m);
                    w.events.push(today, Visibility::Public, EventKind::NationalManagerLeft { staff: m, nation: n, level, sacked: false });
                    replace(w, key);
                }
                Some(m) if w.staff[m].contract_end < today => {
                    // Contract up: the federation renews on a decent record.
                    let (won, drawn, lost) = w.intl.sides[&key].record;
                    let ratio = (f32::from(won) + 0.5 * f32::from(drawn)) / f32::from((won + drawn + lost).max(1));
                    if ratio >= 0.45 {
                        w.staff[m].contract_end = today.add_months(24);
                    } else {
                        w.intl.managers.remove(&m);
                        w.events.push(today, Visibility::Public, EventKind::NationalManagerLeft { staff: m, nation: n, level, sacked: false });
                        replace(w, key);
                    }
                }
                Some(_) => {}
            }
        }
    }
}

fn manager_gone(w: &World, m: StaffId) -> bool {
    m.is_none() || w.staff[m].retired || w.staff[m].club.is_some()
}

fn replace(w: &mut World, key: (NationId, Level)) {
    let m = appoint(w, key.0, key.1);
    if let Some(s) = w.intl.sides.get_mut(&key) {
        s.manager = m;
        s.since = w.date;
        s.record = (0, 0, 0);
    }
}

/// Federations hire from the same unemployed pool clubs do, strongly
/// preferring their own nationals; a newly qualified coach if nobody fits.
fn appoint(w: &mut World, n: NationId, level: Level) -> StaffId {
    let today = w.date;
    let nrep = f32::from(w.nations[n].reputation);
    let target = nrep
        * match level {
            Level::Senior => 1.0,
            Level::U21 => 0.5,
            Level::U19 => 0.4,
            Level::U17 => 0.3,
        };
    let mut rng = Rng::keyed(&[w.seed, stream::INTL, u64::from(n.0), level as u64, today.0 as u64]);
    let best = w
        .staff
        .iter_enumerated()
        .filter(|(id, s)| !s.employed() && !s.retired && !w.intl.managers.contains(id))
        .filter(|(_, s)| match level {
            Level::Senior => s.role == StaffRole::Manager,
            _ => matches!(s.role, StaffRole::Manager | StaffRole::Assistant | StaffRole::Coach | StaffRole::HeadOfYouth),
        })
        .filter(|(_, s)| consider::age(w, s.person) < 70.0)
        .map(|(id, s)| {
            let national = if w.people[s.person].nation == n { 1.2 } else { 0.0 };
            let fit = -((f32::from(s.reputation) - target).abs() / 2000.0);
            let skill = s.role_rating(StaffRole::Manager) / 20.0;
            let youth = if level == Level::Senior { 0.0 } else { s.attrs.f(StaffAttr::Youngsters) / 20.0 * 0.5 };
            (id, national + fit + skill + youth + rng.normal() * 0.05)
        })
        .filter(|&(_, score)| score > 0.9)
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|(id, _)| id);
    let m = best.unwrap_or_else(|| new_coach(w, n, target, &mut rng));
    let econ = w.nations[n].economy;
    let s = &mut w.staff[m];
    s.role = StaffRole::Manager;
    s.joined = today;
    s.contract_end = today.add_months(if level == Level::Senior { 24 } else { 36 });
    s.wage = (target * 2.0 * econ) as i64;
    w.intl.managers.insert(m);
    w.events.push(today, Visibility::Public, EventKind::NationalManagerAppointed { staff: m, nation: n, level });
    m
}

fn new_coach(w: &mut World, n: NationId, target_rep: f32, rng: &mut Rng) -> StaffId {
    let (first, last) = crate::people::random_name(w, n, rng);
    let dob = w.date.add_days(-(365 * rng.range_i32(38, 60)));
    let person = w.people.push(Person {
        first,
        last,
        common: Default::default(),
        dob,
        nation: n,
        nation2: Default::default(),
        hidden: gen_::hidden_random(rng),
        player: Default::default(),
        staff: Default::default(),
        mind: MindKind::Ai,
    });
    let level = 7.0 + target_rep / 1200.0;
    let mut attrs = pw_core::StaffAttrs::default();
    for a in StaffAttr::ALL {
        attrs.set(a, rng.normal_ms(level, 2.5).round().clamp(1.0, 20.0) as u8);
    }
    let nf = w.data.formations.len().max(1) as u32;
    let (press, tempo, directness) = crate::culture::fashion(w, n, rng.range_i32(30, 70) as u8, rng.range_i32(35, 65) as u8, rng.range_i32(25, 75) as u8);
    let phil = Philosophy {
        formations: [rng.below(nf) as u8, rng.below(nf) as u8],
        mentality: rng.range_i32(-1, 1) as i8,
        press,
        tempo,
        directness,
        youth_trust: rng.range_i32(20, 80) as u8,
        archetype: [Archetype::Pragmatist, Archetype::Developer, Archetype::Rotator, Archetype::Loyalist][rng.index(4)],
    };
    let id = w.staff.push(Staff {
        person,
        role: StaffRole::Manager,
        club: ClubId::NONE,
        attrs,
        wage: 0,
        contract_end: w.date,
        reputation: (target_rep * 0.5) as u16,
        philosophy: phil,
        joined: w.date,
        record: ManagerRecord::default(),
        retired: false,
    });
    w.people[person].staff = id;
    id
}

/// What a national manager believes a player's ability is. Evidence comes
/// from caps with this nation, the stage the player plays on (a big club is
/// seen by everyone; a small foreign club by nobody) and domestic proximity.
fn fed_view(w: &World, m: StaffId, n: NationId, p: PlayerId) -> f32 {
    let c = &w.players.cold[p];
    let h = &w.players.hot[p];
    let t = &w.data.tuning.perception;
    let judging = w.staff[m].attrs.f(StaffAttr::JudgingAbility);
    let club_rep = if h.club.is_some() { f32::from(w.clubs[h.club].reputation) } else { 0.0 };
    let domestic = h.club.is_some() && w.clubs[h.club].nation == n;
    let capped: u16 = w.intl.caps.get(&p).map_or(0, |v| v.iter().filter(|x| x.nation == n).map(|x| x.caps).sum());
    let minutes = (f32::from(capped) * 90.0 + club_rep / 10_000.0 * 1500.0 + if domestic { 700.0 } else { 0.0 }).min(60_000.0) as u16;
    let s = sigma(t, Some(Seen { minutes, last: w.date }), judging, w.date, c.rep.world >= t.famous_reputation);
    perceive(f32::from(c.ca), s * 6.0, Observer::Person(w.staff[m].person.0), p, 3000 + u64::from(n.0)).clamp(1.0, 200.0)
}

// ---------------------------------------------------------------------------
// Daily driver
// ---------------------------------------------------------------------------

pub fn daily(w: &mut World) {
    let today = w.date;
    if today.month() == 8 && today.day() == 1 {
        plan_tournaments(w);
    }
    if today.month() == 12 && today.day() == 1 {
        resolve_qualifying(w);
    }
    let windows: Vec<(Date, Date)> = w.data.international_windows.iter().map(|&s| span(today.year(), s)).collect();
    for &(a, b) in &windows {
        if today == a {
            open_window(w, a, b);
        }
    }
    play_today(w);
    advance_tournaments(w);
    for &(_, b) in &windows {
        if today == b.add_days(1) {
            close_window(w, false);
        }
    }
}

// ---------------------------------------------------------------------------
// Windows and squads
// ---------------------------------------------------------------------------

fn open_window(w: &mut World, a: Date, b: Date) {
    w.intl.last_window = a;
    // Finals summer: the drawn tournament starts in this window.
    let finals_now: Vec<usize> = w.intl.tournaments.iter().enumerate().filter(|(_, t)| t.stage == Stage::Drawn && t.start >= a && t.start <= b.add_days(10)).map(|(i, _)| i).collect();
    for &i in &finals_now {
        w.intl.tournaments[i].stage = Stage::Groups;
    }
    let finalists: FxHashSet<NationId> = finals_now.iter().flat_map(|&i| w.intl.tournaments[i].finalists.iter().copied()).collect();

    let pools = pools(w);
    let mut keys: Vec<(NationId, Level)> = w.intl.sides.keys().copied().collect();
    keys.sort_by(|x, y| x.1.cmp(&y.1).then(x.0.cmp(&y.0)));
    let mut taken: FxHashSet<PlayerId> = FxHashSet::default();
    for key in keys {
        let m = w.intl.sides[&key].manager;
        if manager_gone(w, m) {
            continue;
        }
        let size = if key.1 == Level::Senior && finalists.contains(&key.0) { 26 } else { 23 };
        let pool = pools.get(&key.0).map_or(&[][..], |v| v.as_slice());
        call_up(w, key, pool, size, &mut taken);
    }
    schedule_friendlies(w, a, b);
}

/// Name a squad for one side.
fn call_up(w: &mut World, key: (NationId, Level), pool: &[PlayerId], size: usize, taken: &mut FxHashSet<PlayerId>) {
    let today = w.date;
    let (n, level) = key;
    let m = w.intl.sides[&key].manager;
    let mp = w.staff[m].person;
    let arch = w.staff[m].philosophy.archetype;
    let prev: Vec<PlayerId> = w.intl.sides[&key].squad.clone();
    let mut cands: Vec<(f32, PlayerId, PosGroup)> = pool
        .iter()
        .copied()
        .filter(|p| !taken.contains(p) && age_ok(w, *p, level))
        .filter(|&p| !(level == Level::Senior && w.intl.retired.contains(&p)))
        .filter(|&p| level == Level::Senior || w.intl.senior_caps(p) < 5)
        .filter(|&p| w.players.hot[p].injury_days <= 10)
        .map(|p| {
            let h = &w.players.hot[p];
            let c = &w.players.cold[p];
            let est = fed_view(w, m, n, p);
            let form = h.form_avg().map_or(0.0, |f| (f - 6.6) * 4.0);
            let caps = f32::from(w.intl.caps_for(p, n, level)).min(40.0);
            let loyal = if arch == Archetype::Loyalist { caps * 0.25 } else { caps * 0.08 };
            let youth = if arch == Archetype::Developer && w.age_years(p) < 23.0 { 4.0 } else { 0.0 };
            let streak = f32::from(w.intl.sides[&key].streak.get(&p).copied().unwrap_or(0)).min(6.0);
            // His own memories of the player: rows, refusals, apologies.
            let trust = (consider::trust(w, mp, c.person) - 0.5) * 12.0;
            let stage = if h.club.is_some() { f32::from(w.clubs[h.club].reputation) / 10_000.0 * 4.0 } else { -4.0 };
            let knock = if h.injury_days > 0 { -6.0 } else { 0.0 };
            (est + form + loyal + youth + streak + trust + stage + knock, p, c.best_pos.group())
        })
        .collect();
    cands.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)));
    let quota = |g: PosGroup| -> usize {
        let big = size > 23;
        match g {
            PosGroup::Gk => 3,
            PosGroup::Def => {
                if big {
                    9
                } else {
                    8
                }
            }
            PosGroup::Mid => {
                if big {
                    9
                } else {
                    8
                }
            }
            PosGroup::Att => {
                if big {
                    5
                } else {
                    4
                }
            }
        }
    };
    let mut squad: Vec<PlayerId> = Vec::with_capacity(size);
    let mut counts = [0usize; 4];
    let gi = |g: PosGroup| match g {
        PosGroup::Gk => 0,
        PosGroup::Def => 1,
        PosGroup::Mid => 2,
        PosGroup::Att => 3,
    };
    for &(_, p, g) in &cands {
        if squad.len() >= size {
            break;
        }
        if counts[gi(g)] < quota(g) {
            counts[gi(g)] += 1;
            squad.push(p);
        }
    }
    for &(_, p, g) in &cands {
        if squad.len() >= size {
            break;
        }
        if g != PosGroup::Gk && !squad.contains(&p) {
            squad.push(p);
        }
    }

    // Club pressure: a player carrying a knock whose club manager objects may
    // be withdrawn; the national manager remembers who pulled him out.
    //
    // How hard he can push depends on what the window is worth: a friendly is the club's
    // to refuse, a qualifier is the country's unless the body is plainly not ready.
    let stakes = w.intl.fixtures.iter().filter(|f| f.level == level && (f.home == n || f.away == n)).map(|f| importance(f.kind)).fold(0.3f32, f32::max);
    let competitive = stakes >= 0.7;
    let mut withdrawn: Vec<(PlayerId, pw_core::EventId)> = Vec::new();
    for &p in &squad {
        let h = &w.players.hot[p];
        if h.injury_days == 0 && h.condition >= 80 {
            continue;
        }
        let left = f32::from(h.injury_days) / f32::from(h.injury_total.max(1));
        // The club's objection: an unfinished injury above all, then plain tiredness.
        let objection = (left * 1.2 + (1.0 - f32::from(h.condition) / 100.0) * 0.8).clamp(0.0, 1.0);
        let Some(club_mgr) = w.manager_of_player(p) else { continue };
        let roll = (hash_key(&[w.seed, stream::INTL, u64::from(p.0), today.0 as u64]) % 1000) as f32 / 1000.0;
        let out = left > 0.5 || roll < objection * if competitive { 0.35 } else { 0.9 };
        // Only what mattered is remembered: a competitive window, or someone medically out.
        let mut ev = pw_core::EventId::NONE;
        if competitive || left > 0.5 {
            let club_person = club_mgr;
            let mut stances: smallvec::SmallVec<[pw_world::ruling::Stance; 4]> = smallvec::SmallVec::new();
            stances.push(pw_world::ruling::Stance { who: mp, role: pw_world::ruling::StanceRole::Federation, believed_pct: 255, backing: (stakes * 100.0) as i8, authority: competitive && left <= 0.5 });
            stances.push(pw_world::ruling::Stance { who: club_person, role: pw_world::ruling::StanceRole::Manager, believed_pct: (left * 100.0) as u8, backing: -(objection * 100.0) as i8, authority: !competitive || left > 0.5 });
            let id = w.ext.decisions.add(pw_world::ruling::Ruling {
                id: 0,
                kind: pw_world::ruling::RulingKind::ReleaseForCountry,
                date: today,
                club: w.players.hot[p].club,
                subject: p,
                about: pw_core::PersonId::NONE,
                liability: 0,
                decider: if out { club_person } else { mp },
                stances,
                true_pct: (left * 100.0) as u8,
                want: (stakes * 100.0) as u8,
                outcome: pw_world::ruling::Outcome::Enacted,
                resolved: Some(today),
                event: pw_core::EventId::NONE,
            });
            ev = w.events.push(today, Visibility::Club(w.players.hot[p].club), EventKind::Ruling { ruling: id });
            if let Some(r) = w.ext.decisions.get_mut(id) {
                r.event = ev;
            }
        }
        if out {
            withdrawn.push((p, ev));
        }
    }
    for &(p, cause) in &withdrawn {
        squad.retain(|&x| x != p);
        let causes: pw_world::Causes = if cause.is_some() { pw_world::causes![pw_world::Cause::Event(cause)] } else { pw_world::Causes::new() };
        let ev = w.events.push_caused(today, Visibility::Public, EventKind::WithdrewFromSquad { player: p, nation: n }, causes);
        if let Some(cm) = w.manager_of_player(p) {
            let compat = consider::compat(w, mp, cm);
            w.social.remember(mp, cm, MemoryKind::LetDown, today, ev, false, 0.4, compat);
        }
    }

    // Dual nationals who have not committed are asked before a senior cap.
    if level == Level::Senior {
        let undecided: Vec<(PlayerId, NationId)> = squad
            .iter()
            .copied()
            .filter(|&p| w.intl.locked_to(p).is_none() && !w.intl.declared.contains_key(&p))
            .filter_map(|p| {
                let e = eligible_nations(w, p);
                (e.len() == 2).then(|| (p, if e[0] == n { e[1] } else { e[0] }))
            })
            .collect();
        for (p, other) in undecided {
            ask_allegiance(w, p, n, other);
        }
        // An AI who refused has already left the squad.
        squad.retain(|p| w.intl.declared.get(p).is_none_or(|&d| d == n));
    }

    // Consequences of being picked or left out.
    let mut streak: FxHashMap<PlayerId, u8> = FxHashMap::default();
    for &p in &squad {
        let prior = w.intl.sides[&key].streak.get(&p).copied().unwrap_or(0);
        streak.insert(p, prior.saturating_add(1));
        taken.insert(p);
        w.intl.duty.insert(p);
        let first = w.intl.caps_for(p, n, level) == 0 && prior == 0;
        if level == Level::Senior || first {
            w.events.push(today, Visibility::Public, EventKind::NationalSquad { player: p, nation: n, level });
        }
        let h = &mut w.players.hot[p];
        h.morale = (h.morale + if level == Level::Senior { 6 } else { 3 }).min(100);
    }
    for p in prev {
        if squad.contains(&p) {
            continue;
        }
        let run = w.intl.sides[&key].streak.get(&p).copied().unwrap_or(0);
        if run >= 3 && w.players.hot[p].status == PlayerStatus::Active && !w.intl.retired.contains(&p) {
            let who = w.players.cold[p].person;
            let compat = consider::compat(w, who, mp);
            w.social.remember(who, mp, MemoryKind::Dropped, today, pw_core::EventId::NONE, true, 0.6, compat);
            let h = &mut w.players.hot[p];
            h.morale = h.morale.saturating_sub(5);
        }
    }
    let captain = squad
        .iter()
        .copied()
        .max_by(|&x, &y| {
            let s = |p: PlayerId| f32::from(w.intl.caps_for(p, n, level)).min(80.0) * 0.1 + w.players.cold[p].attrs.get(Attr::Leadership);
            s(x).total_cmp(&s(y)).then(y.cmp(&x))
        })
        .unwrap_or(PlayerId::NONE);
    let side = w.intl.sides.get_mut(&key).expect("side exists");
    side.squad = squad;
    side.captain = captain;
    side.selected = today;
    side.streak = streak;
}

/// A dual national called up by `n` who could also play for `other`.
fn ask_allegiance(w: &mut World, p: PlayerId, n: NationId, other: NationId) {
    let today = w.date;
    let who = w.players.cold[p].person;
    let pick = preferred_nation(w, p, n, other);
    if w.people[who].mind != MindKind::External {
        answer_call(w, p, pick, n);
        return;
    }
    if w.intl.asking.contains_key(&p) {
        return;
    }
    w.intl.asking.insert(p, (n, other));
    let kind = DecisionKind::NationChoice { nation: n, other };
    let options = kind.simple_options();
    w.decisions.push(Decision { person: who, player: p, kind, options, created: today, deadline: today.add_days(2), default: if pick == n { 0 } else { 1 }, answer: None, resolved: false });
}

/// How a player weighs two countries: stature, their chance of playing,
/// where they were born and how loyal they are, and what they have already
/// played for at youth level.
fn preferred_nation(w: &World, p: PlayerId, a: NationId, b: NationId) -> NationId {
    let who = w.players.cold[p].person;
    let birth = w.people[who].nation;
    let loyal = consider::hid(w, who, pw_core::Hidden::Loyalty) / 20.0;
    let ambition = consider::hid(w, who, pw_core::Hidden::Ambition) / 20.0;
    let ca = f32::from(w.players.cold[p].ca);
    let score = |n: NationId| -> f32 {
        let stature = f32::from(w.nations[n].reputation) / 10_000.0;
        // A strong country's squad is harder to get into.
        let chance = (1.0 - (stature * 170.0 - ca) / 60.0).clamp(0.05, 1.0);
        let youth: u16 = w.intl.caps.get(&p).map_or(0, |v| v.iter().filter(|c| c.nation == n).map(|c| c.caps).sum());
        stature * (0.3 + 0.5 * ambition) + chance * 0.6 + if n == birth { 0.15 + 0.2 * loyal } else { 0.0 } + f32::from(youth.min(20)) * 0.01
    };
    let coin = crate::decisions::coin(w, p, 0x1a7) * 0.05;
    if score(a) + coin >= score(b) { a } else { b }
}

/// Apply a dual national's choice (from their mind or their keyboard).
pub fn answer_call(w: &mut World, p: PlayerId, chosen: NationId, calling: NationId) {
    let today = w.date;
    w.intl.asking.remove(&p);
    if w.intl.locked_to(p).is_some() {
        return;
    }
    w.intl.declared.insert(p, chosen);
    let ev = w.events.push(today, Visibility::Public, EventKind::ChoseNation { player: p, nation: chosen });
    if chosen != calling {
        withdraw(w, p, calling);
        if let Some(side) = w.intl.sides.get(&(calling, Level::Senior)) {
            let mp = w.staff[side.manager].person;
            let who = w.players.cold[p].person;
            let compat = consider::compat(w, mp, who);
            w.social.remember(mp, who, MemoryKind::Refused, today, ev, true, 0.8, compat);
        }
    }
}

/// A player commits to a nation on their own initiative (intent).
pub fn declare(w: &mut World, p: PlayerId, n: NationId) {
    if !eligible_nations(w, p).contains(&n) || w.intl.locked_to(p).is_some() {
        return;
    }
    let calling = w.intl.asking.get(&p).map_or(n, |&(c, _)| c);
    answer_call(w, p, n, calling);
    // Leave any other squad they are in.
    let others: Vec<NationId> = w.intl.sides.values().filter(|s| s.nation != n && s.squad.contains(&p)).map(|s| s.nation).collect();
    for o in others {
        withdraw(w, p, o);
    }
}

/// Retire from international football (intent; AI veterans submit it too).
pub fn retire(w: &mut World, p: PlayerId) {
    if !w.intl.retired.insert(p) {
        return;
    }
    let nation = w.intl.locked_to(p).or_else(|| w.intl.declared.get(&p).copied()).unwrap_or(w.people[w.players.cold[p].person].nation);
    w.events.push(w.date, Visibility::Public, EventKind::RetiredFromInternational { player: p, nation });
    let sides: Vec<NationId> = w.intl.sides.values().filter(|s| s.squad.contains(&p)).map(|s| s.nation).collect();
    for n in sides {
        withdraw(w, p, n);
    }
}

fn withdraw(w: &mut World, p: PlayerId, n: NationId) {
    let mut was = false;
    for level in Level::ALL {
        if let Some(s) = w.intl.sides.get_mut(&(n, level))
            && s.squad.contains(&p)
        {
            s.squad.retain(|&x| x != p);
            was = true;
        }
    }
    if was {
        w.intl.duty.remove(&p);
        w.events.push(w.date, Visibility::Public, EventKind::WithdrewFromSquad { player: p, nation: n });
    }
}

/// Pair sides without a fixture this window for friendlies, mostly within
/// their confederation and against opponents of similar standing.
fn schedule_friendlies(w: &mut World, a: Date, b: Date) {
    let busy: FxHashSet<(NationId, Level)> = w.intl.fixtures.iter().filter(|f| f.date >= a && f.date <= b.add_days(40)).flat_map(|f| [(f.home, f.level), (f.away, f.level)]).collect();
    let long_window = matches!(a.month(), 3 | 6);
    for level in Level::ALL {
        let mut free: Vec<(f32, NationId)> = w
            .intl
            .sides
            .values()
            .filter(|s| s.level == level && !busy.contains(&(s.nation, level)) && s.squad.len() >= 14)
            .map(|s| {
                let jitter = (hash_key(&[w.seed, stream::INTL, u64::from(s.nation.0), a.0 as u64]) % 1500) as f32;
                let confed = w.nations[s.nation].confed as u8 as f32 * 20_000.0;
                (confed + f32::from(w.nations[s.nation].reputation) + jitter, s.nation)
            })
            .collect();
        free.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
        let dates: &[i32] = if level == Level::Senior && long_window {
            &[2, 6]
        } else if level == Level::Senior {
            &[3]
        } else {
            &[4]
        };
        for (k, &off) in dates.iter().enumerate() {
            // Second friendly: shift the pairing so opponents differ.
            let order: Vec<NationId> = if k == 0 { free.iter().map(|x| x.1).collect() } else { free.iter().skip(1).chain(free.iter().take(1)).map(|x| x.1).collect() };
            for pair in order.chunks(2) {
                if let &[x, y] = pair {
                    let flip = hash_key(&[w.seed, u64::from(x.0), u64::from(y.0), a.0 as u64]).is_multiple_of(2);
                    let (home, away) = if flip { (x, y) } else { (y, x) };
                    w.intl.fixtures.push(IntlFixture { date: a.add_days(off), level, home, away, kind: MatchKind::Friendly, neutral: false });
                }
            }
        }
    }
}

/// Players go back to their clubs: tired, sometimes carrying knocks, having
/// crossed the world or not.
fn close_window(w: &mut World, all: bool) {
    let keys: Vec<(NationId, Level)> = w.intl.sides.keys().copied().collect();
    for key in keys {
        if !all && key.1 == Level::Senior && w.intl.at_finals(key.0) {
            continue;
        }
        let squad = std::mem::take(&mut w.intl.sides.get_mut(&key).expect("side").squad);
        release(w, key.0, &squad);
    }
}

fn release(w: &mut World, n: NationId, squad: &[PlayerId]) {
    let confed = w.nations[n].confed;
    for &p in squad {
        w.intl.duty.remove(&p);
        let club = w.players.hot[p].club;
        let far = club.is_some() && w.nations[w.clubs[club].nation].confed != confed;
        let h = &mut w.players.hot[p];
        let hit = if far { 8 } else { 3 };
        h.condition = h.condition.saturating_sub(hit).max(40);
        if far {
            h.acute += 120.0;
        }
    }
}

// ---------------------------------------------------------------------------
// Matches
// ---------------------------------------------------------------------------

struct Picked {
    slots: [Slot; 11],
    xi: [PlayerId; 11],
    bench: SmallVec<[PlayerId; 12]>,
    tactics: Tactics,
    reactivity: f32,
}

/// The national manager picks eleven from the squad he named.
fn pick(w: &World, n: NationId, level: Level, importance: f32) -> Option<Picked> {
    let side = w.intl.sides.get(&(n, level))?;
    let m = side.manager;
    let mp = w.staff[m].person;
    let phil = w.staff[m].philosophy;
    let avail: Vec<PlayerId> = side
        .squad
        .iter()
        .copied()
        .filter(|&p| {
            let h = &w.players.hot[p];
            h.injury == 0 && matches!(h.status, PlayerStatus::Active | PlayerStatus::FreeAgent) && !w.intl.asking.contains_key(&p)
        })
        .collect();
    if avail.len() < 11 {
        return None;
    }
    let f = usize::from(phil.formations[0]).min(w.data.formations.len().saturating_sub(1));
    let slots = w.data.formations[f].slots;
    let est: Vec<[f32; 11]> = avail
        .iter()
        .map(|&p| {
            let c = &w.players.cold[p];
            let h = &w.players.hot[p];
            let fit = f32::from(h.condition) / 100.0;
            std::array::from_fn(|i| {
                let pos: Pos = slots[i].pos;
                let truth = raw_ability(&c.attrs, pos, &w.data.weights) * familiarity_factor(c.familiarity[pos.idx()]);
                perceive(truth, 3.0, Observer::Person(mp.0), p, 4000 + pos.idx() as u64) * (0.75 + 0.25 * fit)
            })
        })
        .collect();
    let keeper: Vec<bool> = avail.iter().map(|&p| w.players.cold[p].familiarity[Pos::GK.idx()] >= 12).collect();
    let trust: Vec<f32> = avail.iter().map(|&p| (consider::trust(w, mp, w.players.cold[p].person) - 0.5) * 6.0).collect();
    let rotation = 1.0 - importance;
    let score = |r: usize, c: usize| -> f32 {
        let gk_slot = slots[r].pos == Pos::GK;
        if gk_slot && !keeper[c] {
            return f32::NEG_INFINITY;
        }
        if !gk_slot && keeper[c] {
            return -50.0;
        }
        est[c][r] + trust[c] - rotation * f32::from(w.intl.caps_for(avail[c], n, level).min(30)) * 0.1
    };
    let assign = hungarian::maximise(11, avail.len(), score);
    let xi: [PlayerId; 11] = std::array::from_fn(|r| avail[assign[r]]);
    let mut rest: Vec<(f32, PlayerId)> =
        avail.iter().enumerate().filter(|(_, p)| !xi.contains(p)).map(|(c, &p)| ((0..11).map(|r| score(r, c)).filter(|s| s.is_finite()).fold(-99.0, f32::max), p)).collect();
    rest.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)));
    let bench: SmallVec<[PlayerId; 12]> = rest.into_iter().take(12).map(|x| x.1).collect();
    let tactics = Tactics { formation: f as u8, mentality: Mentality::from_level(i32::from(phil.mentality)), tempo: phil.tempo, width: 50, directness: phil.directness, line: 50, press: phil.press };
    let reactivity = match phil.archetype {
        Archetype::Rotator => 0.7,
        Archetype::Developer => 0.5,
        Archetype::Pragmatist => 0.4,
        Archetype::Loyalist => 0.25,
    };
    Some(Picked { slots, xi, bench, tactics, reactivity })
}

fn sheet(w: &World, pk: &Picked) -> TeamSheet {
    // A camp of a few days: nothing like a club's drilling.
    let drill = 0.85 + 0.15 * crate::training::CAMP_DRILL;
    let ps = |p: PlayerId| -> PlayerSheet {
        let mut s = crate::selection::player_sheet(w, p);
        s.sharpness *= drill;
        s
    };
    TeamSheet {
        team: TeamId::NONE,
        tactics: pk.tactics,
        slots: pk.slots,
        xi: std::array::from_fn(|i| ps(pk.xi[i])),
        bench: pk.bench.iter().map(|&p| ps(p)).collect(),
        manager_reactivity: pk.reactivity,
    }
}

fn importance(kind: MatchKind) -> f32 {
    match kind {
        MatchKind::Friendly => 0.3,
        MatchKind::Qualifier { .. } => 0.7,
        MatchKind::Group { .. } => 0.85,
        MatchKind::Knockout { .. } => 1.0,
    }
}

enum Played {
    Match(IntlFixture, Box<MatchResult>),
    /// One side could not raise eleven: competitive games are forfeited,
    /// friendlies are called off.
    Forfeit(IntlFixture, bool),
    Cancelled,
}

fn play_today(w: &mut World) {
    let today = w.date;
    let (todo, rest): (Vec<IntlFixture>, Vec<IntlFixture>) = std::mem::take(&mut w.intl.fixtures).into_iter().partition(|f| f.date == today);
    w.intl.fixtures = rest;
    if todo.is_empty() {
        return;
    }
    let watched: FxHashSet<PlayerId> = w.people.iter().filter(|p| p.mind == MindKind::External && p.player.is_some()).map(|p| p.player).collect();
    let world: &World = w;
    let played: Vec<Played> = todo
        .par_iter()
        .map(|&fx| {
            let imp = importance(fx.kind);
            let (h, a) = (pick(world, fx.home, fx.level, imp), pick(world, fx.away, fx.level, imp));
            let (h, a) = match (h, a) {
                (Some(h), Some(a)) => (h, a),
                (None, _) if fx.kind.competitive() => return Played::Forfeit(fx, true),
                (_, None) if fx.kind.competitive() => return Played::Forfeit(fx, false),
                _ => return Played::Cancelled,
            };
            let seen = h.xi.iter().chain(a.xi.iter()).chain(h.bench.iter()).chain(a.bench.iter()).any(|p| watched.contains(p));
            let uid = hash_key(&[u64::from(fx.home.0), u64::from(fx.away.0), fx.date.0 as u64, fx.level as u64]);
            let input = MatchInput {
                seed: hash_key(&[world.seed, stream::INTL, uid]),
                home: sheet(world, &h),
                away: sheet(world, &a),
                neutral: fx.neutral,
                decisive: matches!(fx.kind, MatchKind::Knockout { .. }),
                first_leg: None,
                away_goals_rule: false,
                importance: imp,
                referee_strictness: 0.8 + 0.4 * (uid % 1000) as f32 / 1000.0,
                max_subs: if fx.kind.competitive() { 5 } else { 6 },
                lod: if seen { Lod::Full } else { Lod::Standard },
                tuning: &world.data.tuning.matches,
            };
            Played::Match(fx, Box::new(simulate(&input)))
        })
        .collect();
    for p in played {
        match p {
            Played::Match(fx, r) => apply(w, fx, *r),
            Played::Forfeit(fx, home_forfeits) => {
                let (hg, ag) = if home_forfeits { (0, 3) } else { (3, 0) };
                record(w, fx, hg, ag, None, Vec::new());
            }
            Played::Cancelled => {}
        }
    }
}

fn apply(w: &mut World, fx: IntlFixture, r: MatchResult) {
    let today = w.date;
    let nations = [fx.home, fx.away];
    let mut rng = Rng::keyed(&[w.seed, stream::HEALTH, u64::from(fx.home.0), u64::from(fx.away.0), today.0 as u64]);
    let sign = [i32::from(r.home_goals).cmp(&i32::from(r.away_goals)) as i32, i32::from(r.away_goals).cmp(&i32::from(r.home_goals)) as i32];
    let competitive = fx.kind.competitive();
    let imp = importance(fx.kind);
    let mut lines = Vec::with_capacity(r.lines.len());
    for line in &r.lines {
        if line.minutes == 0 {
            continue;
        }
        let p = line.player;
        let side = usize::from(line.side);
        let n = nations[side];
        lines.push(IntlLine { player: p, minutes: line.minutes, rating: line.rating, goals: line.goals });
        {
            let h = &mut w.players.hot[p];
            h.condition = line.condition_end;
            h.sharpness = (f32::from(h.sharpness) + f32::from(line.minutes) / 90.0 * 12.0).min(100.0) as u8;
            h.minutes_4w = h.minutes_4w.saturating_add(u16::from(line.minutes));
            h.minutes_week = h.minutes_week.saturating_add(u16::from(line.minutes));
            h.push_rating(line.rating);
            let load = f32::from(line.minutes) * 9.0;
            h.acute += 0.25 * load;
            h.chronic += 0.069 * load;
            let mood = (line.rating - 6.7) * 2.5 + sign[side] as f32 * 2.0 * imp;
            h.morale = (f32::from(h.morale) + mood).clamp(5.0, 100.0) as u8;
            h.confidence = (f32::from(h.confidence) + (line.rating - 6.7) * 4.0).clamp(5.0, 100.0) as u8;
        }
        cap(w, p, n, fx.level, competitive, line.goals);
        // A good night for your country is seen far beyond your league.
        if fx.level == Level::Senior {
            let c = &mut w.players.cold[p];
            let bump = ((line.rating - 6.5) * 60.0 * imp + 20.0).max(0.0);
            c.rep.world = (f32::from(c.rep.world) + bump).min(10_000.0) as u16;
            c.rep.home = (f32::from(c.rep.home) + bump * 1.5).min(10_000.0) as u16;
        }
        if line.injured {
            crate::health::match_injury(w, p, &mut rng, line.injury_noncontact);
            // The club manager does not forget whose game broke his player.
            if let (Some(cm), Some(side)) = (w.manager_of_player(p), w.intl.sides.get(&(n, fx.level))) {
                let nm = w.staff[side.manager].person;
                let star = w.players.cold[p].status == pw_world::SquadStatus::Star;
                let compat = consider::compat(w, cm, nm);
                w.social.remember(cm, nm, MemoryKind::LetDown, today, pw_core::EventId::NONE, false, if star { 1.0 } else { 0.5 }, compat);
            }
        }
    }
    record(w, fx, r.home_goals, r.away_goals, r.pens, lines);
}

/// Record a cap. A competitive senior cap ties the player to the nation.
fn cap(w: &mut World, p: PlayerId, n: NationId, level: Level, competitive: bool, goals: u8) {
    let today = w.date;
    let entry = w.intl.caps.entry(p).or_default();
    let debut = !entry.iter().any(|c| c.nation == n && c.level == level);
    if debut {
        entry.push(Cap { nation: n, level, caps: 0, goals: 0, competitive: 0, debut: today, last: today });
    }
    let was_locked = entry.iter().any(|c| c.level == Level::Senior && c.competitive > 0);
    let c = entry.iter_mut().find(|c| c.nation == n && c.level == level).expect("cap entry");
    c.caps += 1;
    c.goals += u16::from(goals);
    c.last = today;
    if competitive {
        c.competitive += 1;
    }
    if level == Level::Senior {
        let pc = &mut w.players.cold[p];
        pc.caps = pc.caps.saturating_add(1);
        pc.intl_goals = pc.intl_goals.saturating_add(u16::from(goals));
        crate::honours::on_cap(w, p, n);
    }
    if debut {
        let ev = w.events.push(today, Visibility::Public, EventKind::InternationalDebut { player: p, nation: n, level });
        if let Some(side) = w.intl.sides.get(&(n, level)) {
            let who = w.players.cold[p].person;
            let mp = w.staff[side.manager].person;
            let compat = consider::compat(w, who, mp);
            w.social.remember(who, mp, MemoryKind::GaveChance, today, ev, true, 0.8, compat);
        }
    }
    if level == Level::Senior && competitive && !was_locked {
        // Committed now, whether or not they said so.
        w.intl.declared.insert(p, n);
        if w.people[w.players.cold[p].person].nation2.is_some() {
            w.events.push(today, Visibility::Public, EventKind::ChoseNation { player: p, nation: n });
        }
    }
}

fn record(w: &mut World, fx: IntlFixture, hg: u8, ag: u8, pens: Option<(u8, u8)>, lines: Vec<IntlLine>) {
    let today = w.date;
    let index = w.intl.matches.len() as u32;
    w.intl.matches.push(IntlMatch { date: today, level: fx.level, home: fx.home, away: fx.away, kind: fx.kind, home_goals: hg, away_goals: ag, pens, lines });
    if fx.level == Level::Senior || fx.kind.competitive() {
        w.events.push(today, Visibility::Public, EventKind::InternationalResult { index });
    }
    // Manager records.
    for (n, gf, ga) in [(fx.home, hg, ag), (fx.away, ag, hg)] {
        if let Some(s) = w.intl.sides.get_mut(&(n, fx.level)) {
            if fx.kind.competitive() {
                match gf.cmp(&ga) {
                    std::cmp::Ordering::Greater => s.record.0 += 1,
                    std::cmp::Ordering::Equal => s.record.1 += 1,
                    std::cmp::Ordering::Less => s.record.2 += 1,
                }
            }
            let m = s.manager;
            let rec = &mut w.staff[m].record;
            rec.games += 1;
            match gf.cmp(&ga) {
                std::cmp::Ordering::Greater => rec.wins += 1,
                std::cmp::Ordering::Equal => rec.draws += 1,
                std::cmp::Ordering::Less => rec.losses += 1,
            }
        }
    }
    // Group tables.
    match fx.kind {
        MatchKind::Qualifier { tournament } | MatchKind::Group { tournament } => {
            let qual = matches!(fx.kind, MatchKind::Qualifier { .. });
            if let Some(t) = w.intl.tournaments.get_mut(tournament as usize) {
                let groups = if qual { &mut t.qual_groups } else { &mut t.groups };
                for g in groups.iter_mut() {
                    for (n, st) in g.iter_mut() {
                        let (gf, ga) = if *n == fx.home {
                            (hg, ag)
                        } else if *n == fx.away {
                            (ag, hg)
                        } else {
                            continue;
                        };
                        st.played += 1;
                        st.gf += gf;
                        st.ga += ga;
                        st.points += match gf.cmp(&ga) {
                            std::cmp::Ordering::Greater => 3,
                            std::cmp::Ordering::Equal => 1,
                            std::cmp::Ordering::Less => 0,
                        };
                    }
                }
            }
        }
        _ => {}
    }
    if fx.level == Level::Senior {
        elo(w, fx.home, fx.away, hg, ag, pens, fx.kind);
    }
    expose(w, index as usize);
}

/// National strength moves with results (an Elo-style update on the 0–10,000
/// scale). Work permits, allegiance choices and manager appointments read it.
fn elo(w: &mut World, home: NationId, away: NationId, hg: u8, ag: u8, pens: Option<(u8, u8)>, kind: MatchKind) {
    let ra = f32::from(w.nations[home].reputation);
    let rb = f32::from(w.nations[away].reputation);
    let ea = 1.0 / (1.0 + pw_core::math::powf(10.0, (rb - ra) / 2000.0));
    let sa = match hg.cmp(&ag) {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Less => 0.0,
        std::cmp::Ordering::Equal => pens.map_or(0.5, |(h, a)| if h > a { 0.6 } else { 0.4 }),
    };
    let k = match kind {
        MatchKind::Friendly => 40.0,
        MatchKind::Qualifier { .. } => 80.0,
        MatchKind::Group { .. } => 120.0,
        MatchKind::Knockout { .. } => 160.0,
    };
    let margin = 1.0 + (f32::from(hg.abs_diff(ag)).min(4.0) - 1.0).max(0.0) * 0.25;
    let d = k * margin * (sa - ea);
    w.nations[home].reputation = (ra + d).clamp(200.0, 10_000.0) as u16;
    w.nations[away].reputation = (rb - d).clamp(200.0, 10_000.0) as u16;
}

/// Club scouts briefed on either nation watch; finals are on television, so
/// the big clubs see them too.
fn expose(w: &mut World, index: usize) {
    let today = w.date;
    let m = &w.intl.matches[index];
    let (home, away) = (m.home, m.away);
    let finals = matches!(m.kind, MatchKind::Group { .. } | MatchKind::Knockout { .. });
    let youth_level = m.level != Level::Senior;
    let lines: Vec<(PlayerId, u8)> = m.lines.iter().map(|l| (l.player, l.minutes)).collect();
    let mut watchers: Vec<ClubId> = w
        .scouting
        .assignments
        .iter()
        .filter(|a| a.until >= today)
        .filter(|a| match a.brief {
            Brief::Nation(n) => (n == home || n == away) && !youth_level,
            Brief::Youth(n) => (n == home || n == away) && youth_level,
            Brief::Player(p) => lines.iter().any(|l| l.0 == p),
            _ => false,
        })
        .map(|a| a.club)
        .collect();
    if finals {
        watchers.extend(w.clubs.iter_enumerated().filter(|(_, c)| c.reputation >= 7000).map(|(id, _)| id));
    }
    watchers.sort();
    watchers.dedup();
    for club in watchers {
        for &(p, mins) in &lines {
            w.knowledge.observe(club, p, u16::from(mins) / 2, today);
        }
    }
}

// ---------------------------------------------------------------------------
// Tournaments
// ---------------------------------------------------------------------------

/// On 1 August, plan next summer's tournament and its qualifying groups.
fn plan_tournaments(w: &mut World) {
    let year = w.date.year() + 1;
    let kinds: Vec<TournamentKind> = match year.rem_euclid(4) {
        0 => Confed::ALL.iter().map(|&c| TournamentKind::Continental(c)).collect(),
        2 => vec![TournamentKind::World],
        _ => return,
    };
    let seniors: Vec<NationId> = {
        let mut v: Vec<NationId> = w.intl.sides.keys().filter(|k| k.1 == Level::Senior).map(|k| k.0).collect();
        v.sort();
        v
    };
    for kind in kinds {
        let entrants: Vec<NationId> = seniors.iter().copied().filter(|&n| matches!(kind, TournamentKind::World) || TournamentKind::Continental(w.nations[n].confed) == kind).collect();
        if entrants.len() < 4 {
            continue;
        }
        let finals = match kind {
            TournamentKind::World => {
                if seniors.len() >= 64 {
                    32
                } else if seniors.len() >= 24 {
                    16
                } else {
                    8
                }
            }
            TournamentKind::Continental(_) => {
                if entrants.len() >= 32 {
                    16
                } else if entrants.len() >= 12 {
                    8
                } else {
                    4
                }
            }
        };
        let slots = allocate(w, &entrants, finals, kind);
        let id = w.intl.tournaments.len() as u32;
        let mut t = Tournament {
            id,
            kind,
            year,
            stage: Stage::Qualifying,
            qual_groups: Vec::new(),
            groups: Vec::new(),
            bracket: Vec::new(),
            finalists: Vec::new(),
            slots,
            round: 0,
            start: Date(0),
            winner: NationId::NONE,
            runner_up: NationId::NONE,
            best_player: PlayerId::NONE,
        };
        // Qualifying groups per confederation, seeded by strength.
        let dates = qualifier_dates(w, year - 1);
        for &(confed, slot) in &t.slots.clone() {
            let mut field: Vec<NationId> = entrants.iter().copied().filter(|&n| w.nations[n].confed == confed).collect();
            if field.len() <= usize::from(slot) {
                continue; // everyone qualifies
            }
            field.sort_by(|&a, &b| w.nations[b].reputation.cmp(&w.nations[a].reputation).then(a.cmp(&b)));
            let n_groups = field.len().div_ceil(4);
            let mut groups: Vec<Vec<(NationId, Standing)>> = vec![Vec::new(); n_groups];
            for (i, &n) in field.iter().enumerate() {
                let row = i / n_groups;
                let g = if row.is_multiple_of(2) { i % n_groups } else { n_groups - 1 - i % n_groups };
                groups[g].push((n, Standing::default()));
            }
            for g in &groups {
                let rounds = crate::schedule::round_robin(g.len(), 2);
                for (r, round) in rounds.iter().enumerate() {
                    let Some(&date) = dates.get(r) else { break };
                    for &(h, a) in round {
                        w.intl.fixtures.push(IntlFixture { date, level: Level::Senior, home: g[h].0, away: g[a].0, kind: MatchKind::Qualifier { tournament: id }, neutral: false });
                    }
                }
            }
            t.qual_groups.extend(groups);
        }
        w.intl.tournaments.push(t);
    }
}

/// Qualifying slots per confederation, proportional to entrants (the largest
/// remainder method), at least one each.
fn allocate(w: &World, entrants: &[NationId], finals: usize, kind: TournamentKind) -> Vec<(Confed, u8)> {
    if let TournamentKind::Continental(c) = kind {
        return vec![(c, finals as u8)];
    }
    let counts: Vec<(Confed, usize)> = Confed::ALL.iter().map(|&c| (c, entrants.iter().filter(|&&n| w.nations[n].confed == c).count())).filter(|x| x.1 > 0).collect();
    let total: usize = counts.iter().map(|x| x.1).sum();
    let mut out: Vec<(Confed, u8, f32)> = counts
        .iter()
        .map(|&(c, k)| {
            let exact = finals as f32 * k as f32 / total.max(1) as f32;
            (c, (exact.floor() as u8).max(1), exact.fract())
        })
        .collect();
    let mut given: usize = out.iter().map(|x| usize::from(x.1)).sum();
    out.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
    let mut i = 0;
    while given < finals && !out.is_empty() {
        let k = i % out.len();
        out[k].1 += 1;
        given += 1;
        i += 1;
    }
    while given > finals {
        if let Some(x) = out.iter_mut().filter(|x| x.1 > 1).min_by(|a, b| a.2.total_cmp(&b.2)) {
            x.1 -= 1;
            given -= 1;
        } else {
            break;
        }
    }
    out.into_iter().map(|(c, s, _)| (c, s)).collect()
}

/// Two matchdays in each autumn window.
fn qualifier_dates(w: &World, year: i32) -> Vec<Date> {
    let mut v: Vec<Date> = Vec::new();
    for &s in &w.data.international_windows {
        let (a, _) = span(year, s);
        if a.month() >= 9 {
            v.push(a.add_days(2));
            v.push(a.add_days(5));
        }
    }
    v.sort();
    v
}

/// On 1 December: qualifying is over, finalists are drawn into groups.
fn resolve_qualifying(w: &mut World) {
    let year = w.date.year() + 1;
    let june = w.data.international_windows.iter().map(|&s| span(year, s)).find(|(a, _)| a.month() == 6).map_or(Date::from_ymd(year, 6, 11), |x| x.1);
    let start = june.add_days(3);
    for ti in 0..w.intl.tournaments.len() {
        if w.intl.tournaments[ti].stage != Stage::Qualifying || w.intl.tournaments[ti].year != year {
            continue;
        }
        let t = w.intl.tournaments[ti].clone();
        let mut finalists: Vec<NationId> = Vec::new();
        for &(confed, slot) in &t.slots {
            let groups: Vec<&Vec<(NationId, Standing)>> = t.qual_groups.iter().filter(|g| g.first().is_some_and(|x| w.nations[x.0].confed == confed)).collect();
            if groups.is_empty() {
                // No qualifying needed: every side of the confederation goes.
                let mut all: Vec<NationId> = w.intl.sides.keys().filter(|k| k.1 == Level::Senior && w.nations[k.0].confed == confed).map(|k| k.0).collect();
                all.sort_by(|&a, &b| w.nations[b].reputation.cmp(&w.nations[a].reputation).then(a.cmp(&b)));
                finalists.extend(all.into_iter().take(usize::from(slot)));
                continue;
            }
            let mut ranked: Vec<(usize, u8, i16, u8, u16, NationId)> = Vec::new();
            for g in groups {
                let mut rows = g.clone();
                rows.sort_by(|a, b| {
                    b.1.points
                        .cmp(&a.1.points)
                        .then((i16::from(b.1.gf) - i16::from(b.1.ga)).cmp(&(i16::from(a.1.gf) - i16::from(a.1.ga))))
                        .then(b.1.gf.cmp(&a.1.gf))
                        .then(w.nations[b.0].reputation.cmp(&w.nations[a.0].reputation))
                });
                for (pos, (n, st)) in rows.into_iter().enumerate() {
                    ranked.push((pos, st.points, i16::from(st.gf) - i16::from(st.ga), st.gf, w.nations[n].reputation, n));
                }
            }
            ranked.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)).then(b.2.cmp(&a.2)).then(b.3.cmp(&a.3)).then(b.4.cmp(&a.4)));
            finalists.extend(ranked.into_iter().take(usize::from(slot)).map(|x| x.5));
        }
        finalists.sort_by(|&a, &b| w.nations[b].reputation.cmp(&w.nations[a].reputation).then(a.cmp(&b)));
        let n_groups = (finalists.len() / 4).max(1);
        let mut groups: Vec<Vec<(NationId, Standing)>> = vec![Vec::new(); n_groups];
        let mut rng = Rng::keyed(&[w.seed, stream::DRAW, u64::from(t.id), 0x1a]);
        for pot in finalists.chunks(n_groups) {
            let mut pot: Vec<NationId> = pot.to_vec();
            rng.shuffle(&mut pot);
            for (g, n) in pot.into_iter().enumerate() {
                groups[g].push((n, Standing::default()));
            }
        }
        for g in &groups {
            for (r, round) in crate::schedule::round_robin(g.len(), 1).iter().enumerate() {
                let date = start.add_days(4 * r as i32);
                for &(h, a) in round {
                    w.intl.fixtures.push(IntlFixture { date, level: Level::Senior, home: g[h].0, away: g[a].0, kind: MatchKind::Group { tournament: t.id }, neutral: true });
                }
            }
        }
        let tm = &mut w.intl.tournaments[ti];
        tm.finalists = finalists;
        tm.groups = groups;
        tm.start = start;
        tm.stage = Stage::Drawn;
    }
}

/// Move group stages into knockouts and knockouts to their end once every
/// fixture of the stage is played.
fn advance_tournaments(w: &mut World) {
    let today = w.date;
    for ti in 0..w.intl.tournaments.len() {
        let t = &w.intl.tournaments[ti];
        let id = t.id;
        let pending = |w: &World| w.intl.fixtures.iter().any(|f| matches!(f.kind, MatchKind::Group { tournament } | MatchKind::Knockout { tournament, .. } if tournament == id));
        match t.stage {
            Stage::Groups if !pending(w) && t.start <= today => {
                let mut bracket: Vec<NationId> = Vec::new();
                let tables: Vec<Vec<NationId>> = t
                    .groups
                    .iter()
                    .map(|g| {
                        let mut rows = g.clone();
                        rows.sort_by(|a, b| {
                            b.1.points.cmp(&a.1.points).then((i16::from(b.1.gf) - i16::from(b.1.ga)).cmp(&(i16::from(a.1.gf) - i16::from(a.1.ga)))).then(b.1.gf.cmp(&a.1.gf)).then(a.0.cmp(&b.0))
                        });
                        rows.into_iter().map(|r| r.0).collect()
                    })
                    .collect();
                // Eliminated sides go home.
                let out: Vec<NationId> = tables.iter().flat_map(|g| g.iter().skip(2).copied()).collect();
                if tables.len() == 1 {
                    bracket.extend(tables[0].iter().take(2));
                } else {
                    for pair in tables.chunks(2) {
                        if let [a, b] = pair {
                            bracket.extend([a[0], b[1], b[0], a[1]]);
                        } else {
                            bracket.extend(pair[0].iter().take(2));
                        }
                    }
                }
                let tm = &mut w.intl.tournaments[ti];
                tm.bracket = bracket;
                tm.stage = Stage::Knockout;
                tm.round = 0;
                for n in out {
                    go_home(w, n, 0.0);
                }
                schedule_round(w, ti, today.add_days(4));
            }
            Stage::Knockout if !pending(w) => {
                let round = t.round;
                let bracket = t.bracket.clone();
                let mut next: Vec<NationId> = Vec::new();
                let mut losers: Vec<NationId> = Vec::new();
                for pair in bracket.chunks(2) {
                    let &[a, b] = pair else {
                        next.extend(pair);
                        continue;
                    };
                    let win = knockout_winner(w, id, round, a, b);
                    next.push(win);
                    losers.push(if win == a { b } else { a });
                }
                let reached = f32::from(round) + 1.0;
                if next.len() == 1 {
                    let tm = &mut w.intl.tournaments[ti];
                    tm.winner = next[0];
                    tm.runner_up = losers.first().copied().unwrap_or(NationId::NONE);
                    tm.bracket = next;
                    tm.stage = Stage::Done;
                    for n in losers {
                        go_home(w, n, reached);
                    }
                    finish(w, ti, reached + 1.0);
                } else {
                    let tm = &mut w.intl.tournaments[ti];
                    tm.bracket = next;
                    tm.round += 1;
                    for n in losers {
                        go_home(w, n, reached);
                    }
                    schedule_round(w, ti, today.add_days(4));
                }
            }
            _ => {}
        }
    }
}

fn schedule_round(w: &mut World, ti: usize, date: Date) {
    let t = &w.intl.tournaments[ti];
    let (id, round) = (t.id, t.round);
    let pairs: Vec<(NationId, NationId)> = t.bracket.chunks(2).filter_map(|p| if let &[a, b] = p { Some((a, b)) } else { None }).collect();
    for (a, b) in pairs {
        w.intl.fixtures.push(IntlFixture { date, level: Level::Senior, home: a, away: b, kind: MatchKind::Knockout { tournament: id, round }, neutral: true });
    }
}

fn knockout_winner(w: &World, id: u32, round: u8, a: NationId, b: NationId) -> NationId {
    let m = w.intl.matches.iter().rev().find(|m| m.kind == MatchKind::Knockout { tournament: id, round } && ((m.home == a && m.away == b) || (m.home == b && m.away == a)));
    let Some(m) = m else { return a };
    let home_wins = match m.home_goals.cmp(&m.away_goals) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => m.pens.is_none_or(|(h, x)| h > x),
    };
    if home_wins { m.home } else { m.away }
}

/// A nation's tournament is over; `reached` counts knockout rounds survived
/// (0 = out in the groups).
fn go_home(w: &mut World, n: NationId, reached: f32) {
    if let Some(s) = w.intl.sides.get_mut(&(n, Level::Senior)) {
        let squad = std::mem::take(&mut s.squad);
        release(w, n, &squad);
        judge_manager(w, n, reached);
        veterans_consider(w, &squad);
    }
}

/// The trophy: reputations, shared memories, a manager made.
fn finish(w: &mut World, ti: usize, reached: f32) {
    let today = w.date;
    let t = w.intl.tournaments[ti].clone();
    let win = t.winner;
    let ev = w.events.push(today, Visibility::Public, EventKind::TournamentWon { nation: win, tournament: t.id });
    // Best player of the finals by average rating (3+ appearances).
    let mut sums: FxHashMap<PlayerId, (f32, u8)> = FxHashMap::default();
    for m in w.intl.matches.iter().rev().take_while(|m| m.date >= t.start) {
        if matches!(m.kind, MatchKind::Group { tournament } | MatchKind::Knockout { tournament, .. } if tournament == t.id) {
            for l in &m.lines {
                let e = sums.entry(l.player).or_insert((0.0, 0));
                e.0 += l.rating;
                e.1 += 1;
            }
        }
    }
    let best = sums.iter().filter(|(_, v)| v.1 >= 3).max_by(|a, b| (a.1.0 / f32::from(a.1.1)).total_cmp(&(b.1.0 / f32::from(b.1.1))).then(b.0.cmp(a.0))).map(|(&p, _)| p);
    w.intl.tournaments[ti].best_player = best.unwrap_or(PlayerId::NONE);
    if let Some(b) = best {
        let c = &mut w.players.cold[b];
        c.rep.world = c.rep.world.saturating_add(600).min(10_000);
    }
    let bonus = if matches!(t.kind, TournamentKind::World) { 900 } else { 500 };
    w.nations[win].reputation = w.nations[win].reputation.saturating_add(bonus / 2).min(10_000);
    let squad = w.intl.sides.get(&(win, Level::Senior)).map(|s| s.squad.clone()).unwrap_or_default();
    for &p in &squad {
        let c = &mut w.players.cold[p];
        c.rep.world = c.rep.world.saturating_add(bonus).min(10_000);
        c.rep.home = c.rep.home.saturating_add(bonus * 2).min(10_000);
        let h = &mut w.players.hot[p];
        h.morale = (h.morale + 15).min(100);
    }
    // A shared summer binds people.
    let people: Vec<PersonId> = squad.iter().map(|&p| w.players.cold[p].person).collect();
    for &a in &people {
        for &b in &people {
            if a != b {
                let compat = consider::compat(w, a, b);
                w.social.remember(a, b, MemoryKind::Celebrated, today, ev, true, 0.7, compat);
            }
        }
    }
    if let Some(s) = w.intl.sides.get(&(win, Level::Senior)) {
        let m = s.manager;
        w.staff[m].reputation = w.staff[m].reputation.saturating_add(1500).min(10_000);
        w.staff[m].record.trophies += 1;
    }
    go_home(w, win, reached);
    close_window(w, true);
}

/// After a tournament exit the federation weighs its manager against what the
/// nation's standing promised.
fn judge_manager(w: &mut World, n: NationId, reached: f32) {
    let today = w.date;
    let Some(t) = w.intl.tournaments.iter().rev().find(|t| t.finalists.contains(&n) && t.year == today.year()) else { return };
    let seed = t.finalists.iter().position(|&x| x == n).unwrap_or(0) as f32 / t.finalists.len().max(1) as f32;
    let total_rounds = (t.finalists.len() as f32 / 2.0).log2().max(1.0);
    let expected = (1.0 - seed) * total_rounds;
    let shortfall = expected - reached;
    let key = (n, Level::Senior);
    let Some(side) = w.intl.sides.get(&key) else { return };
    let m = side.manager;
    let tenure = side.since.days_until(today) as f32 / 365.0;
    let r = (hash_key(&[w.seed, stream::INTL, u64::from(n.0), today.year() as u64]) % 1000) as f32 / 1000.0;
    if shortfall > 1.2 && r < 0.4 + 0.2 * shortfall - 0.05 * tenure {
        w.intl.managers.remove(&m);
        w.staff[m].record.sackings += 1;
        w.staff[m].reputation = w.staff[m].reputation.saturating_sub(600);
        w.events.push(today, Visibility::Public, EventKind::NationalManagerLeft { staff: m, nation: n, level: Level::Senior, sacked: true });
        replace(w, key);
    } else if shortfall < -0.8 {
        w.staff[m].reputation = w.staff[m].reputation.saturating_add(500).min(10_000);
    }
}

/// Veterans reflect after a tournament. AI minds decide here; a human decides
/// whenever they like through the same intent.
fn veterans_consider(w: &mut World, squad: &[PlayerId]) {
    let today = w.date;
    for &p in squad {
        let who = w.players.cold[p].person;
        if w.people[who].mind != MindKind::Ai {
            continue;
        }
        let age = w.age_years(p);
        let caps = w.intl.senior_caps(p);
        if age < 31.0 || caps < 30 {
            continue;
        }
        let tired = 1.0 - f32::from(w.players.hot[p].condition) / 100.0;
        let club_first = if w.players.cold[p].status == pw_world::SquadStatus::Star { 0.1 } else { 0.0 };
        let chance = (0.15 + (age - 31.0) * 0.08 + tired * 0.2 + club_first).min(0.9);
        let roll = (hash_key(&[w.seed, stream::INTL, u64::from(p.0), today.year() as u64, 0x7e7]) % 1000) as f32 / 1000.0;
        if roll < chance {
            w.intents.submit(who, Intent::RetireFromInternational, today);
        }
    }
}

// ---------------------------------------------------------------------------
// Queries used by other systems
// ---------------------------------------------------------------------------

/// Senior international standing of a player, 0–1: caps and recency. Used by
/// valuations, work permits and media.
pub fn standing(w: &World, p: PlayerId) -> f32 {
    let Some(v) = w.intl.caps.get(&p) else { return 0.0 };
    let senior: Vec<&Cap> = v.iter().filter(|c| c.level == Level::Senior).collect();
    let caps: u16 = senior.iter().map(|c| c.caps).sum();
    let recent = senior.iter().map(|c| c.last).max().is_some_and(|d| d.days_until(w.date) < 400);
    (f32::from(caps.min(60)) / 60.0) * if recent { 1.0 } else { 0.6 }
}
