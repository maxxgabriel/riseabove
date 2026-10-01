//! Normalisation and identity resolution over the validated import representation.
//!
//! * duplicate ids: the first record wins, later ones are dropped and reported;
//! * foreign keys: an unknown reference is fixed only where that loses nothing (a loan origin), otherwise the row is
//!   dropped into the unresolved report — a club that cannot be found is never replaced by "free agent";
//! * people are never merged because their names match: identity is the source id. Rows that look like one person under
//!   two ids are listed, not joined;
//! * staff are joined to clubs only with evidence (see [`link_staff`]).

use pw_core::Date;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::geo;
use crate::model::*;

const MIN_AGE: u32 = 14;
const MAX_AGE: u32 = 48;

fn drop_row(set: &mut ImportSet, source: u8, id: &str, what: &'static str, reason: impl Into<String>) {
    set.unresolved.push(UnresolvedRow { source, id: id.to_string(), what, reason: reason.into() });
}

/// Keep the first record for each key; report the rest.
fn dedupe<T>(items: &mut Vec<T>, key: impl Fn(&T) -> String, file: &str, issues: &mut Issues) {
    let mut seen: FxHashSet<String> = FxHashSet::default();
    items.retain(|it| {
        let k = key(it);
        let fresh = seen.insert(k.clone());
        if !fresh {
            issues.add(Severity::Error, "duplicate_id", file, 0, &k, "id already used by an earlier record; this one is dropped");
        }
        fresh
    });
}

/// A nation that people or clubs refer to but no nation row describes: created as a minor nation, from the country
/// table when the name or code is known there.
fn minor_nation(key: &str, source: u8) -> ImpNation {
    match geo::lookup(key).or_else(|| geo::by_code(key)) {
        Some(g) => ImpNation { source, key: key.to_string(), code: g.code.to_string(), name: g.name.to_string(), confed: Some(g.confed), minor: true, ..Default::default() },
        None => ImpNation { source, key: key.to_string(), code: key.to_ascii_uppercase().chars().take(3).collect(), name: key.to_string(), confed: None, minor: true, ..Default::default() },
    }
}

/// Fold a person's name into a comparison key that ignores order, case, accents and punctuation.
pub fn name_key(parts: &[&str]) -> String {
    let mut toks: Vec<String> = parts.iter().flat_map(|p| p.split(|c: char| !c.is_alphanumeric()).map(geo::fold).filter(|t| !t.is_empty()).collect::<Vec<_>>()).collect();
    toks.sort();
    toks.join(" ")
}

