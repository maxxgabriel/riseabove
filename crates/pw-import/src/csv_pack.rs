//! Adapter for the pack format in `data/IMPORT_FORMAT.md`: a folder of CSV files in Pathway's own layout.

use std::path::Path;

use pw_core::{Attr, Date, Foot, Hidden, StaffAttr};
use pw_world::nation::Confed;
use pw_world::{CompKind, SquadStatus, StaffRole, TeamKind};

use crate::model::*;
use crate::positions::parse_positions;
use crate::table::{Opts, Rec, Table, parse_date};
use crate::ImportError;

pub fn team_kind(s: &str) -> TeamKind {
    match s.to_ascii_lowercase().as_str() {
        "reserve" | "reserves" | "b" | "ii" => TeamKind::Reserve,
        "u21" | "u23" => TeamKind::U21,
        "u19" => TeamKind::U19,
        "u18" | "u17" | "youth" => TeamKind::U18,
        "u16" | "u15" => TeamKind::U16,
        "u14" | "u13" => TeamKind::U14,
        "u12" | "u11" | "u10" | "u9" => TeamKind::U12,
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

pub fn staff_role(s: &str) -> Option<StaffRole> {
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
        "analyst" | "performance_analyst" | "data_analyst" => StaffRole::Analyst,
        _ => return None,
    })
}

#[derive(serde::Deserialize, Default)]
struct WorldConfig {
    start_date: Option<String>,
    seed: Option<u64>,
}

fn colour(s: &str) -> Option<u32> {
    u32::from_str_radix(s.trim().trim_start_matches('#'), 16).ok().filter(|_| s.trim().trim_start_matches('#').len() == 6)
}

fn table_or_err(dir: &Path, name: &str, required: bool) -> Result<Option<Table>, ImportError> {
    match Table::open(dir, name, Opts::default())? {
        Some(t) => Ok(Some(t)),
        None if required => Err(ImportError::Missing(name.into())),
        None => Ok(None),
    }
}

fn note_lossy(set: &mut ImportSet, t: &Table) {
    if t.lossy_fields > 0 {
        set.issues.add(Severity::Warning, "bad_encoding", &t.file, 0, "", format!("{} fields contained bytes that are not valid UTF-8", t.lossy_fields));
    }
}

/// Parse a pack folder. Rows that cannot be used are dropped and reported; nothing fatal short of a missing required file.
pub fn parse(dir: &Path) -> Result<ImportSet, ImportError> {
    let cfg: WorldConfig = match std::fs::read_to_string(dir.join("world.toml")) {
        Ok(s) => toml::from_str(&s).map_err(|e| ImportError::Config(e.to_string()))?,
        Err(_) => WorldConfig::default(),
    };
    let mut set = ImportSet { start: cfg.start_date.as_deref().and_then(parse_date), seed: cfg.seed, ..Default::default() };
    let src = set.add_source("pack", set.start, "Pathway import pack (CSV)");

    // Nations.
    let mut t = table_or_err(dir, "nations.csv", true)?.expect("required");
    let mut rows = Vec::new();
    let bad = t.for_each(|row, r| rows.push(nation_row(row, r, src)))?;
    if bad > 0 {
        set.issues.add(Severity::Error, "malformed_row", &t.file, 0, "", format!("{bad} unreadable rows"));
    }
    for (row, n) in rows.into_iter().enumerate() {
        match n {
            Some(n) => set.nations.push(n),
            None => set.issues.add(Severity::Error, "missing_field", "nations.csv", row + 1, "", "empty nation code"),
        }
    }

    // Competitions.
    let mut t = table_or_err(dir, "competitions.csv", true)?.expect("required");
    let mut rows: Vec<(usize, Result<ImpComp, String>)> = Vec::new();
    t.for_each(|row, r| rows.push((row, comp_row(r, src))))?;
    for (row, c) in rows {
        match c {
            Ok(c) => set.comps.push(c),
            Err(e) => set.issues.add(Severity::Error, "bad_value", "competitions.csv", row, "", e),
        }
    }

    // Clubs.
    let mut t = table_or_err(dir, "clubs.csv", true)?.expect("required");
    let mut rows = Vec::new();
    t.for_each(|row, r| rows.push((row, club_row(r, src))))?;
    note_lossy(&mut set, &t);
    for (row, c) in rows {
        match c {
            Some(c) => set.clubs.push(c),
            None => set.issues.add(Severity::Error, "missing_field", "clubs.csv", row, "", "club has no id or name"),
        }
    }

    // Explicit competition entrants are applied by the assembler through this list.
    if let Some(mut t) = table_or_err(dir, "competition_entrants.csv", false)? {
        let mut rows = Vec::new();
        t.for_each(|row, r| rows.push((row, r.text("competition"), r.text("club"))))?;
        for (row, comp, club) in rows {
            match (comp, club) {
                (Some(comp), Some(club)) => set.entrants.push((comp, club)),
                _ => set.issues.add(Severity::Error, "missing_field", "competition_entrants.csv", row, "", "needs both competition and club"),
            }
        }
    }

    // Staff.
    if let Some(mut t) = table_or_err(dir, "staff.csv", false)? {
        let mut rows = Vec::new();
        t.for_each(|row, r| rows.push((row, staff_row(row, r, src))))?;
        for (row, s) in rows {
            match s {
                Ok(s) => set.staff.push(s),
                Err(e) => set.issues.add(Severity::Warning, "bad_staff_role", "staff.csv", row, "", e),
            }
        }
    }

    // Players.
    let mut t = table_or_err(dir, "players.csv", true)?.expect("required");
    let scale_1_100 = std::cell::Cell::new(false);
    let mut rows = Vec::new();
    t.for_each(|row, r| rows.push(player_row(row, r, src, &scale_1_100)))?;
    note_lossy(&mut set, &t);
    if scale_1_100.get() {
        set.issues.add(Severity::Info, "attribute_scale", "players.csv", 0, "", "attributes look like a 1-100 scale; divided by 5");
        for p in rows.iter_mut().flatten() {
            for (_, v) in &mut p.attrs {
                *v *= 0.2;
            }
        }
    }
    for p in rows {
        match p {
            Ok(p) => set.players.push(p),
            Err((row, key, msg)) => set.issues.add(Severity::Error, "bad_player_row", "players.csv", row, &key, msg),
        }
    }

    // Past seasons.
    if let Some(mut t) = table_or_err(dir, "history.csv", false)? {
        let mut rows = Vec::new();
        t.for_each(|row, r| rows.push((row, r.text("competition"), r.num::<i32>("season"), r.text("champion"), r.text("runner_up"), r.text("top_scorer"), r.num::<u16>("top_goals"))))?;
        for (row, comp, season, champion, runner_up, top_scorer, top_goals) in rows {
            match (comp, season, champion) {
                (Some(comp), Some(season), Some(champion)) => set.seasons.push(ImpSeason { comp, season, champion, runner_up, top_scorer, top_goals }),
                _ => set.issues.add(Severity::Error, "missing_field", "history.csv", row, "", "needs competition, season and champion"),
            }
        }
    }
    Ok(set)
}

