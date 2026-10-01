//! Adapter for the Transfermarkt-style relational archive (`countries`, `competitions`, `clubs`, `players`,
//! `transfers`, `games`, `appearances`, `game_lineups` CSV files). See `docs/DB_INTEGRATION_AUDIT.md`.
//!
//! Scope: clubs and players of the newest season (`last_season` equal to the newest in `clubs.csv`). Every link is
//! made by source id; club names are never used to identify a club. Anything the archive does not state (attributes,
//! wages, balances, facilities) is left UNKNOWN here and filled, and labelled, by the assembler.

use std::path::Path;

use pw_core::{Date, Foot, Pos, PosGroup};
use pw_world::nation::Confed;
use pw_world::{CompKind, TeamKind};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ImportError;
use crate::geo::{self, Geo};
use crate::model::*;
use crate::table::{Opts, Table};

pub struct TmOptions {
    /// World start date; by default 15 July after the newest season.
    pub start: Option<Date>,
    /// Read `appearances.csv` (career counters, top scorers) and `game_lineups.csv` (shirt numbers). Large files.
    pub with_match_files: bool,
}

impl Default for TmOptions {
    fn default() -> Self {
        Self { start: None, with_match_files: true }
    }
}

/// Whether `dir` looks like this archive.
pub fn detect(dir: &Path) -> bool {
    ["players.csv", "clubs.csv", "countries.csv", "competitions.csv"].iter().all(|f| dir.join(f).exists())
        && std::fs::read_to_string(dir.join("competitions.csv")).is_ok_and(|s| s.lines().next().is_some_and(|h| h.contains("competition_code") && h.contains("sub_type")))
}

fn open(dir: &Path, name: &str, required: bool) -> Result<Option<Table>, ImportError> {
    match Table::open(dir, name, Opts::default())? {
        Some(t) => Ok(Some(t)),
        None if required => Err(ImportError::Missing(name.into())),
        None => Ok(None),
    }
}

fn nation_key(g: &Geo) -> Key {
    g.name.to_string()
}

/// `premier-league` → `Premier League`.
fn title_slug(slug: &str) -> String {
    slug.split('-')
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

const CORPORATE: &[&str] = &[
    "fc", "fk", "sk", "sc", "ac", "as", "cf", "cd", "ud", "sd", "rc", "bk", "if", "ff", "fbc", "afc", "club", "clube", "clubul", "football", "futbol", "futebol", "fotball", "fotballforening", "fodbold", "association",
    "associazione", "asociacion", "atletica", "atletico", "sociedade", "esportiva", "esporte", "sportclub", "sportverein", "verein", "klubi", "kulüp", "kulubu", "spor", "sportif", "calcio", "societa", "sportiva", "de", "da",
    "do", "del", "della", "the", "e", "y", "og", "och", "und", "and", "foot", "association", "voetbalvereniging", "vereniging", "betaald", "voetbal", "fotbollsklubb", "idrottsförening", "idrottsforening", "boldklub",
    "idrætsforening",
];

fn ascii_fold(s: &str) -> String {
    geo::fold(s)
}

/// A shorter display form: drops legal-form words and years when three or more words remain meaningful; falls back to the full name.
pub fn shorten(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.len() < 3 {
        return name.to_string();
    }
    let kept: Vec<&str> = words.iter().copied().filter(|w| !CORPORATE.contains(&ascii_fold(w).as_str()) && !(w.len() == 4 && w.chars().all(|c| c.is_ascii_digit()))).collect();
    if kept.is_empty() || kept.len() == words.len() { name.to_string() } else { kept.join(" ") }
}

fn position_of(sub: &str) -> Vec<Pos> {
    match sub {
        "Goalkeeper" => vec![Pos::GK],
        "Centre-Back" => vec![Pos::DC],
        "Right-Back" => vec![Pos::DR],
        "Left-Back" => vec![Pos::DL],
        "Defensive Midfield" => vec![Pos::DM],
        "Central Midfield" => vec![Pos::MC],
        "Attacking Midfield" => vec![Pos::AMC],
        "Right Midfield" => vec![Pos::MR],
        "Left Midfield" => vec![Pos::ML],
        "Right Winger" => vec![Pos::AMR],
        "Left Winger" => vec![Pos::AML],
        "Second Striker" => vec![Pos::ST, Pos::AMC],
        "Centre-Forward" => vec![Pos::ST],
        _ => Vec::new(),
    }
}

fn group_of(pos: &str) -> Option<PosGroup> {
    match pos {
        "Goalkeeper" => Some(PosGroup::Gk),
        "Defender" => Some(PosGroup::Def),
        "Midfield" => Some(PosGroup::Mid),
        "Attack" => Some(PosGroup::Att),
        _ => None,
    }
}

/// Percentile (0..1) of each value in `xs` among `xs`, ties sharing the lower rank.
fn percentiles(xs: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..xs.len()).collect();
    idx.sort_by(|&a, &b| xs[a].total_cmp(&xs[b]));
    let n = xs.len().max(2) as f64 - 1.0;
    let mut out = vec![0.0; xs.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && xs[idx[j + 1]] == xs[idx[i]] {
            j += 1;
        }
        for &k in &idx[i..=j] {
            out[k] = i as f64 / n;
        }
        i = j + 1;
    }
    out
}