pub fn resolve(set: &mut ImportSet) {
    let start = set.start.unwrap_or_else(crate::csv_pack::default_start);
    set.start = Some(start);

    // Duplicate ids.
    let mut issues = std::mem::take(&mut set.issues);
    dedupe(&mut set.nations, |n| n.key.clone(), "nations", &mut issues);
    dedupe(&mut set.comps, |c| c.key.clone(), "competitions", &mut issues);
    dedupe(&mut set.clubs, |c| c.key.clone(), "clubs", &mut issues);
    dedupe(&mut set.players, |p| p.key.clone(), "players", &mut issues);
    set.issues = issues;

    // Nations that are referred to but not described.
    let mut have: FxHashSet<String> = set.nations.iter().map(|n| n.key.clone()).collect();
    let mut codes: FxHashSet<String> = set.nations.iter().map(|n| n.code.clone()).collect();
    let mut wanted: Vec<(String, u8)> = Vec::new();
    for c in &set.comps {
        if let Some(n) = &c.nation {
            wanted.push((n.clone(), c.source));
        }
    }
    for c in &set.clubs {
        if let Some(n) = &c.nation {
            wanted.push((n.clone(), c.source));
        }
    }
    for p in &set.players {
        for n in [&p.nationality, &p.nationality2].into_iter().flatten() {
            wanted.push((n.clone(), p.source));
        }
    }
    for s in &set.staff {
        for n in [&s.nationality, &s.nationality2].into_iter().flatten() {
            wanted.push((n.clone(), s.source));
        }
    }
    for (key, source) in wanted {
        if have.contains(&key) {
            continue;
        }
        let mut n = minor_nation(&key, source);
        if codes.contains(&n.code) {
            // A derived code that collides with another nation's gets a numeric suffix.
            let mut i = 2;
            while codes.contains(&format!("{}{}", n.code.chars().take(2).collect::<String>(), i)) {
                i += 1;
            }
            n.code = format!("{}{}", n.code.chars().take(2).collect::<String>(), i);
        }
        codes.insert(n.code.clone());
        have.insert(key.clone());
        set.issues.add(Severity::Info, "minor_nation", "nations", 0, &key, "referenced but not described; created as a minor nation");
        set.nations.push(n);
    }

    // Competitions need a nation unless continental.
    let nation_keys: FxHashSet<String> = set.nations.iter().map(|n| n.key.clone()).collect();
    let mut dropped: Vec<(u8, String, String)> = Vec::new();
    set.comps.retain(|c| {
        let ok = c.kind == pw_world::CompKind::Continental || c.nation.as_ref().is_some_and(|n| nation_keys.contains(n));
        if !ok {
            dropped.push((c.source, c.key.clone(), "competition has no known nation".into()));
        }
        ok
    });
    for (s, k, r) in dropped {
        set.issues.add(Severity::Error, "unknown_nation", "competitions", 0, &k, r.clone());
        drop_row(set, s, &k, "competition", r);
    }
    let comp_keys: FxHashSet<String> = set.comps.iter().map(|c| c.key.clone()).collect();

    // Clubs: nation must exist; a league that does not is cleared (the club stays, without a league).
    let mut dropped = Vec::new();
    set.clubs.retain(|c| {
        let ok = c.nation.as_ref().is_some_and(|n| nation_keys.contains(n));
        if !ok {
            dropped.push((c.source, c.key.clone()));
        }
        ok
    });
    for (s, k) in dropped {
        set.issues.add(Severity::Error, "unknown_nation", "clubs", 0, &k, "club has no known nation; dropped");
        drop_row(set, s, &k, "club", "club has no known nation");
    }
    for c in &mut set.clubs {
        if let Some(l) = &c.league
            && !comp_keys.contains(l)
        {
            set.issues.add(Severity::Warning, "unknown_league", "clubs", 0, &c.key, format!("league `{l}` does not exist; club has no league"));
            c.league = None;
        }
    }
    let club_keys: FxHashSet<String> = set.clubs.iter().map(|c| c.key.clone()).collect();

    // Entrants.
    let before = set.entrants.len();
    set.entrants.retain(|(comp, club)| comp_keys.contains(comp) && club_keys.contains(club));
    if set.entrants.len() < before {
        set.issues.add(Severity::Warning, "unknown_reference", "competition_entrants", 0, "", format!("{} entries name an unknown competition or club", before - set.entrants.len()));
    }

    // Players.
    let mut kept = Vec::with_capacity(set.players.len());
    let players = std::mem::take(&mut set.players);
    for mut p in players {
        if p.first.is_empty() && p.last.is_empty() && p.common.is_empty() {
            set.issues.add(Severity::Error, "missing_field", "players", p.row, &p.key, "player has no name");
            drop_row(set, p.source, &p.key, "player", "no name");
            continue;
        }
        let Some(dob) = p.dob else {
            set.issues.add(Severity::Error, "missing_dob", "players", p.row, &p.key, "no usable date of birth: a person cannot be created without one");
            drop_row(set, p.source, &p.key, "player", "no usable date of birth");
            continue;
        };
        let age = dob.age_on(start);
        if dob > start || !(MIN_AGE..=MAX_AGE).contains(&age) {
            set.issues.add(Severity::Error, "implausible_age", "players", p.row, &p.key, format!("age {age} at the start date"));
            drop_row(set, p.source, &p.key, "player", format!("age {age} at the start date is not plausible"));
            continue;
        }
        if let Some(c) = &p.club
            && !club_keys.contains(c)
        {
            set.issues.add(Severity::Error, "unknown_club", "players", p.row, &p.key, format!("club `{c}` does not exist"));
            drop_row(set, p.source, &p.key, "player", format!("club `{c}` does not exist"));
            continue;
        }
        if let Some(l) = &p.loan_from
            && (!club_keys.contains(l) || p.club.is_none())
        {
            set.issues.add(Severity::Warning, "unknown_loan_club", "players", p.row, &p.key, format!("loan parent `{l}` is unknown; loan ignored"));
            p.loan_from = None;
            p.loan_end = None;
        }
        if p.nationality.is_none() && p.club.is_none() {
            set.issues.add(Severity::Error, "no_nationality", "players", p.row, &p.key, "a free agent with no readable nationality has none to inherit");
            drop_row(set, p.source, &p.key, "player", "no readable nationality and no club to take one from");
            continue;
        }
        if p.positions.is_empty() && p.position_group.is_none() {
            set.issues.add(Severity::Warning, "no_position", "players", p.row, &p.key, "no position; an assumed midfielder role is used");
        }
        if p.joined.is_some_and(|j| j > start) {
            p.joined = None;
        }
        kept.push(p);
    }
    set.players = kept;

    // Same name and birth date under different ids: reported, not merged.
    let mut by_identity: FxHashMap<(String, Date), Vec<usize>> = FxHashMap::default();
    for (i, p) in set.players.iter().enumerate() {
        if let Some(d) = p.dob {
            by_identity.entry((name_key(&[&p.first, &p.last, &p.common]), d)).or_default().push(i);
        }
    }
    let mut dups: Vec<(String, String, String)> = Vec::new();
    for ((name, dob), idx) in &by_identity {
        if idx.len() > 1 {
            for w in idx.windows(2) {
                dups.push((set.players[w[0]].key.clone(), set.players[w[1]].key.clone(), format!("same name `{name}` and birth date {dob:?}")));
            }
        }
    }
    dups.sort();
    for (a, b, why) in &dups {
        set.issues.add(Severity::Warning, "possible_duplicate", "players", 0, a, format!("{a} and {b}: {why}; kept as two people"));
    }
    set.possible_duplicates = dups;
    let player_keys: FxHashSet<String> = set.players.iter().map(|p| p.key.clone()).collect();

    // Spells and seasons.
    let n = set.spells.len();
    set.spells.retain(|s| player_keys.contains(&s.player) && club_keys.contains(&s.club));
    if set.spells.len() < n {
        set.issues.add(Severity::Info, "history_outside_world", "transfers", 0, "", format!("{} history rows refer to a player or club that is not in the world", n - set.spells.len()));
    }
    let n = set.seasons.len();
    set.seasons.retain(|s| comp_keys.contains(&s.comp) && club_keys.contains(&s.champion));
    if set.seasons.len() < n {
        set.issues.add(Severity::Warning, "unknown_reference", "history", 0, "", format!("{} past seasons name an unknown competition or champion", n - set.seasons.len()));
    }
    for s in &mut set.seasons {
        if s.runner_up.as_ref().is_some_and(|r| !club_keys.contains(r)) {
            s.runner_up = None;
        }
    }

    link_staff(set, &club_keys);
}

