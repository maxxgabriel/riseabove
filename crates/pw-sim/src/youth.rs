//! The youth and amateur game (04 §6, 07 §6, 09 §2).
//!
//! Every season a new cohort of eight-year-olds joins local grassroots clubs.
//! They play weekly age-group football (a statistical model applied to all
//! age-group and local football alike, S27), go to school, and are watched by
//! academy scouts whose reach depends on the academy. Promising children are
//! invited on trials; heads of youth decide, with their own blind spots —
//! physically advanced children look better than they are, late developers
//! worse. Each summer every age group is reviewed and some are released.
//! Sixteen-year-olds are kept on as scholars or let go; scholars earn first
//! professional contracts only if the club believes in them. The released
//! drop into grassroots or amateur football, where scouts from smaller clubs
//! can still find them.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, Hidden, LocalClubId, NationId, PersonId, PlayerId, StaffAttr, TeamId};
use pw_world::contract::ContractKind;
use pw_world::event::{Cause, Causes, EventKind, Fact, Visibility};
use pw_world::scouting::Brief;
use pw_world::youth::{Academy, AcademyStyle, AcademyTrial, LocalClub, LocalLevel, Reach, Release, School};
use pw_world::{Contract, MemoryKind, MindKind, PlayerStatus, StaffRole, Team, TeamKind, World};
use smallvec::SmallVec;

use crate::consider;
use crate::generate as gen_;
use crate::people::{NewPlayer, spawn_player};

const ACADEMY_GROUPS: [(TeamKind, u32); 3] = [(TeamKind::U12, 12), (TeamKind::U14, 14), (TeamKind::U16, 16)];

fn head_of_youth(w: &World, club: ClubId) -> Option<PersonId> {
    let c = &w.clubs[club];
    let s = c.staff.iter().copied().find(|&s| w.staff[s].role == StaffRole::HeadOfYouth).or(c.manager.get())?;
    Some(w.staff[s].person)
}

// ------------------------------------------------------------------ worldgen

/// Local football around every professional club, academies with their own
/// character, and age-group sides for clubs that run them.
pub fn ensure(w: &mut World) {
    if !w.youth.local.is_empty() {
        return;
    }
    let clubs: Vec<ClubId> = w.clubs.ids().collect();
    for club in clubs {
        let mut rng = Rng::keyed(&[w.seed, stream::YOUTH, u64::from(club.0), 0x10ca1]);
        let (nation, city, rep) = {
            let c = &w.clubs[club];
            let city = if c.city.is_empty() { c.short_name.clone() } else { c.city.clone() };
            (c.nation, city, c.reputation)
        };
        let mut feeders: SmallVec<[LocalClubId; 4]> = SmallVec::new();
        let suffixes = ["Juniors", "Boys", "Youth", "Colts", "Rangers", "Athletic", "Wanderers", "Town", "Rovers", "United"];
        for k in 0..3 {
            let level = if k == 2 { LocalLevel::Amateur } else { LocalLevel::Grassroots };
            let name = format!("{city} {}", suffixes[rng.index(suffixes.len())]);
            let feeder = level == LocalLevel::Grassroots && rng.chance(0.5 + f32::from(rep) / 20_000.0);
            let id = w.youth.local.push(LocalClub {
                name,
                nation,
                city: city.clone(),
                level,
                coaching: rng.range_i32(3, 11) as u8,
                facilities: rng.range_i32(2, 9) as u8,
                feeder_of: if feeder { club } else { ClubId::NONE },
                standing: rng.range_i32(50, 600) as u16,
                members: Vec::new(),
            });
            if feeder {
                feeders.push(id);
            }
        }
        // An academy, if the club runs youth football.
        let fac = w.clubs[club].facilities;
        let runs = w.clubs[club].teams.iter().any(|&t| w.teams[t].kind.is_youth()) || fac.youth >= 8 || rep >= 2500;
        if !runs {
            continue;
        }
        let reach = match (fac.academy, rep) {
            (a, r) if a >= 16 || r >= 8000 => Reach::International,
            (a, r) if a >= 12 || r >= 5000 => Reach::National,
            (a, _) if a >= 7 => Reach::Regional,
            _ => Reach::Local,
        };
        let hoy = head_of_youth(w, club);
        let jp = hoy.map_or(10.0, |p| consider::staff_attr(w, p, StaffAttr::JudgingPotential));
        let style = match rng.below(4) {
            0 => AcademyStyle::Technical,
            1 => AcademyStyle::Athletic,
            2 => AcademyStyle::Community,
            _ => AcademyStyle::Balanced,
        };
        let budget = w.governance.get(&club).map_or(0, |g| g.academy_budget);
        w.youth.academies.insert(
            club,
            Academy {
                club,
                reach,
                style,
                strictness: (20.0 + f32::from(rep) / 250.0 + rng.normal() * 6.0).clamp(10.0, 60.0) as u8,
                maturity_bias: ((12.0 - jp) * 0.8 + if style == AcademyStyle::Athletic { 3.0 } else { 0.0 }).round().clamp(-10.0, 10.0) as i8,
                budget,
                feeders,
                study_required: rng.range_i32(4, 10) as u8,
                places: (14.0 + f32::from(fac.youth) * 0.4) as u8,
            },
        );
        for (kind, _) in ACADEMY_GROUPS {
            if w.club_team(club, kind).is_none() {
                let t = w.teams.push(Team { club, kind, squad: Vec::new(), tactics: Default::default(), captain: pw_core::PlayerId::NONE, familiarity_weeks: 0 });
                w.clubs[club].teams.push(t);
            }
        }
    }
    // A first population of children at local clubs, cohorts 8–15.
    let year = w.date.year();
    for age in 8..=15 {
        new_cohort(w, age, year);
    }
    w.youth.last_cohort = year;
}

