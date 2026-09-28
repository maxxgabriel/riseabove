//! Votes with ballots, halls of fame, and the chronicle. See
//! `pw_world::awards`.

use pw_core::rng::stream;
use pw_core::{ClubId, CompId, NationId, PersonId, PlayerId};
use pw_world::awards::{Ballot, Cast, Entry, Feat, FirstTo, Hall, HallScope, Member, Vote, VoterKind, Why};
use pw_world::event::{AwardKind, EventKind, MilestoneKind, Visibility};
use pw_world::history::AwardRecord;
use pw_world::minor::Level as MLevel;
use pw_world::stats::StatLine;
use pw_world::{CompKind, FanReason, FxHashMap, PlayerStatus, TeamKind, World};
use smallvec::SmallVec;

const WHYS: usize = 8;

fn wi(w: Why) -> usize {
    match w {
        Why::Performances => 0,
        Why::Trophies => 1,
        Why::Fame => 2,
        Why::Goals => 3,
        Why::Familiarity => 4,
        Why::Longevity => 5,
        Why::Loyalty => 6,
        Why::International => 7,
    }
}

const ALL_WHY: [Why; WHYS] = [Why::Performances, Why::Trophies, Why::Fame, Why::Goals, Why::Familiarity, Why::Longevity, Why::Loyalty, Why::International];

/// Someone who can be voted for, with the components voters weigh.
#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub person: PersonId,
    pub parts: [f32; WHYS],
    /// Their (sporting) nation and club, for eligibility and familiarity.
    pub nation: NationId,
    pub club: ClubId,
    /// Nation of the league they played in (familiarity).
    pub seen_in: NationId,
}

/// Someone who votes, with their lens.
#[derive(Clone, Copy, Debug)]
pub struct Voter {
    pub person: PersonId,
    pub kind: VoterKind,
    pub weights: [f32; WHYS],
    pub nation: NationId,
    pub club: ClubId,
    /// May not vote for their own nation's / club's candidates.
    pub not_own_nation: bool,
    pub not_own_club: bool,
}

fn lens(kind: VoterKind) -> [f32; WHYS] {
    //        perf trophies fame goals famil  long  loyal intl
    match kind {
        VoterKind::NationalManager => [1.0, 0.4, 0.1, 0.3, 0.2, 0.0, 0.0, 0.3],
        VoterKind::Journalist => [0.6, 0.35, 0.9, 0.5, 0.5, 0.1, 0.1, 0.3],
        VoterKind::Player => [1.0, 0.2, 0.2, 0.3, 0.3, 0.0, 0.0, 0.0],
        VoterKind::Committee => [0.5, 0.6, 0.4, 0.4, 0.1, 0.6, 0.6, 0.6],
    }
}

/// Run a vote: each voter ranks up to `picks` candidates through their own
/// lens (with a little personal noise), 5-3-1 points (or one point per
/// pick for committees). Every ballot is kept with the reason for each pick.
pub fn run(w: &mut World, ballot: Ballot, voters: &[Voter], candidates: &[Candidate], picks: usize) -> Option<u32> {
    if voters.is_empty() || candidates.is_empty() {
        return None;
    }
    let today = w.date;
    let year = today.year();
    let mut casts = Vec::with_capacity(voters.len());
    let mut tally: FxHashMap<PersonId, u32> = FxHashMap::default();
    let bkey = pw_core::rng::hash_key(&[format_ballot(ballot), year as u64]);
    for v in voters {
        let mut scored: SmallVec<[(f32, PersonId, Why); 32]> = SmallVec::new();
        for c in candidates {
            if c.person == v.person || (v.not_own_nation && c.nation == v.nation && v.nation.is_some()) || (v.not_own_club && c.club == v.club && v.club.is_some()) {
                continue;
            }
            let mut parts = c.parts;
            if c.seen_in == v.nation && v.nation.is_some() {
                parts[wi(Why::Familiarity)] = 1.0;
            }
            let mut total = 0.0f32;
            let mut best = (0.0f32, Why::Performances);
            for (i, &why) in ALL_WHY.iter().enumerate() {
                let x = parts[i] * v.weights[i];
                total += x;
                if x > best.0 {
                    best = (x, why);
                }
            }
            let noise = pw_core::rng::noise(&[w.seed, stream::AWARDS, bkey, u64::from(v.person.0), u64::from(c.person.0)]) * 0.3;
            scored.push((total + noise, c.person, best.1));
        }
        scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let points: &[u32] = if v.kind == VoterKind::Committee { &[1, 1, 1, 1, 1] } else { &[5, 3, 1] };
        let chosen: SmallVec<[(PersonId, Why); 3]> = scored.iter().take(picks.min(points.len()).min(3)).map(|x| (x.1, x.2)).collect();
        for (i, &(p, _)) in chosen.iter().enumerate() {
            *tally.entry(p).or_default() += points[i];
        }
        casts.push(Cast { voter: v.person, kind: v.kind, picks: chosen });
    }
    let mut result: Vec<(PersonId, u32)> = tally.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    result.truncate(30);
    let id = w.acclaim.votes.len() as u32;
    w.acclaim.votes.push(Vote { id, ballot, year, date: today, casts, result, voters: voters.len() as u32 });
    w.acclaim.last_votes.insert(ballot, id);
    // Old ballots are summarised away after a decade (results are kept).
    let cutoff = year - 10;
    for v in w.acclaim.votes.iter_mut().filter(|v| v.year < cutoff && !v.casts.is_empty()) {
        v.casts.clear();
        v.casts.shrink_to_fit();
    }
    Some(id)
}

