//! Insights: the short observations a good assistant coach would point out.
//!
//! Every note is computed on request from state the simulation already records
//! (appearances, season lines, medical cases, contracts, standings, results) and
//! carries the numbers it rests on. Nothing here is stored or invented. Each note
//! respects the viewer's knowledge: private state (body condition, contracts, the
//! medical room, hidden ability) is only used for the person themselves, their
//! club, or an observer, and results the viewer has chosen not to see yet are
//! taken out of every count rather than merely hidden from the text.

use pw_core::{CompId, Date, PersonId, PlayerId, PosGroup, TeamId};
use pw_data::BodyRegion;
use pw_sim::{consider, health};
use pw_world::medical::Chronic;
use pw_world::perf::{App, Label, Lens, SeasonLine};
use pw_world::{Fixture, PlayerStatus, Score, StaffRole, TeamKind};
use serde_json::Value;

use crate::ctx::Ctx;
use crate::contract::{InsightItem, InsightVisual, InsightsView};
use crate::model::{ApiError, ApiResult, Named, Ref, Tone};
use crate::pages::person::person_id;

// ---- the shape of a note -----------------------------------------------------------------

struct Item {
    kind: &'static str,
    tone: Tone,
    weight: u8,
    title: String,
    text: String,
    basis: String,
    link: Option<Named>,
    visual: Option<InsightVisual>,
}

#[derive(Default)]
struct Notes {
    items: Vec<Item>,
    /// Results the viewer has not revealed, which the counts leave out.
    held: usize,
}

impl Notes {
    fn add(&mut self, kind: &'static str, tone: Tone, weight: u8, title: impl Into<String>, text: impl Into<String>, basis: impl Into<String>) {
        self.items.push(Item { kind, tone, weight, title: cap(title.into()), text: cap(crate::fmt::singulars(text.into())), basis: crate::fmt::singulars(basis.into()), link: None, visual: None });
    }

    fn visual(&mut self, visual: InsightVisual) {
        if let Some(last) = self.items.last_mut() {
            last.visual = Some(visual);
        }
    }

    fn link(&mut self, r: Ref, name: impl Into<String>) {
        if let Some(last) = self.items.last_mut() {
            last.link = Some(Named::new(r, name));
        }
    }

    fn finish(mut self, args: &Value) -> Value {
        self.items.sort_by(|a, b| b.weight.cmp(&a.weight));
        let limit = args.get("limit").and_then(Value::as_u64).map_or(usize::MAX, |n| n as usize);
        let total = self.items.len();
        self.items.truncate(limit);
        crate::contract::wire(InsightsView {
            items: self
                .items
                .into_iter()
                .map(|i| InsightItem { kind: i.kind.into(), tone: i.tone, title: i.title, text: i.text, basis: i.basis, link: i.link, visual: i.visual })
                .collect(),
            total: total as u32,
            held: self.held as u32,
        })
    }
}


// ---- small helpers -----------------------------------------------------------------------

fn mean(v: &[f32]) -> f32 {
    if v.is_empty() { 0.0 } else { v.iter().sum::<f32>() / v.len() as f32 }
}

fn ordinal(n: usize) -> String {
    let s = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

fn count_word(n: usize) -> String {
    match n {
        1 => "one".into(),
        2 => "two".into(),
        3 => "three".into(),
        4 => "four".into(),
        5 => "five".into(),
        6 => "six".into(),
        7 => "seven".into(),
        8 => "eight".into(),
        9 => "nine".into(),
        10 => "ten".into(),
        n => n.to_string(),
    }
}

fn cap(s: String) -> String {
    let mut ch = s.chars();
    ch.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + ch.as_str())
}

/// "by three points", or "on goal difference" when the margin is nothing (two sides level on points are not separated by 0 points).
fn margin(gap: i32) -> String {
    if gap <= 0 { "on goal difference".to_string() } else { format!("by {}", plural(gap as usize, "point", "points")) }
}

/// "three points ahead of", or "level on points with" when there is no gap.
fn ahead_of(gap: i32) -> String {
    if gap <= 0 { "level on points with".to_string() } else { format!("{} ahead of", plural(gap as usize, "point", "points")) }
}

/// "one point", "three points", "12 points".
fn pts_word(n: f32) -> String {
    plural(n.round().max(0.0) as usize, "point", "points")
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{} {}", count_word(n), if n == 1 { one } else { many })
}

fn short_num(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", f64::from(n) / 1e6)
    } else if n >= 10_000 {
        format!("{}k", n / 1000)
    } else if n >= 1_000 {
        format!("{:.1}k", f64::from(n) / 1e3)
    } else {
        n.to_string()
    }
}

fn months(days: i32) -> String {
    match days {
        d if d < 45 => plural((d.max(1) as usize).div_ceil(7), "week", "weeks"),
        d => plural((d as usize + 15) / 30, "month", "months"),
    }
}

fn region_name(slot: usize) -> &'static str {
    use BodyRegion::*;
    [(Head, "head"), (Shoulder, "shoulder"), (Back, "back"), (Groin, "groin"), (Hamstring, "hamstring"), (Quadriceps, "thigh"), (Knee, "knee"), (Calf, "calf"), (Ankle, "ankle"), (Foot, "foot")]
        .iter()
        .find(|(r, _)| r.wear_slot() == Some(slot))
        .map_or("body", |(_, n)| n)
}

fn group_noun(g: PosGroup) -> &'static str {
    match g {
        PosGroup::Gk => "goalkeeper",
        PosGroup::Def => "defender",
        PosGroup::Mid => "midfielder",
        PosGroup::Att => "forward",
    }
}

/// W, D or L for a team in a played fixture (a shoot-out decides, as in the tables).
fn outcome(f: &Fixture, s: Score, team: TeamId) -> char {
    let (gf, ga) = if f.home == team { (s.home, s.away) } else { (s.away, s.home) };
    match gf.cmp(&ga) {
        std::cmp::Ordering::Greater => 'W',
        std::cmp::Ordering::Less => 'L',
        std::cmp::Ordering::Equal => match s.pens {
            Some((ph, pa)) => {
                let (pf, pg) = if f.home == team { (ph, pa) } else { (pa, ph) };
                match pf.cmp(&pg) {
                    std::cmp::Ordering::Greater => 'W',
                    std::cmp::Ordering::Less => 'L',
                    std::cmp::Ordering::Equal => 'D',
                }
            }
            None => 'D',
        },
    }
}

/// A team's played, revealed results since `from`, oldest first.
fn results<'a>(c: &Ctx<'a>, team: TeamId, from: Date) -> Vec<(&'a Fixture, Score)> {
    let mut v: Vec<_> = c.w.fixtures.of_team_between(team, from, c.w.date).map(|id| c.w.fixtures.get(id)).filter(|f| !c.is_concealed(f.uid)).filter_map(|f| f.score.map(|s| (f, s))).collect();
    v.sort_by_key(|(f, _)| (f.date, f.uid));
    v
}

/// Length of the run of `pred` results at the end of the form string.
fn run_of(form: &[char], pred: impl Fn(char) -> bool) -> usize {
    form.iter().rev().take_while(|&&c| pred(c)).count()
}

/// The calendar-year season line of a player with any unrevealed appearances taken back out.
fn year_line(c: &Ctx, p: PlayerId, hidden: &[Date]) -> Option<SeasonLine> {
    let year = c.w.date.year();
    let mut l = *c.w.perf.season(p, year)?;
    if !hidden.is_empty() {
        for a in c.w.perf.recent.get(&p).into_iter().flatten().filter(|a| hidden.contains(&a.date) && a.date.year() == year) {
            l.apps = l.apps.saturating_sub(1);
            l.starts = l.starts.saturating_sub(u16::from(a.started));
            l.minutes = l.minutes.saturating_sub(u32::from(a.minutes));
            l.goals = l.goals.saturating_sub(u16::from(a.goals));
            l.assists = l.assists.saturating_sub(u16::from(a.assists));
            l.rating_sum = l.rating_sum.saturating_sub(u32::from(a.rating));
            l.xgi = l.xgi.saturating_sub(u32::from(a.xgi));
        }
        // Whether a match counted as big or small is not recorded per match.
        l.big_apps = 0;
        l.small_apps = 0;
    }
    Some(l)
}

// ---- players -------------------------------------------------------------------------------

/// Notes on a person: a player's form and body, a manager's record.
pub fn person(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = person_id(args)?;
    let person = c.w.people.get(id).ok_or_else(|| ApiError::NotFound(format!("person {}", id.0)))?;
    let mut n = Notes::default();
    if let Some(p) = person.player.get() {
        player_notes(c, &mut n, id, p);
    }
    if let Some(s) = person.staff.get() {
        staff_notes(c, &mut n, id, s);
    }
    Ok(n.finish(args))
}


/// Why the latest step happened and, for the omniscient view, who is backing him: read from the pathway records, never invented.
/// A step that happened before reasons were kept has no note, rather than a guessed one.
fn pathway_notes(c: &Ctx, n: &mut Notes, p: PlayerId, me: bool, who: &str, name: &str) {
    let w = c.w;
    if !w.ext.ecosystem.is_configured() || !(c.observer() || me) || !w.ext.ecosystem.story.contains_key(&p) {
        return;
    }
    if let Some(r) = w.ext.pathway.of(p).last()
        && r.date.days_until(w.date) <= 180
        && !matches!(r.why, pw_world::pathway::Why::Emerged)
    {
        let why = r.why.text();
        let text = if me { format!("{who} {}.", why.replacen("was ", "were ", 1)) } else { format!("{name} {why}.") };
        n.add("pathway", Tone::Info, 6, "How the latest step came about", text, "the pathway record");
    }
    if c.observer()
        && let Some(v) = w.ext.recog.vouch.get(&p)
    {
        let by = match v.from {
            pw_world::recog::Source::Club(l) => format!("the coaches of {}", w.youth.local[l].name),
            pw_world::recog::Source::Institution(i) => format!("the coaches of {}", w.minor.institutions[i as usize].name),
            pw_world::recog::Source::District(r) => format!("the selectors of {}", w.ext.ecosystem.regions[r].name),
            pw_world::recog::Source::Person(x) => c.person_name(x),
        };
        n.add("pathway", Tone::Info, 4, "A coach is backing him", format!("{by} rate {name} among the best they know. How far that counts is each scout's call, and the recommendation lapses if he stops earning it."), "the recommendation on record");
    }
}