/// A new cohort of children of `age` joins grassroots clubs.
fn new_cohort(w: &mut World, age: u32, year: i32) {
    let today = w.date;
    let clubs: Vec<LocalClubId> = w.youth.local.ids().filter(|&l| w.youth.local[l].level == LocalLevel::Grassroots).collect();
    for l in clubs {
        let nation = w.youth.local[l].nation;
        let mut rng = Rng::keyed(&[w.seed, stream::YOUTH, u64::from(l.0), year as u64, u64::from(age)]);
        let per = 2 + rng.below(3) as usize;
        let youth = f32::from(w.nations[nation].youth_rating);
        for _ in 0..per {
            // Grassroots talent: the same distribution shape as academies'
            // intakes, but centred lower — the gems are rare and unsorted.
            let pa = crate::people::intake_pa(2.0 + f32::from(w.youth.local[l].facilities) * 0.3, youth, 200.0, &mut rng) - 8.0;
            let pa = pa.clamp(30.0, 200.0);
            let dob = today.add_days(-(age as i32 * 365 + rng.range_i32(0, 364)));
            let a = dob.age_years(today);
            let ca = (pa * gen_::ca_share_at(a) * rng.normal_ms(1.0, 0.1)).clamp(8.0, pa);
            let pos = gen_::random_position(&mut rng);
            let p = spawn_player(
                w,
                NewPlayer { nation, dob, pos, ca, pa: pa as u8, club: ClubId::NONE, team: TeamId::NONE, contract: Contract::default() },
                &mut rng,
            );
            w.players.hot[p].status = PlayerStatus::Amateur;
            w.youth.join(p, l);
            let who = w.players.cold[p].person;
            w.youth.school.insert(who, School::default());
        }
    }
}

// ------------------------------------------------------------------ weekly

/// Weekly (in season): age-group and local football, academy scouting,
/// trials, and unattached adults drifting into the amateur game.
pub fn weekly(w: &mut World) {
    let today = w.date;
    let in_season = !matches!(today.month(), 6 | 7);
    if in_season {
        play_local(w);
        play_academy(w);
        scout_local(w);
    }
    trials(w);
    drift_to_amateur(w);
    let _ = today;
}

