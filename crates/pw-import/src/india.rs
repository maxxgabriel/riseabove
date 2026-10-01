//! India's football ecosystem, built from `data/worlds/india/pack.toml`.
//!
//! The pack names real states, associations, universities and clubs. Anything
//! it does not name (district clubs, schools, the fourth-tier fill) is
//! generated from region attributes and marked as generated. No history is
//! attached to any real name: the world runs from its start date and results
//! diverge freely.
//!
//! The researched reference data under `data/worlds/india/**` (`india_ref`) is read with its provenance and used in three ways, and
//! only these: (1) a club the pack names that has a reference record with exactly its name in its state is labelled by that record's
//! provenance (`Imported` only for a sourced fact, otherwise a scenario seed); (2) a place the builder would have filled with a made-up
//! club is given to a real club the reference lists as a member of that league at the start (same tier, or the state's top league),
//! taking the place's strength and nothing else, and labelled by the record; (3) a number in a reference record (a stadium's capacity, a
//! founding year) replaces the made-up one only when `Prov::allows_value` says so. Real derbies become a list of names. Nothing else
//! comes from the reference: no result, title, record, rivalry strength or reputation.
//!
//! What this builds, layer by layer:
//! - regions (states and district clusters) with their standing qualities,
//!   state associations and the national federation;
//! - the national pyramid: top flight, second tier, third tier, fourth tier,
//!   plus a national cup, a national U21 development league, national U17/U15/U13
//!   youth leagues and a youth league per state;
//! - state pathway leagues (premier and second division) per state, parallel
//!   to the pyramid and feeding its fourth tier (`pw_sim::ecosystem`);
//! - universities and schools with sports budgets, grassroots and amateur
//!   clubs per district, independent academies.
//!
//! Children are NOT built here. They come out of each district's aggregate
//! participation pool as the simulation runs.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, CompId, Date, LocalClubId, NationId, RegionId, TeamId};
use pw_data::DataPack;
use pw_world::contract::ContractKind;
use pw_world::ecosystem::{Association, Climate, InstProfile, PlayerStory, Provider, Region, RegionKind, StageKind};
use pw_world::pathway::{Creation, Draw};
use pw_world::scenario::{CalEvent, CalRule, DataOrigin, KnownDerby, MarketDef, RecognitionTuning, ReferenceReport, ScoutingTuning};
use pw_world::minor::{InstKind, Institution};
use pw_world::nation::Confed;
use pw_world::youth::{LocalClub, LocalLevel};
use pw_world::{CompKind, Contract, Format, TeamKind, World};
use serde::Deserialize;
use smallvec::SmallVec;
use std::collections::{HashMap, VecDeque};

use crate::builder::{self, ClubSpec};
use crate::india_ref::{self, ClubRow as RefClub, Reference};

const PACK: &str = include_str!("../../../data/worlds/india/pack.toml");

#[derive(Deserialize)]
struct Pack {
    zones: Vec<String>,
    languages: Vec<String>,
    state: Vec<StateRow>,
    university: Vec<UniRow>,
    club: Vec<ClubRow>,
    eligibility: Option<EligRow>,
    /// Tuning of how evidence becomes recognition (initial values; see `pw_world::scenario`).
    recognition: Option<RecognitionTuning>,
    scouting: Option<ScoutingTuning>,
    #[serde(default)]
    calendar: Vec<CalRow>,
    #[serde(default)]
    market: Vec<MarketRow>,
    /// Eligibility for national sides, per nation code.
    #[serde(default)]
    national_rules: Vec<NationalRow>,
}

#[derive(Deserialize)]
struct NationalRow {
    nation: String,
    #[serde(flatten)]
    rules: pw_world::scenario::NationalRules,
}

#[derive(Deserialize)]
struct CalRow {
    event: String,
    months: Vec<u8>,
    #[serde(default)]
    day: u8,
}

#[derive(Deserialize)]
struct MarketRow {
    key: String,
    nations: Vec<String>,
    start: f32,
}

#[derive(Deserialize)]
struct EligRow {
    min_age: u8,
    max_age: u8,
    bases: Vec<String>,
    residence_years: u8,
    exclude_top_division: bool,
    one_state_per_year: bool,
}

#[derive(Deserialize)]
struct StateRow {
    key: String,
    name: String,
    assoc: String,
    pop_m: f32,
    x: u8,
    y: u8,
    climate: String,
    lang: u8,
    zone: u8,
    strength: f32,
    econ: f32,
    pro: f32,
    districts: Vec<String>,
}

#[derive(Deserialize)]
struct UniRow {
    name: String,
    city: String,
    state: String,
    scholarships: u8,
}

#[derive(Deserialize)]
struct ClubRow {
    name: String,
    city: String,
    state: String,
    tier: u8,
    rep: u16,
}

/// How much of India to build.
#[derive(Clone, Copy, Debug)]
pub struct IndiaScale {
    /// States to include, in the pack's order (0 = all).
    pub states: usize,
    /// Clubs in each state premier league.
    pub state_league: usize,
    /// Players per senior squad in the state leagues.
    pub squad: usize,
}

impl IndiaScale {
    /// A handful of states for tests.
    pub const TINY: IndiaScale = IndiaScale { states: 6, state_league: 6, squad: 18 };
    pub const FULL: IndiaScale = IndiaScale { states: 0, state_league: 10, squad: 22 };
}

fn climate(s: &str) -> Climate {
    match s {
        "humid_coastal" => Climate::HumidCoastal,
        "hot_dry" => Climate::HotDry,
        "high_altitude" => Climate::HighAltitude,
        "cool_hill" => Climate::CoolHill,
        "heavy_monsoon" => Climate::HeavyMonsoon,
        "north_east" => Climate::NorthEast,
        _ => Climate::Temperate,
    }
}