fn player_notes(c: &Ctx, n: &mut Notes, person: PersonId, p: PlayerId) {
    let w = c.w;
    let h = &w.players.hot[p];
    let cold = &w.players.cold[p];
    if h.status == PlayerStatus::Retired {
        return;
    }
    let me = c.is_me(p);
    let name = c.player_short(p);
    let (who, has) = if me { ("You".to_string(), "have") } else { (name.clone(), "has") };
    let inside = c.observer() || me || (c.my_club().is_some() && c.my_club() == h.club);
    let group = cold.best_pos.group();
    let today = w.date;
    let year = today.year();
    let age = w.age(p);
    let first_team = h.team.is_some() && w.teams[h.team].kind == TeamKind::First;

    pathway_notes(c, n, p, me, &who, &name);

    // Results the viewer has not revealed are left out of everything below.
    let hidden: Vec<Date> = c.concealed_fixtures().into_iter().filter(|f| f.involves(h.team)).map(|f| f.date).collect();
    n.held = hidden.len();
    let all: &[App] = w.perf.recent.get(&p).map_or(&[], |v| v.as_slice());
    let shown: Vec<&App> = all.iter().filter(|a| !hidden.contains(&a.date)).collect();
    let line = year_line(c, p, &hidden);

    // Form: the last five ratings against the year's average.
    let rated: Vec<f32> = shown.iter().filter(|a| a.rating > 0).map(|a| f32::from(a.rating) / 10.0).collect();
    if rated.len() >= 4 {
        let last = &rated[rated.len().saturating_sub(5)..];
        let recent = mean(last);
        let base = line.filter(|l| l.apps >= 8).map(|l| l.avg());
        if let Some(base) = base {
            let d = recent - base;
            if d >= 0.5 {
                n.add(
                    "form",
                    Tone::Pos,
                    80,
                    "In form",
                    format!("{who} {has} averaged {recent:.1} over the last {} appearances, {d:.1} above the {base:.1} for {year}.", last.len()),
                    format!("Last {} match ratings against {} appearances in {year}", last.len(), line.map_or(0, |l| l.apps)),
                );
                n.visual(InsightVisual::Sparkline { label: "Last five match ratings".into(), unit: "rating".into(), values: last.to_vec() });
            } else if d <= -0.5 {
                n.add(
                    "form",
                    if d <= -0.9 { Tone::Neg } else { Tone::Warn },
                    80,
                    "Out of form",
                    format!("{who} {has} averaged {recent:.1} over the last {} appearances, {:.1} below the {base:.1} for {year}.", last.len(), -d),
                    format!("Last {} match ratings against {} appearances in {year}", last.len(), line.map_or(0, |l| l.apps)),
                );
                n.visual(InsightVisual::Sparkline { label: "Last five match ratings".into(), unit: "rating".into(), values: last.to_vec() });
            }
        }
        let best = shown.iter().filter(|a| a.rating > 0).max_by_key(|a| a.rating);
        if let Some(b) = best.filter(|b| b.rating >= 90) {
            n.add(
                "form",
                Tone::Pos,
                40,
                "A standout display",
                format!("A {:.1} rating on {}, the best of the last {} rated appearances.", f32::from(b.rating) / 10.0, crate::fmt::date(b.date), rated.len()),
                "Match ratings",
            );
        }
    }

    // Goals and assists against the chances that were created for them.
    if let Some(l) = line.filter(|l| l.apps >= 8 && group != PosGroup::Gk) {
        let expected = l.xgi as f32 / 100.0;
        let got = f32::from(l.goals + l.assists);
        if expected >= 3.0 {
            if got >= expected + 2.5 && got >= expected * 1.3 {
                n.add(
                    "output",
                    Tone::Pos,
                    60,
                    "Finishing above the chances",
                    format!("{} goal contributions in {year} from chances worth {expected:.1} on the expected measure. Output at that rate is hard to keep up.", got as u32),
                    format!("{} goals and {} assists against {expected:.1} expected, in {} appearances", l.goals, l.assists, l.apps),
                );
            } else if got + 2.5 <= expected && got <= expected * 0.7 {
                let output = if got < 1.0 { "No goals or assists".to_string() } else { format!("Only {} goal {}", got as u32, if got < 2.0 { "contribution" } else { "contributions" }) };
                n.add(
                    "output",
                    Tone::Warn,
                    60,
                    "Not converting",
                    format!("{output} in {year} from chances worth {expected:.1}. The chances are there, the finishing has not followed."),
                    format!("{} goals and {} assists against {expected:.1} expected, in {} appearances", l.goals, l.assists, l.apps),
                );
            }
        }
    }

    // Big matches against weak opposition.
    if let Some(l) = line.filter(|l| l.big_apps >= 3 && l.small_apps >= 3) {
        let big = l.big_sum as f32 / f32::from(l.big_apps) / 10.0;
        let small = l.small_sum as f32 / f32::from(l.small_apps) / 10.0;
        if big >= small + 0.5 {
            n.add(
                "record",
                Tone::Pos,
                55,
                "Rises to the big occasion",
                format!("Rated {big:.1} in {} big matches, against {small:.1} against weaker sides.", l.big_apps),
                format!("{year} ratings: {} big matches, {} against clearly weaker opponents", l.big_apps, l.small_apps),
            );
        } else if small >= big + 0.5 {
            n.add(
                "record",
                Tone::Warn,
                55,
                "Quieter in the big matches",
                format!("Rated {big:.1} in {} big matches, against {small:.1} against weaker sides.", l.big_apps),
                format!("{year} ratings: {} big matches, {} against clearly weaker opponents", l.big_apps, l.small_apps),
            );
        }
    }

    // Standing in the squad: rating and output against team-mates in the same line.
    if h.team.is_some() && h.status == PlayerStatus::Active {
        let mates: Vec<(PlayerId, SeasonLine)> = w.teams[h.team].squad.iter().filter_map(|&q| year_line(c, q, &hidden).map(|l| (q, l))).collect();
        if let Some(l) = line.filter(|l| l.apps >= 8) {
            let mut peers: Vec<(PlayerId, f32)> = mates.iter().filter(|(q, l)| *q == p || (w.players.cold[*q].best_pos.group() == group && l.apps >= 8)).map(|(q, l)| (*q, l.avg())).collect();
            peers.sort_by(|a, b| b.1.total_cmp(&a.1));
            if peers.len() >= 3 {
                let rank = peers.iter().position(|(q, _)| *q == p).unwrap_or(0);
                if rank == 0 {
                    n.add(
                        "standing",
                        Tone::Pos,
                        50,
                        format!("Best-rated {} at the club", group_noun(group)),
                        format!("{:.2} on average in {year}, ahead of {}.", l.avg(), c.player_short(peers[1].0)),
                        format!("Ratings of {} squad {}s with at least eight appearances", peers.len(), group_noun(group)),
                    );
                    n.link(c.player_ref(peers[1].0), c.player_short(peers[1].0));
                } else if rank == peers.len() - 1 && peers.len() >= 4 {
                    n.add(
                        "standing",
                        Tone::Warn,
                        50,
                        format!("Lowest-rated {} at the club", group_noun(group)),
                        format!("{:.2} on average in {year}; {} leads the group on {:.2}.", l.avg(), c.player_short(peers[0].0), peers[0].1),
                        format!("Ratings of {} squad {}s with at least eight appearances", peers.len(), group_noun(group)),
                    );
                    n.link(c.player_ref(peers[0].0), c.player_short(peers[0].0));
                }
            }
        }
        let mut scorers: Vec<(PlayerId, u16)> = mates.iter().map(|(q, l)| (*q, l.goals)).filter(|(_, g)| *g > 0).collect();
        scorers.sort_by_key(|(_, g)| std::cmp::Reverse(*g));
        if let (Some(&(top, goals)), Some(l)) = (scorers.first(), line)
            && top == p
            && goals >= 5
        {
            let next = scorers.get(1).map_or(0, |s| s.1);
            n.add(
                "standing",
                Tone::Pos,
                55,
                "Leading scorer",
                format!("{} goals in {year}{}.", goals, if goals > next { format!(", {} more than the next best in the squad", goals - next) } else { ", level with the next best".to_string() }),
                format!("Goals in {year} across the squad, all competitions ({} appearances)", l.apps),
            );
        }
        // Youngest and oldest of the regulars.
        let regulars: Vec<(PlayerId, u32)> = mates.iter().filter(|(_, l)| l.starts >= 8).map(|(q, _)| (*q, w.age(*q))).collect();
        if regulars.len() >= 5 && line.is_some_and(|l| l.starts >= 8) {
            let (youngest, oldest) = (regulars.iter().min_by_key(|r| r.1), regulars.iter().max_by_key(|r| r.1));
            if youngest.is_some_and(|y| y.0 == p) && age <= 21 {
                n.add(
                    "standing",
                    Tone::Pos,
                    45,
                    "Youngest regular in the side",
                    format!("Starting regularly at {age}, when no other regular starter is younger than {}.", regulars.iter().filter(|r| r.0 != p).map(|r| r.1).min().unwrap_or(age)),
                    format!("{} squad members with at least eight starts in {year}", regulars.len()),
                );
            } else if oldest.is_some_and(|o| o.0 == p) && age >= 33 {
                n.add(
                    "standing",
                    Tone::Info,
                    35,
                    "Oldest regular in the side",
                    format!("Still starting regularly at {age}, older than every other regular starter."),
                    format!("{} squad members with at least eight starts in {year}", regulars.len()),
                );
            }
        }
    }

    // Selection: how often they start.
    let last8: Vec<&&App> = shown.iter().rev().take(8).collect();
    if last8.len() >= 6 {
        let starts = last8.iter().filter(|a| a.started).count();
        if starts * 5 >= last8.len() * 4 {
            n.add("role", Tone::Muted, 30, "A regular starter", format!("Started {starts} of the last {} appearances.", last8.len()), "Appearance record");
        } else if starts * 4 <= last8.len() {
            n.add("role", Tone::Warn, 45, "Used mostly from the bench", format!("Started {starts} of the last {} appearances.", last8.len()), "Appearance record");
        }
    }
    if let Some(l) = line.filter(|l| l.omitted >= 4 && u32::from(l.omitted) * 2 > u32::from(l.apps)) {
        n.add("role", Tone::Warn, 50, "Often left out of the squad", format!("Not in the matchday squad {} times in {year}, against {} appearances.", l.omitted, l.apps), "Matchday squad records");
    }

    // Goals: a scoring run, or a drought for someone who usually scores.
    if group != PosGroup::Gk {
        let run = shown.iter().rev().take_while(|a| a.goals > 0).count();
        if run >= 3 {
            n.add("form", Tone::Pos, 65, "On a scoring run", format!("{who} {has} scored in each of the last {} appearances.", run), "Goals in the last ten appearances");
        } else if group == PosGroup::Att && shown.len() >= 8 {
            let drought = shown.iter().rev().take_while(|a| a.goals == 0).count();
            let season_rate = line.filter(|l| l.apps >= 10).map(|l| f32::from(l.goals) / f32::from(l.apps));
            if drought >= 8 && season_rate.is_some_and(|r| r >= 0.25) {
                n.add(
                    "form",
                    Tone::Warn,
                    55,
                    "A goal drought",
                    format!("No goal in the last {drought} appearances, for a forward who scored {} in {} earlier in {year}.", line.map_or(0, |l| l.goals), line.map_or(0, |l| l.apps)),
                    "Goals in the last ten appearances",
                );
            }
        }
    }

    // Cards.
    let (mut yellows, mut reds) = (0u32, 0u32);
    if hidden.is_empty() {
        let season = w.comps.iter_enumerated().find(|(_, comp)| comp.is_league() && comp.state.entrants.contains(&h.team)).map(|(id, _)| id);
        if let Some(season) = season {
            let s = w.comps[season].state.season;
            for st in w.stats.for_player(p).filter(|s2| s2.season == s) {
                yellows += u32::from(st.yellows);
                reds += u32::from(st.reds);
            }
        }
    }
    if reds > 0 || yellows >= 5 {
        n.add(
            "discipline",
            Tone::Warn,
            45,
            "Discipline",
            format!("{} yellow {} and {} red {} this season.", yellows, if yellows == 1 { "card" } else { "cards" }, reds, if reds == 1 { "card" } else { "cards" }),
            "Season statistics across competitions",
        );
    }

    // The body: a workload spike, tiredness and the hazards that follow from them.
    if c.sees_condition(p) && h.status == PlayerStatus::Active {
        let tune = &w.data.tuning.health;
        let acwr = h.acwr();
        let ahead = consider::fixtures_ahead(w, h.team, 8);
        if h.injury == 0 && acwr > tune.acwr_safe_high + 0.15 {
            n.add(
                "body",
                Tone::Warn,
                70,
                "Workload is spiking",
                format!("Recent load is {acwr:.1} times the longer-term level, above the {:.1} the body copes with easily. Injury risk is up until it settles.", tune.acwr_safe_high),
                "Acute against chronic training load",
            );
        } else if h.injury == 0 && acwr < tune.acwr_safe_low - 0.1 && h.status == PlayerStatus::Active {
            n.add(
                "body",
                Tone::Info,
                35,
                "Load has dropped",
                format!("Recent load is {acwr:.1} times the longer-term level, below the {:.1} that keeps the body ready. A sudden return to full load is when injuries happen.", tune.acwr_safe_low),
                "Acute against chronic training load",
            );
        }
        if h.injury == 0 && h.fatigue >= 70 {
            n.add(
                "body",
                Tone::Warn,
                65,
                "Tired legs",
                format!("Fatigue is {}/100{}.", h.fatigue, if ahead >= 3 { format!(", with {} matches in the next eight days", ahead) } else { String::new() }),
                "Body condition",
            );
        }
        let since = today.days_until(h.last_match).abs();
        if h.injury == 0 && h.sharpness < 45 && h.last_match.0 > 0 && (21..=400).contains(&since) && first_team {
            n.add("body", Tone::Warn, 50, "Short of match sharpness", format!("Match sharpness is {}/100, {} since the last game.", h.sharpness, months(since)), "Body condition and last appearance");
        }
        let (wear_slot, wear) = cold.wear.iter().copied().enumerate().max_by_key(|(_, v)| *v).unwrap_or((0, 0));
        if wear >= 60 {
            n.add(
                "body",
                Tone::Warn,
                40,
                format!("Wear on the {}", region_name(wear_slot)),
                format!("The {} has taken a lot of punishment (wear {wear}/100), which raises the chance of injury there.", region_name(wear_slot)),
                "Accumulated body wear",
            );
        }
    }

    // The medical room.
    if inside {
        if let Some(case) = w.medical.open.get(&p) {
            let mut t = format!("The medical team expect about {} out, and are {} of that.", months(i32::from(case.estimate)), crate::model::sureness(case.certainty));
            if case.rushed {
                t.push_str(" A return before the body was ready has already been tried.");
            }
            if case.setbacks > 0 {
                t.push_str(&format!(" {} so far.", plural(usize::from(case.setbacks), "setback", "setbacks")));
            }
            if case.treatment != pw_world::medical::Treatment::Conservative {
                t.push_str(&format!(" Treatment: {}.", case.treatment.label()));
            }
            n.add("injury", Tone::Neg, 90, format!("Out: {}", health::injury_name(w, case.injury).to_lowercase()), t, "Medical case, estimate as the club sees it");
        }
        let hist: Vec<_> = w.medical.history_of(p).iter().filter(|k| k.date.days_until(today) <= 730).collect();
        if hist.len() >= 3 {
            let mut by_region: Vec<(u8, usize)> = Vec::new();
            for k in hist.iter().filter(|k| k.region != u8::MAX) {
                match by_region.iter_mut().find(|(r, _)| *r == k.region) {
                    Some(e) => e.1 += 1,
                    None => by_region.push((k.region, 1)),
                }
            }
            by_region.sort_by_key(|e| std::cmp::Reverse(e.1));
            let missed: u32 = hist.iter().map(|k| u32::from(k.actual)).sum();
            let mut t = format!("{} in two years, {} days missed in all.", plural(hist.len(), "injury", "injuries"), missed);
            if let Some(&(r, k)) = by_region.first().filter(|e| e.1 >= 2) {
                t.push_str(&format!(" {} of them to the {}.", count_word(k), region_name(usize::from(r))));
            }
            n.add("injury", Tone::Warn, 60, "A troubled body", t, "Closed medical cases in the last two years");
        } else if let Some(k) = hist.iter().find(|k| k.actual >= 60) {
            n.add(
                "injury",
                Tone::Info,
                30,
                "A long absence not long ago",
                format!("{} days out with a {} on {}.", k.actual, health::injury_name(w, k.injury).to_lowercase(), crate::fmt::date(k.date)),
                "Closed medical cases in the last two years",
            );
        }
        let mut fragile: Vec<String> = w.medical.fragile.get(&p).into_iter().flatten().filter(|f| f.level >= 0.2).map(|f| region_name(usize::from(f.region)).to_string()).collect();
        fragile.dedup();
        if !fragile.is_empty() {
            n.add(
                "injury",
                Tone::Warn,
                45,
                "A fragile spot",
                format!("The {} has not fully healed and is more likely to give way again.", fragile.join(" and the ")),
                "Lingering fragility from earlier injuries",
            );
        }
        for ch in w.medical.chronic.get(&p).into_iter().flatten() {
            let (title, text) = match ch {
                Chronic::Managed { region } => {
                    (format!("A chronic {} problem", region_name(usize::from(*region))), format!("Needs managing between matches, so back-to-back games carry a cost for {name}."))
                }
                Chronic::Systemic => ("A recurring illness".to_string(), "Flares up now and then and takes time out of the calendar.".to_string()),
            };
            n.add("injury", Tone::Warn, 45, title, text, "Chronic condition on the medical file");
        }
        if age >= 30 && cold.injuries_career >= 8 {
            n.add("injury", Tone::Info, 25, "A long injury history", format!("{} injuries over the career so far.", cold.injuries_career), "Career injury count");
        }
    }

    // Development.
    if let Some(rec) = w.growth.records.get(&p).filter(|_| inside) {
        let senior = first_team || age >= 21;
        if rec.idle_weeks >= 8 && senior && h.status == PlayerStatus::Active {
            let mut t = format!("About {} without meaningful senior football. Progress stalls and, over a long spell, potential is lost.", plural(usize::from(rec.idle_weeks), "week", "weeks"));
            if c.sees_internal_state() && rec.eroded > 0 {
                t.push_str(&format!(" {} points of potential are already gone.", rec.eroded));
            }
            n.add("growth", Tone::Warn, 60, "Not playing enough to develop", t, "Weeks without senior minutes");
        }
        if rec.mentor.is_some() {
            let m = rec.mentor;
            n.add("growth", Tone::Info, 35, "Has a mentor", format!("{} has taken {name} under their wing since {}.", c.person_short(m), crate::fmt::date(rec.mentor_since)), "Mentoring relationship");
            n.link(Ref::person(m), c.person_short(m));
        }
        if c.sees_internal_state() && rec.ca.len() >= 6 {
            // Against everyone of the same age, so that ordinary growth in the young is not called out.
            let trend = rec.trend();
            let peers: Vec<i16> = w.growth.records.iter().filter(|(q, r)| **q != p && r.ca.len() >= 6 && w.age(**q) == age).map(|(_, r)| r.trend()).collect();
            if peers.len() >= 30 {
                let below = peers.iter().filter(|&&t| t < trend).count() as f32 / peers.len() as f32;
                let above = peers.iter().filter(|&&t| t > trend).count() as f32 / peers.len() as f32;
                if below >= 0.95 && trend >= 3 {
                    n.add(
                        "growth",
                        Tone::Pos,
                        58,
                        "Developing faster than most",
                        format!("Up {trend} points of current ability in a year, more than {:.0}% of the {} players aged {age}.", below * 100.0, peers.len() + 1),
                        "Monthly ability records (internal), against players of the same age",
                    );
                } else if above >= 0.95 && trend <= 0 {
                    n.add(
                        "growth",
                        Tone::Warn,
                        58,
                        "Development has stalled",
                        format!("{} points of current ability over the year, behind {:.0}% of the {} players aged {age}.", trend, above * 100.0, peers.len() + 1),
                        "Monthly ability records (internal), against players of the same age",
                    );
                }
            }
        }
    }

    // Contract, and whether the football matches what was promised.
    if c.sees_contract(p) && h.club.is_some() && cold.contract.club.is_some() {
        let k = &cold.contract;
        let left = k.days_left(today);
        if (0..=180).contains(&left) {
            n.add("contract", Tone::Warn, 65, "Contract running down", format!("The deal ends in {}, on {}.", months(left), crate::fmt::date(k.end)), "Contract");
        } else if (181..=365).contains(&left) {
            n.add("contract", Tone::Info, 35, "Contract in its last year", format!("The deal ends in {}, on {}.", months(left), crate::fmt::date(k.end)), "Contract");
        }
        if consider::team_minutes_4w(w, h.team) >= 180 {
            let (share, expected) = consider::minutes_share(w, p);
            let status = cold.status.label();
            if expected >= 0.25 && share + 0.15 < expected {
                n.add(
                    "contract",
                    Tone::Warn,
                    70,
                    "Playing less than the role implies",
                    format!("{:.0}% of the available minutes over the last four weeks, where a {} is expected to play around {:.0}%.", share * 100.0, status.to_lowercase(), expected * 100.0),
                    "Minutes in the last four weeks against squad status",
                );
            } else if expected <= 0.4 && share >= expected + 0.4 {
                n.add(
                    "contract",
                    Tone::Pos,
                    40,
                    "Has outgrown the role",
                    format!("{:.0}% of the available minutes over the last four weeks, where a {} would expect around {:.0}%.", share * 100.0, status.to_lowercase(), expected * 100.0),
                    "Minutes in the last four weeks against squad status",
                );
            }
        }
        if c.observer() && h.team.is_some() {
            let mut wages: Vec<_> = w.teams[h.team].squad.iter().filter(|&&q| w.players.cold[q].contract.club.is_some()).map(|&q| (q, w.players.cold[q].contract.current_wage(today))).collect();
            wages.sort_by_key(|x| std::cmp::Reverse(x.1));
            if wages.len() >= 8 {
                let rank = wages.iter().position(|x| x.0 == p).unwrap_or(0) + 1;
                if rank <= 3 && line.is_some_and(|l| l.starts < 6 && l.apps + l.starts > 0) && group != PosGroup::Gk {
                    n.add(
                        "contract",
                        Tone::Warn,
                        45,
                        "Highly paid, little used",
                        format!("{} on the wage list of {}, with {} starts in {year}.", ordinal(rank), wages.len(), line.map_or(0, |l| l.starts)),
                        "Weekly wages and appearances",
                    );
                }
            }
        }
    }

    // How others see them.
    let readings = w.perf.labels(p);
    let mut seen: Vec<(Label, Vec<Lens>)> = Vec::new();
    for r in readings {
        let public = matches!(r.lens, Lens::Media | Lens::Fans);
        let club_view = matches!(r.lens, Lens::Manager) && inside;
        if !(public || club_view || c.observer()) {
            continue;
        }
        match seen.iter_mut().find(|(l, _)| *l == r.label) {
            Some(e) => e.1.push(r.lens),
            None => seen.push((r.label, vec![r.lens])),
        }
    }
    for (label, lenses) in seen {
        let names: Vec<&str> = lenses
            .iter()
            .map(|l| match l {
                Lens::Manager => "The manager",
                Lens::Scout => "Scouts",
                Lens::Media => "The press",
                Lens::Fans => "Supporters",
                Lens::Analyst => "Analysts",
            })
            .collect();
        let tone = match label {
            Label::BigGamePlayer | Label::InForm | Label::Underrated | Label::GoalThreat | Label::Workhorse | Label::Breakthrough | Label::Durable => Tone::Pos,
            Label::InSlump | Label::FrozenOut | Label::Unreliable | Label::InjuryProne | Label::FlatTrackBully | Label::Overrated => Tone::Warn,
        };
        let target = if me { "you".to_string() } else { name.clone() };
        let who_sees = if names.len() > 1 {
            format!("{} both see {target} as {}", names.join(" and "), label.text())
        } else {
            format!("{} see{} {target} as {}", names[0], if names[0] == "The press" || names[0] == "The manager" { "s" } else { "" }, label.text())
        };
        n.add(
            "opinion",
            tone,
            42,
            cap(label.text().trim_start_matches("a ").trim_start_matches("having ").to_string()),
            format!("{who_sees}."),
            "Readings formed from the same record, through each group's own lens",
        );
    }

    // Fame, in football and outside it.
    let ren = w.renown.of(person);
    if ren.followers >= 50_000 {
        let mut all: Vec<u32> = w.players.cold.iter().map(|cd| w.renown.of(cd.person).followers).collect();
        all.sort_unstable();
        let below = all.partition_point(|&f| f < ren.followers);
        let pct = 100.0 * below as f32 / all.len().max(1) as f32;
        if pct >= 95.0 {
            n.add(
                "profile",
                Tone::Info,
                38,
                "Followed by a great many",
                format!("{} followers, more than {:.0}% of all players.", short_num(ren.followers), pct),
                "Social following against every player in the world",
            );
        }
    }

    // Milestones close at hand.
    let near = |have: u16, step: u16, within: u16| -> Option<(u16, u16)> {
        let next = (have / step + 1) * step;
        (next - have <= within && have > 0).then_some((next, next - have))
    };
    if let Some((goal, away)) = near(cold.senior_goals, 50, 3).filter(|_| cold.senior_goals >= 45) {
        n.add("milestone", Tone::Info, 30, format!("Closing on {goal} senior goals"), format!("{} more to reach {goal}.", count_word(usize::from(away))), "Career record");
    }
    if let Some((apps, away)) = near(cold.senior_apps, 100, 5).filter(|_| cold.senior_apps >= 90) {
        n.add("milestone", Tone::Info, 28, format!("Closing on {apps} senior appearances"), format!("{} more to reach {apps}.", count_word(usize::from(away))), "Career record");
    }
    if let Some((caps, away)) = near(cold.caps, 25, 2).filter(|_| cold.caps >= 23) {
        n.add("milestone", Tone::Info, 26, format!("Closing on {caps} caps"), format!("{} more to reach {caps}.", count_word(usize::from(away))), "International record");
    }
}