fn nation_row(_row: usize, r: &Rec, source: u8) -> Option<ImpNation> {
    let code = r.s("code").to_ascii_uppercase();
    if code.is_empty() {
        return None;
    }
    Some(ImpNation {
        source,
        key: code.clone(),
        code,
        name: r.s("name").to_string(),
        confed: Confed::from_code(&r.s("confederation")),
        reputation: r.num("reputation"),
        economy: r.num("economy"),
        youth_rating: r.num("youth_rating"),
        calendar: r.text("calendar"),
        minor: false,
    })
}

fn comp_row(r: &Rec, source: u8) -> Result<ImpComp, String> {
    let key = r.s("id").to_string();
    if key.is_empty() {
        return Err("competition has no id".into());
    }
    let kind = match r.s("kind").to_ascii_lowercase().as_str() {
        "league" => CompKind::League,
        "cup" => CompKind::Cup,
        "continental" => CompKind::Continental,
        "super_cup" | "supercup" => CompKind::SuperCup,
        other => return Err(format!("unknown kind `{other}`")),
    };
    let mut c = ImpComp::new(key, r.s("name"), kind);
    c.source = source;
    c.short = r.s("short_name").to_string();
    c.nation = r.text("nation").map(|s| s.to_ascii_uppercase());
    c.confed = Confed::from_code(&r.s("confederation"));
    c.tier = r.num("tier");
    c.team_kind = team_kind(&r.s("team_kind"));
    c.size = r.num("teams");
    c.promote = r.num("promote");
    c.relegate = r.num("relegate");
    c.reputation = r.num("reputation");
    c.format = r.text("format");
    c.legs = r.num("legs");
    c.groups = r.num("groups");
    c.group_size = r.num("group_size");
    c.advance = r.num("advance");
    c.prize_pool = r.num("prize_pool");
    Ok(c)
}

