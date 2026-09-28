//! Reusable world-building steps shared by the CSV import and the synthetic fixture.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, CompId, Date, NationId, PersonId, Pos, StaffAttr, StaffAttrs, StaffId, TeamId};
use pw_world::club::{Board, ClubMarket, Facilities, Finance, Ownership};
use pw_world::comp::{CompRules, CompState};
use pw_world::nation::{Confed, NationSeason};
use pw_world::staff::ManagerRecord;
use pw_world::{Archetype, Club, CompKind, Competition, Format, MindKind, NameId, Nation, Person, Philosophy, Staff, StaffRole, Team, TeamKind, World};
use smallvec::SmallVec;

pub fn add_nation(w: &mut World, code: &str, name: &str, confed: Confed, reputation: u16, calendar: &str, economy: f32, youth_rating: u8) -> NationId {
    let calendar = w.data.calendars.iter().position(|c| c.key == calendar).unwrap_or(0) as u8;
    w.nations.push(Nation {
        code: code.to_string(),
        name: name.to_string(),
        confed,
        reputation,
        economy,
        youth_rating,
        calendar,
        leagues: Vec::new(),
        cups: SmallVec::new(),
        season: NationSeason::default(),
        first_names: Vec::new(),
        last_names: Vec::new(),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn add_comp(
    w: &mut World,
    name: &str,
    short: &str,
    nation: NationId,
    confed: Option<Confed>,
    kind: CompKind,
    tier: u8,
    team_kind: TeamKind,
    size: u16,
    promote: u8,
    relegate: u8,
    reputation: u16,
    format: Format,
    prize_pool: i64,
) -> CompId {
    let rules = CompRules {
        yellow_limit: if kind == CompKind::League { 5 } else { 3 },
        bench: w.data.tuning.matches.bench_size,
        subs: w.data.tuning.matches.max_subs,
        extra_time: kind != CompKind::League,
        away_goals: false,
        foreigner_limit: 0,
    };
    w.comps.push(Competition {
        name: name.to_string(),
        short_name: if short.is_empty() { name.to_string() } else { short.to_string() },
        nation,
        confed,
        kind,
        tier,
        reputation,
        format,
        rules,
        size,
        promote,
        relegate,
        above: CompId::NONE,
        below: CompId::NONE,
        continental: SmallVec::new(),
        team_kind,
        prize_pool,
        state: CompState::default(),
    })
}

pub struct ClubSpec<'a> {
    pub name: &'a str,
    pub short: &'a str,
    pub nation: NationId,
    pub city: &'a str,
    pub league: CompId,
    pub reputation: u16,
    pub balance: i64,
    pub stadium: &'a str,
    pub capacity: u32,
    pub facilities: Facilities,
    pub colors: [u32; 2],
    pub founded: u16,
    pub extra_teams: &'a [TeamKind],
}

pub fn add_club(w: &mut World, s: ClubSpec) -> ClubId {
    let club = w.clubs.next_id();
    let mut teams: SmallVec<[TeamId; 3]> = SmallVec::new();
    for kind in std::iter::once(TeamKind::First).chain(s.extra_teams.iter().copied().filter(|&k| k != TeamKind::First)) {
        teams.push(w.teams.push(Team { club, kind, squad: Vec::new(), tactics: Default::default(), captain: Default::default(), familiarity_weeks: 0 }));
    }
    let id = w.clubs.push(Club {
        name: s.name.to_string(),
        short_name: if s.short.is_empty() { s.name.to_string() } else { s.short.to_string() },
        nation: s.nation,
        city: s.city.to_string(),
        reputation: s.reputation,
        colors: s.colors,
        teams: teams.clone(),
        league: s.league,
        finance: Finance { balance: s.balance, ..Default::default() },
        facilities: s.facilities,
        stadium: s.stadium.to_string(),
        capacity: s.capacity,
        staff: Vec::new(),
        manager: StaffId::NONE,
        board: Board::default(),
        ownership: Ownership::Private,
        founded: s.founded,
        market: ClubMarket::default(),
        fan_mood: 55,
    });
    debug_assert_eq!(id, club);
    if s.league.is_some() {
        w.comps[s.league].state.entrants.push(teams[0]);
    }
    id
}

/// Staff for clubs that have none: a full backroom scaled to reputation.
pub fn ensure_staff(w: &mut World) {
    for club in w.clubs.ids() {
        let have: Vec<StaffRole> = w.clubs[club].staff.iter().map(|&s| w.staff[s].role).collect();
        let mut rng = Rng::keyed(&[w.seed, stream::STAFF, u64::from(club.0), 1]);
        let rep = f32::from(w.clubs[club].reputation);
        let level = 5.0 + rep / 800.0;
        let big = rep >= 5000.0;
        let roles: &[(StaffRole, usize)] = &[
            (StaffRole::Manager, 1),
            (StaffRole::Assistant, 1),
            (StaffRole::Coach, if big { 4 } else { 2 }),
            (StaffRole::GkCoach, 1),
            (StaffRole::FitnessCoach, 1),
            (StaffRole::Scout, if big { 4 } else { 1 }),
            (StaffRole::Physio, if big { 2 } else { 1 }),
            (StaffRole::HeadOfYouth, 1),
        ];
        for &(role, n) in roles {
            let existing = have.iter().filter(|&&r| r == role).count();
            for _ in existing..n {
                let s = new_staff(w, club, role, level, &mut rng);
                w.clubs[club].staff.push(s);
                if role == StaffRole::Manager {
                    w.clubs[club].manager = s;
                }
            }
        }
        if w.clubs[club].manager.is_none() {
            if let Some(&m) = w.clubs[club].staff.iter().find(|&&s| w.staff[s].role == StaffRole::Manager) {
                w.clubs[club].manager = m;
            }
        }
    }
}

pub fn new_staff(w: &mut World, club: ClubId, role: StaffRole, level: f32, rng: &mut Rng) -> StaffId {
    let nation = w.clubs[club].nation;
    let (first, last) = pw_sim::people::random_name(w, nation, rng);
    let dob = w.date.add_days(-(365 * rng.range_i32(32, 62)));
    let person = w.people.push(Person {
        first,
        last,
        common: NameId::NONE,
        dob,
        nation,
        nation2: NationId::NONE,
        hidden: pw_sim::generate::hidden_random(rng),
        player: Default::default(),
        staff: Default::default(),
        mind: MindKind::Ai,
    });
    let mut attrs = StaffAttrs::default();
    for a in StaffAttr::ALL {
        let bonus = if role.key_attrs().contains(&a) { 2.5 } else { -1.0 };
        attrs.set(a, rng.normal_ms(level + bonus, 2.2).round().clamp(1.0, 20.0) as u8);
    }
    let n_form = w.data.formations.len() as u32;
    let phil = Philosophy {
        formations: [rng.below(n_form) as u8, rng.below(n_form) as u8],
        mentality: rng.range_i32(-1, 1) as i8,
        press: rng.range_i32(30, 75) as u8,
        tempo: rng.range_i32(35, 70) as u8,
        directness: rng.range_i32(25, 75) as u8,
        youth_trust: rng.range_i32(20, 80) as u8,
        archetype: [Archetype::Pragmatist, Archetype::Developer, Archetype::Rotator, Archetype::Loyalist][rng.index(4)],
    };
    let rep = (f32::from(w.clubs[club].reputation) * rng.range_f32(0.5, 0.9)) as u16;
    let revenue = pw_sim::finance::season_revenue(w, club) as f32;
    let wage_share = match role {
        StaffRole::Manager => 0.006,
        StaffRole::Assistant | StaffRole::DirectorOfFootball => 0.002,
        _ => 0.0008,
    };
    let id = w.staff.push(Staff {
        person,
        role,
        club,
        attrs,
        wage: (revenue * wage_share / 52.0) as i64,
        contract_end: w.date.add_months(24),
        reputation: rep,
        philosophy: phil,
        joined: w.date,
        record: ManagerRecord::default(),
        retired: false,
    });
    w.people[person].staff = id;
    id
}

/// Cross-links and derived state once all rows are in.
pub fn finalize(w: &mut World) {
    // Nation league chains and cups.
    for n in w.nations.ids() {
        let mut leagues: Vec<CompId> = w.comps.iter_enumerated().filter(|(_, c)| c.nation == n && c.kind == CompKind::League && c.team_kind == TeamKind::First).map(|(id, _)| id).collect();
        leagues.sort_by_key(|&c| (w.comps[c].tier, c));
        // One league per tier forms the promotion chain; extra regional
        // leagues at the same tier hang off the chain above.
        let mut chain: Vec<CompId> = Vec::new();
        for &c in &leagues {
            if chain.last().is_none_or(|&l| w.comps[l].tier != w.comps[c].tier) {
                chain.push(c);
            }
        }
        for pair in chain.windows(2) {
            w.comps[pair[0]].below = pair[1];
            w.comps[pair[1]].above = pair[0];
        }
        w.nations[n].leagues = chain;
        w.nations[n].cups = w.comps.iter_enumerated().filter(|(_, c)| c.nation == n && c.kind == CompKind::Cup).map(|(id, _)| id).collect();
    }
    // Youth and reserve leagues without explicit entrants take every club side of that kind.
    for c in w.comps.ids() {
        let comp = &w.comps[c];
        if comp.kind != CompKind::League || comp.team_kind == TeamKind::First || !comp.state.entrants.is_empty() {
            continue;
        }
        let (nation, kind, size) = (comp.nation, comp.team_kind, usize::from(comp.size));
        let mut teams: Vec<TeamId> = w.clubs.iter().filter(|cl| cl.nation == nation).flat_map(|cl| cl.teams.iter().copied()).filter(|&t| w.teams[t].kind == kind).collect();
        teams.sort_by_key(|&t| std::cmp::Reverse(w.clubs[w.teams[t].club].reputation));
        if size >= 2 {
            teams.truncate(size);
        }
        w.comps[c].state.entrants = teams;
    }
    // Regen name pools from the imported people of each nation.
    let mut firsts: Vec<rustc_hash::FxHashSet<u32>> = vec![Default::default(); w.nations.len()];
    let mut lasts: Vec<rustc_hash::FxHashSet<u32>> = vec![Default::default(); w.nations.len()];
    for p in w.people.iter() {
        if p.nation.is_none() {
            continue;
        }
        let i = p.nation.0 as usize;
        if !p.first.is_none() && firsts[i].len() < 4000 {
            firsts[i].insert(p.first.0);
        }
        if !p.last.is_none() && lasts[i].len() < 6000 {
            lasts[i].insert(p.last.0);
        }
    }
    for n in w.nations.ids() {
        let mut f: Vec<u32> = firsts[n.0 as usize].iter().copied().collect();
        let mut l: Vec<u32> = lasts[n.0 as usize].iter().copied().collect();
        f.sort_unstable();
        l.sort_unstable();
        w.nations[n].first_names = f.into_iter().map(NameId).collect();
        w.nations[n].last_names = l.into_iter().map(NameId).collect();
    }
}

pub fn person_age_days(dob: Date, today: Date) -> i32 {
    dob.days_until(today)
}

pub fn default_facilities(rep: u16) -> Facilities {
    let lvl = (4.0 + f32::from(rep) / 650.0).round().clamp(1.0, 20.0) as u8;
    Facilities { training: lvl, youth: lvl, academy: lvl, medical: lvl }
}

/// Reserve a person for an imported player; the player id is filled in by the caller.
pub fn add_person(w: &mut World, first: &str, last: &str, common: &str, dob: Date, nation: NationId, nation2: NationId) -> PersonId {
    let first = w.names.intern(first);
    let last = w.names.intern(last);
    let common = w.names.intern(common);
    w.people.push(Person { first, last, common, dob, nation, nation2, hidden: Default::default(), player: Default::default(), staff: Default::default(), mind: MindKind::Ai })
}

pub fn familiarity_from(naturals: &[Pos]) -> [u8; pw_core::N_POS] {
    let mut f = [1u8; pw_core::N_POS];
    for p in Pos::ALL {
        let best = naturals.iter().map(|&n| n.similarity(p)).fold(0.0f32, f32::max);
        f[p.idx()] = (1.0 + best * 13.0).round().clamp(1.0, 14.0) as u8;
    }
    for (i, &n) in naturals.iter().enumerate() {
        f[n.idx()] = if i == 0 { 20 } else { 18 };
    }
    f
}
