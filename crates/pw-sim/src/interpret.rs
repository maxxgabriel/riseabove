//! Performance records and their interpretation.
//!
//! `record` stores what happened in every senior match — who played, who sat
//! on the bench, who was left out — with enough context (opponent, stakes) to
//! be read later. `monthly` derives how each observer reads a player through
//! their own lens. The lenses disagree on purpose: that disagreement is where
//! "underrated" and "overrated" come from, and it is what media stories,
//! scouting pitches and fan moods pick up.

use pw_core::{ClubId, PlayerId};
use pw_match::MatchResult;
use pw_world::perf::{App, Label, Lens, RECENT, Reading};
use pw_world::{Archetype, Fixture, PlayerStatus, TeamKind, World};
use smallvec::SmallVec;

use crate::selection::Selection;

/// Record a played senior fixture.
pub fn record(w: &mut World, fx: &Fixture, home: &Selection, away: &Selection, r: &MatchResult, importance: f32) {
    let today = w.date;
    let sides = [home, away];
    let clubs = [w.teams[fx.home].club, w.teams[fx.away].club];
    let senior = [w.teams[fx.home].kind == TeamKind::First, w.teams[fx.away].kind == TeamKind::First];
    let reps = [w.clubs[clubs[0]].reputation, w.clubs[clubs[1]].reputation];
    let sign = [i8::try_from(i32::from(r.home_goals).cmp(&i32::from(r.away_goals)) as i32).unwrap_or(0), i8::try_from(i32::from(r.away_goals).cmp(&i32::from(r.home_goals)) as i32).unwrap_or(0)];
    let year = today.year();
    for line in &r.lines {
        let side = usize::from(line.side);
        if !senior[side] {
            continue;
        }
        let p = line.player;
        let club = clubs[side];
        let opp_rep = reps[1 - side];
        if line.minutes == 0 {
            w.perf.line_mut(p, year, club).benched += 1;
            continue;
        }
        let app = App {
            date: today,
            comp: fx.comp,
            club,
            opp_rep,
            started: line.started,
            pos: line.pos,
            minutes: line.minutes,
            rating: (line.rating * 10.0).round().clamp(0.0, 100.0) as u8,
            goals: line.goals,
            assists: line.assists,
            xgi: ((line.xg + line.xa) * 100.0).round().clamp(0.0, 60_000.0) as u16,
            key_passes: line.key_passes,
            tackles_won: line.tackles_won,
            result: sign[side],
            importance: (importance * 100.0) as u8,
        };
        let recent = w.perf.recent.entry(p).or_default();
        recent.push(app);
        if recent.len() > RECENT {
            recent.remove(0);
        }
        crate::honours::on_appearance(w, p, club, line.goals);
        let own_rep = reps[side];
        let big = importance >= 0.75 || opp_rep > own_rep.saturating_add(1000);
        let small = opp_rep.saturating_add(1500) < own_rep;
        let l = w.perf.line_mut(p, year, club);
        l.apps += 1;
        l.starts += u16::from(line.started);
        l.minutes += u32::from(line.minutes);
        l.goals += u16::from(line.goals);
        l.assists += u16::from(line.assists);
        l.rating_sum += u32::from(app.rating);
        l.xgi += u32::from(app.xgi);
        l.motm += u16::from(r.pom == p);
        if big {
            l.big_sum += u32::from(app.rating);
            l.big_apps += 1;
        }
        if small {
            l.small_sum += u32::from(app.rating);
            l.small_apps += 1;
        }
    }
    // Biggest wins.
    if senior[0] && senior[1] && r.home_goals != r.away_goals {
        let (win, lose) = if r.home_goals > r.away_goals { (clubs[0], clubs[1]) } else { (clubs[1], clubs[0]) };
        crate::honours::on_result(w, win, lose, r.home_goals.abs_diff(r.away_goals));
    }
    // Left out altogether.
    for (i, sel) in sides.iter().enumerate() {
        if !senior[i] {
            continue;
        }
        let squad = w.teams[sel.team].squad.clone();
        for p in squad {
            let h = &w.players.hot[p];
            if h.status != PlayerStatus::Active || h.injury != 0 || h.ban != 0 || sel.xi.contains(&p) || sel.bench.contains(&p) {
                continue;
            }
            w.perf.line_mut(p, year, clubs[i]).omitted += 1;
        }
    }
}

