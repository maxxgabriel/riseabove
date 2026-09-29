//! Adapter for `Staff list.csv`: Windows-1252, `;`-separated, `Last, First` names, French nation names, abbreviated
//! team names and coaching ratings written as `57% (3.0)`.
//!
//! The file has no ids and no club keys, so club links are made later by evidence (`resolve::link_staff`).
//! Only coaching roles keep their ratings: for scouts, physios, analysts and sports scientists the same columns
//! hold a uniform placeholder, so those roles' attributes stay UNKNOWN.

use std::path::Path;

use pw_core::StaffAttr;
use pw_world::StaffRole;

use crate::ImportError;
use crate::geo;
use crate::model::*;
use crate::table::{Opts, Table};

pub const FILE: &str = "Staff list.csv";

/// `57% (3.0)` → 57.0
fn percent(s: &str) -> Option<f32> {
    s.split('%').next()?.trim().parse::<f32>().ok().filter(|v| (0.0..=100.0).contains(v))
}

fn scale(p: f32) -> u8 {
    (1.0 + p / 100.0 * 19.0).round().clamp(1.0, 20.0) as u8
}

fn mean(vs: &[Option<f32>]) -> Option<u8> {
    let known: Vec<f32> = vs.iter().flatten().copied().collect();
    if known.is_empty() { None } else { Some(scale(known.iter().sum::<f32>() / known.len() as f32)) }
}

enum Job {
    Role(StaffRole, bool),
    Board,
    Unsupported,
}

/// The role and whether the ratings columns carry real coaching values for it.
fn job(s: &str) -> Job {
    use StaffRole::*;
    match s {
        "Manager" => Job::Role(Manager, true),
        "Assistant Manager" => Job::Role(Assistant, true),
        "Coach" | "Youth Coach" | "Youth Team Manager" => Job::Role(Coach, true),
        "Fitness Coach" => Job::Role(FitnessCoach, true),
        "Goalkeeping Coach" => Job::Role(GkCoach, true),
        "Head of Youth Development" => Job::Role(HeadOfYouth, true),
        "Scout" | "Chief Scout" => Job::Role(Scout, false),
        "Physio" | "Head Physio" => Job::Role(Physio, false),
        "Data Analyst" => Job::Role(Analyst, false),
        "Head of Sports Science" => Job::Role(SportsScientist, false),
        "Director Of Football" => Job::Role(DirectorOfFootball, false),
        "Owner" | "Chairman" | "Director" | "Managing Director" | "President" | "General Manager" => Job::Board,
        _ => Job::Unsupported,
    }
}

/// `Aarab, Hamza` → (`Hamza`, `Aarab`); a name without a comma is kept whole as the last name.
pub fn split_name(name: &str) -> (String, String) {
    match name.split_once(',') {
        Some((last, first)) => (first.trim().to_string(), last.trim().to_string()),
        None => (String::new(), name.trim().to_string()),
    }
}

