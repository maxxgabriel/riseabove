use std::path::Path;

use pw_core::rng::{Rng, hash_key, stream};
use pw_core::{Attr, Attrs, ClubId, CompId, Date, Hidden, NationId, PlayerId, StaffAttr, StaffAttrs, TeamId};
use pw_data::DataPack;
use pw_world::contract::{ContractKind, Loan};
use pw_world::nation::Confed;
use pw_world::player::Reputation;
use pw_world::staff::ManagerRecord;
use pw_world::{CompKind, Contract, Format, Philosophy, PlayerCold, PlayerHot, PlayerStatus, SquadStatus, Staff, StaffRole, Team, TeamKind, World};
use rustc_hash::FxHashMap;

use crate::builder::{self, ClubSpec};
use crate::positions::parse_positions;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("{file}: {source}")]
    Csv { file: String, source: csv::Error },
    #[error("missing required file {0}")]
    Missing(String),
    #[error("{file} row {row}: {message}")]
    Row { file: String, row: usize, message: String },
    #[error("world.toml: {0}")]
    Config(String),
}

#[derive(Debug, Default)]
pub struct ImportReport {
    pub nations: usize,
    pub competitions: usize,
    pub clubs: usize,
    pub players: usize,
    pub staff: usize,
    pub warnings: Vec<String>,
}

impl ImportReport {
    fn warn(&mut self, msg: String) {
        if self.warnings.len() < 200 {
            self.warnings.push(msg);
        }
    }
}

struct Table {
    file: String,
    cols: FxHashMap<String, usize>,
    rows: Vec<csv::StringRecord>,
}

impl Table {
    fn load(dir: &Path, name: &str, required: bool) -> Result<Option<Table>, ImportError> {
        let path = dir.join(name);
        if !path.exists() {
            return if required { Err(ImportError::Missing(name.into())) } else { Ok(None) };
        }
        let err = |source| ImportError::Csv { file: name.into(), source };
        let mut rdr = csv::ReaderBuilder::new().flexible(true).trim(csv::Trim::All).from_path(&path).map_err(err)?;
        let cols = rdr.headers().map_err(err)?.iter().enumerate().map(|(i, h)| (h.trim().trim_start_matches('\u{feff}').to_ascii_lowercase(), i)).collect();
        let rows = rdr.records().collect::<Result<Vec<_>, _>>().map_err(err)?;
        Ok(Some(Table { file: name.into(), cols, rows }))
    }

    fn s<'a>(&self, r: &'a csv::StringRecord, col: &str) -> &'a str {
        self.cols.get(col).and_then(|&i| r.get(i)).unwrap_or("").trim()
    }

    fn num<T: std::str::FromStr>(&self, r: &csv::StringRecord, col: &str) -> Option<T> {
        let s = self.s(r, col).replace([',', '_'], "");
        if s.is_empty() { None } else { s.parse().ok() }
    }

    fn err(&self, row: usize, message: impl Into<String>) -> ImportError {
        ImportError::Row { file: self.file.clone(), row: row + 2, message: message.into() }
    }
}

fn parse_date(s: &str) -> Option<Date> {
    let s = s.trim();
    let parts: Vec<&str> = s.split(['-', '/', '.']).collect();
    let [a, b, c] = parts[..] else { return None };
    let (y, m, d) = if a.len() == 4 { (a.parse().ok()?, b.parse().ok()?, c.parse().ok()?) } else { (c.parse().ok()?, b.parse().ok()?, a.parse().ok()?) };
    ((1..=12).contains(&m) && (1..=31).contains(&d)).then(|| Date::from_ymd(y, m, d))
}

fn parse_color(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0x444444)
}

fn team_kind(s: &str) -> TeamKind {
    match s.to_ascii_lowercase().as_str() {
        "reserve" | "reserves" | "b" | "ii" => TeamKind::Reserve,
        "u21" | "u23" => TeamKind::U21,
        "u19" => TeamKind::U19,
        "u18" | "u17" | "youth" => TeamKind::U18,
        _ => TeamKind::First,
    }
}