/// How a manager of a given archetype rates a stretch of performances, on
/// the rating scale. Pragmatists value work without the ball; developers
/// forgive the young; rotators value output per minute; loyalists value
/// reliability.
pub fn manager_reading(w: &World, arch: Archetype, p: PlayerId) -> Option<f32> {
    let recent = w.perf.recent.get(&p)?;
    if recent.is_empty() {
        return None;
    }
    let n = recent.len() as f32;
    let avg = recent.iter().map(|a| f32::from(a.rating)).sum::<f32>() / n / 10.0;
    let work = recent.iter().map(|a| f32::from(a.tackles_won)).sum::<f32>() / n;
    let output = recent.iter().map(|a| f32::from(a.goals + a.assists) * 90.0 / f32::from(a.minutes.max(1))).sum::<f32>() / n;
    let var = recent.iter().map(|a| (f32::from(a.rating) / 10.0 - avg).powi(2)).sum::<f32>() / n;
    let young = w.age_years(p) <= 21.0;
    Some(match arch {
        Archetype::Pragmatist => avg + (work - 1.5).max(0.0) * 0.1 - var.sqrt() * 0.2,
        Archetype::Developer => avg + if young { 0.25 } else { 0.0 },
        Archetype::Rotator => avg + output * 0.3,
        Archetype::Loyalist => avg - var.sqrt() * 0.4 + (n / RECENT as f32) * 0.2,
    })
}