pub fn parse(dir: &Path, opt: &TmOptions) -> Result<ImportSet, ImportError> {
    let mut set = ImportSet::default();
    let src = set.add_source("transfermarkt", None, "Transfermarkt-style archive (players, clubs, competitions, transfers, games)");

    // ---- countries ----------------------------------------------------------------
    let mut nations: FxHashMap<Key, ImpNation> = FxHashMap::default();
    let mut country_by_code: FxHashMap<String, Key> = FxHashMap::default();
    let mut t = open(dir, "countries.csv", true)?.expect("required");
    let mut rows = Vec::new();
    t.for_each(|row, r| rows.push((row, r.s("country_name").to_string(), r.s("country_code").to_string(), geo::confed_from_tm(&r.s("confederation")), r.num::<u32>("total_clubs"))))?;
    // Clubs the source says each country's league has, kept as evidence for leagues the competition file lacks.
    let mut declared_clubs: FxHashMap<Key, u32> = FxHashMap::default();
    for (row, name, code, tm_confed, total) in rows {
        let Some(g) = geo::lookup(&name) else {
            set.issues.add(Severity::Warning, "unknown_country", "countries.csv", row, &name, "country name is not in the country table; skipped");
            continue;
        };
        let key = nation_key(g);
        if nations.contains_key(&key) {
            set.issues.add(Severity::Info, "duplicate_country", "countries.csv", row, &name, format!("same country as {}", g.name));
            continue;
        }
        // The table decides the confederation (the source lumps both Americas into one word); a disagreement is reported.
        let confed = g.confed;
        if let Some(tm) = tm_confed
            && tm != confed
        {
            set.issues.add(Severity::Warning, "confederation_mismatch", "countries.csv", row, &name, format!("source says {tm:?}, country table says {confed:?}; table used"));
        }
        let confed = Some(confed);
        nations.insert(key.clone(), ImpNation { source: src, key: key.clone(), code: g.code.to_string(), name: g.name.to_string(), confed, minor: false, ..Default::default() });
        if let Some(t) = total {
            declared_clubs.insert(key.clone(), t);
        }
        if !code.is_empty() {
            country_by_code.insert(code, key);
        }
    }
    let listed_nations: FxHashSet<Key> = nations.keys().cloned().collect();

    // ---- competitions ---------------------------------------------------------------
    let mut comps: Vec<ImpComp> = Vec::new();
    let mut comp_keys: FxHashSet<Key> = FxHashSet::default();
    let mut t = open(dir, "competitions.csv", true)?.expect("required");
    let mut rows = Vec::new();
    t.for_each(|row, r| rows.push((row, r.s("competition_id").to_string(), r.s("name").to_string(), r.s("type").to_string(), r.s("sub_type").to_string(), r.s("country_name").to_string(), r.num::<u16>("total_clubs"))))?;
    for (row, id, slug, ty, sub, country, total) in rows {
        let kind = match (ty.as_str(), sub.as_str()) {
            ("domestic_league", "first_tier") => Some(CompKind::League),
            ("domestic_cup", _) => Some(CompKind::Cup),
            (_, "domestic_super_cup") => Some(CompKind::SuperCup),
            ("international_cup", "uefa_champions_league") | (_, "uefa_europa_league") | (_, "uefa_conference_league") => Some(CompKind::Continental),
            _ => None,
        };
        let Some(kind) = kind else {
            set.issues.add(Severity::Info, "out_of_scope", "competitions.csv", row, &id, format!("{ty}/{sub} is not modelled as a club competition"));
            continue;
        };
        let nation = if kind == CompKind::Continental {
            None
        } else {
            match geo::lookup(&country) {
                Some(g) => Some(nation_key(g)),
                None => {
                    set.issues.add(Severity::Error, "unknown_country", "competitions.csv", row, &id, format!("competition country `{country}` is not in the country table"));
                    continue;
                }
            }
        };
        let mut c = ImpComp::new(id.clone(), title_slug(&slug), kind);
        c.source = src;
        c.nation = nation;
        c.confed = if kind == CompKind::Continental { Some(Confed::Uefa) } else { None };
        c.size = total;
        c.tier = Some(1);
        c.format = Some(match kind {
            CompKind::League => "league",
            CompKind::Continental => "groups",
            _ => "knockout",
        }.to_string());
        comp_keys.insert(id);
        comps.push(c);
    }

    // Two competitions may share a display name (`bundesliga`: Austria and Germany): the country tells them apart.
    let mut count: FxHashMap<String, u32> = FxHashMap::default();
    for c in &comps {
        *count.entry(c.name.clone()).or_default() += 1;
    }
    for c in &mut comps {
        if count[&c.name] > 1
            && let Some(n) = c.nation.as_ref()
        {
            c.name = format!("{} ({})", c.name, n);
        }
        c.short = c.name.clone();
    }

    // ---- clubs ----------------------------------------------------------------------
    let mut t = open(dir, "clubs.csv", true)?.expect("required");
    struct RawClub {
        row: usize,
        id: String,
        name: String,
        comp: String,
        nation: Option<Key>,
        stadium: String,
        seats: Option<u32>,
        coach: Option<String>,
        last: i32,
        source: u8,
        extra_teams: Vec<TeamKind>,
    }
    let mut raw = Vec::new();
    t.for_each(|row, r| {
        raw.push(RawClub { row, id: r.s("club_id").to_string(), name: r.s("name").to_string(), comp: r.s("domestic_competition_id").to_string(), nation: None, stadium: r.s("stadium_name").to_string(), seats: r.num("stadium_seats"), coach: r.text("coach_name"), last: r.num("last_season").unwrap_or(0), source: src, extra_teams: vec![TeamKind::U21, TeamKind::U18] })
    })?;
    let newest = raw.iter().map(|c| c.last).max().unwrap_or(0);
    if newest == 0 {
        return Err(ImportError::Empty("clubs.csv has no club with a last_season".into()));
    }
    let start = opt.start.unwrap_or_else(|| Date::from_ymd(newest + 1, 7, 15));
    set.start = Some(start);
    set.sources[usize::from(src)].snapshot = Some(start);

    // This optional, generated crosswalk admits current player clubs outside the modeled leagues only when
    // Transfermarkt's numeric club ID has an explicit validated Reep team bridge and an exact known country.
    // It intentionally carries no competition assignment or invented club facts.
    if let Some(mut verified) = open(dir, "verified_unmodeled_clubs.csv", false)? {
        let reep_src = set.add_source("Reep team registry exact-ID crosswalk", Some(Date::from_ymd(2026, 9, 26)),
            "Club identity and country use an active men's Reep team reached by a unique Transfermarkt verein provider_claim; league membership is unknown.");
        let mut seen = FxHashSet::default();
        verified.for_each(|row, r| {
            let id = r.s("club_id").to_string();
            let name = r.s("club_name").to_string();
            let country = r.s("country");
            let is_verified = r.s("crosswalk_validation") == "unique_provider_claim"
                && r.s("registry_status") == "active"
                && r.s("gender") == "men"
                && !r.s("reep_id").is_empty()
                && !r.s("snapshot").is_empty();
            let nation = geo::lookup(&country).map(nation_key);
            if id.is_empty() || name.is_empty() || !is_verified || nation.is_none() || !seen.insert(id.clone()) {
                set.issues.add(Severity::Warning, "invalid_verified_club", "verified_unmodeled_clubs.csv", row, &id,
                    "crosswalk row is incomplete, duplicated, unsupported, or lacks an exact supported country");
                return;
            }
            // A latest-season club row wins; stale raw rows are deliberately replaceable by this exact-ID supplement.
            if raw.iter().any(|c| c.id == id && c.last == newest) { return; }
            raw.push(RawClub { row, id, name: name.clone(), comp: String::new(), nation, stadium: String::new(),
                seats: None, coach: None, last: newest, source: reep_src, extra_teams: Vec::new() });
        })?;
    }

    let mut league_clubs: FxHashMap<String, u16> = FxHashMap::default();
    let mut clubs: Vec<ImpClub> = Vec::new();
    for c in &raw {
        if c.id.is_empty() || c.name.is_empty() {
            set.issues.add(Severity::Error, "missing_field", "clubs.csv", c.row, &c.id, "club has no id or name");
            continue;
        }
        if c.last < newest {
            set.issues.add(Severity::Info, "stale_club", "clubs.csv", c.row, &c.id, format!("last season {} is before {newest}", c.last));
            continue;
        }
        if !c.comp.is_empty() {
            *league_clubs.entry(c.comp.clone()).or_default() += 1;
        }
        clubs.push(ImpClub {
            source: c.source,
            key: c.id.clone(),
            name: c.name.clone(),
            short: shorten(&c.name),
            league: (!c.comp.is_empty()).then(|| c.comp.clone()),
            stadium: c.stadium.clone(),
            capacity: c.seats.filter(|&s| s >= 500),
            manager_name: c.coach.clone(),
            extra_teams: c.extra_teams.clone(),
            nation: c.nation.clone(),
            ..Default::default()
        });
    }
    // A league the clubs name but the competition file lacks: created from the clubs. Its country needs evidence, never a
    // club's name: the country whose league code equals the id, or else the country most of the clubs' players are citizens
    // of when that country also declares exactly this many league clubs (two independent pieces of evidence).
    let mut missing: Vec<(String, u16)> = league_clubs.iter().filter(|(k, _)| !comp_keys.contains(*k)).map(|(k, n)| (k.clone(), *n)).collect();
    missing.sort();
    let mut citizens: FxHashMap<String, FxHashMap<Key, u32>> = FxHashMap::default();
    if !missing.is_empty()
        && let Some(mut pt) = open(dir, "players.csv", false)?
    {
        let missing_ids: FxHashSet<&str> = missing.iter().map(|(k, _)| k.as_str()).collect();
        let league_of_club: FxHashMap<&str, &str> = raw.iter().filter(|c| c.last == newest && missing_ids.contains(c.comp.as_str())).map(|c| (c.id.as_str(), c.comp.as_str())).collect();
        pt.for_each(|_, r| {
            if r.num::<i32>("last_season") != Some(newest) {
                return;
            }
            if let (Some(league), Some(g)) = (league_of_club.get(r.s("current_club_id").as_ref()), geo::lookup(&r.s("country_of_citizenship"))) {
                *citizens.entry((*league).to_string()).or_default().entry(nation_key(g)).or_default() += 1;
            }
        })?;
    }
    for (id, n) in missing {
        let by_citizens = citizens.get(&id).and_then(|m| {
            let total: u32 = m.values().sum();
            let (top, count) = m.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))?;
            (total > 0 && *count * 2 > total && declared_clubs.get(top) == Some(&u32::from(n))).then(|| top.clone())
        });
        match country_by_code.get(&id).cloned().or(by_citizens) {
            Some(nation) => {
                let mut c = ImpComp::new(id.clone(), format!("{nation} First Division"), CompKind::League);
                c.source = src;
                c.nation = Some(nation.clone());
                c.tier = Some(1);
                c.size = Some(n);
                c.format = Some("league".into());
                c.derived = true;
                c.short = c.name.clone();
                comp_keys.insert(id.clone());
                comps.push(c);
                set.issues.add(Severity::Warning, "derived_competition", "clubs.csv", 0, &id, format!("{n} clubs name a league that competitions.csv lacks; created for {nation} (name not supplied by the source)"));
            }
            None => set.issues.add(Severity::Error, "unknown_league", "clubs.csv", 0, &id, format!("{n} clubs name league `{id}`, which no file describes")),
        }
    }
    let league_nation: FxHashMap<Key, Key> = comps.iter().filter_map(|c| c.nation.clone().map(|n| (c.key.clone(), n))).collect();
    for c in &mut clubs {
        if c.nation.is_none() {
            c.nation = c.league.as_ref().and_then(|l| league_nation.get(l)).cloned();
        }
        if c.nation.is_none() {
            set.issues.add(Severity::Error, "unknown_league", "clubs.csv", 0, &c.key, "club's league is unknown; club dropped");
        }
    }
    clubs.retain(|c| c.nation.is_some());
    // Short names must stay unambiguous inside a nation.
    let mut short_count: FxHashMap<(Key, String), u32> = FxHashMap::default();
    for c in &clubs {
        *short_count.entry((c.nation.clone().unwrap_or_default(), c.short.clone())).or_default() += 1;
    }
    for c in &mut clubs {
        if short_count[&(c.nation.clone().unwrap_or_default(), c.short.clone())] > 1 {
            c.short = c.name.clone();
        }
    }
    let club_keys: FxHashSet<Key> = clubs.iter().map(|c| c.key.clone()).collect();

    // ---- players ----------------------------------------------------------------------
    let mut t = open(dir, "players.csv", true)?.expect("required");
    let mut players: Vec<ImpPlayer> = Vec::new();
    let mut minor_nations: FxHashMap<Key, &'static Geo> = FxHashMap::default();
    let mut unknown_country_seen: FxHashSet<String> = FxHashSet::default();
    let mut stale = 0u32;
    let mut untracked = 0u32;
    t.for_each(|row, r| {
        let key = r.s("player_id").to_string();
        if key.is_empty() {
            set.issues.add(Severity::Error, "missing_field", "players.csv", row, "", "player has no id");
            return;
        }
        let season: i32 = r.num("last_season").unwrap_or(0);
        let club = r.s("current_club_id").to_string();
        if season < newest {
            stale += 1;
            return;
        }
        if !club_keys.contains(&club) {
            untracked += 1;
            return;
        }
        let mut p = ImpPlayer::new(key.clone(), row);
        p.source = src;
        p.first = r.s("first_name").to_string();
        p.last = r.s("last_name").to_string();
        if p.last.is_empty() {
            // The archive always gives `name`; a row with only that keeps it whole.
            p.last = r.s("name").to_string();
        }
        p.club = Some(club);
        if r.bad_date("date_of_birth") {
            set.issues.add(Severity::Warning, "bad_date", "players.csv", row, &key, format!("date of birth `{}` is not a date", r.s("date_of_birth")));
        }
        p.dob = r.date("date_of_birth");
        let cit = r.s("country_of_citizenship");
        if !cit.is_empty() {
            match geo::lookup(&cit) {
                Some(g) => {
                    let k = nation_key(g);
                    if !listed_nations.contains(&k) {
                        minor_nations.insert(k.clone(), g);
                    }
                    p.nationality = Some(k);
                }
                None => {
                    if unknown_country_seen.insert(cit.to_string()) {
                        set.issues.add(Severity::Warning, "unknown_country", "players.csv", row, &key, format!("citizenship `{cit}` is not in the country table"));
                    }
                }
            }
        }
        let sub = r.s("sub_position");
        p.positions = position_of(&sub);
        p.position_group = group_of(&r.s("position"));
        p.foot = match r.s("foot").as_ref() {
            "right" => Some(Foot::Right),
            "left" => Some(Foot::Left),
            "both" => Some(Foot::Either),
            _ => None,
        };
        if let Some(h) = r.int("height_in_cm") {
            if (150..=215).contains(&h) {
                p.height = Some(h as u8);
            } else {
                set.issues.add(Severity::Warning, "out_of_range", "players.csv", row, &key, format!("height {h} cm is not plausible; treated as unknown"));
            }
        }
        if r.bad_date("contract_expiration_date") {
            set.issues.add(Severity::Warning, "bad_date", "players.csv", row, &key, format!("contract end `{}` is not a date; treated as unknown", r.s("contract_expiration_date")));
        }
        p.contract_end = r.date("contract_expiration_date");
        if p.contract_end.is_some_and(|d| d < start) {
            set.issues.add(Severity::Info, "contract_expired", "players.csv", row, &key, "contract end is before the start date; treated as unknown");
            p.contract_end = None;
        }
        p.value = r.int("market_value_in_eur").filter(|&v| v >= 0);
        p.caps = r.num::<u16>("international_caps");
        p.intl_goals = r.num::<u16>("international_goals");
        p.agent = r.text("agent_name");
        players.push(p);
    })?;
    if stale > 0 {
        set.issues.add(Severity::Info, "stale_player", "players.csv", 0, "", format!("{stale} rows have an older last season and are history only"));
    }
    if untracked > 0 {
        set.issues.add(Severity::Info, "club_not_modelled", "players.csv", 0, "", format!("{untracked} current-season rows belong to clubs outside the modelled leagues"));
    }
    let player_keys: FxHashSet<Key> = players.iter().map(|p| p.key.clone()).collect();

    // Recover missing current values from a recent, dated record for this exact player ID.
    // A historical value is carried forward as an estimate; future or stale values never initialise the snapshot.
    let missing_values: FxHashSet<&str> = players.iter().filter(|p| p.value.is_none()).map(|p| p.key.as_str()).collect();
    if !missing_values.is_empty() && let Some(mut t) = open(dir, "player_valuations.csv", false)? {
        let mut latest: FxHashMap<Key, (Date, Option<i64>)> = FxHashMap::default();
        let cutoff = start.add_days(-365);
        t.for_each(|_, r| {
            let pid = r.s("player_id");
            if !missing_values.contains(pid.as_ref()) { return; }
            let (Some(date), Some(value)) = (r.date("date"), r.int("market_value_in_eur").filter(|&v| v >= 0)) else { return };
            if date < cutoff || date > start { return; }
            let e = latest.entry(pid.to_string()).or_insert((date, Some(value)));
            if date > e.0 { *e = (date, Some(value)); }
            else if date == e.0 && e.1 != Some(value) { e.1 = None; } // Conflicting same-date values are not resolved by row order.
        })?;
        for p in &mut players {
            if p.value.is_none() && let Some((date, Some(value))) = latest.get(&p.key) {
                p.value = Some(*value);
                p.value_inferred = true;
                set.issues.add(Severity::Info, "value_from_history", "player_valuations.csv", 0, &p.key, format!("valuation dated {date:?} carried forward as an estimate"));
            }
        }
    }

    // ---- strength derived from imported values ------------------------------------------
    let mut squad_value: FxHashMap<Key, i64> = FxHashMap::default();
    for p in &players {
        if let (Some(c), Some(v)) = (&p.club, p.value) {
            *squad_value.entry(c.clone()).or_default() += v;
        }
    }
    for c in &mut clubs {
        c.squad_value = squad_value.get(&c.key).copied();
    }
    // Club reputation: percentile of squad value among all imported clubs (INFERRED).
    let vals: Vec<f64> = clubs.iter().map(|c| c.squad_value.unwrap_or(0) as f64).collect();
    let mut value_pct: FxHashMap<Key, f64> = FxHashMap::default();
    for (c, p) in clubs.iter_mut().zip(percentiles(&vals)) {
        if c.squad_value.is_some() {
            c.reputation = Some((2200.0 + 7300.0 * p.powf(1.35)).round() as u16);
            value_pct.insert(c.key.clone(), p);
        }
    }
    // Nation strength: the eight strongest clubs' squad values (leagues), or the 25 best players' values.
    let mut by_nation_clubs: FxHashMap<Key, Vec<f64>> = FxHashMap::default();
    for c in &clubs {
        if let (Some(n), Some(v)) = (&c.nation, c.squad_value) {
            by_nation_clubs.entry(n.clone()).or_default().push(v as f64);
        }
    }
    let mut by_nation_players: FxHashMap<Key, Vec<f64>> = FxHashMap::default();
    for p in &players {
        if let (Some(n), Some(v)) = (&p.nationality, p.value) {
            by_nation_players.entry(n.clone()).or_default().push(v as f64);
        }
    }
    let top_sum = |mut v: Vec<f64>, n: usize| {
        v.sort_by(|a, b| b.total_cmp(a));
        v.iter().take(n).sum::<f64>()
    };
    let mut strengths: Vec<(Key, f64, bool)> = Vec::new();
    for k in nations.keys().cloned().chain(minor_nations.keys().cloned()).collect::<FxHashSet<_>>() {
        match by_nation_clubs.get(&k) {
            Some(v) => strengths.push((k, top_sum(v.clone(), 8), true)),
            None => strengths.push((k.clone(), top_sum(by_nation_players.get(&k).cloned().unwrap_or_default(), 25), false)),
        }
    }
    strengths.sort_by(|a, b| a.0.cmp(&b.0));
    let league_strengths: Vec<f64> = strengths.iter().filter(|s| s.2).map(|s| s.1).collect();
    let league_pct = percentiles(&league_strengths);
    let mut li = 0;
    let mut nation_league_pct: FxHashMap<Key, f64> = FxHashMap::default();
    let mut nation_rep: FxHashMap<Key, (u16, f32, u8)> = FxHashMap::default();
    for (k, s, has_league) in &strengths {
        let (rep, econ) = if *has_league {
            let p = league_pct[li];
            li += 1;
            nation_league_pct.insert(k.clone(), p);
            (3800.0 + 5200.0 * p.powf(0.9), 0.25 + 0.75 * p.powf(1.2))
        } else {
            let p = (s.max(1.0).ln() / 22.0).clamp(0.0, 1.0);
            (600.0 + 2800.0 * p, 0.10 + 0.20 * p)
        };
        nation_rep.insert(k.clone(), (rep.round() as u16, econ as f32, (7.0 + rep / 9500.0 * 9.0).round().clamp(4.0, 18.0) as u8));
    }
    for k in minor_nations.keys() {
        let g = minor_nations[k];
        nations.insert(k.clone(), ImpNation { source: src, key: k.clone(), code: g.code.to_string(), name: g.name.to_string(), confed: Some(g.confed), minor: true, ..Default::default() });
    }
    for n in nations.values_mut() {
        if let Some(&(rep, econ, youth)) = nation_rep.get(&n.key) {
            n.reputation = Some(rep);
            n.economy = Some(econ);
            n.youth_rating = Some(youth);
        }
    }
    for c in &mut comps {
        if c.reputation.is_none()
            && let Some(n) = c.nation.as_ref().and_then(|n| nation_rep.get(n))
        {
            c.reputation = Some(n.0);
        }
    }
    // Squad-value-less clubs get their league's floor from the assembler.

    // ---- transfers → spells -------------------------------------------------------------------
    if let Some(mut t) = open(dir, "transfers.csv", false)? {
        let mut future = 0u32;
        let mut moves: FxHashMap<Key, Vec<(Date, Key, Option<i64>)>> = FxHashMap::default();
        t.for_each(|row, r| {
            let pid = r.s("player_id");
            if !player_keys.contains(pid.as_ref()) {
                return;
            }
            let Some(date) = r.date("transfer_date") else {
                set.issues.add(Severity::Warning, "bad_date", "transfers.csv", row, &pid, "transfer date is not a date");
                return;
            };
            if date > start {
                future += 1;
                return;
            }
            let to = r.s("to_club_id").to_string();
            let fee = r.f64("transfer_fee").filter(|f| *f >= 0.0).map(|f| f.round() as i64);
            if fee.is_none() && !r.s("transfer_fee").is_empty() {
                set.issues.add(Severity::Warning, "bad_amount", "transfers.csv", row, &pid, format!("fee `{}` is not a valid amount; treated as unknown", r.s("transfer_fee")));
            }
            moves.entry(pid.to_string()).or_default().push((date, to, fee));
        })?;
        if future > 0 {
            set.issues.add(Severity::Info, "future_transfer", "transfers.csv", 0, "", format!("{future} transfers are dated after the start date (pre-agreed moves); not applied"));
        }
        for (player, mut list) in moves {
            list.sort_by_key(|m| m.0);
            for (i, (date, to, fee)) in list.iter().enumerate() {
                if !club_keys.contains(to) {
                    continue; // a move to a club outside the modelled world leaves a gap, not a guess
                }
                let end = list.get(i + 1).map(|n| n.0);
                set.spells.push(ImpSpell { player: player.clone(), club: to.clone(), from: *date, to: end, fee: *fee, loan: false });
            }
        }
        set.spells.sort_by(|a, b| a.player.cmp(&b.player).then(a.from.cmp(&b.from)));
    }
    // The date a player joined the current club: the open spell there (a last move elsewhere would have ended it).
    let cur_club: FxHashMap<Key, Key> = players.iter().filter_map(|p| p.club.clone().map(|c| (p.key.clone(), c))).collect();
    let last_open: FxHashMap<Key, (Key, Date)> = set.spells.iter().filter(|s| s.to.is_none()).map(|s| (s.player.clone(), (s.club.clone(), s.from))).collect();
    for p in &mut players {
        if let Some((club, from)) = last_open.get(&p.key)
            && Some(club) == p.club.as_ref()
        {
            p.joined = Some(*from);
        }
    }

    // ---- games → league seasons ------------------------------------------------------------------
    let mut game_meta: FxHashMap<String, (Key, i32)> = FxHashMap::default();
    let mut game_dates: FxHashMap<String, Option<Date>> = FxHashMap::default();
    if let Some(mut t) = open(dir, "games.csv", false)? {
        #[derive(Default)]
        struct Row {
            pts: i32,
            gd: i32,
            gf: i32,
            games: u32,
        }
        let league_ids: FxHashSet<Key> = comps.iter().filter(|c| c.kind == CompKind::League).map(|c| c.key.clone()).collect();
        let mut tables: FxHashMap<(Key, i32), FxHashMap<Key, Row>> = FxHashMap::default();
        let mut first_game: FxHashMap<(Key, i32), Date> = FxHashMap::default();
        t.for_each(|_, r| {
            let Some(date) = r.date("date") else { return; };
            game_dates.entry(r.s("game_id").to_string()).and_modify(|existing| {
                if *existing != Some(date) { *existing = None; }
            }).or_insert(Some(date));
            if date > start { return; }
            let comp = r.s("competition_id");
            if !league_ids.contains(comp.as_ref()) {
                return;
            }
            let (Some(season), Some(hg), Some(ag)) = (r.num::<i32>("season"), r.int("home_club_goals"), r.int("away_club_goals")) else { return };
            let (h, a) = (r.s("home_club_id").to_string(), r.s("away_club_id").to_string());
            game_meta.insert(r.s("game_id").to_string(), (comp.to_string(), season));
            if let Some(d) = r.date("date") {
                let e = first_game.entry((comp.to_string(), season)).or_insert(d);
                *e = (*e).min(d);
            }
            let tab = tables.entry((comp.to_string(), season)).or_default();
            let (hp, ap) = if hg > ag { (3, 0) } else if hg < ag { (0, 3) } else { (1, 1) };
            for (club, pts, gf, ga) in [(h, hp, hg, ag), (a, ap, ag, hg)] {
                let e = tab.entry(club).or_default();
                e.pts += pts;
                e.gf += gf as i32;
                e.gd += (gf - ga) as i32;
                e.games += 1;
            }
        })?;
        // Season calendar (INFERRED from imported dates): a league whose seasons usually open between January and May runs
        // through the calendar year; one that opens later runs autumn to spring. Needs three recent seasons to decide.
        let mut opens: FxHashMap<Key, Vec<u32>> = FxHashMap::default();
        for ((comp, season), d) in &first_game {
            if *season >= newest - 6 && *season < newest {
                opens.entry(comp.clone()).or_default().push(d.month());
            }
        }
        for c in comps.iter().filter(|c| c.kind == CompKind::League) {
            let (Some(months), Some(nation)) = (opens.get(&c.key), c.nation.as_ref().and_then(|n| nations.get_mut(n))) else { continue };
            if months.len() >= 3 {
                let spring = months.iter().filter(|m| (1..=5).contains(*m)).count();
                nation.calendar = Some(if spring * 2 > months.len() { "calendar_year" } else { "autumn_spring" }.to_string());
            }
        }
        // Club standing gets a second, independent input: where the club finished in its own league last season. Squad value alone
        // would count the same price signal twice once it also feeds each player's ability (locked design §11.14).
        let mut position_pct: FxHashMap<Key, f64> = FxHashMap::default();
        for ((comp, season), tab) in &tables {
            if *season != newest {
                continue;
            }
            let mut ppg: Vec<(&Key, f64)> = tab.iter().filter(|(_, r)| r.games >= 10).map(|(club, r)| (club, f64::from(r.pts) / f64::from(r.games))).collect();
            if ppg.len() < 6 {
                continue;
            }
            ppg.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(b.0)));
            let n = ppg.len() as f64 - 1.0;
            for (i, (club, _)) in ppg.into_iter().enumerate() {
                position_pct.insert(club.clone(), i as f64 / n);
            }
            let _ = comp;
        }
        for c in &mut clubs {
            if let (Some(pv), Some(pp), Some(np)) = (value_pct.get(&c.key), position_pct.get(&c.key), c.nation.as_ref().and_then(|n| nation_league_pct.get(n))) {
                let blend = 0.65 * pv + 0.35 * (0.5 * np + 0.5 * pp);
                c.reputation = Some((2200.0 + 7300.0 * blend.powf(1.35)).round() as u16);
            }
        }
        let mut keys: Vec<_> = tables.keys().cloned().collect();
        keys.sort();
        for k in keys {
            let tab = &tables[&k];
            let n = tab.len() as u32;
            // Only a complete double round robin gives a table that decides the title; splits, playoffs and
            // multi-stage seasons are left to be generated.
            let complete = n >= 4 && tab.values().all(|r| r.games == 2 * (n - 1));
            if !complete || k.1 >= newest {
                set.issues.add(Severity::Info, "season_not_derived", "games.csv", 0, &format!("{}:{}", k.0, k.1), "not a complete double round robin, or the season is not finished");
                continue;
            }
            let mut order: Vec<(&Key, &Row)> = tab.iter().collect();
            order.sort_by(|a, b| b.1.pts.cmp(&a.1.pts).then(b.1.gd.cmp(&a.1.gd)).then(b.1.gf.cmp(&a.1.gf)).then(a.0.cmp(b.0)));
            // Equal on points, goal difference and goals scored: the source does not decide it.
            if order.len() > 1 && (order[0].1.pts, order[0].1.gd, order[0].1.gf) == (order[1].1.pts, order[1].1.gd, order[1].1.gf) {
                set.issues.add(Severity::Info, "season_not_derived", "games.csv", 0, &format!("{}:{}", k.0, k.1), "title tie not decidable from the results");
                continue;
            }
            let champion = order[0].0.clone();
            if !club_keys.contains(&champion) {
                set.issues.add(Severity::Info, "season_not_derived", "games.csv", 0, &format!("{}:{}", k.0, k.1), "champion is not a modelled club");
                continue;
            }
            let runner_up = order.get(1).map(|r| r.0.clone()).filter(|c| club_keys.contains(c));
            set.seasons.push(ImpSeason { comp: k.0.clone(), season: k.1, champion, runner_up, top_scorer: None, top_goals: None });
        }
    }

    // ---- match files: career counters, top scorers, shirt numbers ---------------------------------------
    if opt.with_match_files {
        if let Some(mut t) = open(dir, "appearances.csv", false)? {
            let national: FxHashSet<Key> = {
                let mut ids = FxHashSet::default();
                if let Some(mut c) = open(dir, "competitions.csv", false)? {
                    c.for_each(|_, r| {
                        if r.s("type") == "national_team_competition" {
                            ids.insert(r.s("competition_id").to_string());
                        }
                    })?;
                }
                ids
            };
            let mut tot: FxHashMap<Key, (u32, Option<u32>)> = FxHashMap::default();
            // Minutes in the year before the start: the level of football a player is actually trusted with.
            let mut recent: FxHashMap<Key, u32> = FxHashMap::default();
            let window_from = start.add_days(-365);
            let mut scorers: FxHashMap<(Key, i32), FxHashMap<String, (String, u32)>> = FxHashMap::default();
            let season_ids: FxHashSet<(Key, i32)> = set.seasons.iter().map(|s| (s.comp.clone(), s.season)).collect();
            t.for_each(|_, r| {
                let Some(date) = r.date("date").or_else(|| game_dates.get(r.s("game_id").as_ref()).copied().flatten())
                    .filter(|&d| d <= start) else { return; };
                let comp = r.s("competition_id");
                if national.contains(comp.as_ref()) {
                    return;
                }
                let pid = r.s("player_id");
                let goals = r.num::<u32>("goals");
                let minutes = r.num::<u32>("minutes_played");
                if player_keys.contains(pid.as_ref()) && minutes.is_some_and(|m| m > 0) {
                    let e = tot.entry(pid.to_string()).or_insert((0, Some(0)));
                    e.0 += 1;
                    e.1 = e.1.zip(goals).map(|(a, b)| a.saturating_add(b));
                }
                if player_keys.contains(pid.as_ref()) && date > window_from && let Some(m) = minutes {
                    *recent.entry(pid.to_string()).or_default() += m;
                }
                if let Some(goals) = goals.filter(|&g| g > 0)
                    && let Some((c, s)) = game_meta.get(r.s("game_id").as_ref())
                    && season_ids.contains(&(c.clone(), *s))
                {
                    let e = scorers.entry((c.clone(), *s)).or_default().entry(pid.to_string()).or_insert_with(|| (r.s("player_name").to_string(), 0));
                    e.1 += goals;
                }
            })?;
            for p in &mut players {
                if let Some(&(a, g)) = tot.get(&p.key) {
                    p.apps = Some(a);
                    p.goals = g;
                }
                // No recorded appearance is not evidence of zero playing time outside this dataset.
                p.minutes_12m = recent.get(&p.key).copied();
            }
            for s in &mut set.seasons {
                if let Some(best) = scorers.get(&(s.comp.clone(), s.season)).and_then(|m| m.values().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))) {
                    // A shared top spot is not named: the source would have to break the tie.
                    let tied = scorers[&(s.comp.clone(), s.season)].values().filter(|v| v.1 == best.1).count() > 1;
                    if !tied {
                        s.top_scorer = Some(best.0.clone());
                        s.top_goals = Some(best.1.min(u32::from(u16::MAX)) as u16);
                    }
                }
            }
        }
        if let Some(mut t) = open(dir, "game_lineups.csv", false)? {
            let mut latest: FxHashMap<Key, (Date, u8)> = FxHashMap::default();
            let missing_roles: FxHashSet<&str> = players.iter().filter(|p| p.positions.is_empty()).map(|p| p.key.as_str()).collect();
            let mut roles: FxHashMap<Key, FxHashMap<Pos, u32>> = FxHashMap::default();
            t.for_each(|_, r| {
                let pid = r.s("player_id");
                let Some(club) = cur_club.get(pid.as_ref()) else { return };
                if r.s("club_id") != club.as_str() {
                    return;
                }
                let Some(date) = r.date("date").filter(|&d| d <= start) else { return };
                if missing_roles.contains(pid.as_ref()) && date >= start.add_days(-730) && let Some(&pos) = position_of(&r.s("position")).first() {
                    *roles.entry(pid.to_string()).or_default().entry(pos).or_default() += 1;
                }
                let Some(n) = r.num::<u8>("number") else { return };
                if !(1..=99).contains(&n) {
                    return;
                }
                let e = latest.entry(pid.to_string()).or_insert((date, n));
                if date > e.0 || (date == e.0 && n < e.1) {
                    *e = (date, n);
                }
            })?;
            for p in &mut players {
                p.shirt = latest.get(&p.key).map(|x| x.1);
                if p.positions.is_empty() && let Some(counts) = roles.get(&p.key) {
                    let total: u32 = counts.values().sum();
                    if let Some((&pos, &count)) = counts.iter().find(|(_, count)| **count >= 3 && **count * 2 > total) {
                        p.positions = vec![pos];
                        p.position_inferred = true;
                        set.issues.add(Severity::Info, "position_from_lineups", "game_lineups.csv", 0, &p.key, format!("role estimated from {count} of {total} recent club lineups"));
                    }
                }
            }
        }
    }

    set.nations = { let mut v: Vec<_> = nations.into_values().collect(); v.sort_by(|a, b| a.key.cmp(&b.key)); v };
    set.comps = comps;
    set.clubs = clubs;
    set.players = players;
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_drop_legal_words_but_keep_two_word_names() {
        assert_eq!(shorten("Bologna Football Club 1909"), "Bologna");
        assert_eq!(shorten("Club Atlético Vélez Sársfield"), "Vélez Sársfield");
        assert_eq!(shorten("Asociación Atlética Argentinos Juniors"), "Argentinos Juniors");
        assert_eq!(shorten("Paris FC"), "Paris FC");
        assert_eq!(shorten("Leicester City"), "Leicester City");
        assert_eq!(shorten("Sarpsborg 08 Fotballforening"), "Sarpsborg 08");
        assert_eq!(shorten("Real Madrid Club de Fútbol"), "Real Madrid");
    }

    #[test]
    fn slugs_become_names() {
        assert_eq!(title_slug("premier-league"), "Premier League");
        assert_eq!(title_slug("j1-league"), "J1 League");
    }

    #[test]
    fn percentiles_share_ties() {
        let p = percentiles(&[10.0, 20.0, 20.0, 40.0]);
        assert_eq!(p[0], 0.0);
        assert_eq!(p[1], p[2]);
        assert_eq!(p[3], 1.0);
    }
}
