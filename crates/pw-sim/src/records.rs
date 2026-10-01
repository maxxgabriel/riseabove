//! Feeding the record book (see `pw_world::records`) from what happens at
//! every level, and announcing records that fall with their context.

use pw_core::{ClubId, CompId, NationId, PersonId, PlayerId};
use pw_world::event::{EventKind, Visibility};
use pw_world::minor::{Entrant, InstKind, Level, MinorKind, MinorLine};
use pw_world::records::{Broken, Holder, Mark, Record, RecordKey, Scope, Stat};
use pw_world::{FanReason, TableRow, TeamKind, World};

/// Consider a mark for a record. Returns the index of the `Broken` entry if
/// it set a new record (the first mark ever only establishes one). `min` is
/// the least value worth a record at all; `announce` publishes it.
pub fn note(w: &mut World, key: RecordKey, mark: Mark, min: i64, announce: bool) -> Option<u32> {
    // An estimate is kept beside the facts but is never announced as if it were one (its provenance says what it is).
    let announce = announce && key.stat.provenance().official();
    let lower = key.stat.lower_is_better();
    if if lower { mark.value > min } else { mark.value < min } {
        return None;
    }
    let today = w.date;
    let Some(r) = w.records.records.get_mut(&key) else {
        w.records.records.insert(key, Record { key, current: mark, previous: Default::default(), broken: 0 });
        return None;
    };
    let better = if lower { mark.value < r.current.value } else { mark.value > r.current.value };
    if !better {
        return None;
    }
    let old = r.current;
    let own = old.holder == mark.holder;
    if !own {
        r.previous.push(old);
        if r.previous.len() > 3 {
            r.previous.remove(0);
        }
        r.broken = r.broken.saturating_add(1);
    }
    r.current = mark;
    if own {
        // Extending your own record is not news every time.
        return None;
    }
    let idx = w.records.broken.len() as u32;
    w.records.broken.push(Broken { key, new: mark, old: Some(old), stood_days: old.date.days_until(today), own, announced: announce });
    if announce {
        let (person, club) = match mark.holder {
            Holder::Person(p) => (p, w.club_of_person(p)),
            Holder::Club(c) => (PersonId::NONE, c),
            _ => (PersonId::NONE, ClubId::NONE),
        };
        w.events.push(today, Visibility::Public, EventKind::Record { broken: idx, person, club });
    }
    Some(idx)
}

/// Like `note`, but the record simply changes hands: no `Broken` entry, no announcement.
/// For the many small-scope marks the almanac offers, most of which are not news.
pub fn note_quiet(w: &mut World, key: RecordKey, mark: Mark, min: i64) {
    let lower = key.stat.lower_is_better();
    if if lower { mark.value > min } else { mark.value < min } {
        return;
    }
    let Some(r) = w.records.records.get_mut(&key) else {
        w.records.records.insert(key, Record { key, current: mark, previous: Default::default(), broken: 0 });
        return;
    };
    if if lower { mark.value < r.current.value } else { mark.value > r.current.value } {
        if r.current.holder != mark.holder {
            let old = r.current;
            r.previous.push(old);
            if r.previous.len() > 3 {
                r.previous.remove(0);
            }
            r.broken = r.broken.saturating_add(1);
        }
        r.current = mark;
    }
}

/// The most recent fall of a record (for text about a `RecordBroken`
/// event from `honours`, which feeds the book silently).
pub fn last_broken(w: &World, key: RecordKey) -> Option<&Broken> {
    w.records.broken.iter().rev().take(500).find(|b| b.key == key)
}

fn person(w: &World, p: PlayerId) -> PersonId {
    w.players.cold[p].person
}

fn age_days(w: &World, p: PlayerId) -> i64 {
    i64::from(w.people[person(w, p)].dob.days_until(w.date))
}

// ---------------------------------------------------------------------------
// The professional game
// ---------------------------------------------------------------------------