fn staff_notes(c: &Ctx, n: &mut Notes, _person: PersonId, s: pw_core::StaffId) {
    let w = c.w;
    let st = &w.staff[s];
    if st.role != StaffRole::Manager {
        return;
    }
    let r = &st.record;
    if r.games >= 10 {
        let win = 100.0 * r.wins as f32 / r.games as f32;
        let ppg = (3.0 * r.wins as f32 + r.draws as f32) / r.games as f32;
        n.add(
            "record",
            if ppg >= 1.8 {
                Tone::Pos
            } else if ppg < 1.1 {
                Tone::Warn
            } else {
                Tone::Muted
            },
            60,
            "Record in the job",
            format!("{} wins from {} matches ({win:.0}%), {ppg:.2} points a game.", r.wins, r.games),
            "Managerial record",
        );
    }
    if r.trophies > 0 {
        n.add("record", Tone::Pos, 45, "Trophies", format!("{} won as a manager.", plural(usize::from(r.trophies), "trophy", "trophies")), "Managerial record");
    }
    if r.sackings > 0 {
        n.add("record", Tone::Warn, 40, "Has been sacked before", format!("{} so far.", plural(usize::from(r.sackings), "dismissal", "dismissals")), "Managerial record");
    }
    if st.employed() {
        let tenure = st.joined.days_until(w.date);
        if tenure >= 730 {
            n.add("record", Tone::Info, 30, "Long in the job", format!("In charge for {} years.", tenure / 365), "Appointment date");
        }
    }
}