/// One statistical match's worth of football for a group of players: coaches
/// pick by how good they think each is, ratings follow ability relative to the
/// group, minutes and form accumulate. Used for all local and age-group football.
fn play_group(w: &mut World, members: &[PlayerId], judge: Option<ClubId>, key: u64) {
    if members.is_empty() {
        return;
    }
    let today = w.date;
    let mut rated: Vec<(PlayerId, f32)> = members
        .iter()
        .map(|&p| {
            let seen = judge.map_or(f32::from(w.players.cold[p].ca), |c| crate::scouting::view(w, c, p).0);
            (p, seen)
        })
        .collect();
    rated.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mean = rated.iter().map(|(p, _)| f32::from(w.players.cold[*p].ca)).sum::<f32>() / rated.len() as f32;
    for (rank, (p, _)) in rated.into_iter().enumerate() {
        let h = w.players.hot[p];
        if h.injury != 0 {
            continue;
        }
        let minutes: u16 = match rank {
            0..=10 => 70,
            11..=15 => 25,
            _ => 0,
        };
        if minutes == 0 {
            continue;
        }
        let mut rng = Rng::keyed(&[w.seed, stream::YOUTH, u64::from(p.0), key, today.0 as u64]);
        let rating = (6.6 + (f32::from(w.players.cold[p].ca) - mean) / 12.0 + rng.normal() * 0.6).clamp(4.5, 9.5);
        let hh = &mut w.players.hot[p];
        hh.minutes_4w = hh.minutes_4w.saturating_add(minutes);
        hh.minutes_week = hh.minutes_week.saturating_add(minutes);
        hh.push_rating(rating);
        hh.last_match = today;
        hh.sharpness = (f32::from(hh.sharpness) + f32::from(minutes) / 90.0 * 10.0).min(100.0) as u8;
    }
}

fn play_local(w: &mut World) {
    let ids: Vec<LocalClubId> = w.youth.local.ids().collect();
    for l in ids {
        let members: Vec<PlayerId> = w.youth.local[l].members.iter().copied().filter(|&p| w.players.hot[p].status == PlayerStatus::Amateur).collect();
        // Age groups at grassroots clubs: children play with their own year.
        if w.youth.local[l].level == LocalLevel::Grassroots {
            let mut by_age: pw_world::FxHashMap<u32, Vec<PlayerId>> = Default::default();
            for p in members {
                by_age.entry(w.age(p)).or_default().push(p);
            }
            let mut ages: Vec<u32> = by_age.keys().copied().collect();
            ages.sort();
            for a in ages {
                let g = by_age.remove(&a).unwrap_or_default();
                play_group(w, &g, None, u64::from(l.0) << 8 | u64::from(a));
            }
        } else {
            play_group(w, &members, None, u64::from(l.0));
        }
    }
    // Children who have grown out of grassroots move to the local adult side.
    let grown: Vec<PlayerId> = w.youth.member_of.iter().filter(|(p, l)| w.youth.local[**l].level == LocalLevel::Grassroots && w.age(**p) >= 17).map(|(p, _)| *p).collect();
    let mut grown = grown;
    grown.sort();
    for p in grown {
        let city = w.youth.local[w.youth.member_of[&p]].city.clone();
        if let Some(adult) = w.youth.local.ids().find(|&x| w.youth.local[x].level == LocalLevel::Amateur && w.youth.local[x].city == city) {
            w.youth.join(p, adult);
        }
    }
}

fn play_academy(w: &mut World) {
    let clubs: Vec<ClubId> = w.youth.academies.keys().copied().collect();
    let mut clubs = clubs;
    clubs.sort();
    for club in clubs {
        for (kind, _) in ACADEMY_GROUPS {
            if let Some(t) = w.club_team(club, kind) {
                let squad = w.teams[t].squad.clone();
                play_group(w, &squad, Some(club), u64::from(t.0));
            }
        }
    }
}

/// How far an academy's scouts can see.
fn in_reach(w: &World, academy: &Academy, l: LocalClubId) -> f32 {
    let local = &w.youth.local[l];
    let home = &w.clubs[academy.club];
    if academy.feeders.contains(&l) {
        return 1.0;
    }
    let same_city = local.city == home.city || local.city == home.short_name;
    let same_nation = local.nation == home.nation;
    match academy.reach {
        Reach::Local => if same_city { 0.6 } else { 0.0 },
        Reach::Regional => if same_city { 0.7 } else if same_nation { 0.15 } else { 0.0 },
        Reach::National => if same_nation { 0.35 } else { 0.0 },
        Reach::International => if same_nation { 0.4 } else { 0.05 },
    }
}