/// After a senior appearance (after `honours::on_appearance`).
pub fn appearance(w: &mut World, p: PlayerId, club: ClubId, comp: CompId, goals: u8) {
    let today = w.date;
    let who = person(w, p);
    let age = age_days(w, p);
    let pro = Level::Professional;
    let mark = |v: i64| Mark { holder: Holder::Person(who), value: v, date: today, against: None };
    let tally = w.honours.tally(club, p);
    // Mirrors of the club tallies (announced by honours).
    note(w, RecordKey { scope: Scope::Club(club), stat: Stat::Goals, level: pro }, mark(i64::from(tally.goals)), 20, false);
    note(w, RecordKey { scope: Scope::Club(club), stat: Stat::Apps, level: pro }, mark(i64::from(tally.apps)), 100, false);
    if tally.apps == 1 {
        let b = note(w, RecordKey { scope: Scope::Club(club), stat: Stat::YoungestDebut, level: pro }, mark(age), 18 * 365, true);
        if b.is_some() {
            w.media.move_fans(club, who, 40, FanReason::Performances, today);
        }
    }
    if goals > 0 {
        let b = note(w, RecordKey { scope: Scope::Comp(comp), stat: Stat::YoungestScorer, level: pro }, mark(age), 18 * 365 + 180, true);
        if b.is_some() {
            w.media.move_fans(club, who, 60, FanReason::Performances, today);
        }
        note(w, RecordKey { scope: Scope::Comp(comp), stat: Stat::OldestScorer, level: pro }, mark(age), 37 * 365, true);
    }
}

/// After a senior result: winning and unbeaten runs.
pub fn result(w: &mut World, clubs: [ClubId; 2], goals: [u8; 2]) {
    let today = w.date;
    for (i, &c) in clubs.iter().enumerate() {
        if c.is_none() {
            continue;
        }
        let (f, a) = (goals[i], goals[1 - i]);
        let run = w.records.runs.entry(c).or_default();
        match f.cmp(&a) {
            std::cmp::Ordering::Greater => {
                run.0 += 1;
                run.1 += 1;
            }
            std::cmp::Ordering::Equal => {
                run.0 = 0;
                run.1 += 1;
            }
            std::cmp::Ordering::Less => *run = (0, 0),
        }
        let (wins, unbeaten) = *run;
        let mark = |v: u16| Mark { holder: Holder::Club(c), value: i64::from(v), date: today, against: None };
        let pro = Level::Professional;
        note(w, RecordKey { scope: Scope::Club(c), stat: Stat::WinsInRow, level: pro }, mark(wins), 6, true);
        note(w, RecordKey { scope: Scope::Club(c), stat: Stat::UnbeatenRun, level: pro }, mark(unbeaten), 15, true);
        let n = w.clubs[c].nation;
        note(w, RecordKey { scope: Scope::Nation(n), stat: Stat::WinsInRow, level: pro }, mark(wins), 10, true);
    }
}

/// A league season's end: points totals.
pub fn league_season(w: &mut World, comp: CompId, rows: &[TableRow]) {
    if w.comps[comp].team_kind != TeamKind::First {
        return;
    }
    let Some(top) = rows.first() else { return };
    let club = w.teams[top.team].club;
    let mark = Mark { holder: Holder::Club(club), value: i64::from(top.points), date: w.date, against: None };
    note(w, RecordKey { scope: Scope::Comp(comp), stat: Stat::PointsInSeason, level: Level::Professional }, mark, 60, true);
}

/// Mirror a professional record kept by `honours` (announced there).
pub fn mirror(w: &mut World, scope: Scope, stat: Stat, holder: Holder, value: i64, against: Option<Holder>) {
    let level = if matches!(stat, Stat::Caps | Stat::IntlGoals) { Level::International } else { Level::Professional };
    let mark = Mark { holder, value, date: w.date, against };
    note(w, RecordKey { scope, stat, level }, mark, 1, false);
}

// ---------------------------------------------------------------------------
// Below the professional game
// ---------------------------------------------------------------------------

fn entrant_scope(e: Entrant) -> Scope {
    match e {
        Entrant::Inst(i) => Scope::Institution(i),
        Entrant::Local(l) => Scope::Local(l),
    }
}