fn format_ballot(b: Ballot) -> u64 {
    match b {
        Ballot::WorldPlayer { young } => 1 + u64::from(young),
        Ballot::PlayersPlayer { comp } => (3 << 32) | u64::from(comp.0),
        Ballot::MinorPlayer { nation, university } => (4 << 32) | (u64::from(nation.0) << 1) | u64::from(university),
        Ballot::Hall { hall } => (5 << 32) | u64::from(hall),
    }
}

fn person_of(w: &World, p: PlayerId) -> PersonId {
    w.players.cold[p].person
}

fn journalists(w: &World, nation: Option<NationId>, n: usize) -> Vec<Voter> {
    let mut v: Vec<Voter> = w
        .media
        .journalists
        .values()
        .filter(|j| j.outlet.is_some())
        .filter(|j| nation.is_none_or(|x| w.media.outlets[j.outlet].nation == x))
        .map(|j| Voter { person: j.person, kind: VoterKind::Journalist, weights: lens(VoterKind::Journalist), nation: w.media.outlets[j.outlet].nation, club: ClubId::NONE, not_own_nation: false, not_own_club: false })
        .collect();
    v.sort_by_key(|x| x.person);
    v.truncate(n);
    v
}

// ---------------------------------------------------------------------------
// The world player of the year
// ---------------------------------------------------------------------------

/// National managers, national captains and journalists vote; managers and
/// captains may not vote for their own nation's players.
pub fn world_player(w: &mut World, young: bool, pool: &[(PlayerId, f32, f32, f32)]) -> Vec<(PlayerId, u32)> {
    let candidates: Vec<Candidate> = pool
        .iter()
        .map(|&(p, perf, trophies, fame)| {
            let s = w.perf.season(p, w.date.year()).map_or(0.0, |s| f32::from(s.goals + s.assists) / 40.0);
            let club = w.players.hot[p].club;
            let intl = f32::from(w.players.cold[p].caps.min(100)) / 100.0;
            let mut parts = [0.0; WHYS];
            parts[wi(Why::Performances)] = perf;
            parts[wi(Why::Trophies)] = trophies;
            parts[wi(Why::Fame)] = fame;
            parts[wi(Why::Goals)] = s;
            parts[wi(Why::International)] = intl;
            Candidate { person: person_of(w, p), parts, nation: w.intl.locked_to(p).unwrap_or(w.people[person_of(w, p)].nation), club, seen_in: if club.is_some() { w.clubs[club].nation } else { NationId::NONE } }
        })
        .collect();
    let mut voters: Vec<Voter> = Vec::new();
    let mut sides: Vec<&pw_world::intl::NationalSide> = w.intl.sides.values().filter(|s| s.level == pw_world::intl::Level::Senior).collect();
    sides.sort_by_key(|s| s.nation);
    for s in sides {
        if let Some(m) = s.manager.get() {
            voters.push(Voter { person: w.staff[m].person, kind: VoterKind::NationalManager, weights: lens(VoterKind::NationalManager), nation: s.nation, club: ClubId::NONE, not_own_nation: true, not_own_club: false });
        }
        if s.captain.is_some() {
            let cap = s.captain;
            voters.push(Voter { person: person_of(w, cap), kind: VoterKind::Player, weights: lens(VoterKind::Player), nation: s.nation, club: w.players.hot[cap].club, not_own_nation: true, not_own_club: false });
        }
    }
    voters.extend(journalists(w, None, 200));
    let Some(id) = run(w, Ballot::WorldPlayer { young }, &voters, &candidates, 3) else { return Vec::new() };
    w.acclaim.votes[id as usize].result.iter().filter_map(|&(person, pts)| w.people[person].player.get().map(|p| (p, pts))).collect()
}