/// Youth scouts and the head of youth watch local football within reach; so
/// do lower-level clubs' scouts looking for adults in the amateur game.
fn scout_local(w: &mut World) {
    let today = w.date;
    let week = (today.0 / 7) as u64;
    let assignments = w.scouting.assignments.clone();
    for a in assignments {
        let Brief::Youth(_) = a.brief else {
            // Senior scouts of smaller clubs sometimes take in an amateur game.
            if let Brief::Nation(n) = a.brief {
                if w.clubs[a.club].reputation < 3500 {
                    let mut rng = Rng::keyed(&[w.seed, stream::YOUTH, u64::from(a.scout.0), week, 0xa]);
                    let amateur: Vec<LocalClubId> = w.youth.local.ids().filter(|&l| w.youth.local[l].level == LocalLevel::Amateur && w.youth.local[l].nation == n).collect();
                    if !amateur.is_empty() && rng.chance(0.3) {
                        let l = amateur[rng.index(amateur.len())];
                        watch(w, a.scout, a.club, l);
                    }
                }
            }
            continue;
        };
        let Some(academy) = w.youth.academies.get(&a.club).cloned() else { continue };
        let mut rng = Rng::keyed(&[w.seed, stream::YOUTH, u64::from(a.scout.0), week]);
        let weights: Vec<f32> = w.youth.local.ids().map(|l| if w.youth.local[l].level == LocalLevel::Grassroots { in_reach(w, &academy, l) * (0.5 + f32::from(w.youth.local[l].standing) / 1000.0) } else { 0.0 }).collect();
        if weights.iter().all(|&x| x <= 0.0) {
            continue;
        }
        for _ in 0..2 {
            let l = LocalClubId(rng.weighted(&weights) as u32);
            watch(w, a.scout, a.club, l);
        }
    }
}

fn watch(w: &mut World, scout: pw_core::StaffId, club: ClubId, l: LocalClubId) {
    let today = w.date;
    let members: Vec<PlayerId> = w.youth.local[l].members.iter().copied().filter(|&p| w.players.hot[p].last_match.days_until(today) <= 7).collect();
    for p in members {
        w.knowledge.observe(club, p, 60, today);
        let r = crate::scouting::judge(w, scout, club, p, w.youth.local[l].standing);
        w.scouting.file(club, p, r);
    }
}

// ------------------------------------------------------------------ trials

/// The level an academy looks for in a child of a given age (perceived PA).
fn bar(w: &World, club: ClubId) -> f32 {
    let rep = f32::from(w.clubs[club].reputation);
    75.0 + rep / 110.0
}

/// A head of youth's reading of a child: potential as their people see it,
/// skewed by how physically advanced the child is right now.
fn judged_potential(w: &World, club: ClubId, p: PlayerId) -> f32 {
    let (_, _, pa, _) = crate::scouting::view(w, club, p);
    let bias = w.youth.academies.get(&club).map_or(0, |a| a.maturity_bias);
    // bio_offset > 0 means a late developer: small now, looks worse.
    pa - f32::from(w.players.cold[p].bio_offset) / 10.0 * f32::from(bias) * 0.8
}

fn trials(w: &mut World) {
    let today = w.date;
    // Trials that end: the head of youth decides.
    let ending: Vec<AcademyTrial> = w.youth.trials.iter().filter(|t| t.until <= today).copied().collect();
    w.youth.trials.retain(|t| t.until > today);
    for t in ending {
        decide_trial(w, t);
    }
    // New invitations from academy views of local children.
    let clubs: Vec<ClubId> = {
        let mut v: Vec<ClubId> = w.youth.academies.keys().copied().collect();
        v.sort();
        v
    };
    for club in clubs {
        if (club.0 + (today.0 / 7) as u32) % 2 != 0 {
            continue;
        }
        let threshold = bar(w, club);
        let mut cands: Vec<(PlayerId, f32)> = w
            .knowledge
            .known(club)
            .filter(|(p, s)| s.minutes >= 60 && w.players.hot[*p].status == PlayerStatus::Amateur && (8..=15).contains(&w.age(*p)))
            .map(|(p, _)| (p, judged_potential(w, club, p)))
            .filter(|&(p, v)| v >= threshold && !w.youth.trials.iter().any(|t| t.player == p) && !w.market.on_cooldown(club, p, today))
            .collect();
        cands.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        for (p, _) in cands.into_iter().take(2) {
            invite(w, club, p);
        }
    }
}