/// Monthly: every lens reads every player with enough football.
pub fn monthly(w: &mut World) {
    let today = w.date;
    let year = today.year();
    let ids: Vec<PlayerId> = w.perf.recent.keys().copied().collect();
    let mut changed: Vec<(PlayerId, Label, Lens)> = Vec::new();
    for p in ids {
        if w.players.hot[p].status != PlayerStatus::Active {
            w.perf.readings.remove(&p);
            continue;
        }
        let old: SmallVec<[Reading; 3]> = w.perf.readings.get(&p).cloned().unwrap_or_default();
        let mut new: SmallVec<[Reading; 3]> = SmallVec::new();
        let mut add = |lens: Lens, label: Label| {
            let since = old.iter().find(|r| r.lens == lens && r.label == label).map_or(today, |r| r.since);
            if !new.iter().any(|r: &Reading| r.label == label) {
                new.push(Reading { lens, label, since });
            }
        };
        let season = w.perf.season(p, year).copied().or_else(|| w.perf.season(p, year - 1).copied());
        let recent = w.perf.recent.get(&p).cloned().unwrap_or_default();
        let c = &w.players.cold[p];
        if let Some(s) = season {
            let avg = s.avg();
            let recent_avg = if recent.is_empty() { avg } else { recent.iter().rev().take(5).map(|a| f32::from(a.rating)).sum::<f32>() / recent.len().min(5) as f32 / 10.0 };
            // Manager lens: form relative to their own baseline.
            let arch = w.clubs.get(w.players.hot[p].club).and_then(|cl| cl.manager.get()).map(|m| w.staff[m].philosophy.archetype);
            if let Some(a) = arch {
                if let Some(mr) = manager_reading(w, a, p) {
                    if s.apps >= 5 && mr - avg > 0.35 {
                        add(Lens::Manager, Label::InForm);
                    } else if s.apps >= 5 && avg - mr > 0.35 {
                        add(Lens::Manager, Label::InSlump);
                    }
                }
                if a == Archetype::Pragmatist && s.apps >= 8 && s.minutes as f32 / f32::from(s.apps.max(1)) > 75.0 {
                    let tackles: f32 = recent.iter().map(|x| f32::from(x.tackles_won)).sum::<f32>() / recent.len().max(1) as f32;
                    if tackles >= 2.5 {
                        add(Lens::Manager, Label::Workhorse);
                    }
                }
            }
            // Fans lens: the last few games and the goals.
            if recent.len() >= 3 && recent_avg - avg > 0.4 {
                add(Lens::Fans, Label::InForm);
            } else if recent.len() >= 3 && avg - recent_avg > 0.5 {
                add(Lens::Fans, Label::InSlump);
            }
            // Media lens: big nights, goals, breakthroughs.
            if s.big_apps >= 3 && s.small_apps >= 3 {
                let big = s.big_sum as f32 / f32::from(s.big_apps) / 10.0;
                let small = s.small_sum as f32 / f32::from(s.small_apps) / 10.0;
                if big - small > 0.4 {
                    add(Lens::Media, Label::BigGamePlayer);
                } else if small - big > 0.6 {
                    add(Lens::Media, Label::FlatTrackBully);
                }
            }
            if s.minutes >= 900 && f32::from(s.goals) * 90.0 / s.minutes as f32 >= 0.5 {
                add(Lens::Media, Label::GoalThreat);
            }
            if w.age_years(p) <= 21.0 && s.starts >= 8 && w.perf.seasons.get(&p).is_none_or(|v| v.iter().filter(|l| l.starts >= 8).count() <= 1) {
                add(Lens::Media, Label::Breakthrough);
            }
            // Analyst lens vs reputation: the disagreement is the story.
            if s.minutes >= 900 {
                let per90 = s.xgi as f32 / 100.0 * 90.0 / s.minutes as f32;
                let numbers = avg + per90 * 0.8;
                let expected = 6.2 + f32::from(c.rep.current) / 10_000.0 * 1.6;
                if numbers - expected > 0.45 {
                    add(Lens::Analyst, Label::Underrated);
                } else if expected - numbers > 0.45 {
                    add(Lens::Analyst, Label::Overrated);
                }
            }
            if s.omitted >= 6 && s.apps <= 2 {
                add(Lens::Fans, Label::FrozenOut);
            }
            if recent.len() >= 6 {
                let ravg = recent.iter().map(|a| f32::from(a.rating)).sum::<f32>() / recent.len() as f32 / 10.0;
                let sd = (recent.iter().map(|a| (f32::from(a.rating) / 10.0 - ravg).powi(2)).sum::<f32>() / recent.len() as f32).sqrt();
                if sd > 0.9 {
                    add(Lens::Scout, Label::Unreliable);
                }
            }
        }
        // Medical record.
        if w.medical.serious_recent(p, today, 730) >= 2 || w.medical.needs_managing(p) {
            add(Lens::Media, Label::InjuryProne);
        } else if c.injuries_career == 0 && w.age_years(p) >= 26.0 {
            add(Lens::Media, Label::Durable);
        }
        for r in &new {
            if !old.iter().any(|o| o.label == r.label) {
                changed.push((p, r.label, r.lens));
            }
        }
        if new.is_empty() {
            w.perf.readings.remove(&p);
        } else {
            w.perf.readings.insert(p, new);
        }
    }
    // New public readings go into the media's notebook (media drains it).
    for (p, label, lens) in changed {
        if matches!(lens, Lens::Media | Lens::Fans | Lens::Analyst) {
            w.perf.fresh.push((p, label, lens, today));
        }
    }
}

/// Mid-season: the club reads its own squad's records when it thinks about
/// statuses (e.g. a manager's in-form player climbs, a frozen-out one falls).
pub fn club_reading(w: &World, club: ClubId, p: PlayerId) -> f32 {
    let arch = w.clubs[club].manager.get().map_or(Archetype::Pragmatist, |m| w.staff[m].philosophy.archetype);
    manager_reading(w, arch, p).unwrap_or(6.6)
}