// ---------------------------------------------------------------------------
// The players' player of a league season
// ---------------------------------------------------------------------------

/// The league's players vote (a delegation from each club; not for their
/// own teammates).
pub fn players_player(w: &mut World, comp: CompId, year: i32, lines: &[StatLine], min_apps: u16) {
    let today = w.date;
    let nation = w.comps[comp].nation;
    let candidates: Vec<Candidate> = lines
        .iter()
        .filter(|l| l.apps >= min_apps.max(1))
        .map(|l| {
            let mut parts = [0.0; WHYS];
            parts[wi(Why::Performances)] = (l.avg_rating() - 6.3).max(0.0);
            parts[wi(Why::Goals)] = f32::from(l.goals + l.assists) / 30.0;
            parts[wi(Why::Fame)] = f32::from(w.players.cold[l.player].rep.current) / 10_000.0;
            Candidate { person: person_of(w, l.player), parts, nation: NationId::NONE, club: l.club, seen_in: nation }
        })
        .collect();
    if candidates.len() < 3 {
        return;
    }
    let mut voters: Vec<Voter> = Vec::new();
    let teams: Vec<pw_core::TeamId> = w.comps[comp].state.table.iter().map(|r| r.team).collect();
    for t in teams {
        let club = w.teams[t].club;
        let mut squad: Vec<PlayerId> = w.teams[t].squad.clone();
        squad.sort_by_key(|&p| std::cmp::Reverse(w.players.cold[p].rep.current));
        for p in squad.into_iter().take(3) {
            voters.push(Voter { person: person_of(w, p), kind: VoterKind::Player, weights: lens(VoterKind::Player), nation, club, not_own_nation: false, not_own_club: true });
        }
    }
    let Some(id) = run(w, Ballot::PlayersPlayer { comp }, &voters, &candidates, 3) else { return };
    let Some(&(winner, _)) = w.acclaim.votes[id as usize].result.first() else { return };
    let Some(p) = w.people[winner].player.get() else { return };
    let club = w.players.hot[p].club;
    w.history.awards.push(AwardRecord { comp, season: year, kind: AwardKind::PlayersPlayer, player: p, club, value: 0.0 });
    w.events.push(today, Visibility::Public, EventKind::Award { player: p, comp, award: AwardKind::PlayersPlayer, season: year });
    w.events.push(today, Visibility::Public, EventKind::Voted { vote: id, person: winner });
    // Respect from peers lands in the dressing room.
    let h = &mut w.players.hot[p];
    h.morale = (h.morale + 5).min(100);
    let r = w.renown.people.entry(winner).or_default();
    r.fame = r.fame.saturating_add(150).min(10_000);
}

// ---------------------------------------------------------------------------
// Minor football's players of the year
// ---------------------------------------------------------------------------