pub fn parse_into(set: &mut ImportSet, dir: &Path) -> Result<(), ImportError> {
    let Some(mut t) = Table::open(dir, FILE, Opts { delimiter: b';', cp1252: true })? else { return Ok(()) };
    let src = set.add_source("fm-staff", None, "Staff list (Windows-1252, French nation names, abbreviated teams)");
    let mut rows = Vec::new();
    let bad = t.for_each(|row, r| {
        let name = r.s("name").to_string();
        let job_text = r.s("job").to_string();
        let nation = r.s("nation").to_string();
        let team = r.s("team").to_string();
        let age = r.num::<u8>("age");
        let pct = |c: &str| percent(&r.s(c));
        let v = [pct("def. tact."), pct("def. tech."), pct("att. tact."), pct("att. tech."), pct("poss. tact."), pct("poss. tech."), pct("strength"), pct("quickness"), pct("gk shot stopping"), pct("gk handling")];
        rows.push((row, name, job_text, nation, team, age, v, r.s("wage").to_string()));
    })?;
    if bad > 0 {
        set.issues.add(Severity::Error, "malformed_row", FILE, 0, "", format!("{bad} unreadable rows"));
    }
    let mut wages = 0u32;
    for (row, name, job_text, nation, team, age, v, wage) in rows {
        if name.is_empty() {
            set.issues.add(Severity::Error, "missing_field", FILE, row, "", "staff row has no name");
            continue;
        }
        let (role, ratings) = match job(&job_text) {
            Job::Role(r, ratings) => (r, ratings),
            Job::Board => {
                set.issues.add(Severity::Info, "board_role", FILE, row, &name, format!("`{job_text}` has no matching entity type"));
                continue;
            }
            Job::Unsupported => {
                set.issues.add(Severity::Info, "unsupported_role", FILE, row, &name, format!("`{job_text}` is not modelled"));
                continue;
            }
        };
        let (first, last) = split_name(&name);
        let mut s = ImpStaff::new(format!("row{row}"), row);
        s.source = src;
        s.first = first;
        s.last = last;
        s.age = age.filter(|a| (16..=90).contains(a));
        if age.is_some() && s.age.is_none() {
            set.issues.add(Severity::Warning, "out_of_range", FILE, row, &name, "age is not plausible; treated as unknown");
        }
        s.role = Some(role);
        let mut tokens = nation.split(" / ").map(str::trim);
        s.nationality = tokens.next().and_then(geo::french_country).map(|g| g.name.to_string());
        s.nationality2 = tokens.next().and_then(geo::french_country).map(|g| g.name.to_string());
        if s.nationality.is_none() {
            set.issues.add(Severity::Info, "unknown_country", FILE, row, &name, format!("nation `{nation}` is a region or not in the country table"));
        }
        if team != "-" && !team.is_empty() {
            s.team_text = Some(team);
        }
        if ratings {
            let [dta, dte, ata, ate, pta, pte, st, qu, gs, gh] = v;
            for (attr, val) in [
                (StaffAttr::Defending, mean(&[dta, dte])),
                (StaffAttr::Attacking, mean(&[ata, ate])),
                (StaffAttr::Tactical, mean(&[dta, ata, pta])),
                (StaffAttr::Technical, mean(&[dte, ate, pte])),
                (StaffAttr::Fitness, mean(&[st, qu])),
                (StaffAttr::Goalkeeping, mean(&[gs, gh])),
            ] {
                if let Some(x) = val {
                    s.attrs.push((attr, x));
                }
            }
        }
        if !wage.is_empty() && wage != "0" {
            wages += 1;
        }
        set.staff.push(s);
    }
    if wages > 0 {
        set.issues.add(Severity::Info, "unsupported_field", FILE, 0, "", format!("{wages} wages not imported: currency and period are not stated"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_ratings_and_jobs() {
        assert_eq!(split_name("Aarab, Hamza"), ("Hamza".into(), "Aarab".into()));
        assert_eq!(split_name("Pelé"), ("".into(), "Pelé".into()));
        assert_eq!(percent("57% (3.0)"), Some(57.0));
        assert_eq!(percent("nonsense"), None);
        assert_eq!(scale(0.0), 1);
        assert_eq!(scale(100.0), 20);
        assert!(matches!(job("Head Physio"), Job::Role(StaffRole::Physio, false)));
        assert!(matches!(job("Chairman"), Job::Board));
        assert!(matches!(job("Player / Manager"), Job::Unsupported));
    }

    #[test]
    fn a_small_staff_file_parses_with_evidence_rules() {
        let dir = std::env::temp_dir().join(format!("pw-staff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // "é" as Windows-1252 byte 0xE9.
        let mut bytes = b"\"Name\";\"Nation\";\"Team\";\"Job\";\"Age\";\"Wage\";\"Def. Tact.\";\"Def. Tech.\";\"Att. Tact.\";\"Att. Tech.\";\"Poss. Tact.\";\"Poss. Tech.\";\"Strength\";\"Quickness\";\"GK Shot Stopping\";\"GK Handling\";\"Best Rating\"\n".to_vec();
        bytes.extend_from_slice(b"\"Gomez, Jos\xe9\";\"Espagne / Pays Basque\";\"Man Utd\";\"Manager\";\"51\";\"1\xa0830\";\"57% (3.0)\";\"57% (3.0)\";\"63% (3.5)\";\"63% (3.5)\";\"60% (3.0)\";\"60% (3.0)\";\"40% (2.5)\";\"40% (2.5)\";\"30% (2.0)\";\"30% (2.0)\";\"61% (M)\"\n");
        bytes.extend_from_slice(b"\"Physio, Pat\";\"Angleterre\";\"-\";\"Physio\";\"40\";\"0\";\"20% (1.0)\";\"20% (1.0)\";\"20% (1.0)\";\"20% (1.0)\";\"20% (1.0)\";\"20% (1.0)\";\"20% (1.0)\";\"20% (1.0)\";\"20% (1.0)\";\"20% (1.0)\";\"30% (Ch)\"\n");
        bytes.extend_from_slice(b"\"Rich, Ann\";\"Qatar\";\"Paris SG\";\"Owner\";\"38\";\"0\";\"44% (2.5)\";\"42% (2.5)\";\"50% (3.0)\";\"48% (2.5)\";\"50% (3.0)\";\"46% (2.5)\";\"47% (2.5)\";\"47% (2.5)\";\"32% (2.0)\";\"30% (2.0)\";\"62% (Ch)\"\n");
        std::fs::write(dir.join(FILE), bytes).unwrap();
        let mut set = ImportSet::default();
        parse_into(&mut set, &dir).unwrap();
        assert_eq!(set.staff.len(), 2, "owner is not a staff role");
        let m = &set.staff[0];
        assert_eq!((m.first.as_str(), m.last.as_str()), ("José", "Gomez"));
        assert_eq!(m.nationality.as_deref(), Some("Spain"));
        assert_eq!(m.nationality2, None, "a region is not a nationality");
        assert_eq!(m.team_text.as_deref(), Some("Man Utd"));
        assert_eq!(m.age, Some(51));
        assert!(m.attrs.iter().any(|(a, _)| *a == StaffAttr::Goalkeeping));
        assert!(set.staff[1].attrs.is_empty(), "placeholder ratings of a physio are not attributes");
        assert_eq!(set.issues.count("board_role"), 1);
        assert_eq!(set.issues.count("unsupported_field"), 1);
    }
}