fn club_row(r: &Rec, source: u8) -> Option<ImpClub> {
    let key = r.s("id").to_string();
    if key.is_empty() || r.s("name").is_empty() {
        return None;
    }
    Some(ImpClub {
        source,
        key,
        name: r.s("name").to_string(),
        short: r.s("short_name").to_string(),
        nation: r.text("nation").map(|s| s.to_ascii_uppercase()),
        city: r.s("city").to_string(),
        league: r.text("league"),
        reputation: r.num("reputation"),
        balance: r.num("balance"),
        transfer_budget: r.num("transfer_budget"),
        wage_budget: r.num("wage_budget"),
        stadium: r.s("stadium").to_string(),
        capacity: r.num("capacity"),
        training: r.num("training_facilities"),
        youth: r.num("youth_facilities"),
        academy: r.num("youth_recruitment"),
        colours: [colour(&r.s("colour1")), colour(&r.s("colour2"))],
        founded: r.num("founded"),
        extra_teams: r.s("teams").split(';').filter(|s| !s.trim().is_empty()).map(|s| team_kind(s.trim())).collect(),
        manager_name: None,
        squad_value: None,
    })
}

fn staff_row(row: usize, r: &Rec, source: u8) -> Result<ImpStaff, String> {
    let role = staff_role(&r.s("role")).ok_or_else(|| format!("unknown role `{}`", r.s("role")))?;
    let key = r.text("id").unwrap_or_else(|| format!("row{row}"));
    let mut s = ImpStaff::new(key, row);
    s.source = source;
    s.first = r.s("first_name").to_string();
    s.last = r.s("last_name").to_string();
    s.dob = r.date("dob");
    s.nationality = r.text("nationality").map(|s| s.to_ascii_uppercase());
    s.club = r.text("club");
    s.club_link = s.club.as_ref().map(|_| Link::Stated);
    s.role = Some(role);
    for a in StaffAttr::ALL {
        if let Some(v) = r.num::<f32>(a.key()) {
            s.attrs.push((a, v.round().clamp(1.0, 20.0) as u8));
        }
    }
    s.wage = r.num("wage");
    s.reputation = r.num("reputation");
    s.formation = r.text("formation");
    s.contract_end = r.date("contract_end");
    Ok(s)
}

type PlayerErr = (usize, String, String);

fn player_row(row: usize, r: &Rec, source: u8, scale_1_100: &std::cell::Cell<bool>) -> Result<ImpPlayer, PlayerErr> {
    let key = r.s("id").to_string();
    if key.is_empty() {
        return Err((row, key, "player has no id".into()));
    }
    let mut p = ImpPlayer::new(key.clone(), row);
    p.source = source;
    p.first = r.s("first_name").to_string();
    p.last = r.s("last_name").to_string();
    p.common = r.s("common_name").to_string();
    if p.first.is_empty() && p.last.is_empty() && p.common.is_empty() {
        return Err((row, key, "player has no name".into()));
    }
    p.dob = r.date("dob");
    p.nationality = r.text("nationality").map(|s| s.to_ascii_uppercase());
    p.nationality2 = r.text("second_nationality").map(|s| s.to_ascii_uppercase());
    p.club = r.text("club");
    p.team = team_kind(&r.s("team"));
    p.positions = parse_positions(&r.s("positions")).to_vec();
    for a in Attr::ALL {
        if let Some(v) = r.num::<f32>(a.key()) {
            if v > 20.5 {
                scale_1_100.set(true);
            }
            p.attrs.push((a, v));
        }
    }
    for h in Hidden::ALL {
        if let Some(v) = r.num::<f32>(h.key()) {
            p.hidden.push((h, v.round().clamp(1.0, 20.0) as u8));
        }
    }
    p.feet = match (r.num::<u8>("left_foot"), r.num::<u8>("right_foot")) {
        (Some(l), Some(rr)) => Some((l.clamp(1, 20), rr.clamp(1, 20))),
        _ => None,
    };
    p.foot = match r.s("foot").to_ascii_lowercase().as_str() {
        "left" => Some(Foot::Left),
        "right" => Some(Foot::Right),
        "either" | "both" => Some(Foot::Either),
        _ => None,
    };
    p.height = r.num("height");
    p.weight = r.num("weight");
    p.ca = r.num("ca");
    p.pa = r.num("pa");
    p.wage = r.num("wage");
    p.contract_end = r.date("contract_end");
    p.value = r.num("value");
    p.rep_current = r.num("reputation_current");
    p.rep_home = r.num("reputation_home");
    p.rep_world = r.num("reputation_world");
    p.status = squad_status(&r.s("squad_status"));
    p.loan_from = r.text("loan_from");
    p.loan_end = r.date("loan_end");
    p.shirt = r.num("shirt");
    p.caps = r.num("caps");
    p.intl_goals = r.num("international_goals");
    p.apps = r.num("apps");
    p.goals = r.num("goals");
    Ok(p)
}

/// Start date used when the pack states none.
pub const DEFAULT_START: (i32, u32, u32) = (2022, 7, 1);

pub fn default_start() -> Date {
    Date::from_ymd(DEFAULT_START.0, DEFAULT_START.1, DEFAULT_START.2)
}
