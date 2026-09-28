//! Extract the facts of a senior match once it is played (goals and their
//! minutes, late winners, comebacks, hat-tricks, red cards, the best
//! player, debut goals, what the fixture meant). Everything downstream —
//! reports, supporter reactions, awards, records — reads these facts rather
//! than the raw engine output.

use pw_match::{Ev, MatchResult};
use pw_world::matchfacts::{Goal, MatchFacts};
use pw_world::{Fixture, TeamKind, World};
use smallvec::SmallVec;

pub fn record(w: &mut World, fx: &Fixture, fixture: pw_core::FixtureId, r: &MatchResult) {
    if w.teams[fx.home].kind != TeamKind::First || w.teams[fx.away].kind != TeamKind::First {
        return;
    }
    let mut goals: SmallVec<[Goal; 8]> = r
        .events
        .iter()
        .filter(|e| matches!(e.kind, Ev::Goal | Ev::PenaltyGoal | Ev::OwnGoal))
        .map(|e| Goal {
            player: e.player,
            assist: if e.kind == Ev::Goal { e.other } else { pw_core::PlayerId::NONE },
            minute: (e.t / 60).min(130) as u8,
            side: e.side,
            penalty: e.kind == Ev::PenaltyGoal,
            own_goal: e.kind == Ev::OwnGoal,
        })
        .collect();
    goals.sort_by_key(|g| g.minute);
    // Walk the score: comebacks and late winners.
    let result = i32::from(r.home_goals).cmp(&i32::from(r.away_goals));
    let winner_side: Option<u8> = match result {
        std::cmp::Ordering::Greater => Some(0),
        std::cmp::Ordering::Less => Some(1),
        std::cmp::Ordering::Equal => None,
    };
    let (mut h, mut a) = (0i32, 0i32);
    let mut trailed = false;
    let mut late_winner = None;
    for g in &goals {
        let before_level = h == a;
        // An own goal counts for the other side; `side` is the scorer's side.
        let for_home = (g.side == 0) != g.own_goal;
        if for_home {
            h += 1
        } else {
            a += 1
        }
        if let Some(ws) = winner_side {
            let winner_behind = if ws == 0 { h < a } else { a < h };
            if winner_behind {
                trailed = true;
            }
            let scored_by_winner = (ws == 0) == for_home;
            if before_level && scored_by_winner && g.minute >= 85 && (h - a).abs() == 1 {
                late_winner = Some(*g);
            }
        }
    }
    // A late goal is only the winner if the match ended one apart.
    if (i32::from(r.home_goals) - i32::from(r.away_goals)).abs() != 1 {
        late_winner = None;
    }
    let mut counts: SmallVec<[(pw_core::PlayerId, u8); 8]> = SmallVec::new();
    for g in goals.iter().filter(|g| !g.own_goal) {
        if let Some(c) = counts.iter_mut().find(|c| c.0 == g.player) {
            c.1 += 1;
        } else {
            counts.push((g.player, 1));
        }
    }
    let hat_tricks: SmallVec<[pw_core::PlayerId; 1]> = counts.iter().filter(|c| c.1 >= 3).map(|c| c.0).collect();
    let reds: SmallVec<[(pw_core::PlayerId, u8); 2]> = r.events.iter().filter(|e| matches!(e.kind, Ev::Red | Ev::SecondYellow)).map(|e| (e.player, (e.t / 60).min(130) as u8)).collect();
    let pom_rating = r.line(r.pom).map_or(0, |l| (l.rating * 10.0).round().clamp(0.0, 100.0) as u8);
    let debut_goals: SmallVec<[pw_core::PlayerId; 2]> = counts.iter().filter(|c| w.players.cold[c.0].senior_apps == 1).map(|c| c.0).collect();
    let meaning = crate::culture::meaning(w, fx);
    let facts = MatchFacts {
        uid: fx.uid,
        fixture,
        date: w.date,
        comp: fx.comp,
        home_team: fx.home,
        away_team: fx.away,
        home: w.teams[fx.home].club,
        away: w.teams[fx.away].club,
        hg: r.home_goals,
        ag: r.away_goals,
        pens: r.pens,
        goals,
        reds,
        late_winner,
        hat_tricks,
        comeback: trailed,
        pom: r.pom,
        pom_rating,
        debut_goals,
        significance: meaning.significance,
        derby: meaning.derby,
    };
    w.recent_matches.push(facts);
}