/// Name pools by broad region of origin: (first names, surnames). Generic and common, not real people.
fn names(group: usize) -> (&'static [&'static str], &'static [&'static str]) {
    match group {
        0 => (
            &["Arjun", "Rahul", "Aman", "Vikram", "Sandeep", "Harpreet", "Gurpreet", "Manish", "Rohit", "Deepak", "Karan", "Nitin", "Sahil", "Ajay", "Ravi", "Imran"],
            &["Singh", "Kumar", "Sharma", "Verma", "Yadav", "Gill", "Chauhan", "Malik", "Khan", "Bhat", "Dar", "Thakur", "Rawat", "Mehra", "Kapoor", "Bajwa"],
        ),
        1 => (
            &["Subrata", "Sourav", "Debashis", "Anirban", "Biswajit", "Rajib", "Pritam", "Sumit", "Abhijit", "Tapas", "Bibhuti", "Jitendra", "Manoj", "Rupam", "Dipankar", "Hemanta"],
            &["Das", "Ghosh", "Banerjee", "Mondal", "Sen", "Roy", "Bose", "Nayak", "Sahoo", "Behera", "Pradhan", "Mishra", "Bora", "Hazarika", "Dutta", "Saha"],
        ),
        2 => (
            &["Anas", "Sreejith", "Vishnu", "Rahim", "Ashique", "Jithin", "Naveen", "Karthik", "Mohan", "Prasanth", "Sathish", "Arun", "Sajid", "Ganesh", "Suresh", "Vignesh"],
            &["Nair", "Menon", "Pillai", "Thomas", "Kurian", "Rao", "Reddy", "Naidu", "Iyer", "Gowda", "Shetty", "Murugan", "Kumaran", "Hameed", "Varghese", "Krishnan"],
        ),
        3 => (
            &["Rohan", "Aditya", "Pratik", "Omkar", "Siddhesh", "Jayesh", "Dhruv", "Hiren", "Savio", "Cleofas", "Brandon", "Nikhil", "Sagar", "Tejas", "Yash", "Darshan"],
            &["Patil", "Jadhav", "Deshmukh", "Shinde", "Patel", "Shah", "Desai", "Naik", "Fernandes", "Gaonkar", "Kamat", "Pereira", "Mehta", "Joshi", "Kulkarni", "Solanki"],
        ),
        _ => (
            &["Lalrinzuala", "Lallianzuala", "Thoiba", "Bikash", "Kipgen", "Nongsiej", "Ibomcha", "Rakesh", "Sanjeev", "Boris", "Lalthathanga", "Nim", "Joseph", "Tenzing", "Ricky", "Samuel"],
            &["Singh", "Sangma", "Lyngdoh", "Marak", "Thapa", "Tamang", "Khongsit", "Chhangte", "Hmar", "Zote", "Kom", "Tiwari", "Rai", "Lepcha", "Ao", "Angami"],
        ),
    }
}

fn name_group(lang: u8) -> usize {
    match lang {
        0 | 8 | 16 | 7 => 0,
        1 | 9 | 10 => 1,
        2..=5 => 2,
        6 | 14 => 3,
        _ => 4,
    }
}

fn person_names(w: &mut World, lang: u8, rng: &mut Rng) -> (pw_world::NameId, pw_world::NameId) {
    let (f, l) = names(name_group(lang));
    (w.names.intern(f[rng.index(f.len())]), w.names.intern(l[rng.index(l.len())]))
}

fn ability_target(rep: u16) -> f32 {
    40.0 + 100.0 * (f32::from(rep) / 10_000.0).powf(0.9)
}