/// June, after the minor seasons close: a nation's journalists vote for the
/// best university and school players.
pub fn minor_players(w: &mut World) {
    let today = w.date;
    let season = w.minor.season;
    let mut nations: Vec<NationId> = w.minor.institutions.iter().map(|i| i.nation).collect();
    nations.sort();
    nations.dedup();
    for n in nations {
        let voters = journalists(w, Some(n), 20);
        if voters.is_empty() {
            continue;
        }
        for university in [true, false] {
            let level = if university { MLevel::University } else { MLevel::School };
            let mut candidates: Vec<Candidate> = Vec::new();
            for (&p, lines) in &w.minor.careers {
                let Some(l) = lines.iter().rev().find(|l| l.season == season && l.kind.level() == level && l.apps >= 5) else { continue };
                if w.people[person_of(w, p)].nation != n && w.minor.member_of.get(&p).is_none_or(|&i| w.minor.institutions[i as usize].nation != n) {
                    continue;
                }
                let won = w.minor.history.iter().rev().take_while(|s| s.season == season).any(|s| s.winner == l.entrant);
                let mut parts = [0.0; WHYS];
                parts[wi(Why::Performances)] = (l.rating as f32 / f32::from(l.apps) / 10.0 - 6.0).max(0.0);
                parts[wi(Why::Goals)] = f32::from(l.goals) / 20.0;
                parts[wi(Why::Trophies)] = if won { 0.8 } else { 0.0 };
                candidates.push(Candidate { person: person_of(w, p), parts, nation: n, club: ClubId::NONE, seen_in: n });
            }
            candidates.sort_by(|a, b| b.parts.iter().sum::<f32>().total_cmp(&a.parts.iter().sum::<f32>()).then(a.person.cmp(&b.person)));
            candidates.truncate(25);
            if candidates.len() < 3 {
                continue;
            }
            let Some(id) = run(w, Ballot::MinorPlayer { nation: n, university }, &voters, &candidates, 3) else { continue };
            if let Some(&(winner, _)) = w.acclaim.votes[id as usize].result.first() {
                w.events.push(today, Visibility::Public, EventKind::Voted { vote: id, person: winner });
                // Being named best in the country gets a young player seen.
                if let Some(p) = w.people[winner].player.get() {
                    let clubs: Vec<ClubId> = w.clubs.ids().filter(|&c| w.clubs[c].nation == n && w.clubs[c].reputation >= 2000).collect();
                    for c in clubs {
                        w.knowledge.observe(c, p, 270, today);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Halls of fame
// ---------------------------------------------------------------------------

fn hall_id(w: &mut World, scope: HallScope, threshold: u8, class_size: u8) -> u32 {
    if let Some(&h) = w.acclaim.hall_index.get(&scope) {
        return h;
    }
    let id = w.acclaim.halls.len() as u32;
    let founded = w.date.year();
    w.acclaim.halls.push(Hall { id, scope, founded, members: Vec::new(), threshold, class_size });
    w.acclaim.hall_index.insert(scope, id);
    id
}

/// Retired, with the last playing spell over a year ago.
fn retired_a_year(w: &World, p: PlayerId) -> bool {
    w.players.hot[p].status == PlayerStatus::Retired && w.history.spells.get(&p).and_then(|v| v.last()).and_then(|s| s.to).is_some_and(|d| d.days_until(w.date) >= 365)
}

/// January: every hall considers a class.
pub fn inductions(w: &mut World) {
    // The world's hall.
    let world = hall_id(w, HallScope::World, 75, 5);
    let pool: Vec<PlayerId> = w.players.ids().filter(|&p| retired_a_year(w, p) && crate::honours::career_score(w, p) >= 700).collect();
    let cands: Vec<Candidate> = pool.iter().map(|&p| career_candidate(w, p, None)).collect();
    let voters = journalists(w, None, 60);
    induct(w, world, &voters, cands);
    // Nations' halls.
    let nations: Vec<NationId> = w.nations.ids().filter(|&n| !w.nations[n].leagues.is_empty()).collect();
    for n in nations {
        let h = hall_id(w, HallScope::Nation(n), 70, 3);
        let pool: Vec<PlayerId> = w
            .players
            .ids()
            .filter(|&p| retired_a_year(w, p))
            .filter(|&p| w.intl.locked_to(p) == Some(n) && w.intl.caps_for(p, n, pw_world::intl::Level::Senior) >= 20 || w.honours.clubs.iter().any(|(c, r)| w.clubs[*c].nation == n && r.legends.contains(&person_of(w, p))))
            .collect();
        let cands: Vec<Candidate> = pool.iter().map(|&p| career_candidate(w, p, Some(n))).collect();
        let mut voters = journalists(w, Some(n), 25);
        voters.extend(committee(w, h));
        induct(w, h, &voters, cands);
    }
    // Clubs' halls: from their legends.
    let clubs: Vec<ClubId> = w.honours.clubs.iter().filter(|(_, r)| !r.legends.is_empty()).map(|(&c, _)| c).collect();
    for c in clubs {
        let h = hall_id(w, HallScope::Club(c), 60, 2);
        let legends: Vec<PersonId> = w.honours.clubs[&c].legends.to_vec();
        let cands: Vec<Candidate> = legends
            .iter()
            .filter_map(|&who| w.people[who].player.get())
            .filter(|&p| w.players.hot[p].club != c || w.players.hot[p].status == PlayerStatus::Retired)
            .map(|p| club_candidate(w, p, c))
            .collect();
        let mut voters: Vec<Voter> = w
            .media
            .journalists
            .values()
            .filter(|j| j.beat.contains(&c))
            .map(|j| Voter { person: j.person, kind: VoterKind::Journalist, weights: lens(VoterKind::Committee), nation: w.clubs[c].nation, club: ClubId::NONE, not_own_nation: false, not_own_club: false })
            .collect();
        voters.sort_by_key(|v| v.person);
        voters.extend(committee(w, h));
        induct(w, h, &voters, cands);
    }
    // Schools and universities: their professional alumni.
    let insts: Vec<u32> = w.minor.institutions.iter().filter(|i| !i.alumni_pros.is_empty()).map(|i| i.id).collect();
    for i in insts {
        let n = w.minor.institutions[i as usize].nation;
        let h = hall_id(w, HallScope::Institution(i), 50, 2);
        let cands: Vec<Candidate> = w.minor.institutions[i as usize]
            .alumni_pros
            .iter()
            .filter_map(|&who| w.people[who].player.get())
            .filter(|&p| w.players.cold[p].senior_apps >= 50)
            .map(|p| career_candidate(w, p, None))
            .collect();
        let voters = journalists(w, Some(n), 8);
        induct(w, h, &voters, cands);
    }
}

/// Members of a hall sit on its committee.
fn committee(w: &World, hall: u32) -> Vec<Voter> {
    w.acclaim.halls[hall as usize]
        .members
        .iter()
        .map(|m| Voter { person: m.person, kind: VoterKind::Committee, weights: lens(VoterKind::Committee), nation: w.people[m.person].nation, club: ClubId::NONE, not_own_nation: false, not_own_club: false })
        .collect()
}

fn career_candidate(w: &World, p: PlayerId, nation: Option<NationId>) -> Candidate {
    let c = &w.players.cold[p];
    let who = c.person;
    let trophies = w.honours.tallies.iter().filter(|((_, x), _)| *x == p).map(|(&(club, _), t)| w.history.honours.iter().filter(|h| h.club == club && h.season >= t.first.year() && h.season <= t.last.year()).count()).sum::<usize>() as f32;
    let caps = nation.map_or(c.caps, |n| w.intl.caps_for(p, n, pw_world::intl::Level::Senior));
    let mut parts = [0.0; WHYS];
    parts[wi(Why::Trophies)] = (trophies / 8.0).min(2.0);
    parts[wi(Why::Performances)] = f32::from(w.renown.of(who).peak_world.max(c.rep.world)) / 5000.0;
    parts[wi(Why::Fame)] = f32::from(w.renown.of(who).fame) / 10_000.0;
    parts[wi(Why::Goals)] = f32::from(c.senior_goals) / 300.0;
    parts[wi(Why::Longevity)] = f32::from(c.senior_apps) / 600.0;
    parts[wi(Why::International)] = f32::from(caps) / 80.0;
    Candidate { person: who, parts, nation: nation.unwrap_or(w.people[who].nation), club: ClubId::NONE, seen_in: nation.unwrap_or(NationId::NONE) }
}

fn club_candidate(w: &World, p: PlayerId, club: ClubId) -> Candidate {
    let who = person_of(w, p);
    let t = w.honours.tally(club, p);
    let trophies = w.history.honours.iter().filter(|h| h.club == club && h.season >= t.first.year() && h.season <= t.last.year()).count() as f32;
    let love = w.media.fan(club, who).map_or(0, |f| f.score);
    let mut parts = [0.0; WHYS];
    parts[wi(Why::Loyalty)] = f32::from(t.apps) / 350.0;
    parts[wi(Why::Goals)] = f32::from(t.goals) / 120.0;
    parts[wi(Why::Trophies)] = trophies / 4.0;
    parts[wi(Why::Fame)] = (f32::from(love) / 800.0).max(0.0);
    Candidate { person: who, parts, nation: w.people[who].nation, club, seen_in: w.clubs[club].nation }
}

/// A hall's committee votes; those named on enough ballots go in.
fn induct(w: &mut World, hall: u32, voters: &[Voter], mut cands: Vec<Candidate>) {
    cands.retain(|c| !w.acclaim.halls[hall as usize].members.iter().any(|m| m.person == c.person));
    if cands.is_empty() || voters.len() < 3 {
        return;
    }
    cands.sort_by(|a, b| b.parts.iter().sum::<f32>().total_cmp(&a.parts.iter().sum::<f32>()).then(a.person.cmp(&b.person)));
    cands.truncate(15);
    let (threshold, class) = {
        let h = &w.acclaim.halls[hall as usize];
        (h.threshold, h.class_size)
    };
    // Committees name as many as the class allows.
    let voters: Vec<Voter> = voters.iter().map(|v| Voter { kind: VoterKind::Committee, ..*v }).collect();
    let Some(id) = run(w, Ballot::Hall { hall }, &voters, &cands, usize::from(class).min(3)) else { return };
    let today = w.date;
    let year = today.year();
    let n = voters.len() as f32;
    let elected: Vec<(PersonId, u8)> = w.acclaim.votes[id as usize]
        .result
        .iter()
        .map(|&(p, pts)| (p, (pts as f32 / n * 100.0).round().min(100.0) as u8))
        .filter(|&(_, share)| share >= threshold)
        .take(usize::from(class))
        .collect();
    for (person, share) in elected {
        let score = w.people[person].player.get().map_or(0, |p| crate::honours::career_score(w, p));
        w.acclaim.halls[hall as usize].members.push(Member { person, year, vote: id, share, score });
        let scope = w.acclaim.halls[hall as usize].scope;
        match scope {
            HallScope::World => {
                w.honours.hall.push(pw_world::honours::Inductee { person, date: today, score });
                w.events.push(today, Visibility::Public, EventKind::InductedHallOfFame { person });
                let r = w.renown.people.entry(person).or_default();
                r.fame = r.fame.saturating_add(800).min(10_000);
            }
            HallScope::Club(c) => {
                w.events.push(today, Visibility::Public, EventKind::HallInduction { hall, person });
                w.media.move_fans(c, person, 150, FanReason::Loyalty, today);
            }
            _ => {
                w.events.push(today, Visibility::Public, EventKind::HallInduction { hall, person });
                let r = w.renown.people.entry(person).or_default();
                r.fame = r.fame.saturating_add(200).min(10_000);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The chronicle
// ---------------------------------------------------------------------------

fn chronicle(w: &mut World, feat: Feat, kind: u8, scope: u32) {
    let today = w.date;
    let c = w.acclaim.counts.entry((kind, scope)).or_default();
    let before = *c;
    *c += 1;
    let id = w.acclaim.chronicle.len() as u32;
    w.acclaim.chronicle.push(Entry { id, date: today, feat, first: before == 0, before });
    w.events.push(today, Visibility::Public, EventKind::Chronicle { entry: id });
}

/// Monthly: look at what history gained and record the feats in it.
pub fn scan(w: &mut World) {
    // Titles and doubles.
    let new: Vec<pw_world::history::Honour> = w.history.honours[w.acclaim.honours_seen.min(w.history.honours.len())..].to_vec();
    w.acclaim.honours_seen = w.history.honours.len();
    for h in &new {
        let comp = &w.comps[h.comp];
        if comp.team_kind != TeamKind::First || h.club.is_none() {
            continue;
        }
        let nation = comp.nation;
        if comp.kind == CompKind::League && comp.tier == 1 {
            let before = w.history.honours.iter().filter(|x| x.club == h.club && x.comp == h.comp && x.season < h.season).count();
            if before == 0 && w.history.honours.iter().any(|x| x.comp == h.comp && x.season < h.season) {
                chronicle(w, Feat::FirstTitle { club: h.club, comp: h.comp, season: h.season }, 1, u32::from(h.club.0));
            }
        }
        // A double: this honour completes league + cup for the season.
        let won: Vec<(CompKind, u8)> = w
            .history
            .honours
            .iter()
            .filter(|x| x.club == h.club && x.season == h.season && w.comps[x.comp].nation == nation && w.comps[x.comp].team_kind == TeamKind::First)
            .map(|x| (w.comps[x.comp].kind, w.comps[x.comp].tier))
            .collect();
        let league = won.iter().any(|&(k, t)| k == CompKind::League && t == 1);
        let cup = won.iter().any(|&(k, _)| k == CompKind::Cup);
        let this_completes = match comp.kind {
            CompKind::League => comp.tier == 1 && cup,
            CompKind::Cup => league && won.iter().filter(|&&(k, _)| k == CompKind::Cup).count() == 1,
            _ => false,
        };
        if this_completes {
            chronicle(w, Feat::Double { club: h.club, season: h.season }, 2, u32::from(nation.0));
        }
    }
    // Unbeaten league seasons.
    let tables: Vec<pw_world::history::ArchivedTable> = w.history.tables[w.acclaim.tables_seen.min(w.history.tables.len())..].to_vec();
    w.acclaim.tables_seen = w.history.tables.len();
    for t in tables {
        let c = &w.comps[t.comp];
        if c.kind != CompKind::League || c.team_kind != TeamKind::First {
            continue;
        }
        let nation = c.nation;
        if let Some(r) = t.rows.first().filter(|r| r.lost == 0 && r.played >= 20) {
            let club = w.teams[r.team].club;
            chronicle(w, Feat::Unbeaten { club, comp: t.comp, season: t.season }, 3, u32::from(nation.0));
        }
    }
    // International tournaments.
    let n_t = w.intl.tournaments.len();
    for i in w.acclaim.tournaments_seen.min(n_t)..n_t {
        let (winner, id, year) = {
            let t = &w.intl.tournaments[i];
            (t.winner, t.id, t.year)
        };
        if winner.is_none() {
            // Not finished yet; look again next month.
            w.acclaim.tournaments_seen = i;
            break;
        }
        let before = w.intl.tournaments.iter().filter(|t| t.winner == winner && t.year < year).count();
        if before == 0 {
            chronicle(w, Feat::FirstTournament { nation: winner, tournament: id }, 4, u32::from(winner.0));
        }
        w.acclaim.tournaments_seen = i + 1;
    }
    // World awards.
    let new_awards: Vec<AwardRecord> = w.history.awards[w.acclaim.awards_seen.min(w.history.awards.len())..].iter().filter(|a| a.kind == AwardKind::WorldPlayer { rank: 1 }).cloned().collect();
    w.acclaim.awards_seen = w.history.awards.len();
    for a in new_awards {
        let who = person_of(w, a.player);
        let times = w.history.awards.iter().filter(|x| x.kind == AwardKind::WorldPlayer { rank: 1 } && x.player == a.player).count() as u8;
        if times >= 2 {
            chronicle(w, Feat::WorldPlayerAgain { person: who, times, year: a.season }, 5, u32::from(times));
        }
        let nation = w.intl.locked_to(a.player).unwrap_or(w.people[who].nation);
        let from_nation = w.history.awards.iter().filter(|x| x.kind == AwardKind::WorldPlayer { rank: 1 } && x.season < a.season).any(|x| w.intl.locked_to(x.player).unwrap_or(w.people[person_of(w, x.player)].nation) == nation);
        if !from_nation {
            chronicle(w, Feat::FirstWorldPlayerFrom { nation, person: who, year: a.season }, 6, u32::from(nation.0));
        }
    }
    // Firsts to a career milestone.
    let since = w.date.add_months(-1);
    let firsts: Vec<(PersonId, FirstTo)> = w
        .events
        .since(since)
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Milestone { player, kind: MilestoneKind::SeniorApps, count, .. } if count >= 1000 => Some((person_of(w, player), FirstTo::SeniorApps(count))),
            EventKind::Milestone { player, kind: MilestoneKind::CareerGoals, count, .. } if count >= 400 => Some((person_of(w, player), FirstTo::CareerGoals(count))),
            EventKind::Milestone { player, kind: MilestoneKind::Caps, count, .. } if count >= 150 => Some((person_of(w, player), FirstTo::Caps(count))),
            _ => None,
        })
        .collect();
    for (who, what) in firsts {
        if w.acclaim.claimed.contains(&what) {
            continue;
        }
        w.acclaim.claimed.push(what);
        chronicle(w, Feat::FirstTo { person: who, what }, 7, 0);
    }
}