// ---- clubs -------------------------------------------------------------------------------

/// A team's league table position, points and matches left, from the revealed results.
struct Standing {
    pos: usize,
    n: usize,
    row: pw_world::comp::TableRow,
    left: usize,
}

fn standing(c: &Ctx, comp: CompId, team: TeamId) -> Option<Standing> {
    let co = &c.w.comps[comp];
    let pw_world::Format::League { rounds } = co.format else { return None };
    let (rows, _) = crate::tables::visible_table(c, comp);
    let pos = rows.iter().position(|r| r.team == team)?;
    let n = rows.len();
    let total = usize::from(rounds) * (n.saturating_sub(1));
    let row = rows[pos];
    Some(Standing { pos: pos + 1, n, row, left: total.saturating_sub(usize::from(row.played)) })
}

fn ppg(form: &[char]) -> f32 {
    if form.is_empty() {
        0.0
    } else {
        form.iter()
            .map(|&r| {
                if r == 'W' {
                    3.0
                } else if r == 'D' {
                    1.0
                } else {
                    0.0
                }
            })
            .sum::<f32>()
            / form.len() as f32
    }
}

/// Runs and form for a team from its league results this season.
fn league_form(c: &Ctx, comp: CompId, team: TeamId) -> (Vec<char>, Vec<char>, Vec<char>) {
    let from = c.w.comps[comp].state.start.add_days(-1);
    let mut all = Vec::new();
    let (mut home, mut away) = (Vec::new(), Vec::new());
    for (f, s) in results(c, team, from).into_iter().filter(|(f, _)| f.comp == comp) {
        let r = outcome(f, s, team);
        all.push(r);
        if f.home == team { home.push(r) } else { away.push(r) }
    }
    (all, home, away)
}

fn streaks(n: &mut Notes, who: &str, form: &[char], scope: &str, weight: u8) {
    let count_before = n.items.len();
    let win = run_of(form, |r| r == 'W');
    let unbeaten = run_of(form, |r| r != 'L');
    let winless = run_of(form, |r| r != 'W');
    let losing = run_of(form, |r| r == 'L');
    if win >= 3 {
        n.add("form", Tone::Pos, weight + 5, format!("{} wins in a row", count_word(win)), format!("{who} have won each of the last {win} {scope} matches."), "Results this season");
    } else if unbeaten >= 5 {
        n.add("form", Tone::Pos, weight, format!("Unbeaten in {}", count_word(unbeaten)), format!("{who} have not lost in the last {unbeaten} {scope} matches."), "Results this season");
    } else if losing >= 3 {
        n.add("form", Tone::Neg, weight + 5, format!("{} defeats in a row", count_word(losing)), format!("{who} have lost each of the last {losing} {scope} matches."), "Results this season");
    } else if winless >= 5 {
        n.add("form", Tone::Warn, weight, format!("Winless in {}", count_word(winless)), format!("{who} have not won in the last {winless} {scope} matches."), "Results this season");
    }
    if n.items.len() > count_before {
        let sequence: String = form.iter().rev().take(6).rev().collect();
        n.visual(InsightVisual::Sequence { label: "Last league results".into(), unit: "result".into(), values: sequence.chars().map(|x| x.to_string()).collect() });
    }
}