pub fn build(pack: DataPack, seed: u64, scale: IndiaScale) -> World {
    let data: Pack = toml::from_str(PACK).expect("india pack parses");
    let reference = india_ref::builtin();
    let start = Date::from_ymd(2026, 7, 1);
    let year = start.year();
    let mut w = World::new(pack, seed, start);
    let mut rng = Rng::keyed(&[seed, stream::WORLDGEN, 0x1d1a]);
    let states: Vec<&StateRow> = data.state.iter().take(if scale.states == 0 { usize::MAX } else { scale.states }).collect();

    let india = builder::add_nation(&mut w, "IND", "India", Confed::Afc, 3000, "autumn_spring", 0.35, 9);
    // Foreign recruits come from a few other nations.
    let foreign: Vec<NationId> = [("BRA", "Brazil"), ("ESP", "Spain"), ("AUS", "Australia"), ("JPN", "Japan"), ("KOR", "South Korea"), ("UZB", "Uzbekistan")]
        .iter()
        .map(|(c, name)| builder::add_nation(&mut w, c, name, Confed::Afc, 5000, "autumn_spring", 0.5, 10))
        .collect();

    let eco = &mut w.ext.ecosystem;
    eco.zones = data.zones.clone();
    eco.languages = data.languages.clone();
    eco.last_year = start.year();
    eco.federation = Some(assoc(&mut rng, 0.5, 0.5));
    {
        let pools: Vec<(Vec<pw_world::NameId>, Vec<pw_world::NameId>)> = (0..data.languages.len())
            .map(|l| {
                let (f, sn) = names(name_group(l as u8));
                (f.iter().map(|x| w.names.intern(x)).collect(), sn.iter().map(|x| w.names.intern(x)).collect())
            })
            .collect();
        w.ext.ecosystem.lang_names = pools;
    }
    // The scenario's own tuning and calendar: generic code implements the concepts, this pack configures them.
    {
        let sc = &mut w.ext.scenario;
        sc.source = "data/worlds/india/pack.toml".into();
        if let Some(r) = &data.recognition {
            sc.recognition = r.clone();
        }
        if let Some(s) = &data.scouting {
            sc.scouting = s.clone();
        }
        if !data.calendar.is_empty() {
            sc.calendar = data.calendar.iter().map(|c| CalRule { event: CalEvent::parse(&c.event).unwrap_or_else(|| panic!("india pack: unknown calendar event {}", c.event)), months: c.months.clone(), day: c.day }).collect();
        }
        sc.national = data.national_rules.iter().map(|r| (r.nation.clone(), r.rules.clone())).collect();
        sc.national.sort_by(|a, b| a.0.cmp(&b.0));
        sc.markets = data.market.iter().map(|m| MarketDef { key: m.key.clone(), nations: m.nations.clone(), start: m.start }).collect();
    }
    let eco = &mut w.ext.ecosystem;
    if let Some(e) = &data.eligibility {
        let bases: Vec<_> = e.bases.iter().filter_map(|b| pw_world::ecosystem::Basis::parse(b)).collect();
        eco.eligibility = pw_world::ecosystem::Eligibility { min_age: e.min_age, max_age: e.max_age, bases, residence_years: e.residence_years, exclude_top_division: e.exclude_top_division, one_state_per_year: e.one_state_per_year };
    }

    // ------------------------------------------------------------ regions
    let mut state_region: Vec<(String, RegionId)> = Vec::new();
    let mut districts_of: Vec<(String, Vec<(String, RegionId)>)> = Vec::new();
    for s in &states {
        let base = |r: &mut Rng, m: f32, sd: f32| (m + r.normal() * sd).clamp(2.0, 98.0);
        let mk = |r: &mut Rng, kind: RegionKind, name: &str, parent: RegionId, pop_k: u32, jitter: f32| Region {
            name: name.to_string(),
            nation: india,
            kind,
            parent,
            x: (f32::from(s.x) + r.normal() * jitter).clamp(0.0, 100.0) as u8,
            y: (f32::from(s.y) + r.normal() * jitter).clamp(0.0, 100.0) as u8,
            climate: climate(&s.climate),
            language: s.lang,
            zone: s.zone,
            population_k: pop_k,
            participation: base(r, s.strength * 60.0 + 5.0, 6.0),
            facilities: base(r, 15.0 + s.pro * 25.0 + s.econ * 25.0, 6.0),
            coach_density: base(r, 12.0 + s.strength * 30.0 + s.econ * 15.0, 6.0),
            academy_access: base(r, s.pro * 60.0, 10.0),
            economic_access: base(r, s.econ * 90.0, 8.0),
            pro_proximity: base(r, s.pro * 90.0, 12.0),
            competition_density: base(r, 10.0 + s.strength * 45.0, 6.0),
            scouting_coverage: base(r, 8.0 + s.pro * 40.0 + s.strength * 15.0, 7.0),
            invest_public: base(r, 25.0, 6.0),
            invest_private: base(r, 20.0 + s.econ * 15.0, 6.0),
            culture: base(r, s.strength * 70.0, 5.0),
        };
        let state_id = w.ext.ecosystem.regions.push(mk(&mut rng, RegionKind::State, &s.name, RegionId::NONE, (s.pop_m * 1000.0) as u32, 0.0));
        w.ext.ecosystem.assoc.insert(state_id, assoc(&mut rng, s.econ, s.strength));
        w.ext.ecosystem.assoc_name.insert(state_id, s.assoc.clone());
        state_region.push((s.key.clone(), state_id));
        let n = s.districts.len().max(1);
        let mut ds = Vec::new();
        for d in &s.districts {
            let id = w.ext.ecosystem.regions.push(mk(&mut rng, RegionKind::District, d, state_id, (s.pop_m * 1000.0 / n as f32) as u32, 4.0));
            w.ext.ecosystem.pools.insert(id, Default::default());
            ds.push((d.clone(), id));
        }
        districts_of.push((s.key.clone(), ds));
    }
    let state_ix = |key: &str| states.iter().position(|s| s.key == key);
    let region_of_state = |key: &str| state_region.iter().find(|(k, _)| k == key).map(|x| x.1);
    let districts = |key: &str| districts_of.iter().find(|(k, _)| k == key).map(|x| x.1.clone()).unwrap_or_default();
    let district_near = |key: &str, city: &str| -> RegionId {
        let ds = districts(key);
        ds.iter().find(|(n, _)| city.contains(n.as_str()) || n.contains(city)).or(ds.first()).map_or(RegionId::NONE, |x| x.1)
    };
    // The district a real club's city names, when the pack lists that district; else the caller's choice.
    let district_of_city = |key: &str, city: &str| -> Option<RegionId> { districts(key).iter().find(|(n, _)| city.contains(n.as_str()) || n.contains(city)).map(|x| x.1) };
    // A reference state id (`state.wb`) as the pack's key (`WB`), for the states this world has.
    let key_of = |state_id: &str| states.iter().find(|s| reference.state_of_key(&s.key).is_some_and(|r| r.id == state_id)).map(|s| s.key.clone());

    // ------------------------------------------------------------ competitions
    let tier_sizes = [13usize, 12, 16, 20];
    let mut pyramid: Vec<CompId> = Vec::new();
    let names = ["Indian Super League", "Indian Football League", "I-League 2", "I-League 3"];
    let shorts = ["ISL", "IFL", "IL2", "IL3"];
    for t in 0..4 {
        let rep = [5000u16, 3400, 2200, 1400][t];
        pyramid.push(builder::add_comp(
            &mut w,
            names[t],
            shorts[t],
            india,
            None,
            CompKind::League,
            (t + 1) as u8,
            TeamKind::First,
            tier_sizes[t] as u16,
            if t > 0 { 2 } else { 0 },
            if t < 3 { 2 } else { 0 },
            rep,
            Format::League { rounds: 2 },
            i64::from(rep) * 6_000,
        ));
    }
    builder::add_comp(&mut w, "Super Cup", "Super Cup", india, None, CompKind::Cup, 2, TeamKind::First, 0, 0, 0, 2500, Format::Knockout { legs: 1, final_legs: 1 }, 20_000_000);
    let state_prem: Vec<(String, CompId, CompId)> = states
        .iter()
        .map(|s| {
            let p = builder::add_comp(&mut w, &format!("{} Premier League", s.name), &format!("{} PL", s.key), india, None, CompKind::League, 50, TeamKind::First, scale.state_league as u16, 0, 0, 700, Format::League { rounds: 2 }, 500_000);
            let d = builder::add_comp(&mut w, &format!("{} Second Division", s.name), &format!("{} D2", s.key), india, None, CompKind::League, 51, TeamKind::First, scale.state_league as u16, 0, 0, 400, Format::League { rounds: 2 }, 200_000);
            (s.key.clone(), p, d)
        })
        .collect();
    // Development and youth leagues (entrants: the best sides of each kind; state leagues get their state's).
    let rfdl = builder::add_comp(&mut w, "Development League (U21)", "U21 Dev", india, None, CompKind::League, 1, TeamKind::U21, 24, 0, 0, 1800, Format::League { rounds: 2 }, 0);
    let _ = rfdl;
    builder::add_comp(&mut w, "National Youth League U17", "U17", india, None, CompKind::League, 1, TeamKind::U18, 24, 0, 0, 1200, Format::League { rounds: 2 }, 0);
    builder::add_comp(&mut w, "National Youth League U15", "U15", india, None, CompKind::League, 1, TeamKind::U16, 24, 0, 0, 900, Format::League { rounds: 2 }, 0);
    builder::add_comp(&mut w, "National Youth League U13", "U13", india, None, CompKind::League, 1, TeamKind::U14, 24, 0, 0, 700, Format::League { rounds: 2 }, 0);
    let state_youth: Vec<(String, CompId)> =
        states.iter().map(|s| (s.key.clone(), builder::add_comp(&mut w, &format!("{} Youth League", s.name), &format!("{} Youth", s.key), india, None, CompKind::League, 60, TeamKind::U18, 0, 0, 0, 500, Format::League { rounds: 2 }, 0))).collect();

    // ------------------------------------------------------------ clubs
    struct Made {
        club: ClubId,
        tier: u8,
        key: String,
        region: RegionId,
        rep: u16,
        lang: u8,
    }
    let mut made: Vec<Made> = Vec::new();
    // `region` is where the club sits; `rr` is the reference record the club is, if it is one (a real club of the reference, matched by
    // id or by exact name and state). The random draws inside do not depend on it, so a world built with and without reference data
    // draws the same stream.
    let mut spawn = |w: &mut World, rng: &mut Rng, name: &str, city: &str, key: &str, region: RegionId, tier: u8, rep: u16, league: CompId, extra: &[TeamKind], origin: DataOrigin, rr: Option<&RefClub>| -> ClubId {
        let club = builder::add_club(
            w,
            ClubSpec {
                name,
                short: "",
                nation: india,
                city,
                league,
                reputation: rep,
                balance: i64::from(rep) * 2_500,
                stadium: "",
                capacity: u32::from(rep) * 4 + 1_500,
                facilities: builder::default_facilities(rep),
                colors: [rng.next_u32() & 0xffffff, 0xffffff],
                founded: 1900 + rng.below(100) as u16,
                extra_teams: extra,
            },
        );
        if let Some(r) = rr {
            apply_reference(w, club, r, reference);
        }
        let lang = state_ix(key).map_or(0, |i| states[i].lang);
        w.ext.ecosystem.club_region.insert(club, region);
        w.ext.scenario.club_origin.insert(club, origin);
        made.push(Made { club, tier, key: key.to_string(), region, rep, lang });
        club
    };
    let big: &[TeamKind] = &[TeamKind::U21, TeamKind::U18];
    let small: &[TeamKind] = &[TeamKind::U18];
    let mut counts = [0usize; 5];
    // Reference clubs that are clubs of this world, by reference id; and the problems the builder met matching the pack to the reference.
    let mut placed: HashMap<String, ClubId> = HashMap::new();
    let mut build_findings: Vec<String> = Vec::new();
    let mut from_reference = 0u32;
    for c in data.club.iter().filter(|c| state_ix(&c.state).is_some()) {
        let tier = usize::from(c.tier);
        if tier >= 1 && counts[tier] >= tier_sizes[tier - 1] {
            continue;
        }
        counts[tier] += 1;
        let (league, extra) = match c.tier {
            1..=3 => (pyramid[tier - 1], big),
            4 => (pyramid[3], small),
            _ => (CompId::NONE, big),
        };
        // The pack names the club; the reference confirms it only by an exact name in the same state.
        let state_id = reference.state_of_key(&c.state).map(|s| s.id.clone()).unwrap_or_default();
        let rr = reference.club_exact(&c.name, &state_id).filter(|r| r.prov.names_real_entity() && !placed.contains_key(&r.id));
        if rr.is_none() {
            build_findings.push(format!("pack club {} ({}) has no reference record with exactly that name in that state: it stays a scenario seed", c.name, c.state));
        }
        let origin = rr.map_or(DataOrigin::ScenarioSeed, |r| r.prov.origin());
        let region = district_near(&c.state, &c.city);
        let club = spawn(&mut w, &mut rng, &c.name, &c.city, &c.state, region, c.tier, c.rep, league, extra, origin, rr);
        if let Some(r) = rr {
            placed.insert(r.id.clone(), club);
        }
    }
    // Fill each tier from the states' own districts, so divisions are full and every club sits somewhere real. A place the builder
    // would make up goes instead to a real club the reference lists in that tier's competition (in the order it lists them) when its
    // state is in this world; it takes the place's strength. What is left over is generated as before.
    let mut pools: Vec<VecDeque<&RefClub>> = (1..=4u8)
        .map(|t| reference.national_league(t).map(|c| reference.members(&c.id, year).into_iter().filter(|m| key_of(&m.state).is_some()).collect()).unwrap_or_default())
        .collect();
    let suffix = ["FC", "United", "Athletic", "Sporting", "Rovers"];
    for tier in 1..=4usize {
        let mut k = 0;
        while counts[tier] < tier_sizes[tier - 1] {
            let s = states[k % states.len()];
            let d = &s.districts[(k / states.len()) % s.districts.len()];
            let rep = [4200u16, 2900, 1900, 1200][tier - 1] - (k as u16 % 5) * 90;
            // A made-up name is never a real club's name: take the next suffix if the reference has a club of exactly that name here.
            let state_id = reference.state_of_key(&s.key).map_or("", |r| r.id.as_str());
            let name = (0..suffix.len())
                .map(|j| format!("{d} {}", suffix[(k + tier + j) % suffix.len()]))
                .find(|n| !reference.names_club(n, state_id))
                .unwrap_or_else(|| format!("{d} {} II", suffix[(k + tier) % suffix.len()]));
            counts[tier] += 1;
            k += 1;
            let extra = if tier <= 3 { big } else { small };
            let real = loop {
                match pools[tier - 1].pop_front() {
                    Some(r) if placed.contains_key(&r.id) => continue,
                    other => break other,
                }
            };
            if let Some(r) = real {
                let key = key_of(&r.state).expect("the pool holds only clubs of states in this world");
                let in_state = districts(&key);
                let region = district_of_city(&key, &r.city).unwrap_or_else(|| in_state.get((k + tier) % in_state.len().max(1)).map_or(RegionId::NONE, |x| x.1));
                let club = spawn(&mut w, &mut rng, &r.name, &r.city, &key, region, tier as u8, rep, pyramid[tier - 1], extra, r.prov.origin(), Some(r));
                placed.insert(r.id.clone(), club);
                from_reference += 1;
            } else {
                let region = district_near(&s.key, d);
                spawn(&mut w, &mut rng, &name, d, &s.key, region, tier as u8, rep, pyramid[tier - 1], extra, DataOrigin::Generated, None);
            }
        }
    }
    // State leagues: a state's premier league takes the real clubs the reference lists in the state's top league, then generated
    // district clubs under the state's association. The second division has no reference data and stays generated.
    for (key, prem, div2) in &state_prem {
        let ds = districts(key);
        if ds.is_empty() {
            continue;
        }
        let state_id = reference.state_of_key(key).map(|s| s.id.clone()).unwrap_or_default();
        let mut pool: VecDeque<&RefClub> = reference
            .state_top_league(&state_id, year)
            .map(|c| reference.members(&c.id, year).into_iter().filter(|m| m.state == state_id).collect())
            .unwrap_or_default();
        for (league, offset, rep0) in [(*prem, 0usize, 900u16), (*div2, scale.state_league, 500u16)] {
            for k in 0..scale.state_league {
                let (dname, _) = &ds[(k + offset) % ds.len()];
                let number = if k >= ds.len() { format!(" {}", k / ds.len() + 1) } else { String::new() };
                let name = (0..suffix.len())
                    .map(|j| format!("{dname} {}{number}", suffix[(k + offset + j) % suffix.len()]))
                    .find(|n| !reference.names_club(n, &state_id))
                    .unwrap_or_else(|| format!("{dname} {} II{number}", suffix[(k + offset) % suffix.len()]));
                let rep = rep0.saturating_sub(k as u16 * 25);
                let real = if league == *prem {
                    loop {
                        match pool.pop_front() {
                            Some(r) if placed.contains_key(&r.id) => continue,
                            other => break other,
                        }
                    }
                } else {
                    None
                };
                let extra: &[TeamKind] = if league == *prem { small } else { &[] };
                if let Some(r) = real {
                    let region = district_of_city(key, &r.city).unwrap_or_else(|| district_near(key, dname));
                    let club = spawn(&mut w, &mut rng, &r.name, &r.city, key, region, 5, rep, league, extra, r.prov.origin(), Some(r));
                    placed.insert(r.id.clone(), club);
                    from_reference += 1;
                } else {
                    let region = district_near(key, dname);
                    spawn(&mut w, &mut rng, &name, dname, key, region, 5, rep, league, extra, DataOrigin::Generated, None);
                }
            }
        }
    }
    // What the reference says about this world, kept as labels and counts: the derbies between clubs it has, and what loading found.
    {
        let sc = &mut w.ext.scenario;
        sc.known_derbies = reference
            .derbies()
            .into_iter()
            .filter_map(|d| Some(KnownDerby { source_id: d.id.to_string(), name: d.name.to_string(), a: *placed.get(d.a)?, b: *placed.get(d.b)?, derby: d.is_derby, origin: d.prov.origin() }))
            .collect();
        let mut problems: Vec<String> = reference.findings.iter().map(ToString::to_string).collect();
        problems.extend(build_findings);
        sc.reference = ReferenceReport {
            files: reference.files as u32,
            records: reference.records(),
            by_status: reference.by_status,
            findings: problems.len() as u32,
            finding_samples: problems.into_iter().take(8).collect(),
            clubs_matched: placed.len() as u32,
            clubs_from_reference: from_reference,
        };
    }

    // ------------------------------------------------------------ players
    let clubs: Vec<(ClubId, u8, u16, RegionId, u8, String)> = made.iter().map(|m| (m.club, m.tier, m.rep, m.region, m.lang, m.key.clone())).collect();
    for (club, tier, rep, region, lang, _) in clubs {
        let teams: Vec<TeamId> = w.clubs[club].teams.to_vec();
        let target = ability_target(rep);
        for t in teams {
            let kind = w.teams[t].kind;
            let (count, ages) = match kind {
                TeamKind::First if tier == 0 => (0, (19, 30)),
                TeamKind::First => (if tier == 5 { scale.squad } else { 24 }, (18, 34)),
                TeamKind::U21 => (18, (17, 20)),
                _ => (18, (15, 17)),
            };
            for i in 0..count {
                let age = rng.range_i32(ages.0, ages.1);
                let dob = start.add_days(-(age * 365 + rng.range_i32(0, 364)));
                let pos = if i == 0 || i == 1 && count > 18 { pw_core::Pos::GK } else { pw_sim::generate::random_position(&mut rng) };
                let youth_scale = pw_sim::generate::ca_share_at(age as f32);
                let pa = rng.normal_ms(target + 10.0, 14.0).clamp(30.0, 190.0);
                let ca = (pa * youth_scale * rng.normal_ms(1.0, 0.07)).clamp(15.0, pa);
                // Foreigners in the top two tiers (their number is governed by rules, not here).
                let is_foreign = kind == TeamKind::First && tier <= 2 && i >= count.saturating_sub(if tier == 1 { 4 } else { 2 });
                let nat = if is_foreign { foreign[rng.index(foreign.len())] } else { india };
                let contract = Contract {
                    club,
                    kind: if age < 17 { ContractKind::Youth } else { ContractKind::Professional },
                    wage: 0,
                    start,
                    end: Date::from_ymd(2027 + rng.range_i32(0, 3), 6, 30),
                    yearly_rise: 3,
                    ..Default::default()
                };
                let np = pw_sim::people::NewPlayer { nation: nat, dob, pos, ca, pa: pa as u8, club, team: t, contract, source: pw_world::player::PlayerSource::SyntheticFixture };
                let p = pw_sim::people::spawn_player(&mut w, np, &mut rng);
                let (f, l) = person_names(&mut w, if is_foreign { 18 } else { lang }, &mut rng);
                let person = w.players.cold[p].person;
                w.people[person].first = f;
                w.people[person].last = l;
                if !is_foreign {
                    w.ext.ecosystem.story.insert(p, PlayerStory { home: region, dev: region, provider: Provider::Community, found_by: pw_core::PersonId::NONE, found_club: ClubId::NONE, found_on: start });
                    // Made to populate the first year: recorded as that, with nothing about how he was found or where he learned.
                    let first_env = match (kind, tier) {
                        (TeamKind::First, 5) => StageKind::StateLeague,
                        (TeamKind::First, _) => StageKind::Professional,
                        _ => StageKind::Academy,
                    };
                    w.ext.pathway.created.insert(p, Creation { date: start, region, provider: Provider::Community, institution: None, age: Some(age as u8), first_env, first_finder: pw_core::PersonId::NONE, why: Draw::WorldStart, legacy: false });
                }
            }
        }
    }

    // ------------------------------------------------------------ overseas
    // A few clubs abroad, so that being seen can lead somewhere: their scouts come to Indian events when the
    // country's export reputation is high enough, and Indian players who go there are what raises it.
    for (n, (nat, code)) in [(foreign[1], "ESP"), (foreign[3], "JPN"), (foreign[4], "KOR")].into_iter().enumerate() {
        let country = w.nations[nat].name.clone();
        let league = builder::add_comp(&mut w, &format!("{country} Premier Division"), &format!("{code} PD"), nat, None, CompKind::League, 1, TeamKind::First, 6, 0, 0, 6000, Format::League { rounds: 2 }, 20_000_000);
        for k in 0..6usize {
            let rep = 6800u16 - (k as u16) * 380 - (n as u16) * 200;
            let (name, city) = foreign_club_name(code, k, n);
            let club = builder::add_club(
                &mut w,
                ClubSpec {
                    name: &name,
                    short: "",
                    nation: nat,
                    city: &city,
                    league,
                    reputation: rep,
                    balance: i64::from(rep) * 9_000,
                    stadium: "",
                    capacity: u32::from(rep) * 5 + 3_000,
                    facilities: builder::default_facilities(rep),
                    colors: [rng.next_u32() & 0xffffff, 0xffffff],
                    founded: 1900 + rng.below(100) as u16,
                    extra_teams: &[TeamKind::U18],
                },
            );
            w.ext.scenario.club_origin.insert(club, DataOrigin::Generated);
            let target = ability_target(rep);
            let teams: Vec<TeamId> = w.clubs[club].teams.to_vec();
            for t in teams {
                let count = if w.teams[t].kind == TeamKind::First { 24 } else { 18 };
                for i in 0..count {
                    let ages = if w.teams[t].kind == TeamKind::First { (18, 34) } else { (15, 17) };
                    let age = rng.range_i32(ages.0, ages.1);
                    let dob = start.add_days(-(age * 365 + rng.range_i32(0, 364)));
                    let pos = if i < 2 { pw_core::Pos::GK } else { pw_sim::generate::random_position(&mut rng) };
                    let pa = rng.normal_ms(target + 10.0, 14.0).clamp(30.0, 190.0);
                    let ca = (pa * pw_sim::generate::ca_share_at(age as f32) * rng.normal_ms(1.0, 0.07)).clamp(15.0, pa);
                    let contract = Contract { club, kind: if age < 17 { ContractKind::Youth } else { ContractKind::Professional }, wage: 0, start, end: Date::from_ymd(2027 + rng.range_i32(0, 3), 6, 30), yearly_rise: 3, ..Default::default() };
                    let np = pw_sim::people::NewPlayer { nation: nat, dob, pos, ca, pa: pa as u8, club, team: t, contract, source: pw_world::player::PlayerSource::SyntheticFixture };
                    let p = pw_sim::people::spawn_player(&mut w, np, &mut rng);
                    let (f, l) = person_names(&mut w, 18, &mut rng);
                    let person = w.players.cold[p].person;
                    w.people[person].first = f;
                    w.people[person].last = l;
                }
            }
        }
    }

    // ------------------------------------------------------------ institutions
    for s in &states {
        let region = region_of_state(&s.key).unwrap_or(RegionId::NONE);
        let ds = districts(&s.key);
        let add = |w: &mut World, rng: &mut Rng, kind: InstKind, name: String, city: &str, region: RegionId, prestige: u16, coaching: u8, resources: f32, sch: u8, residential: bool, real: bool| {
            let id = w.minor.institutions.len() as u32;
            w.minor.institutions.push(Institution { id, kind, name, nation: india, city: city.to_string(), founded: 1880 + rng.below(140) as i32, prestige, coaching, members: Vec::new(), alumni_pros: SmallVec::new() });
            w.ext.ecosystem.inst.insert(id, InstProfile { region, resources, facilities: resources * 0.8, scholarships: sch, residential, success: 30.0, real });
        };
        for (d, rid) in &ds {
            // Schools: more where there are more children and more football.
            let pop = w.ext.ecosystem.regions[*rid].population_k;
            let n = (pop / 4000).clamp(2, 6) as usize;
            for k in 0..n {
                let (prestige, coaching, res) = (rng.range_i32(100, 700) as u16, rng.range_i32(2, 8) as u8, 20.0 + rng.range_f32(0.0, 25.0));
                let label = ["Government School", "Public School", "Model School", "Higher Secondary School", "Convent School", "Vidyalaya"][k % 6];
                add(&mut w, &mut rng, InstKind::School, format!("{d} {label}"), d, *rid, prestige, coaching, res, 0, false, false);
            }
        }
        // A state whose reference lists a real sports school, hostel or SAI centre gets that one (`india_lore`), not a made-up hostel.
        let real_hostel = reference
            .state_of_key(&s.key)
            .is_some_and(|sr| reference.schools.iter().any(|x| x.state == sr.id && x.prov.names_real_entity() && matches!(x.kind.as_str(), "sports_school" | "sports_hostel" | "sai_centre")));
        if let (Some((d, rid)), false) = (ds.first(), real_hostel) {
            add(&mut w, &mut rng, InstKind::School, format!("{} State Sports Hostel", s.name), d, *rid, 400, 7, 45.0, 6, true, false);
        }
        let _ = region;
    }
    for u in data.university.iter().filter(|u| state_ix(&u.state).is_some()) {
        let region = district_near(&u.state, &u.city);
        let id = w.minor.institutions.len() as u32;
        let c = rng.range_i32(6, 12) as u8;
        w.minor.institutions.push(Institution { id, kind: InstKind::University, name: u.name.clone(), nation: india, city: u.city.clone(), founded: 1900, prestige: rng.range_i32(300, 800) as u16, coaching: c, members: Vec::new(), alumni_pros: SmallVec::new() });
        w.ext.ecosystem.inst.insert(id, InstProfile { region, resources: 25.0 + f32::from(u.scholarships) * 3.0, facilities: 30.0, scholarships: u.scholarships, residential: true, success: 30.0, real: true });
    }

    // ------------------------------------------------------------ grassroots and amateur clubs per district
    for s in &states {
        for (d, rid) in districts(&s.key) {
            let reg = w.ext.ecosystem.regions[rid].clone();
            for k in 0..3 {
                let level = if k == 2 { LocalLevel::Amateur } else { LocalLevel::Grassroots };
                let name = match k {
                    0 => format!("{d} Grassroots Centre"),
                    1 => format!("{d} Community FC"),
                    _ => format!("{d} Amateur FC"),
                };
                let id: LocalClubId = w.youth.local.push(LocalClub {
                    name,
                    nation: india,
                    city: d.clone(),
                    level,
                    coaching: (2.0 + reg.coach_density / 12.0 + rng.range_f32(0.0, 2.0)).min(14.0) as u8,
                    facilities: (2.0 + reg.facilities / 12.0 + rng.range_f32(0.0, 2.0)).min(14.0) as u8,
                    feeder_of: ClubId::NONE,
                    standing: rng.range_i32(50, 500) as u16,
                    members: Vec::new(),
                });
                w.ext.ecosystem.local_region.insert(id, rid);
            }
        }
    }

    // Names, institutions, press and words from the reference data (see `india_lore`). Its draws come from a stream of their own, so
    // the rest of the world is drawn exactly as it was without them.
    {
        let lore_states: Vec<crate::india_lore::StateOf> =
            states.iter().filter_map(|s| Some(crate::india_lore::StateOf { key: s.key.clone(), region: region_of_state(&s.key)?, districts: districts(&s.key) })).collect();
        let premier: Vec<(String, CompId)> = state_prem.iter().map(|(k, p, _)| (k.clone(), *p)).collect();
        let built = crate::india_lore::Built { nation: india, year, states: &lore_states, placed: &placed, pyramid: &pyramid, state_premier: &premier };
        let mut lore_rng = Rng::keyed(&[seed, stream::WORLDGEN, 0x10e5]);
        crate::india_lore::apply(&mut w, reference, &built, &mut lore_rng);
    }
    builder::finalize(&mut w);
    // The pyramid is exactly the four national tiers; state leagues run alongside and feed the fourth.
    w.nations[india].leagues = pyramid.clone();
    for c in w.comps.ids().collect::<Vec<_>>() {
        if w.comps[c].tier >= 50 {
            w.comps[c].above = CompId::NONE;
            w.comps[c].below = CompId::NONE;
        }
    }
    w.comps[pyramid[3]].below = CompId::NONE;
    // State youth leagues take their state's clubs' U18 sides.
    for (key, comp) in &state_youth {
        let region = region_of_state(key).unwrap_or(RegionId::NONE);
        let mut teams: Vec<TeamId> = w
            .clubs
            .iter_enumerated()
            .filter(|(id, _)| w.ext.ecosystem.state_of(w.ext.ecosystem.region_of_club(*id)) == region)
            .filter_map(|(id, c)| c.teams.iter().copied().find(|&t| w.teams[t].kind == TeamKind::U18).map(|t| (id, t)))
            .map(|x| x.1)
            .collect();
        teams.sort();
        w.comps[*comp].size = teams.len() as u16;
        w.comps[*comp].state.entrants = teams;
    }
    builder::ensure_staff(&mut w);
    for p in w.players.ids() {
        let club = w.players.hot[p].club;
        if club.is_some() && w.players.cold[p].contract.wage == 0 {
            w.players.cold[p].contract.wage = pw_sim::market::wage_demand(&w, p, club);
        }
    }
    w
}