fn squad_status(s: &str) -> Option<SquadStatus> {
    Some(match s.to_ascii_lowercase().replace([' ', '-'], "_").as_str() {
        "star" | "star_player" => SquadStatus::Star,
        "important" | "important_player" => SquadStatus::Important,
        "regular" | "regular_starter" | "first_team" => SquadStatus::Regular,
        "squad" | "squad_player" => SquadStatus::Squad,
        "impact_sub" => SquadStatus::ImpactSub,
        "fringe" | "fringe_player" => SquadStatus::Fringe,
        "backup" | "emergency_backup" => SquadStatus::Backup,
        "youngster" | "hot_prospect" | "prospect" => SquadStatus::Youngster,
        "not_needed" => SquadStatus::NotNeeded,
        _ => return None,
    })
}

fn staff_role(s: &str) -> Option<StaffRole> {
    Some(match s.to_ascii_lowercase().replace([' ', '-'], "_").as_str() {
        "manager" | "head_coach" => StaffRole::Manager,
        "assistant" | "assistant_manager" => StaffRole::Assistant,
        "coach" | "first_team_coach" | "youth_coach" => StaffRole::Coach,
        "gk_coach" | "goalkeeping_coach" => StaffRole::GkCoach,
        "fitness_coach" => StaffRole::FitnessCoach,
        "scout" | "chief_scout" => StaffRole::Scout,
        "physio" | "physiotherapist" => StaffRole::Physio,
        "sports_scientist" => StaffRole::SportsScientist,
        "head_of_youth" | "head_of_youth_development" => StaffRole::HeadOfYouth,
        "director_of_football" | "dof" => StaffRole::DirectorOfFootball,
        _ => return None,
    })
}

#[derive(serde::Deserialize, Default)]
struct WorldConfig {
    start_date: Option<String>,
    seed: Option<u64>,
}