pub fn club(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = pw_core::ClubId(args.get("id").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing club id".into()))? as u32);
    if id.0 as usize >= c.w.clubs.len() {
        return Err(ApiError::NotFound(format!("club {}", id.0)));
    }
    let w = c.w;
    let club = &w.clubs[id];
    let team = club.first_team();
    let name = c.club_short(id);
    let mut n = Notes { held: c.concealed_fixtures().into_iter().filter(|f| f.involves(team)).count(), ..Default::default() };
    let league = w.league_of(team);

    if let Some(comp) = league {
        let co = &w.comps[comp];
        if let Some(st) = standing(c, comp, team) {
            let (rows, _) = crate::tables::visible_table(c, comp);
            let pts = |i: usize| i32::from(rows[i].points);
            let mine = pts(st.pos - 1);
            // Where the season is pointing (nothing to say before anyone has played).
            if rows.iter().all(|r| r.played == 0) {
            } else if usize::from(co.relegate) > 0 && st.pos > st.n - usize::from(co.relegate) {
                let safe = pts(st.n - usize::from(co.relegate) - 1);
                n.add(
                    "table",
                    Tone::Neg,
                    90,
                    "In the relegation places",
                    format!("{} on {}, {} from safety with {} matches left.", ordinal(st.pos), pts_word(mine as f32), plural((safe - mine).max(0) as usize, "point", "points"), st.left),
                    format!("{} table, {} teams", co.short_name, st.n),
                );
            } else if usize::from(co.relegate) > 0 {
                let line = st.n - usize::from(co.relegate);
                let cushion = mine - pts(line);
                if st.pos + 3 > line && cushion <= 4 {
                    n.add(
                        "table",
                        Tone::Warn,
                        75,
                        "Close to the drop",
                        format!(
                            "{} on {}, only {} above the relegation places with {} matches left.",
                            ordinal(st.pos),
                            pts_word(mine as f32),
                            plural(cushion.max(0) as usize, "point", "points"),
                            st.left
                        ),
                        format!("{} table, {} teams", co.short_name, st.n),
                    );
                }
            }
            let started = rows.iter().any(|r| r.played > 0);
            if !started {
            } else if co.promote > 0 && st.pos <= usize::from(co.promote) {
                n.add(
                    "table",
                    Tone::Pos,
                    70,
                    "In the promotion places",
                    format!("{} on {} with {} matches left.", ordinal(st.pos), pts_word(mine as f32), st.left),
                    format!("{} table", co.short_name),
                );
            } else if co.promote > 0 && st.pos <= usize::from(co.promote) + 3 {
                let gap = pts(usize::from(co.promote) - 1) - mine;
                n.add(
                    "table",
                    Tone::Info,
                    60,
                    "Within reach of promotion",
                    format!("{} from the last promotion place with {} matches left.", plural(gap.max(0) as usize, "point", "points"), st.left),
                    format!("{} table", co.short_name),
                );
            }
            if let Some(&(cont, places)) = co.continental.first().filter(|_| started) {
                let places = usize::from(places);
                if st.pos > places && st.pos <= places + 3 {
                    let gap = pts(places - 1) - mine;
                    n.add(
                        "table",
                        Tone::Info,
                        55,
                        "Chasing a continental place",
                        format!("{} from {} qualification with {} matches left.", plural(gap.max(0) as usize, "point", "points"), w.comps[cont].short_name, st.left),
                        format!("{} table", co.short_name),
                    );
                } else if st.pos <= places {
                    n.add(
                        "table",
                        Tone::Pos,
                        55,
                        "In a continental place",
                        format!("{} on {}, inside the {} places.", ordinal(st.pos), pts_word(mine as f32), w.comps[cont].short_name),
                        format!("{} table", co.short_name),
                    );
                }
            }
            if st.pos == 1 && st.n > 1 {
                let gap = mine - pts(1);
                let caught = gap > 3 * st.left as i32;
                n.add(
                    "table",
                    Tone::Pos,
                    85,
                    if caught { "Champions in all but name" } else { "Top of the table" },
                    if caught {
                        format!("{} clear with {} matches left: nobody can catch them.", pts_word(gap as f32), st.left)
                    } else {
                        if gap <= 0 {
                            format!("Level on points with {}, ahead on goal difference, with {} matches left.", c.team_short(rows[1].team), st.left)
                        } else {
                            format!("{} clear of {} with {} matches left.", plural(gap as usize, "point", "points"), c.team_short(rows[1].team), st.left)
                        }
                    },
                    format!("{} table", co.short_name),
                );
            }
            // What the board expects (the board's own numbers are only for an observer).
            if c.sees_club_internals(id) && club.board.target_position > 0 {
                let t = usize::from(club.board.target_position);
                if st.pos > t + 2 {
                    n.add(
                        "table",
                        Tone::Warn,
                        72,
                        "Below the board's expectation",
                        format!("{} in the table; the board expects {} or better.", ordinal(st.pos), ordinal(t)),
                        "Board target position (internal)",
                    );
                } else if st.pos + 2 <= t {
                    n.add(
                        "table",
                        Tone::Pos,
                        45,
                        "Beating the board's expectation",
                        format!("{} in the table; the board expects {}.", ordinal(st.pos), ordinal(t)),
                        "Board target position (internal)",
                    );
                }
            }
            // Goals for and against, against the rest of the league.
            if st.row.played >= 6 {
                let rate = |r: &pw_world::comp::TableRow, f: fn(&pw_world::comp::TableRow) -> u16| f64::from(f(r)) / f64::from(r.played.max(1));
                let mut by_att: Vec<_> = rows.iter().filter(|r| r.played > 0).map(|r| (r.team, rate(r, |r| r.gf))).collect();
                by_att.sort_by(|a, b| b.1.total_cmp(&a.1));
                let mut by_def: Vec<_> = rows.iter().filter(|r| r.played > 0).map(|r| (r.team, rate(r, |r| r.ga))).collect();
                by_def.sort_by(|a, b| a.1.total_cmp(&b.1));
                let (a, d) = (by_att.iter().position(|x| x.0 == team).unwrap_or(0), by_def.iter().position(|x| x.0 == team).unwrap_or(0));
                if a == 0 {
                    n.add(
                        "goals",
                        Tone::Pos,
                        50,
                        "The league's best attack",
                        format!("{} goals in {} matches, {:.1} a game.", st.row.gf, st.row.played, f64::from(st.row.gf) / f64::from(st.row.played)),
                        format!("{} table", co.short_name),
                    );
                } else if a == by_att.len() - 1 {
                    n.add(
                        "goals",
                        Tone::Warn,
                        50,
                        "The league's weakest attack",
                        format!("{} goals in {} matches, {:.1} a game.", st.row.gf, st.row.played, f64::from(st.row.gf) / f64::from(st.row.played)),
                        format!("{} table", co.short_name),
                    );
                }
                if d == 0 {
                    n.add(
                        "goals",
                        Tone::Pos,
                        50,
                        "The league's best defence",
                        format!("{} conceded in {} matches, {:.1} a game.", st.row.ga, st.row.played, f64::from(st.row.ga) / f64::from(st.row.played)),
                        format!("{} table", co.short_name),
                    );
                } else if d == by_def.len() - 1 {
                    n.add(
                        "goals",
                        Tone::Warn,
                        50,
                        "The league's leakiest defence",
                        format!("{} conceded in {} matches, {:.1} a game.", st.row.ga, st.row.played, f64::from(st.row.ga) / f64::from(st.row.played)),
                        format!("{} table", co.short_name),
                    );
                }
            }
        }

        // Form and runs.
        let (all, home, away) = league_form(c, comp, team);
        streaks(&mut n, &name, &all, "league", 65);
        if all.len() >= 10 {
            let recent = &all[all.len() - 6..];
            let before = &all[..all.len() - 6];
            let (r, b) = (ppg(recent), ppg(before));
            if r >= b + 0.8 {
                n.add("form", Tone::Pos, 62, "Form has picked up", format!("{} from the last six league matches, against {b:.1} a game before that.", pts_word(r * 6.0)), "League results this season");
            } else if r + 0.8 <= b {
                n.add("form", Tone::Warn, 62, "Form has dipped", format!("{} from the last six league matches, against {b:.1} a game before that.", pts_word(r * 6.0)), "League results this season");
            }
        }
        if home.len() >= 5 && away.len() >= 5 {
            let (h, a) = (ppg(&home), ppg(&away));
            if h >= a + 1.0 {
                n.add(
                    "form",
                    Tone::Info,
                    40,
                    "A far better side at home",
                    format!("{h:.1} points a game at home, {a:.1} away from home."),
                    format!("{} home and {} away league matches", home.len(), away.len()),
                );
            } else if a >= h + 1.0 {
                n.add("form", Tone::Info, 40, "Better away than at home", format!("{a:.1} points a game away, {h:.1} at home."), format!("{} home and {} away league matches", home.len(), away.len()));
            }
        }
    }

    // The squad: where the goals come from, age, depth.
    let squad: Vec<PlayerId> = w.teams[team].squad.iter().copied().filter(|&p| w.players.hot[p].status == PlayerStatus::Active).collect();
    let hidden_dates: Vec<Date> = c.concealed_fixtures().into_iter().filter(|f| f.involves(team)).map(|f| f.date).collect();
    let mut scorers: Vec<(PlayerId, u16)> = squad.iter().filter_map(|&p| year_line(c, p, &hidden_dates).map(|l| (p, l.goals))).filter(|(_, g)| *g > 0).collect();
    scorers.sort_by_key(|x| std::cmp::Reverse(x.1));
    let total: u16 = scorers.iter().map(|x| x.1).sum();
    if total >= 12 {
        let top = scorers[0];
        let two = top.1 + scorers.get(1).map_or(0, |x| x.1);
        if f32::from(top.1) / f32::from(total) >= 0.35 {
            n.add(
                "squad",
                Tone::Warn,
                58,
                "Goals come from one player",
                format!("{} has {} of the squad's {} goals in {} ({:.0}%).", c.player_short(top.0), top.1, total, w.date.year(), 100.0 * f32::from(top.1) / f32::from(total)),
                "Goals by current squad members, all competitions",
            );
            n.link(c.player_ref(top.0), c.player_short(top.0));
        } else if f32::from(two) / f32::from(total) >= 0.55 && scorers.len() >= 2 {
            n.add(
                "squad",
                Tone::Info,
                45,
                "Goals come from two players",
                format!("{} and {} have {} of the squad's {} goals in {}.", c.player_short(top.0), c.player_short(scorers[1].0), two, total, w.date.year()),
                "Goals by current squad members, all competitions",
            );
            n.link(c.player_ref(top.0), c.player_short(top.0));
        }
    }
    if let Some(comp) = league {
        let age_of = |t: TeamId| -> Option<f32> {
            let v: Vec<f32> = w.teams[t].squad.iter().filter(|&&p| w.players.hot[p].status == PlayerStatus::Active).map(|&p| w.age_years(p)).collect();
            (v.len() >= 12).then(|| mean(&v))
        };
        let mut ages: Vec<(TeamId, f32)> = w.comps[comp].state.entrants.iter().filter_map(|&t| age_of(t).map(|a| (t, a))).collect();
        ages.sort_by(|a, b| b.1.total_cmp(&a.1));
        if ages.len() >= 8 {
            let mine = ages.iter().position(|x| x.0 == team);
            if let Some(i) = mine {
                if i == 0 {
                    n.add(
                        "squad",
                        Tone::Info,
                        40,
                        "The oldest squad in the league",
                        format!("Average age {:.1}, against {:.1} for the youngest squad.", ages[i].1, ages[ages.len() - 1].1),
                        format!("First-team squads in the {}", w.comps[comp].short_name),
                    );
                } else if i == ages.len() - 1 {
                    n.add(
                        "squad",
                        Tone::Info,
                        40,
                        "The youngest squad in the league",
                        format!("Average age {:.1}, against {:.1} for the oldest squad.", ages[i].1, ages[0].1),
                        format!("First-team squads in the {}", w.comps[comp].short_name),
                    );
                }
            }
        }
    }
    // Who is missing, by line of the team.
    let mut avail = [(0usize, 0usize); 4];
    for &p in &squad {
        let g = w.players.cold[p].best_pos.group().idx();
        avail[g].0 += 1;
        if w.players.hot[p].injury == 0 && w.players.hot[p].ban == 0 {
            avail[g].1 += 1;
        }
    }
    let out: usize = avail.iter().map(|a| a.0 - a.1).sum();
    let need = [(PosGroup::Gk, 2usize), (PosGroup::Def, 4), (PosGroup::Mid, 3), (PosGroup::Att, 2)];
    for (g, min) in need {
        let (have, fit) = avail[g.idx()];
        if have >= min && fit < min {
            n.add(
                "squad",
                if fit + 1 < min { Tone::Neg } else { Tone::Warn },
                72,
                format!("Short of fit {}", group_noun(g)),
                format!("{} of {} are available.", plural(fit, &format!("fit {}", group_noun(g)), &format!("fit {}s", group_noun(g))), have),
                "Injuries and suspensions in the first-team squad",
            );
        }
    }
    if out >= 5 {
        n.add(
            "squad",
            Tone::Warn,
            66,
            "A long treatment list",
            format!("{} first-team players are unavailable through injury or suspension.", count_word(out)),
            "Injuries and suspensions in the first-team squad",
        );
    }
    let home_grown = squad.iter().filter(|&&p| w.players.cold[p].youth_club == id).count();
    if squad.len() >= 12 && home_grown * 3 >= squad.len() && home_grown * 10 <= squad.len() * 9 {
        n.add(
            "squad",
            Tone::Info,
            30,
            "Rooted in the academy",
            format!("{home_grown} of {} first-team players came through the club's own youth system.", squad.len()),
            "Youth club recorded for each player",
        );
    }

    // Manager.
    if club.manager.is_some() {
        let m = &w.staff[club.manager];
        let r = &m.record;
        let tenure = m.joined.days_until(w.date);
        if tenure > 0 && tenure <= 90 {
            n.add("manager", Tone::Info, 44, "A new manager", format!("{} took over {} ago.", c.person_short(m.person), months(tenure)), "Appointment date");
            n.link(Ref::person(m.person), c.person_short(m.person));
        }
        if r.games >= 15 && tenure >= 90 {
            let ppg = (3.0 * r.wins as f32 + r.draws as f32) / r.games as f32;
            n.add(
                "manager",
                if ppg >= 1.8 {
                    Tone::Pos
                } else if ppg < 1.1 {
                    Tone::Warn
                } else {
                    Tone::Muted
                },
                36,
                "The manager's record",
                format!("{} has won {} of {} matches in charge here, {ppg:.2} points a game.", c.person_short(m.person), r.wins, r.games),
                "Managerial record with this club",
            );
            n.link(Ref::person(m.person), c.person_short(m.person));
        }
    }
    if c.sees_club_internals(id) {
        let b = &club.board;
        if b.satisfaction <= 30 {
            n.add(
                "board",
                Tone::Neg,
                80,
                "The board is losing patience",
                format!(
                    "Satisfaction {}/100, patience {}/100{}.",
                    b.satisfaction,
                    b.patience,
                    if b.warnings > 0 { format!(", after {}", plural(usize::from(b.warnings), "warning", "warnings")) } else { String::new() }
                ),
                "Board state (internal)",
            );
        }
        let f = &club.finance;
        if f.wage_budget > 0 && f.wage_bill * 100 > f.wage_budget * 105 {
            n.add(
                "finance",
                Tone::Warn,
                60,
                "Over the wage budget",
                format!("The wage bill is {:.0}% of the weekly budget.", 100.0 * f.wage_bill as f64 / f.wage_budget as f64),
                "Club finances (internal)",
            );
        }
        if f.debt > 0 && f.balance < 0 {
            n.add("finance", Tone::Neg, 65, "In the red", "The club is in debt and its balance is negative.".to_string(), "Club finances (internal)");
        }
    }

    // A crowded calendar.
    let ahead = w.fixtures.of_team_between(team, w.date, w.date.add_days(14)).count();
    if ahead >= 5 {
        n.add("calendar", Tone::Info, 46, "A crowded fortnight", format!("{} matches in the next 14 days.", count_word(ahead)), "Fixture list");
    }
    Ok(n.finish(args))
}

// ---- competitions --------------------------------------------------------------------------

pub fn comp(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = CompId(args.get("id").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing competition id".into()))? as u32);
    if id.0 as usize >= c.w.comps.len() {
        return Err(ApiError::NotFound(format!("competition {}", id.0)));
    }
    let w = c.w;
    let co = &w.comps[id];
    let mut n = Notes::default();
    if !co.is_league() {
        return Ok(n.finish(args));
    }
    let (rows, hidden) = crate::tables::visible_table(c, id);
    n.held = hidden;
    let cnt = rows.len();
    if cnt < 4 {
        return Ok(n.finish(args));
    }
    let pw_world::Format::League { rounds } = co.format else { return Ok(n.finish(args)) };
    let per_team = usize::from(rounds) * (cnt - 1);
    let left = |r: &pw_world::comp::TableRow| per_team.saturating_sub(usize::from(r.played));
    let pts = |i: usize| i32::from(rows[i].points);
    let played: usize = rows.iter().map(|r| usize::from(r.played)).sum::<usize>() / 2;
    let name = |i: usize| c.team_short(rows[i].team);
    let tr = |i: usize| c.team_ref(rows[i].team);
    let basis = format!("{} table, {} teams", co.short_name, cnt);
    if rows.iter().all(|r| r.played == 0) {
        return Ok(n.finish(args));
    }

    // The top of the table.
    let gap = pts(0) - pts(1);
    let l1 = left(&rows[1]);
    if left(&rows[0]) == 0 && rows[0].played > 0 {
        n.add("table", Tone::Pos, 90, format!("{} are champions", name(0)), if gap <= 0 { format!("{} points, ahead of {} on goal difference.", pts(0), name(1)) } else { format!("{} points, {} clear of {}.", pts(0), plural(gap as usize, "point", "points"), name(1)) }, basis.clone());
    } else if gap > 3 * l1 as i32 {
        n.add(
            "table",
            Tone::Pos,
            90,
            format!("{} have won the title", name(0)),
            format!("{} clear with {} matches left for {}: the gap cannot be closed.", pts_word(gap as f32), l1, name(1)),
            basis.clone(),
        );
    } else if gap <= 3 && rows[0].played >= 8 {
        n.add(
            "table",
            Tone::Info,
            80,
            "A tight race at the top",
            format!("{} lead {} {} with {} matches left for the leaders.", name(0), name(1), margin(gap), left(&rows[0])),
            basis.clone(),
        );
    } else {
        n.add(
            "table",
            Tone::Info,
            70,
            "The title race",
            format!("{} lead {} {} with {} matches left for the leaders.", name(0), name(1), margin(gap), left(&rows[0])),
            basis.clone(),
        );
    }
    n.visual(InsightVisual::Comparison { label: "Top two teams on points".into(), unit: "points".into(), names: vec![name(0), name(1)], values: vec![pts(0), pts(1)] });
    n.link(tr(0), name(0));
    if co.promote > 0 {
        let k = usize::from(co.promote);
        if k < cnt {
            let d = pts(k - 1) - pts(k);
            n.add(
                "table",
                Tone::Info,
                66,
                "The promotion line",
                if d > 0 {
                    format!("{} are in the last promotion place, {} ahead of {} with {} matches left.", name(k - 1), plural(d as usize, "point", "points"), name(k), left(&rows[k - 1]))
                } else {
                    format!("{} hold the last promotion place on goal difference, level on points with {} with {} matches left.", name(k - 1), name(k), left(&rows[k - 1]))
                },
                basis.clone(),
            );
            n.link(tr(k), name(k));
        }
    }
    if let Some(&(cont, places)) = co.continental.first() {
        let k = usize::from(places);
        if k > 0 && k < cnt {
            let d = pts(k - 1) - pts(k);
            n.add(
                "table",
                Tone::Info,
                58,
                format!("The race for {}", w.comps[cont].short_name),
                if d > 0 {
                    format!("{} hold the last place, {} ahead of {} with {} matches left.", name(k - 1), plural(d as usize, "point", "points"), name(k), left(&rows[k - 1]))
                } else {
                    format!("{} hold the last place on goal difference, level on points with {} with {} matches left.", name(k - 1), name(k), left(&rows[k - 1]))
                },
                basis.clone(),
            );
            n.link(tr(k), name(k));
        }
    }
    if co.relegate > 0 {
        let r = usize::from(co.relegate);
        if r < cnt {
            let safe = cnt - r - 1;
            let d = pts(safe) - pts(safe + 1);
            let within: usize = rows.iter().enumerate().filter(|(i, _)| *i + r + 3 >= cnt && *i + r < cnt + 3).filter(|(i, _)| (pts(safe) - pts(*i)).abs() <= 6).count();
            n.add(
                "table",
                if d <= 3 { Tone::Warn } else { Tone::Info },
                68,
                "The relegation battle",
                format!(
                    "{} are the last safe side, {} {} in the drop zone, with {} matches left. {} teams are within six points of the line.",
                    name(safe),
                    ahead_of(d),
                    name(safe + 1),
                    left(&rows[safe]),
                    within
                ),
                basis.clone(),
            );
            n.link(tr(safe + 1), name(safe + 1));
        }
    }

    // Scoring across the league.
    let goals: u32 = rows.iter().map(|r| u32::from(r.gf)).sum();
    if played >= 10 {
        let per = goals as f32 / played as f32;
        let fixtures: Vec<_> =
            w.fixtures.between(co.state.start.add_days(-1), w.date).map(|f| w.fixtures.get(f)).filter(|f| f.comp == id && !c.is_concealed(f.uid)).filter_map(|f| f.score.map(|s| (f, s))).collect();
        let home_wins = fixtures.iter().filter(|(_, s)| s.home > s.away).count();
        let draws = fixtures.iter().filter(|(_, s)| s.home == s.away).count();
        if !fixtures.is_empty() {
            let pct = |k: usize| 100.0 * k as f32 / fixtures.len() as f32;
            n.add(
                "goals",
                Tone::Muted,
                30,
                "How the season is playing out",
                format!("{per:.2} goals a game. Home sides win {:.0}%, {:.0}% end level.", pct(home_wins), pct(draws)),
                format!("{} results this season", fixtures.len()),
            );
        }
    }
    let mut by_gd: Vec<usize> = (0..cnt).filter(|&i| rows[i].played > 0).collect();
    by_gd.sort_by(|a, b| (f64::from(rows[*b].gf) / f64::from(rows[*b].played)).total_cmp(&(f64::from(rows[*a].gf) / f64::from(rows[*a].played))));
    if let Some(&i) = by_gd.first().filter(|_| rows[by_gd[0]].played >= 6) {
        n.add(
            "goals",
            Tone::Info,
            44,
            "The best attack",
            format!("{} have scored {} in {} matches, {:.1} a game.", name(i), rows[i].gf, rows[i].played, f64::from(rows[i].gf) / f64::from(rows[i].played)),
            basis.clone(),
        );
        n.link(tr(i), name(i));
    }
    by_gd.sort_by(|a, b| (f64::from(rows[*a].ga) / f64::from(rows[*a].played)).total_cmp(&(f64::from(rows[*b].ga) / f64::from(rows[*b].played))));
    if let Some(&i) = by_gd.first().filter(|_| rows[by_gd[0]].played >= 6) {
        n.add(
            "goals",
            Tone::Info,
            44,
            "The best defence",
            format!("{} have conceded {} in {} matches, {:.1} a game.", name(i), rows[i].ga, rows[i].played, f64::from(rows[i].ga) / f64::from(rows[i].played)),
            basis.clone(),
        );
        n.link(tr(i), name(i));
    }

    // Form: points from the last five, over the league.
    let from = co.state.start.add_days(-1);
    let mut last: rustc_hash::FxHashMap<TeamId, Vec<char>> = rustc_hash::FxHashMap::default();
    let mut fx: Vec<_> = w.fixtures.between(from, w.date).map(|f| w.fixtures.get(f)).filter(|f| f.comp == id && !c.is_concealed(f.uid)).filter_map(|f| f.score.map(|s| (f, s))).collect();
    fx.sort_by_key(|(f, _)| (f.date, f.uid));
    for (f, s) in &fx {
        last.entry(f.home).or_default().push(outcome(f, *s, f.home));
        last.entry(f.away).or_default().push(outcome(f, *s, f.away));
    }
    let mut form: Vec<(TeamId, f32, usize)> = last.iter().filter(|(_, v)| v.len() >= 5).map(|(&t, v)| (t, ppg(&v[v.len() - 5..]), run_of(v, |r| r == 'W'))).collect();
    form.sort_by(|a, b| b.1.total_cmp(&a.1).then(b.2.cmp(&a.2)));
    if form.len() >= 6 {
        let (best, worst) = (form[0], form[form.len() - 1]);
        n.add(
            "form",
            Tone::Pos,
            52,
            "The form side",
            format!("{} took {} from their last five{}.", c.team_short(best.0), pts_word(best.1 * 5.0), if best.2 >= 3 { format!(", and have won {} in a row", best.2) } else { String::new() }),
            "The last five league results of each team",
        );
        n.link(c.team_ref(best.0), c.team_short(best.0));
        n.add("form", Tone::Warn, 48, "The side out of form", format!("{} took {} from their last five.", c.team_short(worst.0), pts_word(worst.1 * 5.0)), "The last five league results of each team");
        n.link(c.team_ref(worst.0), c.team_short(worst.0));
    }

    // Individuals.
    if hidden == 0 {
        let s = co.state.season;
        let mut lines: Vec<&pw_world::stats::StatLine> = w.stats.for_comp(id).filter(|l| l.season == s).collect();
        lines.sort_by_key(|l| std::cmp::Reverse((l.goals, l.assists)));
        if let Some(top) = lines.first().filter(|l| l.goals >= 4) {
            let second = lines.get(1).map_or(0, |l| l.goals);
            n.add(
                "players",
                Tone::Pos,
                56,
                "Top scorer",
                format!(
                    "{} leads with {} goals{}.",
                    c.player_short(top.player),
                    top.goals,
                    if top.goals > second { format!(", {} clear of the next", top.goals - second) } else { ", level with the next".to_string() }
                ),
                "Season statistics",
            );
            n.link(c.player_ref(top.player), c.player_short(top.player));
        }
        lines.sort_by_key(|l| std::cmp::Reverse(l.assists));
        if let Some(top) = lines.first().filter(|l| l.assists >= 4) {
            n.add("players", Tone::Info, 40, "Most assists", format!("{} has {} assists.", c.player_short(top.player), top.assists), "Season statistics");
            n.link(c.player_ref(top.player), c.player_short(top.player));
        }
        let mut rated: Vec<&&pw_world::stats::StatLine> = lines.iter().filter(|l| l.apps >= 8).collect();
        rated.sort_by(|a, b| b.avg_rating().total_cmp(&a.avg_rating()));
        if let Some(top) = rated.first() {
            n.add(
                "players",
                Tone::Info,
                42,
                "Best average rating",
                format!("{} averages {:.2} over {} matches.", c.player_short(top.player), top.avg_rating(), top.apps),
                "Season statistics, players with at least eight appearances",
            );
            n.link(c.player_ref(top.player), c.player_short(top.player));
        }
        let mut keepers: Vec<&&pw_world::stats::StatLine> = lines.iter().filter(|l| l.apps >= 8 && l.clean_sheets > 0).collect();
        keepers.sort_by(|a, b| (f32::from(b.clean_sheets) / f32::from(b.apps)).total_cmp(&(f32::from(a.clean_sheets) / f32::from(a.apps))));
        if let Some(top) = keepers.first().filter(|l| l.clean_sheets >= 4) {
            n.add(
                "players",
                Tone::Info,
                36,
                "Most clean sheets",
                format!("{} has kept {} in {} matches.", c.player_short(top.player), plural(usize::from(top.clean_sheets), "clean sheet", "clean sheets"), top.apps),
                "Season statistics",
            );
            n.link(c.player_ref(top.player), c.player_short(top.player));
        }
    }
    Ok(n.finish(args))
}

// ---- matches -------------------------------------------------------------------------------

/// Talking points for a match: what is at stake before it, and what it meant after.
pub fn matchup(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let uid = crate::pages::matchp::uid_arg(args)?;
    let (_, fx) = crate::pages::matchp::find(c, uid).ok_or_else(|| crate::pages::matchp::gone(c, uid))?;
    let w = c.w;
    let mut n = Notes::default();
    if c.is_concealed(uid) {
        n.held = 1;
        return Ok(n.finish(args));
    }
    let co = &w.comps[fx.comp];
    let (home, away) = (fx.home, fx.away);
    let (hn, an) = (c.team_short(home), c.team_short(away));
    let played = fx.score.is_some();
    // Everything is measured as of the day before, so a played match is not counted in its own preview.
    let before = fx.date.add_days(-1);
    let league = co.is_league();

    let side = |t: TeamId| -> Vec<(&Fixture, Score)> { results(c, t, co.state.start.add_days(-1)).into_iter().filter(|(f, _)| f.comp == fx.comp && f.date <= before && f.uid != uid).collect() };
    let (hr, ar) = (side(home), side(away));
    let form_of = |v: &[(&Fixture, Score)], t: TeamId| -> Vec<char> { v.iter().map(|(f, s)| outcome(f, *s, t)).collect() };
    let (hf, af) = (form_of(&hr, home), form_of(&ar, away));

    if league
        && !played
        && let (Some(h), Some(a)) = (standing(c, fx.comp, home), standing(c, fx.comp, away))
    {
        let (rows, _) = crate::tables::visible_table(c, fx.comp);
        let gap = (i32::from(h.row.points) - i32::from(a.row.points)).abs();
        let r = usize::from(co.relegate);
        let bottom = |s: &Standing| r > 0 && s.pos + r + 2 > s.n;
        let top = |s: &Standing| s.pos <= 3;
        if h.row.played >= 6 && a.row.played >= 6 {
            if bottom(&h) && bottom(&a) && gap <= 4 {
                n.add(
                    "stakes",
                    Tone::Warn,
                    90,
                    "A relegation six-pointer",
                    format!("{} are {} and {} {}, {} apart. The winner takes a big step towards safety.", hn, ordinal(h.pos), an, ordinal(a.pos), plural(gap as usize, "point", "points")),
                    format!("{} table", co.short_name),
                );
            } else if top(&h) && top(&a) {
                n.add(
                    "stakes",
                    Tone::Info,
                    90,
                    "A meeting near the top",
                    format!("{} are {} and {} {}, {} apart.", hn, ordinal(h.pos), an, ordinal(a.pos), plural(gap as usize, "point", "points")),
                    format!("{} table", co.short_name),
                );
            } else if h.pos == 1 || a.pos == 1 {
                let (lead, other, o) = if h.pos == 1 { (&hn, &an, &a) } else { (&an, &hn, &h) };
                let second = rows.get(1).map_or(0, |r| i32::from(r.points));
                n.add(
                    "stakes",
                    Tone::Info,
                    60,
                    "The leaders in action",
                    format!("{lead} are top, {} ahead of second place. {other} are {}.", plural((i32::from(rows[0].points) - second).max(0) as usize, "point", "points"), ordinal(o.pos)),
                    format!("{} table", co.short_name),
                );
            }
        }
        // A win could change the picture.
        if h.pos > a.pos && h.row.played >= 6 && i32::from(a.row.points) - i32::from(h.row.points) <= 3 && h.pos - a.pos <= 4 {
            n.add("stakes", Tone::Info, 55, "A chance to climb", format!("A win would lift {hn} above {an} in the table."), format!("{} table", co.short_name));
        } else if a.pos > h.pos && a.row.played >= 6 && i32::from(h.row.points) - i32::from(a.row.points) <= 3 && a.pos - h.pos <= 4 {
            n.add("stakes", Tone::Info, 55, "A chance to climb", format!("A win would lift {an} above {hn} in the table."), format!("{} table", co.short_name));
        }
    }

    // Form going in.
    let last5 = |f: &[char]| -> String { f.iter().rev().take(5).rev().collect() };
    if hf.len() >= 5 && af.len() >= 5 {
        let (hp, ap) = (ppg(&hf[hf.len() - 5..]), ppg(&af[af.len() - 5..]));
        if (hp - ap).abs() >= 1.2 {
            let (better, worse, bf, wf) = if hp > ap { (&hn, &an, &hf, &af) } else { (&an, &hn, &af, &hf) };
            n.add(
                "form",
                Tone::Info,
                62,
                "Contrasting form",
                format!("{better} come in on {} from their last five; {worse} on {}.", last5(bf), last5(wf)),
                "The last five league results of each side before this match",
            );
        }
    }
    for (who, f) in [(&hn, &hf), (&an, &af)] {
        let w3 = run_of(f, |r| r == 'W');
        let l3 = run_of(f, |r| r == 'L');
        if w3 >= 3 {
            n.add("form", Tone::Pos, 58, format!("{who} on a run"), format!("{who} come in on {} wins in a row.", count_word(w3).to_lowercase()), "Results before this match");
        } else if l3 >= 3 {
            n.add("form", Tone::Warn, 58, format!("{who} struggling"), format!("{who} come in on {} defeats in a row.", count_word(l3).to_lowercase()), "Results before this match");
        }
    }
    // Home against away records.
    let home_only: Vec<char> = hr.iter().filter(|(f, _)| f.home == home).map(|(f, s)| outcome(f, *s, home)).collect();
    let away_only: Vec<char> = ar.iter().filter(|(f, _)| f.away == away).map(|(f, s)| outcome(f, *s, away)).collect();
    if home_only.len() >= 5 && away_only.len() >= 5 && !played {
        let (h, a) = (ppg(&home_only), ppg(&away_only));
        if h >= 2.0 && a <= 0.9 {
            n.add(
                "form",
                Tone::Info,
                50,
                "Strong at home against a poor traveller",
                format!("{hn} take {h:.1} points a game at home; {an} take {a:.1} away."),
                "Home and away league records this season",
            );
        } else if a >= 1.8 && h <= 1.0 {
            n.add("form", Tone::Info, 50, "A good traveller meets a shaky host", format!("{an} take {a:.1} points a game away; {hn} take {h:.1} at home."), "Home and away league records this season");
        }
    }

    // Meetings between the two, from what has been revealed.
    let mut meets: Vec<(&Fixture, Score)> = w
        .fixtures
        .of_team_between(home, Date(fx.date.0 - 2000), fx.date.add_days(-1))
        .map(|i| w.fixtures.get(i))
        .filter(|f| f.uid != uid && f.involves(away) && !c.is_concealed(f.uid))
        .filter_map(|f| f.score.map(|s| (f, s)))
        .collect();
    meets.sort_by_key(|(f, _)| (f.date, f.uid));
    if meets.len() >= 3 {
        let hw = meets.iter().filter(|(f, s)| outcome(f, *s, home) == 'W').count();
        let aw = meets.iter().filter(|(f, s)| outcome(f, *s, away) == 'W').count();
        let dr = meets.len() - hw - aw;
        let last3: Vec<char> = meets.iter().rev().take(3).map(|(f, s)| outcome(f, *s, home)).collect();
        if hw > meets.len() * 3 / 5 {
            n.add(
                "history",
                Tone::Info,
                44,
                format!("{hn} have the upper hand"),
                format!("{hw} wins, {dr} draws and {aw} defeats in {} meetings.", meets.len()),
                "Earlier meetings still in the records",
            );
        } else if aw > meets.len() * 3 / 5 {
            n.add(
                "history",
                Tone::Info,
                44,
                format!("{an} have the upper hand"),
                format!("{aw} wins, {dr} draws and {hw} defeats in {} meetings.", meets.len()),
                "Earlier meetings still in the records",
            );
        } else if last3.iter().all(|&r| r == 'D') {
            n.add("history", Tone::Muted, 32, "Stalemate", format!("The last {} meetings ended level.", count_word(last3.len())), "Earlier meetings still in the records");
        }
    }

    // Who is missing, and who to watch.
    if !played {
        for (t, who) in [(home, &hn), (away, &an)] {
            let hidden: Vec<Date> = c.concealed_fixtures().into_iter().filter(|f| f.involves(t)).map(|f| f.date).collect();
            let squad: Vec<PlayerId> = w.teams[t].squad.iter().copied().filter(|&p| w.players.hot[p].status == PlayerStatus::Active).collect();
            let mut used: Vec<(PlayerId, u32)> = squad.iter().filter_map(|&p| year_line(c, p, &hidden).map(|l| (p, l.minutes))).collect();
            used.sort_by_key(|x| std::cmp::Reverse(x.1));
            let key: Vec<PlayerId> = used.iter().take(8).map(|x| x.0).collect();
            let out: Vec<PlayerId> = key.iter().copied().filter(|&p| w.players.hot[p].injury != 0 || w.players.hot[p].ban > 0).collect();
            if !out.is_empty() {
                let names: Vec<String> = out.iter().take(3).map(|&p| c.player_short(p)).collect();
                n.add(
                    "squad",
                    if out.len() >= 3 { Tone::Neg } else { Tone::Warn },
                    74,
                    format!("{who} without key players"),
                    format!("{} of the eight players with most minutes this year {} out: {}.", count_word(out.len()), if out.len() == 1 { "is" } else { "are" }, names.join(", ")),
                    "Injuries and suspensions against minutes played",
                );
                n.link(c.player_ref(out[0]), c.player_short(out[0]));
            }
            let mut sc: Vec<(PlayerId, u16)> = squad.iter().filter_map(|&p| year_line(c, p, &hidden).map(|l| (p, l.goals))).filter(|x| x.1 >= 4).collect();
            sc.sort_by_key(|x| std::cmp::Reverse(x.1));
            if let Some(&(p, g)) = sc.first().filter(|(p, _)| w.players.hot[*p].injury == 0 && w.players.hot[*p].ban == 0) {
                n.add(
                    "players",
                    Tone::Info,
                    46,
                    format!("{who}: the one to watch"),
                    format!(
                        "{} has {g} goals in {}{}.",
                        c.player_short(p),
                        w.date.year(),
                        if let Some(f) = year_line(c, p, &hidden).filter(|l| l.apps > 0) { format!(", in {} appearances", f.apps) } else { String::new() }
                    ),
                    "Goals this calendar year, all competitions",
                );
                n.link(c.player_ref(p), c.player_short(p));
            }
        }
    }

    // After the match: what it means for the run each side is on.
    if let Some(sc) = fx.score.filter(|_| played) {
        for (t, who, prior) in [(home, &hn, &hf), (away, &an, &af)] {
            let r = outcome(fx, sc, t);
            let mut f = prior.clone();
            f.push(r);
            let win = run_of(&f, |x| x == 'W');
            let unb = run_of(&f, |x| x != 'L');
            let los = run_of(&f, |x| x == 'L');
            if r == 'W' && win >= 3 {
                n.add("form", Tone::Pos, 70, format!("{who}: {} wins in a row", count_word(win)), "The run goes on.".to_string(), "League results up to and including this match");
            } else if r == 'L' && prior.len() >= 5 && run_of(prior, |x| x != 'L') >= 5 {
                n.add("form", Tone::Warn, 70, format!("{who}'s unbeaten run ends"), format!("It had lasted {} matches.", run_of(prior, |x| x != 'L')), "League results up to and including this match");
            } else if r == 'W' && prior.len() >= 4 && run_of(prior, |x| x != 'W') >= 4 {
                n.add(
                    "form",
                    Tone::Pos,
                    66,
                    format!("{who} end a winless run"),
                    format!("The first win in {} matches.", run_of(prior, |x| x != 'W') + 1),
                    "League results up to and including this match",
                );
            } else if r == 'L' && los >= 3 {
                n.add("form", Tone::Neg, 66, format!("{who}: {} defeats in a row", count_word(los)), "The slide continues.".to_string(), "League results up to and including this match");
            } else if r != 'L' && unb >= 6 {
                n.add("form", Tone::Pos, 60, format!("{who} unbeaten in {}", count_word(unb)), "The run goes on.".to_string(), "League results up to and including this match");
            }
        }
        if let Some(mf) = w.recent_matches.by_uid(uid) {
            if mf.hg + mf.ag >= 6 {
                n.add("match", Tone::Info, 50, "A goal fest", format!("{} goals in one match.", mf.hg + mf.ag), "Result");
            }
            if let Some(g) = mf.late_winner {
                n.add("match", Tone::Info, 62, "Decided late", format!("{} scored the winner in the {}th minute.", c.player_short(g.player), g.minute), "Match facts");
                n.link(c.player_ref(g.player), c.player_short(g.player));
            }
            if mf.comeback {
                n.add("match", Tone::Info, 60, "A comeback", "The winners were behind at one stage.".to_string(), "Match facts");
            }
        }
    }
    Ok(n.finish(args))
}