/// What a made-up club abroad is called, in the way clubs of its country are, and the made-up place it is from. Invented names, never
/// real clubs; their origin is Generated.
fn foreign_club_name(code: &str, k: usize, n: usize) -> (String, String) {
    let i = (k + n) % 6;
    match code {
        "ESP" => {
            let place = ["Norte", "Sur", "Este", "Oeste", "Centro", "Puerto"][i];
            (format!("{} {place}", ["Atlético", "Unión", "Deportivo", "Real", "Racing", "Sporting"][k]), place.to_string())
        }
        "JPN" => {
            let place = ["Kita", "Minami", "Higashi", "Nishi", "Chuo", "Minato"][i];
            (format!("{place} {}", ["FC", "United", "SC", "Athletic", "City", "Rovers"][k]), place.to_string())
        }
        "KOR" => {
            let place = ["Bukbu", "Nambu", "Dongbu", "Seobu", "Jungang", "Hanggu"][i];
            (format!("{place} {}", ["FC", "United", "Citizen", "Athletic", "City", "Dragons"][k]), place.to_string())
        }
        _ => {
            let place = ["North", "South", "East", "West", "Central", "Harbour"][i];
            (format!("{place} {}", ["United", "City", "Athletic", "Rovers", "Wanderers", "Albion"][k]), place.to_string())
        }
    }
}