/// Attach staff to clubs, with evidence only.
///
/// A club's own manager name is known from its record. A staff-list `Manager` whose name matches it (same words in any
/// order) and who is the only such candidate is that club's manager; the match also teaches the list's spelling of the
/// club (`Man Utd`). The alias then places the same team's other staff, provided the spelling belongs to exactly one
/// manager row in the whole list. Everything else is reported as unresolved and not created: placing someone at a
/// guessed club would be inventing their employment. Staff with no team in the list are kept as unemployed.
pub fn link_staff(set: &mut ImportSet, club_keys: &FxHashSet<String>) {
    // Stated links must point at clubs that exist.
    let mut stale = Vec::new();
    for s in &mut set.staff {
        if let Some(c) = &s.club
            && !club_keys.contains(c)
        {
            stale.push((s.source, s.key.clone(), c.clone()));
            s.club = None;
            s.club_link = None;
        }
    }
    for (_, key, c) in stale {
        set.issues.add(Severity::Warning, "unknown_club", "staff", 0, &key, format!("club `{c}` does not exist; staff member left unattached"));
    }

    let mut by_name: FxHashMap<String, Vec<usize>> = FxHashMap::default();
    for (i, s) in set.staff.iter().enumerate() {
        if s.role == Some(pw_world::StaffRole::Manager) && s.club.is_none() && s.team_text.is_some() {
            by_name.entry(name_key(&[&s.first, &s.last])).or_default().push(i);
        }
    }
    // How many manager rows carry each team spelling: an alias is trusted only when exactly one does.
    let mut managers_per_text: FxHashMap<String, u32> = FxHashMap::default();
    for s in &set.staff {
        if s.role == Some(pw_world::StaffRole::Manager)
            && let Some(t) = &s.team_text
        {
            *managers_per_text.entry(geo::fold(t)).or_default() += 1;
        }
    }
    // A staff row matched by two clubs is ambiguous for both.
    let mut claims: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
    let mut bare: Vec<usize> = Vec::new();
    for (ci, c) in set.clubs.iter().enumerate() {
        let Some(name) = &c.manager_name else { continue };
        let cands = by_name.get(&name_key(&[name])).cloned().unwrap_or_default();
        match cands.len() {
            1 => claims.entry(cands[0]).or_default().push(ci),
            0 => bare.push(ci),
            _ => {
                set.issues.add(Severity::Warning, "ambiguous_manager", "clubs", 0, &c.key, format!("{} staff rows match manager `{name}`", cands.len()));
                bare.push(ci);
            }
        }
    }
    let mut alias: FxHashMap<String, Option<usize>> = FxHashMap::default();
    for (si, clubs) in claims {
        if clubs.len() != 1 {
            for &ci in &clubs {
                set.issues.add(Severity::Warning, "ambiguous_manager", "clubs", 0, &set.clubs[ci].key.clone(), "the same staff row matches several clubs' managers");
                bare.push(ci);
            }
            continue;
        }
        let ci = clubs[0];
        let ckey = set.clubs[ci].key.clone();
        set.staff[si].club = Some(ckey);
        set.staff[si].club_link = Some(Link::Inferred);
        if let Some(t) = set.staff[si].team_text.clone()
            && managers_per_text.get(&geo::fold(&t)) == Some(&1)
        {
            alias.insert(geo::fold(&t), Some(ci));
        }
    }
    // Same spelling learned for two clubs cannot happen with the single-manager rule, but stay safe.
    let mut placed = 0u32;
    for s in &mut set.staff {
        if s.club.is_some() {
            continue;
        }
        let Some(t) = &s.team_text else { continue };
        if let Some(Some(ci)) = alias.get(&geo::fold(t)) {
            s.club = Some(set.clubs[*ci].key.clone());
            s.club_link = Some(Link::Inferred);
            placed += 1;
        }
    }
    if placed > 0 {
        set.issues.add(Severity::Info, "staff_placed_by_alias", "staff", 0, "", format!("{placed} staff placed at a club through the team spelling learned from its manager"));
    }
    // Rows with a team that no evidence placed: reported, then dropped.
    let staff = std::mem::take(&mut set.staff);
    for s in staff {
        if s.club.is_none() && s.team_text.is_some() {
            set.issues.add(Severity::Info, "staff_club_unresolved", "staff", s.row, &s.key, format!("team `{}` could not be tied to a modelled club", s.team_text.as_deref().unwrap_or("")));
            drop_row(set, s.source, &s.key, "staff", format!("team `{}` not matched to a modelled club", s.team_text.as_deref().unwrap_or("")));
        } else {
            set.staff.push(s);
        }
    }
    // A club's manager that the staff list does not describe is created from the club's own record.
    bare.sort_unstable();
    bare.dedup();
    for ci in bare {
        let c = &set.clubs[ci];
        let Some(name) = c.manager_name.clone() else { continue };
        if set.staff.iter().any(|s| s.club.as_deref() == Some(c.key.as_str()) && s.role == Some(pw_world::StaffRole::Manager)) {
            continue;
        }
        let mut s = ImpStaff::new(format!("manager:{}", c.key), 0);
        s.source = c.source;
        s.last = name;
        s.role = Some(pw_world::StaffRole::Manager);
        s.club = Some(c.key.clone());
        s.club_link = Some(Link::Stated);
        set.staff.push(s);
    }
    // A person needs a nation. Staff at a club inherit the club's (labelled as inferred); an unemployed person whose nation the
    // source does not give in a form we can read is reported and left out, not given someone else's.
    let staff = std::mem::take(&mut set.staff);
    for s in staff {
        if s.club.is_none() && s.nationality.is_none() {
            set.issues.add(Severity::Warning, "no_nationality", "staff", s.row, &s.key, "no readable nationality and no club to take one from; left out");
            drop_row(set, s.source, &s.key, "staff", "no readable nationality and no club to take one from");
        } else {
            set.staff.push(s);
        }
    }
    // One manager per club: a second staff-list manager linked to the same club is demoted to unresolved.
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let staff = std::mem::take(&mut set.staff);
    for s in staff {
        if s.role == Some(pw_world::StaffRole::Manager)
            && let Some(c) = &s.club
            && !seen.insert(c.clone())
        {
            set.issues.add(Severity::Warning, "second_manager", "staff", s.row, &s.key, format!("club `{c}` already has a manager"));
            drop_row(set, s.source, &s.key, "staff", format!("club `{c}` already has a manager"));
            continue;
        }
        set.staff.push(s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pw_world::{CompKind, StaffRole};

    fn nation(key: &str) -> ImpNation {
        ImpNation { key: key.into(), code: key.into(), name: key.into(), ..Default::default() }
    }

    fn club(key: &str, nation: &str, manager: Option<&str>) -> ImpClub {
        ImpClub { key: key.into(), name: format!("Club {key}"), nation: Some(nation.into()), manager_name: manager.map(Into::into), ..Default::default() }
    }

    fn player(key: &str, club: Option<&str>, dob: Option<Date>) -> ImpPlayer {
        let mut p = ImpPlayer::new(key, 1);
        p.first = "Ana".into();
        p.last = format!("Player{key}");
        p.dob = dob;
        p.club = club.map(Into::into);
        p.nationality = Some("AAA".into());
        p
    }

    fn base() -> ImportSet {
        let mut s = ImportSet { start: Some(Date::from_ymd(2026, 7, 15)), ..Default::default() };
        s.nations.push(nation("AAA"));
        s.comps.push(ImpComp::new("L1", "League", CompKind::League));
        s.comps[0].nation = Some("AAA".into());
        s.clubs.push(club("c1", "AAA", None));
        s.clubs[0].league = Some("L1".into());
        s
    }

    #[test]
    fn duplicate_ids_keep_the_first_and_report() {
        let mut s = base();
        let d = Some(Date::from_ymd(2000, 1, 1));
        s.players.push(player("1", Some("c1"), d));
        let mut second = player("1", Some("c1"), d);
        second.last = "Other".into();
        s.players.push(second);
        resolve(&mut s);
        assert_eq!(s.players.len(), 1);
        assert_eq!(s.players[0].last, "Player1");
        assert_eq!(s.issues.count("duplicate_id"), 1);
    }

    #[test]
    fn invalid_foreign_keys_and_dates_are_dropped_into_the_unresolved_report_not_defaulted() {
        let mut s = base();
        s.players.push(player("ok", Some("c1"), Some(Date::from_ymd(2000, 1, 1))));
        s.players.push(player("nodob", Some("c1"), None));
        s.players.push(player("ghost", Some("nope"), Some(Date::from_ymd(2000, 1, 1))));
        s.players.push(player("baby", Some("c1"), Some(Date::from_ymd(2024, 1, 1))));
        s.players.push(player("ancient", Some("c1"), Some(Date::from_ymd(1900, 1, 1))));
        s.players.push(player("free", None, Some(Date::from_ymd(2000, 1, 1))));
        resolve(&mut s);
        let kept: Vec<_> = s.players.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(kept, ["ok", "free"], "an explicit empty club is a free agent; an unknown club is not");
        let mut lost: Vec<_> = s.unresolved.iter().map(|u| u.id.as_str()).collect();
        lost.sort();
        assert_eq!(lost, ["ancient", "baby", "ghost", "nodob"]);
        assert_eq!(s.issues.count("missing_dob"), 1);
        assert_eq!(s.issues.count("unknown_club"), 1);
        assert_eq!(s.issues.count("implausible_age"), 2);
    }

    #[test]
    fn same_name_alone_never_merges_people() {
        let mut s = base();
        let d = Some(Date::from_ymd(2000, 1, 1));
        let mut a = player("1", Some("c1"), d);
        let mut b = player("2", Some("c1"), Some(Date::from_ymd(1999, 5, 5)));
        let mut c = player("3", Some("c1"), d);
        for p in [&mut a, &mut b, &mut c] {
            p.first = "Jose".into();
            p.last = "Gomez".into();
        }
        s.players.extend([a, b, c]);
        resolve(&mut s);
        assert_eq!(s.players.len(), 3, "nobody is merged");
        assert_eq!(s.possible_duplicates.len(), 1, "only same name AND birth date is flagged");
        assert_eq!((s.possible_duplicates[0].0.as_str(), s.possible_duplicates[0].1.as_str()), ("1", "3"));
    }

    #[test]
    fn unknown_nations_become_minor_nations_with_unique_codes() {
        let mut s = base();
        let mut p = player("1", Some("c1"), Some(Date::from_ymd(2000, 1, 1)));
        p.nationality = Some("Ghana".into());
        let mut q = player("2", Some("c1"), Some(Date::from_ymd(2000, 1, 1)));
        q.nationality = Some("Zzzland".into());
        s.players.extend([p, q]);
        resolve(&mut s);
        let gh = s.nations.iter().find(|n| n.key == "Ghana").unwrap();
        assert_eq!((gh.code.as_str(), gh.minor), ("GHA", true));
        let z = s.nations.iter().find(|n| n.key == "Zzzland").unwrap();
        assert_eq!(z.confed, None, "an unknown country's confederation stays unknown");
    }

    #[test]
    fn a_comp_without_a_nation_and_a_league_that_does_not_exist_are_handled() {
        let mut s = base();
        s.comps.push(ImpComp::new("L2", "Nowhere League", CompKind::League));
        s.clubs.push(club("c2", "AAA", None));
        s.clubs[1].league = Some("GHOST".into());
        resolve(&mut s);
        assert_eq!(s.comps.len(), 1);
        assert_eq!(s.clubs[1].league, None);
        assert_eq!(s.issues.count("unknown_league"), 1);
        assert!(s.unresolved.iter().any(|u| u.id == "L2" && u.what == "competition"));
    }

    fn staff(row: usize, name: (&str, &str), role: StaffRole, team: Option<&str>) -> ImpStaff {
        let mut s = ImpStaff::new(format!("row{row}"), row);
        s.first = name.0.into();
        s.last = name.1.into();
        s.role = Some(role);
        s.team_text = team.map(Into::into);
        s.nationality = Some("AAA".into());
        s
    }

    #[test]
    fn staff_join_clubs_only_through_evidence() {
        let mut s = base();
        s.clubs.push(club("c2", "AAA", Some("Jose Gomez")));
        s.clubs.push(club("c3", "AAA", Some("Nobody Listed")));
        s.staff.push(staff(1, ("José", "Gómez"), StaffRole::Manager, Some("Man Utd")));
        s.staff.push(staff(2, ("Pat", "Coach"), StaffRole::Coach, Some("Man Utd")));
        s.staff.push(staff(3, ("Lee", "Elsewhere"), StaffRole::Coach, Some("Carshalton")));
        s.staff.push(staff(4, ("Una", "Free"), StaffRole::Scout, None));
        resolve(&mut s);
        let by = |name: &str| s.staff.iter().find(|x| x.last == name);
        assert_eq!(by("Gómez").unwrap().club.as_deref(), Some("c2"), "name matched the club's own manager");
        assert_eq!(by("Gómez").unwrap().club_link, Some(Link::Inferred));
        assert_eq!(by("Coach").unwrap().club.as_deref(), Some("c2"), "team spelling learned from its manager");
        assert!(by("Elsewhere").is_none(), "unmatched team is not placed at a guessed club");
        assert!(s.unresolved.iter().any(|u| u.id == "row3"));
        let free = by("Free").unwrap();
        assert_eq!((free.club.clone(), free.team_text.clone()), (None, None), "no team in the list means unemployed");
        // The club whose manager the list does not describe gets him from its own record, as stated.
        let bare = s.staff.iter().find(|x| x.club.as_deref() == Some("c3")).unwrap();
        assert_eq!((bare.last.as_str(), bare.club_link), ("Nobody Listed", Some(Link::Stated)));
    }

    #[test]
    fn a_team_spelling_shared_by_two_manager_rows_teaches_nothing() {
        let mut s = base();
        s.clubs.push(club("c2", "AAA", Some("Ann One")));
        s.staff.push(staff(1, ("Ann", "One"), StaffRole::Manager, Some("Independiente")));
        s.staff.push(staff(2, ("Bob", "Two"), StaffRole::Manager, Some("Independiente")));
        s.staff.push(staff(3, ("Pat", "Coach"), StaffRole::Coach, Some("Independiente")));
        resolve(&mut s);
        assert_eq!(s.staff.iter().find(|x| x.last == "One").unwrap().club.as_deref(), Some("c2"));
        assert!(s.staff.iter().all(|x| x.last != "Coach"), "the shared spelling could be either club");
    }

    #[test]
    fn a_staff_row_matching_two_clubs_managers_is_ambiguous() {
        let mut s = base();
        s.clubs.push(club("c2", "AAA", Some("Sam Same")));
        s.clubs.push(club("c3", "AAA", Some("Sam Same")));
        s.staff.push(staff(1, ("Sam", "Same"), StaffRole::Manager, Some("X")));
        resolve(&mut s);
        assert!(s.staff.iter().all(|x| x.key != "row1"), "not placed at either club");
        assert!(s.issues.count("ambiguous_manager") >= 2);
    }
}