fn invite(w: &mut World, club: ClubId, p: PlayerId) {
    let today = w.date;
    w.market.cooldown.insert((club, p), today.add_days(240));
    let who = w.players.cold[p].person;
    // Minors abroad: a family would have to move; most don't, and rules may forbid it.
    let abroad = w.people[who].nation != w.clubs[club].nation;
    if abroad {
        let parents_move = consider::household_move_cost(w, who, w.clubs[club].nation) < 0.25;
        if !parents_move {
            return;
        }
    }
    if w.people[who].mind == MindKind::External {
        let kind = pw_world::DecisionKind::Trial { club, days: 21 };
        let options = kind.simple_options();
        w.decisions.push(pw_world::Decision { person: who, player: p, kind, options, created: today, deadline: today.add_days(5), default: 0, answer: None, resolved: false });
        return;
    }
    // Families usually say yes; unsupportive or far-away ones sometimes don't.
    let support = f32::from(w.lives[who].household.parents.support) / 20.0;
    let mut rng = Rng::keyed(&[w.seed, stream::FAMILY, u64::from(p.0), today.0 as u64]);
    if rng.chance(0.5 + support * 0.5) {
        start_trial(w, club, p);
    }
}

pub fn start_trial(w: &mut World, club: ClubId, p: PlayerId) {
    let today = w.date;
    if w.youth.trials.iter().any(|t| t.player == p) {
        return;
    }
    w.youth.trials.push(AcademyTrial { player: p, club, from: today, until: today.add_days(21) });
    w.knowledge.observe(club, p, 270, today);
    w.events.push(today, Visibility::Person(w.players.cold[p].person), EventKind::AcademyTrialStarted { player: p, club });
}

fn decide_trial(w: &mut World, t: AcademyTrial) {
    let today = w.date;
    let p = t.player;
    if w.players.hot[p].status != PlayerStatus::Amateur {
        return;
    }
    let Some(academy) = w.youth.academies.get(&t.club).cloned() else { return };
    let age = w.age(p);
    let kind = ACADEMY_GROUPS.iter().find(|(_, max)| age < *max).map(|(k, _)| *k).unwrap_or(TeamKind::U16);
    let Some(team) = w.club_team(t.club, kind) else { return };
    let v = judged_potential(w, t.club, p);
    let full = w.teams[team].squad.len() >= usize::from(academy.places);
    let weakest = w.teams[team].squad.iter().map(|&x| judged_potential(w, t.club, x)).fold(f32::MAX, f32::min);
    let take = v >= bar(w, t.club) && (!full || v > weakest + 3.0);
    let who = w.players.cold[p].person;
    if !take {
        if let Some(h) = head_of_youth(w, t.club) {
            let compat = consider::compat(w, who, h);
            let ev = w.events.push(today, Visibility::Person(who), EventKind::TrialEnded { player: p, club: t.club, offered: false });
            w.social.remember(who, h, MemoryKind::Refused, today, ev, false, 0.8, compat);
        }
        return;
    }
    join_academy(w, t.club, team, p);
}

fn join_academy(w: &mut World, club: ClubId, team: TeamId, p: PlayerId) {
    let today = w.date;
    w.youth.leave(p);
    let dob = w.people[w.players.cold[p].person].dob;
    {
        let h = &mut w.players.hot[p];
        h.status = PlayerStatus::Active;
        h.club = club;
        h.team = team;
    }
    {
        let c = &mut w.players.cold[p];
        c.contract = Contract { club, kind: ContractKind::Youth, wage: 0, start: today, end: dob.add_months(16 * 12 + 11), ..Default::default() };
        c.youth_club = club;
        c.joined = today;
    }
    w.teams[team].squad.push(p);
    w.history.start_spell(p, club, today, false, 0);
    w.events.push(today, Visibility::Public, EventKind::AcademyJoined { player: p, club });
    if let Some(h) = head_of_youth(w, club) {
        let who = w.players.cold[p].person;
        let compat = consider::compat(w, who, h);
        w.social.remember(who, h, MemoryKind::GaveChance, today, pw_core::EventId::NONE, false, 1.0, compat);
    }
}

// ------------------------------------------------------------------ reviews

