//! Generating the past before the start date. See `pw_world::backfill`.
//!
//! For each top flight, thirty seasons of champions and runners-up, drawn
//! from the clubs' present standing with a little momentum (dynasties
//! happen) and a lot of noise; each season's top scorer; two or three past
//! legends per leading club. Imported seasons are kept and never
//! overwritten. The past then shapes the present: droughts and
//! expectations in club culture, title-race rivalries, and the records a
//! new generation has to beat.

use pw_core::rng::stream;
use pw_core::{ClubId, CompId, Date, NationId, Rng};
use pw_world::backfill::{PastFigure, PastSeason, Provenance};
use pw_world::culture::{RivalryKind, Side};
use pw_world::minor::Level;
use pw_world::records::{Holder, Mark, RecordKey, Scope, Stat};
use pw_world::{CompKind, FxHashMap, TeamKind, World};

const YEARS: i32 = 30;

fn figure(w: &mut World, nation: NationId, club: ClubId, born: i32, apps: u16, goals: u16, rng: &mut Rng) -> u32 {
    let (first, last) = crate::people::random_name(w, nation, rng);
    let name = match (w.names.get(first), w.names.get(last)) {
        ("", l) => l.to_string(),
        (f, "") => f.to_string(),
        (f, l) => format!("{f} {l}"),
    };
    let id = w.backfill.figures.len() as u32;
    w.backfill.figures.push(PastFigure { id, name, nation, club, born, apps, goals, provenance: Provenance::Generated });
    id
}

/// Once, when a world is prepared.
pub fn generate(w: &mut World) {
    if w.backfill.done {
        return;
    }
    let start = w.date.year();
    let from = start - YEARS;
    let leagues: Vec<CompId> = w.comps.iter_enumerated().filter(|(_, c)| c.kind == CompKind::League && c.tier == 1 && c.team_kind == TeamKind::First).map(|(id, _)| id).collect();
    for comp in leagues {
        let nation = w.comps[comp].nation;
        let clubs: Vec<ClubId> = {
            let mut v: Vec<ClubId> = w.comps[comp].state.table.iter().map(|r| w.teams[r.team].club).collect();
            v.sort();
            v
        };
        if clubs.len() < 4 {
            continue;
        }
        let mut last_champion = ClubId::NONE;
        for season in from..start {
            if w.backfill.seasons.iter().any(|s| s.comp == comp && s.season == season) {
                last_champion = w.backfill.seasons.iter().find(|s| s.comp == comp && s.season == season).map_or(ClubId::NONE, |s| s.champion);
                continue;
            }
            let mut rng = Rng::keyed(&[w.seed, stream::WORLDGEN, 0xb4c, u64::from(comp.0), season as u64]);
            let weights: Vec<f32> = clubs
                .iter()
                .map(|&c| {
                    let rep = f32::from(w.clubs[c].reputation);
                    (rep / 1500.0).exp() * if c == last_champion { 1.6 } else { 1.0 } * rng.range_f32(0.6, 1.4)
                })
                .collect();
            let i = rng.weighted(&weights);
            let champion = clubs[i];
            let mut rest = weights.clone();
            rest[i] = 0.0;
            let runner_up = clubs[rng.weighted(&rest)];
            let goals = rng.normal_ms(22.0, 5.0).clamp(12.0, 45.0) as u16;
            let scorer_nation = if rng.chance(0.75) { nation } else { w.nations.ids().nth(rng.index(w.nations.len())).unwrap_or(nation) };
            let scorer_club = if rng.chance(0.4) { champion } else { clubs[rng.index(clubs.len())] };
            let born = season - rng.range_i32(22, 31);
            let fig = figure(w, scorer_nation, scorer_club, born, 0, goals, &mut rng);
            w.backfill.seasons.push(PastSeason { comp, season, champion, runner_up, top_scorer: fig, top_goals: goals, provenance: Provenance::Generated });
            last_champion = champion;
        }
        // Past legends at the leading clubs.
        let mut leading = clubs.clone();
        leading.sort_by_key(|&c| std::cmp::Reverse(w.clubs[c].reputation));
        for &c in leading.iter().take(6) {
            let mut rng = Rng::keyed(&[w.seed, stream::WORLDGEN, 0x1e6, u64::from(c.0)]);
            for _ in 0..rng.range_i32(1, 3) {
                let apps = rng.range_i32(280, 620) as u16;
                let scorer = rng.chance(0.4);
                let goals = if scorer { rng.range_i32(120, 260) } else { rng.range_i32(5, 60) } as u16;
                let born = from + rng.range_i32(-25, 5);
                figure(w, nation, c, born, apps, goals, &mut rng);
            }
        }
    }
    w.backfill.from = from;
    w.backfill.to = start - 1;
    w.backfill.done = true;
    consequences(w);
}