/// A finished minor season's lines: all-time tallies and season records at
/// the institution or club, and competition records.
pub fn minor_lines(w: &mut World, lines: &[(PlayerId, MinorLine)]) {
    let today = w.date;
    for &(p, l) in lines {
        let who = person(w, p);
        let level = l.kind.level();
        let t = w.records.minor_tallies.entry((who, l.entrant)).or_default();
        t.0 += l.apps;
        t.1 += l.goals;
        let (apps, goals) = *t;
        let mark = |v: u16| Mark { holder: Holder::Person(who), value: i64::from(v), date: today, against: None };
        let scope = entrant_scope(l.entrant);
        note(w, RecordKey { scope, stat: Stat::Goals, level }, mark(goals), 15, true);
        note(w, RecordKey { scope, stat: Stat::Apps, level }, mark(apps), 40, false);
        note(w, RecordKey { scope, stat: Stat::GoalsInSeason, level }, mark(l.goals), 8, false);
    }
}

/// A finished minor competition season (`World::minor.history` index).
pub fn minor_season(w: &mut World, hidx: u32) {
    let Some(s) = w.minor.history.get(hidx as usize).cloned() else { return };
    let today = w.date;
    let code = s.kind.code();
    let scope = Scope::Minor(s.nation, code);
    let level = s.kind.level();
    if s.top_scorer.is_some() {
        let who = person(w, s.top_scorer);
        let m = Mark { holder: Holder::Person(who), value: i64::from(s.top_goals), date: today, against: None };
        note(w, RecordKey { scope, stat: Stat::GoalsInSeason, level }, m, 10, true);
    }
    if let Some(b) = s.biggest {
        let margin = i64::from(b.score.0) - i64::from(b.score.1);
        let m = Mark { holder: Holder::Entrant(b.winner), value: margin, date: b.date, against: Some(Holder::Entrant(b.loser)) };
        note(w, RecordKey { scope, stat: Stat::BiggestWin, level }, m, 5, true);
    }
    let titles = w.records.minor_titles.entry((s.winner, code, s.nation)).or_default();
    *titles += 1;
    let t = *titles;
    let m = Mark { holder: Holder::Entrant(s.winner), value: i64::from(t), date: today, against: None };
    note(w, RecordKey { scope, stat: Stat::Titles, level }, m, 3, true);
}

/// Where a player's minor football happened, as a level for their biography.
pub fn minor_level(w: &World, e: Entrant, kind: MinorKind) -> Level {
    match e {
        Entrant::Inst(i) if w.minor.institutions.get(i as usize).is_some_and(|x| x.kind == InstKind::University) => Level::University,
        Entrant::Inst(_) => Level::School,
        Entrant::Local(_) => kind.level(),
    }
}

/// A nation's records for display.
pub fn of_nation(w: &World, n: NationId) -> Vec<&Record> {
    let mut v: Vec<&Record> = w
        .records
        .records
        .values()
        .filter(|r| match r.key.scope {
            Scope::Nation(x) | Scope::Minor(x, _) | Scope::Event(x, _) => x == n,
            Scope::Region(g) => w.ext.ecosystem.regions.get(g).is_some_and(|r| r.nation == n),
            Scope::Club(c) => w.clubs[c].nation == n,
            Scope::Comp(c) => w.comps[c].nation == n,
            Scope::Institution(i) => w.minor.institutions.get(i as usize).is_some_and(|x| x.nation == n),
            Scope::Local(l) => w.youth.local.get(l).is_some_and(|x| x.nation == n),
            Scope::World => false,
        })
        .collect();
    v.sort_by_key(|r| r.key);
    v
}

/// What evidence made this mark: its kind's provenance, unless it was set before the simulation began, in which case it is history
/// (imported if the world came from a database, else generated).
pub fn mark_provenance(w: &World, stat: pw_world::records::Stat, mark: &Mark) -> pw_world::records::Provenance {
    use pw_world::records::Provenance;
    let start = w.date.add_days(-(w.days_simulated.min(1_000_000) as i32));
    if mark.date < start {
        return if w.origins.sources.is_empty() { Provenance::SimulatedHistorical } else { Provenance::ImportedHistorical };
    }
    stat.provenance()
}