/// Early summer: every academy age group is reviewed; sixteen-year-olds are
/// offered scholarships or let go. Late developers are the ones most often
/// wrongly released.
pub fn reviews(w: &mut World) {
    let today = w.date;
    let clubs: Vec<ClubId> = {
        let mut v: Vec<ClubId> = w.youth.academies.keys().copied().collect();
        v.sort();
        v
    };
    for club in clubs {
        let academy = w.youth.academies[&club].clone();
        for (kind, max_age) in ACADEMY_GROUPS {
            let Some(team) = w.club_team(club, kind) else { continue };
            let squad = w.teams[team].squad.clone();
            let mut ranked: Vec<(PlayerId, f32)> = squad
                .iter()
                .map(|&p| {
                    let attitude = f32::from(w.players.hot[p].training) / 10.0 - 6.0;
                    (p, judged_potential(w, club, p) + attitude * 3.0)
                })
                .collect();
            ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            let n = ranked.len();
            let cut = (n as f32 * f32::from(academy.strictness) / 100.0).round() as usize;
            for (i, (p, score)) in ranked.into_iter().enumerate() {
                let age = w.age(p);
                let bottom = i + cut >= n;
                let below_bar = score < bar(w, club) - 10.0;
                if bottom || below_bar {
                    release(w, club, p);
                    continue;
                }
                // Moving up an age group.
                if age >= max_age {
                    if kind == TeamKind::U16 {
                        scholarship(w, club, p);
                    } else {
                        let next = if kind == TeamKind::U12 { TeamKind::U14 } else { TeamKind::U16 };
                        if let Some(t) = w.club_team(club, next) {
                            move_team(w, p, t);
                        }
                    }
                }
            }
        }
    }
    let _ = today;
}

fn move_team(w: &mut World, p: PlayerId, to: TeamId) {
    let from = w.players.hot[p].team;
    if from.is_some() {
        w.teams[from].squad.retain(|&x| x != p);
    }
    w.teams[to].squad.push(p);
    w.players.hot[p].team = to;
}

fn scholarship(w: &mut World, club: ClubId, p: PlayerId) {
    let today = w.date;
    let dob = w.people[w.players.cold[p].person].dob;
    let team = [TeamKind::U18, TeamKind::U19, TeamKind::U21, TeamKind::Reserve].iter().find_map(|&k| w.club_team(club, k)).unwrap_or(w.clubs[club].first_team());
    move_team(w, p, team);
    let c = &mut w.players.cold[p];
    c.contract = Contract { club, kind: ContractKind::Youth, wage: 90, start: today, end: dob.add_months(18 * 12 + 11), ..Default::default() };
    w.events.push(today, Visibility::Public, EventKind::ScholarshipOffered { player: p, club });
}

/// Let a young player go. They return to local football near home.
pub fn release(w: &mut World, club: ClubId, p: PlayerId) {
    let today = w.date;
    let team = w.players.hot[p].team;
    if team.is_some() {
        w.teams[team].squad.retain(|&x| x != p);
    }
    let who = w.players.cold[p].person;
    let age = w.age(p);
    {
        let h = &mut w.players.hot[p];
        h.status = PlayerStatus::Amateur;
        h.club = ClubId::NONE;
        h.team = TeamId::NONE;
        h.morale = h.morale.saturating_sub(15);
    }
    w.players.cold[p].contract = Contract::default();
    w.history.end_spell(p, today);
    w.youth.released.entry(p).or_default().push(Release { club, date: today, age: age as u8 });
    let causes: Causes = pw_world::causes![Cause::Fact(Fact::FormSlump { player: p })];
    let ev = w.events.push_caused(today, Visibility::Person(who), EventKind::AcademyReleased { player: p, club }, causes);
    if let Some(h) = head_of_youth(w, club) {
        let compat = consider::compat(w, who, h);
        w.social.remember(who, h, MemoryKind::Refused, today, ev, false, 1.2, compat);
    }
    let l = w.lives.get(who).map(|l| l.stress).unwrap_or(0);
    if let Some(life) = w.lives.get_mut(who) {
        life.stress = l.saturating_add(15).min(100);
    }
    join_local_near(w, p, who);
}