/// Load a world from an import folder (see `data/IMPORT_FORMAT.md`).
pub fn load_dir(dir: &Path, pack: DataPack) -> Result<(World, ImportReport), ImportError> {
    let cfg: WorldConfig = match std::fs::read_to_string(dir.join("world.toml")) {
        Ok(s) => toml::from_str(&s).map_err(|e| ImportError::Config(e.to_string()))?,
        Err(_) => WorldConfig::default(),
    };
    let start = cfg.start_date.as_deref().and_then(parse_date).unwrap_or(Date::from_ymd(2022, 7, 1));
    let seed = cfg.seed.unwrap_or_else(|| hash_key(&[dir.to_string_lossy().bytes().fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b)))]));
    let mut w = World::new(pack, seed, start);
    let mut rep = ImportReport::default();

    // Nations.
    let t = Table::load(dir, "nations.csv", true)?.expect("required");
    let mut nations: FxHashMap<String, NationId> = FxHashMap::default();
    for (i, r) in t.rows.iter().enumerate() {
        let code = t.s(r, "code").to_ascii_uppercase();
        if code.is_empty() {
            return Err(t.err(i, "empty code"));
        }
        let confed = Confed::from_code(t.s(r, "confederation")).unwrap_or(Confed::Uefa);
        let cal = t.s(r, "calendar");
        let id = builder::add_nation(
            &mut w,
            &code,
            t.s(r, "name"),
            confed,
            t.num(r, "reputation").unwrap_or(3000),
            if cal.is_empty() { "autumn_spring" } else { cal },
            t.num(r, "economy").unwrap_or(0.5),
            t.num(r, "youth_rating").unwrap_or(10),
        );
        nations.insert(code, id);
    }
    rep.nations = nations.len();

    // Competitions.
    let t = Table::load(dir, "competitions.csv", true)?.expect("required");
    let mut comps: FxHashMap<String, CompId> = FxHashMap::default();
    for (i, r) in t.rows.iter().enumerate() {
        let id = t.s(r, "id").to_string();
        let kind = match t.s(r, "kind").to_ascii_lowercase().as_str() {
            "league" => CompKind::League,
            "cup" => CompKind::Cup,
            "continental" => CompKind::Continental,
            "super_cup" | "supercup" => CompKind::SuperCup,
            other => return Err(t.err(i, format!("unknown kind `{other}`"))),
        };
        let nation = nations.get(&t.s(r, "nation").to_ascii_uppercase()).copied().unwrap_or(NationId::NONE);
        if nation.is_none() && kind != CompKind::Continental {
            rep.warn(format!("{}: competition `{id}` has unknown nation", t.file));
            continue;
        }
        let legs: u8 = t.num(r, "legs").unwrap_or(if kind == CompKind::League { 2 } else { 1 });
        let format = match (t.s(r, "format").to_ascii_lowercase().as_str(), kind) {
            ("groups", _) | ("", CompKind::Continental) => Format::Groups {
                groups: t.num(r, "groups").unwrap_or(8),
                size: t.num(r, "group_size").unwrap_or(4),
                advance: t.num(r, "advance").unwrap_or(2),
                legs: 2,
                ko_legs: t.num(r, "legs").unwrap_or(2),
                final_legs: 1,
            },
            ("knockout", _) | ("", CompKind::Cup) | ("", CompKind::SuperCup) => Format::Knockout { legs, final_legs: 1 },
            _ => Format::League { rounds: legs },
        };
        let tier = t.num(r, "tier").unwrap_or(if kind == CompKind::Cup { 4 } else { 1 });
        let size = t.num(r, "teams").unwrap_or(if kind == CompKind::Continental { 32 } else { 0 });
        let reputation = t.num(r, "reputation").unwrap_or(if nation.is_some() { w.nations[nation].reputation / (u16::from(tier).max(1)) } else { 6000 });
        let confed = Confed::from_code(t.s(r, "confederation")).or_else(|| nation.get().map(|n| w.nations[n].confed));
        let c = builder::add_comp(
            &mut w,
            t.s(r, "name"),
            t.s(r, "short_name"),
            nation,
            confed,
            kind,
            tier,
            team_kind(t.s(r, "team_kind")),
            size,
            t.num(r, "promote").unwrap_or(0),
            t.num(r, "relegate").unwrap_or(0),
            reputation,
            format,
            t.num(r, "prize_pool").unwrap_or(i64::from(reputation) * 4_000),
        );
        comps.insert(id, c);
    }
    rep.competitions = comps.len();

    // Clubs.
    let t = Table::load(dir, "clubs.csv", true)?.expect("required");
    let mut clubs: FxHashMap<String, ClubId> = FxHashMap::default();
    for (i, r) in t.rows.iter().enumerate() {
        let id = t.s(r, "id").to_string();
        let Some(&nation) = nations.get(&t.s(r, "nation").to_ascii_uppercase()) else {
            return Err(t.err(i, "unknown nation"));
        };
        let league = comps.get(t.s(r, "league")).copied().unwrap_or(CompId::NONE);
        let league_rep = if league.is_some() { w.comps[league].reputation } else { 1000 };
        let reputation: u16 = t.num(r, "reputation").unwrap_or(league_rep);
        let extra: Vec<TeamKind> = t.s(r, "teams").split(';').filter(|s| !s.trim().is_empty()).map(team_kind).collect();
        let mut fac = builder::default_facilities(reputation);
        if let Some(v) = t.num(r, "training_facilities") {
            fac.training = v;
            fac.medical = v;
        }
        if let Some(v) = t.num(r, "youth_facilities") {
            fac.youth = v;
        }
        if let Some(v) = t.num(r, "youth_recruitment") {
            fac.academy = v;
        }
        let club = builder::add_club(
            &mut w,
            ClubSpec {
                name: t.s(r, "name"),
                short: t.s(r, "short_name"),
                nation,
                city: t.s(r, "city"),
                league,
                reputation,
                balance: t.num(r, "balance").unwrap_or(i64::from(reputation) * 2_000),
                stadium: t.s(r, "stadium"),
                capacity: t.num(r, "capacity").unwrap_or(u32::from(reputation) * 5 + 3_000),
                facilities: fac,
                colors: [parse_color(t.s(r, "colour1")), parse_color(t.s(r, "colour2"))],
                founded: t.num(r, "founded").unwrap_or(1900),
                extra_teams: &extra,
            },
        );
        if let Some(b) = t.num(r, "transfer_budget") {
            w.clubs[club].finance.transfer_budget = b;
        }
        if let Some(b) = t.num(r, "wage_budget") {
            w.clubs[club].finance.wage_budget = b;
        }
        clubs.insert(id, club);
    }
    rep.clubs = clubs.len();

    // Explicit competition entrants.
    if let Some(t) = Table::load(dir, "competition_entrants.csv", false)? {
        for r in &t.rows {
            let (Some(&c), Some(&club)) = (comps.get(t.s(r, "competition")), clubs.get(t.s(r, "club"))) else {
                rep.warn(format!("{}: unknown competition or club", t.file));
                continue;
            };
            let kind = w.comps[c].team_kind;
            let team = club_team(&mut w, club, kind);
            if !w.comps[c].state.entrants.contains(&team) {
                w.comps[c].state.entrants.push(team);
            }
        }
    }

    // Staff.
    if let Some(t) = Table::load(dir, "staff.csv", false)? {
        for (i, r) in t.rows.iter().enumerate() {
            let Some(role) = staff_role(t.s(r, "role")) else {
                rep.warn(format!("{} row {}: unknown role", t.file, i + 2));
                continue;
            };
            let nation = nation_or_new(&mut w, &mut nations, t.s(r, "nationality"));
            let dob = parse_date(t.s(r, "dob")).unwrap_or(start.add_days(-365 * 45));
            let person = builder::add_person(&mut w, t.s(r, "first_name"), t.s(r, "last_name"), t.s(r, "common_name"), dob, nation, NationId::NONE);
            let club = clubs.get(t.s(r, "club")).copied().unwrap_or(ClubId::NONE);
            let mut attrs = StaffAttrs::default();
            for a in StaffAttr::ALL {
                if let Some(v) = t.num::<f32>(r, a.key()) {
                    attrs.set(a, v.round().clamp(1.0, 20.0) as u8);
                }
            }
            let mut phil = Philosophy::default();
            if let Some(f) = w.data.formation_index(t.s(r, "formation")) {
                phil.formations = [f, f];
            }
            let id = w.staff.push(Staff {
                person,
                role,
                club,
                attrs,
                wage: t.num(r, "wage").unwrap_or(0),
                contract_end: parse_date(t.s(r, "contract_end")).unwrap_or(start.add_months(24)),
                reputation: t.num(r, "reputation").unwrap_or(2000),
                philosophy: phil,
                joined: start,
                record: ManagerRecord::default(),
                retired: false,
            });
            w.people[person].staff = id;
            if club.is_some() {
                w.clubs[club].staff.push(id);
                if role == StaffRole::Manager && w.clubs[club].manager.is_none() {
                    w.clubs[club].manager = id;
                }
            }
            rep.staff += 1;
        }
    }

    // Players.
    let t = Table::load(dir, "players.csv", true)?.expect("required");
    let scale = if Attr::ALL.iter().any(|a| t.rows.iter().any(|r| t.num::<f32>(r, a.key()).is_some_and(|v| v > 20.5))) { 0.2 } else { 1.0 };
    if scale != 1.0 {
        rep.warn("players.csv: attributes look like a 1–100 scale; dividing by 5".into());
    }
    let mut fm_ca: Vec<(PlayerId, f32, i32)> = Vec::new();
    for (i, r) in t.rows.iter().enumerate() {
        let Some(dob) = parse_date(t.s(r, "dob")) else {
            return Err(t.err(i, "missing or bad dob"));
        };
        let nation = nation_or_new(&mut w, &mut nations, t.s(r, "nationality"));
        let nation2 = nations.get(&t.s(r, "second_nationality").to_ascii_uppercase()).copied().unwrap_or(NationId::NONE);
        let positions = parse_positions(t.s(r, "positions"));
        if positions.is_empty() {
            rep.warn(format!("{} row {}: no positions, using MC", t.file, i + 2));
        }
        let naturals: Vec<pw_core::Pos> = if positions.is_empty() { vec![pw_core::Pos::MC] } else { positions.to_vec() };
        let mut rng = Rng::keyed(&[seed, stream::WORLDGEN, i as u64]);

        let mut attrs = Attrs::splat(800);
        for a in Attr::ALL {
            if let Some(v) = t.num::<f32>(r, a.key()) {
                attrs.set(a, (v * scale).clamp(1.0, 20.0));
            }
        }
        let mut hidden = pw_sim::generate::hidden_random(&mut rng);
        for h in Hidden::ALL {
            if let Some(v) = t.num::<f32>(r, h.key()) {
                hidden.set(h, v.round().clamp(1.0, 20.0) as u8);
            }
        }
        let (left, right) = match (t.num::<u8>(r, "left_foot"), t.num::<u8>(r, "right_foot"), t.s(r, "foot").to_ascii_lowercase().as_str()) {
            (Some(l), Some(rr), _) => (l.clamp(1, 20), rr.clamp(1, 20)),
            (_, _, "left") => (20, rng.range_i32(4, 12) as u8),
            (_, _, "either" | "both") => (18, 19),
            _ => (rng.range_i32(4, 12) as u8, 20),
        };
        let person = builder::add_person(&mut w, t.s(r, "first_name"), t.s(r, "last_name"), t.s(r, "common_name"), dob, nation, nation2);
        w.people[person].hidden = hidden;

        let club = clubs.get(t.s(r, "club")).copied().unwrap_or(ClubId::NONE);
        let loan_from = clubs.get(t.s(r, "loan_from")).copied().filter(|c| c.is_some());
        let registered = loan_from.unwrap_or(club);
        let team = if club.is_some() { club_team(&mut w, club, team_kind(t.s(r, "team"))) } else { TeamId::NONE };
        let contract = if registered.is_some() {
            Contract {
                club: registered,
                kind: if dob.age_on(start) < 17 { ContractKind::Youth } else { ContractKind::Professional },
                wage: t.num(r, "wage").unwrap_or(0),
                start,
                end: parse_date(t.s(r, "contract_end")).unwrap_or(Date(0)),
                yearly_rise: 3,
                ..Default::default()
            }
        } else {
            Contract::default()
        };
        let loan = loan_from.map(|parent| Loan {
            parent,
            club,
            start,
            end: parse_date(t.s(r, "loan_end")).unwrap_or(start.add_months(11)),
            wage_share: 70,
            fee: 0,
            buy_option: 0,
            recall: true,
        });
        let height = t.num(r, "height").unwrap_or_else(|| pw_sim::generate::height_for(naturals[0], &mut rng));
        let mut cold = PlayerCold {
            person,
            attrs,
            pa: 0,
            ca: 0,
            familiarity: builder::familiarity_from(&naturals),
            best_pos: naturals[0],
            left_foot: left,
            right_foot: right,
            height,
            weight: t.num(r, "weight").unwrap_or((f32::from(height) * 0.42) as u8),
            traits: Default::default(),
            bio_offset: (rng.normal() * 6.0).round() as i8,
            pa_rerolled: dob.age_on(start) >= 20,
            wear: [0; pw_data::N_BODY_REGIONS],
            contract,
            loan,
            value: t.num(r, "value").unwrap_or(0),
            rep: Reputation {
                current: t.num(r, "reputation_current").unwrap_or(0),
                home: t.num(r, "reputation_home").unwrap_or(0),
                world: t.num(r, "reputation_world").unwrap_or(0),
            },
            status: squad_status(t.s(r, "squad_status")).unwrap_or(SquadStatus::Squad),
            shirt: t.num(r, "shirt").unwrap_or(0),
            caps: t.num(r, "caps").unwrap_or(0),
            intl_goals: t.num(r, "international_goals").unwrap_or(0),
            joined: start,
            youth_club: registered,
            injuries_career: 0,
            senior_apps: t.num(r, "apps").unwrap_or(0),
            senior_goals: t.num(r, "goals").unwrap_or(0),
            plan: Default::default(),
        };
        if let Some(s) = squad_status(t.s(r, "squad_status")) {
            cold.contract.promised_status = Some(s);
        }
        cold.refresh_ca(&w.data.weights);
        let hot = PlayerHot {
            club: registered,
            team,
            status: if registered.is_some() { PlayerStatus::Active } else { PlayerStatus::FreeAgent },
            condition: 95,
            sharpness: 70,
            fitness: 85,
            ..PlayerHot::default()
        };
        let pid = w.players.push(hot, cold);
        w.people[person].player = pid;
        if team.is_some() {
            w.teams[team].squad.push(pid);
        }
        if registered.is_some() {
            w.history.start_spell(pid, club, start, loan_from.is_some(), 0);
        }
        fm_ca.push((pid, t.num::<f32>(r, "ca").unwrap_or(0.0), t.num::<i32>(r, "pa").unwrap_or(0)));
    }
    rep.players = w.players.len();

    // Potential: keep FM's headroom ratio on our CA scale; estimate when absent.
    for (pid, fm_ca, fm_pa) in fm_ca {
        let mut rng = Rng::keyed(&[seed, stream::WORLDGEN, 0x9a, u64::from(pid.0)]);
        let ours = f32::from(w.players.cold[pid].ca);
        let age = w.people[w.players.cold[pid].person].dob.age_years(start);
        let pa = if fm_pa != 0 && fm_ca > 0.0 {
            let fm_pa = f32::from(pw_sim::generate::resolve_pa(fm_pa, fm_ca.min(200.0) as u8, &mut rng));
            ours * fm_pa / fm_ca
        } else {
            ours / pw_sim::generate::ca_share_at(age) * rng.normal_ms(1.0, 0.1)
        };
        w.players.cold[pid].pa = pa.round().clamp(ours, 200.0) as u8;
    }

    builder::finalize(&mut w);
    builder::ensure_staff(&mut w);
    fill_contracts(&mut w, start);
    Ok((w, rep))
}