/// A reference record applied to the club the builder made for it. The short name is identity and always comes. Numbers replace the
/// made-up ones only where `Prov::allows_value` says the record may give a number: the club's city and founding year from the club's
/// own record, and a ground (its name and its capacity together, so a real name never sits beside an invented capacity) from the
/// stadium record the club is tied to by id or exact name, only when that record dates its capacity. Absent values change nothing.
fn apply_reference(w: &mut World, club: ClubId, r: &RefClub, reference: &Reference) {
    let c = &mut w.clubs[club];
    if let Some(short) = r.short.as_deref().filter(|s| !s.is_empty()) {
        c.short_name = short.to_string();
    }
    if r.prov.allows_value() {
        c.city = r.city.clone();
        if let Some(y) = r.founded.filter(|y| (1850..=2026).contains(y)) {
            c.founded = y as u16;
        }
    }
    if let Some(st) = reference.stadium_of(r) {
        if st.prov.names_real_entity() && st.prov.allows_value() && st.capacity_as_of.is_some() {
            if let Some(cap) = st.capacity.filter(|cap| *cap > 0) {
                c.stadium = st.name.clone();
                c.capacity = cap;
            }
        }
    }
}

fn assoc(rng: &mut Rng, econ: f32, strength: f32) -> Association {
    let mut v = |m: f32| (m + rng.normal() * 8.0).clamp(10.0, 95.0);
    Association {
        admin: v(35.0 + econ * 25.0),
        youth_invest: v(30.0 + strength * 30.0),
        coach_ed: v(28.0 + econ * 20.0 + strength * 12.0),
        comp_quality: v(30.0 + strength * 35.0),
        scouting: v(25.0 + strength * 30.0 + econ * 10.0),
        finance: v(30.0 + econ * 35.0),
        grassroots_reach: v(28.0 + strength * 35.0),
        academy_coord: v(25.0 + econ * 20.0),
        referee_dev: v(30.0 + econ * 20.0),
        facilities: v(28.0 + econ * 30.0),
        commercial: v(20.0 + econ * 35.0),
        governance: v(45.0),
    }
}