pub fn join_local_near(w: &mut World, p: PlayerId, who: PersonId) {
    let age = w.age(p);
    let level = if age >= 17 { LocalLevel::Amateur } else { LocalLevel::Grassroots };
    let home: NationId = w.lives.get(who).map_or(w.people[who].nation, |l| if l.home.is_some() { l.home } else { w.people[who].nation });
    let club = w.youth.local.ids().filter(|&l| w.youth.local[l].level == level && w.youth.local[l].nation == home).min_by_key(|&l| (w.youth.local[l].members.len(), l));
    if let Some(l) = club {
        w.youth.join(p, l);
        w.events.push(w.date, Visibility::Person(who), EventKind::JoinedLocalClub { player: p, local: l });
    }
}

/// Unattached adults who can't find a professional club play amateur
/// football rather than nothing.
fn drift_to_amateur(w: &mut World) {
    let today = w.date;
    let free: Vec<PlayerId> = w
        .players
        .ids()
        .filter(|&p| w.players.hot[p].status == PlayerStatus::FreeAgent && w.age(p) < 32 && consider::days_unattached(w, p) > 75)
        .collect();
    for p in free {
        let who = w.players.cold[p].person;
        // A human decides for themselves; they can sign up through their own choices.
        if w.people[who].mind == MindKind::External {
            continue;
        }
        w.players.hot[p].status = PlayerStatus::Amateur;
        join_local_near(w, p, who);
    }
    let _ = today;
}

// ------------------------------------------------------------------ school

/// Monthly: schoolwork for everyone of school age, with academies enforcing
/// their study requirement; exams at sixteen.
pub fn school(w: &mut World) {
    let today = w.date;
    let kids: Vec<PersonId> = w.youth.school.keys().copied().collect();
    let mut kids = kids;
    kids.sort();
    for who in kids {
        let age = consider::age(w, who);
        if age >= 18.5 {
            w.youth.school.remove(&who);
            continue;
        }
        let p = w.people[who].player;
        let hours = f32::from(w.lives[who].routine.study);
        let prof = w.people[who].hidden.f(Hidden::Professionalism);
        let support = f32::from(w.lives[who].household.parents.support) / 20.0;
        let club = if p.is_some() { w.players.hot[p].club } else { ClubId::NONE };
        let required = w.youth.academies.get(&club).map_or(4, |a| a.study_required);
        let s = w.youth.school.get_mut(&who).unwrap();
        let target = 40.0 + hours * 4.0 + (prof - 10.0) * 1.5 + support * 10.0;
        s.grades = (f32::from(s.grades) * 0.85 + target.clamp(0.0, 100.0) * 0.15) as u8;
        // Academies notice when schoolwork is being neglected.
        if club.is_some() && hours < f32::from(required) && today.month() % 3 == 0 {
            if let Some(h) = head_of_youth(w, club) {
                let compat = consider::compat(w, h, who);
                w.social.remember(h, who, MemoryKind::PoorAttitude, today, pw_core::EventId::NONE, false, 0.5, compat);
            }
        }
        if (16.0..16.1).contains(&age) {
            let passed = w.youth.school[&who].grades >= 50;
            if let Some(s) = w.youth.school.get_mut(&who) {
                s.exams_passed = passed;
            }
            if passed {
                let l = &mut w.lives[who];
                l.education = l.education.max(2);
            }
            w.events.push(today, Visibility::Person(who), EventKind::ExamsSat { person: who, passed });
        }
    }
}

/// Is a scholar worth a first professional contract, as their club sees it?
pub fn worth_pro_contract(w: &World, p: PlayerId) -> bool {
    let club = w.players.hot[p].club;
    if club.is_none() {
        return false;
    }
    let (ca, _, pa, _) = crate::scouting::view(w, club, p);
    let ideal = crate::market::ideal_ca(w.clubs[club].reputation);
    pa >= ideal - 12.0 || ca >= ideal - 35.0
}

/// Yearly cohort (start of the youth season).
pub fn yearly(w: &mut World) {
    let year = w.date.year();
    if w.youth.last_cohort >= year {
        return;
    }
    w.youth.last_cohort = year;
    new_cohort(w, 8, year);
    // Academies' budgets follow the board's youth investment.
    let clubs: Vec<ClubId> = w.youth.academies.keys().copied().collect();
    for c in clubs {
        let budget = w.governance.get(&c).map_or(0, |g| g.academy_budget);
        if let Some(a) = w.youth.academies.get_mut(&c) {
            a.budget = budget;
        }
    }
}