/// What the past means for the present: records to beat, droughts and
/// expectations, title-race rivalries.
fn consequences(w: &mut World) {
    let start = w.date.year();
    let broken_before = w.records.broken.len();
    // Club records: all-time goals and appearances by past legends.
    let figures = w.backfill.figures.clone();
    for f in &figures {
        if f.club.is_none() || f.apps == 0 {
            continue;
        }
        let date = Date::from_ymd(f.born + 32, 6, 1);
        for (stat, value) in [(Stat::Goals, f.goals), (Stat::Apps, f.apps)] {
            let key = RecordKey { scope: Scope::Club(f.club), stat, level: Level::Professional };
            let mark = Mark { holder: Holder::Past(f.id), value: i64::from(value), date, against: None };
            crate::records::note(w, key, mark, 1, false);
        }
    }
    // League goals-in-a-season records.
    let seasons = w.backfill.seasons.clone();
    for s in &seasons {
        if s.top_scorer == u32::MAX {
            continue;
        }
        let key = RecordKey { scope: Scope::Comp(s.comp), stat: Stat::GoalsInSeason, level: Level::Professional };
        let mark = Mark { holder: Holder::Past(s.top_scorer), value: i64::from(s.top_goals), date: Date::from_ymd(s.season + 1, 5, 31), against: None };
        crate::records::note(w, key, mark, 1, false);
        let rec = w.honours.comps.entry(s.comp).or_default();
        if i64::from(s.top_goals) > rec.goals_in_season.value {
            rec.goals_in_season = pw_world::honours::Holder { player: pw_core::PlayerId::NONE, value: i64::from(s.top_goals), date: Date::from_ymd(s.season + 1, 5, 31) };
            rec.goals_in_season_year = s.season;
        }
    }
    // Setting the past's own records in order is not news.
    w.records.broken.truncate(broken_before);
    // Droughts and expectations.
    let clubs: Vec<ClubId> = w.culture.clubs.keys().copied().collect();
    for c in clubs {
        let titles = w.backfill.seasons.iter().filter(|s| s.champion == c).count();
        let last = w.backfill.last_title(c);
        let cc = w.culture.clubs.get_mut(&c).expect("club culture");
        if let Some(y) = last {
            cc.drought = (start - 1 - y).max(0) as u16;
        } else if titles == 0 {
            cc.drought = cc.drought.max(YEARS as u16);
        }
        cc.expectations = (i32::from(cc.expectations) + titles as i32 * 3).clamp(0, 100) as u8;
    }
    // Clubs that fought for titles again and again.
    let mut races: FxHashMap<(ClubId, ClubId), u16> = FxHashMap::default();
    for s in &seasons {
        let k = if s.champion < s.runner_up { (s.champion, s.runner_up) } else { (s.runner_up, s.champion) };
        *races.entry(k).or_default() += 1;
    }
    let today = w.date;
    let mut races: Vec<((ClubId, ClubId), u16)> = races.into_iter().filter(|x| x.1 >= 3).collect();
    races.sort();
    for ((a, b), n) in races {
        let r = w.culture.rivalries.ensure(Side::Club(a), Side::Club(b), RivalryKind::TitleRace, 20, today);
        r.intensity = r.intensity.max((20 + n * 5).min(80) as u8);
    }
}