/// Missing wages, contract ends and reputations get plausible values.
fn fill_contracts(w: &mut World, start: Date) {
    for p in w.players.ids() {
        let club = w.players.hot[p].club;
        if club.is_none() {
            continue;
        }
        let mut rng = Rng::keyed(&[w.seed, stream::CONTRACTS, u64::from(p.0)]);
        if w.players.cold[p].contract.wage == 0 {
            w.players.cold[p].contract.wage = pw_sim::market::wage_demand(w, p, club);
        }
        if w.players.cold[p].contract.end.0 == 0 {
            let years = rng.range_i32(1, 4);
            w.players.cold[p].contract.end = Date::from_ymd(start.year() + years, 6, 30);
        }
        if w.players.cold[p].rep.current == 0 {
            let c = &w.players.cold[p];
            let league = w.clubs[club].league;
            let stage = if league.is_some() { f32::from(w.comps[league].reputation) / 10_000.0 } else { 0.1 };
            let r = (10_000.0 * (f32::from(c.ca) / 200.0).powf(1.6) * (0.35 + 0.65 * stage)) as u16;
            w.players.cold[p].rep = Reputation { current: r, home: r, world: (f32::from(r) * stage.sqrt()) as u16 };
        }
    }
}

fn nation_or_new(w: &mut World, nations: &mut FxHashMap<String, NationId>, code: &str) -> NationId {
    let code = code.trim().to_ascii_uppercase();
    if code.is_empty() {
        return NationId::NONE;
    }
    if let Some(&n) = nations.get(&code) {
        return n;
    }
    let n = builder::add_nation(w, &code, &code, Confed::Uefa, 1000, "autumn_spring", 0.3, 8);
    nations.insert(code, n);
    n
}

/// The club's side of `kind`, created if the export places players there.
fn club_team(w: &mut World, club: ClubId, kind: TeamKind) -> TeamId {
    if let Some(t) = w.club_team(club, kind) {
        return t;
    }
    let t = w.teams.push(Team { club, kind, squad: Vec::new(), tactics: Default::default(), captain: Default::default(), familiarity_weeks: 0 });
    w.clubs[club].teams.push(t);
    t
}

