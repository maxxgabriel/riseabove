//! Pre-match press conferences (M in the brief).
//!
//! The day before a big club plays, its manager meets the journalists who
//! cover it. Each journalist asks about something *they* can know: the last
//! result, a transfer story or a row that has been reported, a public injury,
//! a poor run, the next opponent, something the manager said before, a
//! player's form. Answers are stances from the manager's own mind (media
//! style, trust in the player, what is true, what the club wants) — or a
//! human's choice. Evasive answers to hard questions invite a follow-up.
//! Every answer is a quote on the record with its effects (`press::speak`).

use pw_core::rng::{period, stream};
use pw_core::{ClubId, Hidden, PersonId};
use pw_world::careers::MediaStyle;
use pw_world::decision::{Choice, Decision, DecisionKind, MindKind};
use pw_world::media::{Stance, StoryKind};
use pw_world::pressroom::{Conference, QTopic, Question};
use pw_world::World;

use crate::consider;
use crate::media::big_enough;

/// Daily: conferences for big clubs that play tomorrow.
pub fn daily(w: &mut World) {
    let today = w.date;
    let tomorrow: Vec<pw_core::FixtureId> = w.fixtures.on(today.add_days(1)).to_vec();
    let mut clubs: Vec<(ClubId, ClubId)> = Vec::new();
    for f in tomorrow {
        let fx = w.fixtures.get(f);
        for (t, o) in [(fx.home, fx.away), (fx.away, fx.home)] {
            if w.teams[t].kind == pw_world::TeamKind::First {
                let c = w.teams[t].club;
                if big_enough(w, c) {
                    clubs.push((c, w.teams[o].club));
                }
            }
        }
    }
    clubs.sort();
    clubs.dedup();
    for (club, opponent) in clubs {
        hold(w, club, opponent);
    }
}

fn hold(w: &mut World, club: ClubId, opponent: ClubId) {
    let today = w.date;
    let Some(m) = w.clubs[club].manager.get() else { return };
    let speaker = w.staff[m].person;
    let nation = w.clubs[club].nation;
    let mut js: Vec<PersonId> = w
        .media
        .journalists
        .values()
        .filter(|j| j.outlet.is_some() && w.media.outlets[j.outlet].nation == nation && (j.beat.contains(&club) || w.media.outlets[j.outlet].leaning == club))
        .map(|j| j.person)
        .collect();
    js.sort();
    js.truncate(3);
    if js.is_empty() {
        return;
    }
    let mut questions: Vec<Question> = Vec::new();
    for &j in &js {
        if let Some(q) = ask(w, j, club, opponent, speaker, &questions) {
            questions.push(q);
        }
    }
    if questions.is_empty() {
        return;
    }
    let id = w.pressroom.conferences.len() as u32;
    let n = questions.len();
    w.pressroom.conferences.push(Conference { id, date: today, club, speaker, questions, answers: vec![None; n] });
    for qi in 0..n {
        answer_or_ask(w, id, qi);
    }
}

/// What this journalist can and wants to ask about.
fn ask(w: &World, j: PersonId, club: ClubId, opponent: ClubId, speaker: PersonId, asked: &[Question]) -> Option<Question> {
    let today = w.date;
    let taken = |t: &QTopic| asked.iter().any(|q| q.topic == *t);
    let mut options: Vec<(f32, QTopic, PersonId)> = Vec::new();
    // Published stories about the club's people (last fortnight).
    for s in w.media.stories.iter().rev().take(600) {
        if s.date.days_until(today) > 14 || s.club != club {
            continue;
        }
        let t = match s.kind {
            StoryKind::TransferRumour if s.player.is_some() => QTopic::Transfer { story: s.id, player: s.player },
            StoryKind::Leak | StoryKind::IncidentNews | StoryKind::Discipline | StoryKind::Unhappy => QTopic::Trouble { story: s.id },
            _ => continue,
        };
        if !taken(&t) {
            options.push((0.6 + f32::from(s.news) / 200.0, t, s.person));
        }
    }
    if let Some(m) = w.recent_matches.of_club(club).next_back() {
        let t = QTopic::LastMatch { uid: m.uid };
        if !taken(&t) {
            options.push((0.4 + f32::from(m.significance) / 200.0, t, PersonId::NONE));
        }
    }
    let defeats = w.recent_matches.of_club(club).rev().take(5).filter(|m| m.winner().is_some_and(|x| x != club)).count();
    if defeats >= 3 && !taken(&QTopic::Pressure) {
        options.push((0.5 + defeats as f32 * 0.1, QTopic::Pressure, speaker));
    }
    let rivalry = w.media.rivalry(club, opponent);
    if rivalry >= 40 {
        let t = QTopic::Rival { club: opponent };
        if !taken(&t) {
            options.push((f32::from(rivalry) / 100.0, t, PersonId::NONE));
        }
    }
    // Public injuries to important players.
    let first = w.clubs[club].first_team();
    for &p in &w.teams[first].squad {
        if w.players.hot[p].injury_days > 7 && matches!(w.players.cold[p].status, pw_world::SquadStatus::Star | pw_world::SquadStatus::Important) {
            let t = QTopic::Injury { player: p };
            if !taken(&t) {
                options.push((0.45, t, w.players.cold[p].person));
            }
        }
    }
    // Something the manager said in the last two months.
    if let Some(q) = w.pressroom.quotes_by(speaker).rev().find(|q| q.date.days_until(today) <= 60 && q.date.days_until(today) >= 7) {
        let t = QTopic::EarlierQuote { quote: q.id };
        if !taken(&t) {
            options.push((0.35, t, q.about));
        }
    }
    let mut best: Option<(f32, QTopic, PersonId)> = None;
    for (score, t, about) in options {
        let n = w.roll(stream::PRESS, &[u64::from(j.0), period::day(today), score.to_bits() as u64]) * 0.3;
        if best.is_none_or(|b| score + n > b.0) {
            best = Some((score + n, t, about));
        }
    }
    best.map(|(_, topic, about)| Question { journalist: j, topic, about, follow_up: false })
}

/// The speaker's own answer, or a decision for a human.
fn answer_or_ask(w: &mut World, conf: u32, qi: usize) {
    let today = w.date;
    let (speaker, q) = {
        let c = &w.pressroom.conferences[conf as usize];
        (c.speaker, c.questions[qi])
    };
    let options = stances(q.topic);
    if w.people[speaker].mind == MindKind::External {
        let default = options.iter().position(|&s| s == ai_stance(w, speaker, &q)).unwrap_or(0) as u8;
        w.decisions.push(Decision {
            person: speaker,
            player: w.people[speaker].player,
            kind: DecisionKind::PressQuestion { conference: conf, question: qi as u8 },
            options: options.iter().map(|&s| Choice::Say(s)).collect(),
            created: today,
            // A human gets until kick-off day to answer.
            deadline: today.add_days(1),
            default,
            answer: None,
            resolved: false,
        });
        return;
    }
    let s = ai_stance(w, speaker, &q);
    answer(w, conf, qi, s);
}

/// The stances that make sense for a question.
fn stances(t: QTopic) -> smallvec::SmallVec<[Stance; 5]> {
    use Stance::*;
    let v: &[Stance] = match t {
        QTopic::Transfer { .. } => &[Deflect, Deny, Loyalty, Support, Ambition],
        QTopic::Trouble { .. } => &[Deflect, Deny, Support, Criticise],
        QTopic::Injury { .. } => &[Deflect, Support],
        QTopic::Pressure => &[Deflect, Complain, Support, Criticise],
        QTopic::Rival { .. } => &[Deflect, Praise, Criticise],
        QTopic::EarlierQuote { .. } => &[Deflect, Support, Criticise, Deny],
        QTopic::LastMatch { .. } | QTopic::Selection { .. } => &[Praise, Criticise, Deflect, Support],
    };
    v.iter().copied().collect()
}

/// How a manager answers, from who they are and what is true.
fn ai_stance(w: &World, speaker: PersonId, q: &Question) -> Stance {
    let s = w.people[speaker].staff;
    let style = if s.is_some() { w.careers.managers.get(&s).map(|p| p.media_style) } else { None };
    let temper = 1.0 - consider::hid(w, speaker, Hidden::Temperament) / 20.0;
    let trust = if q.about.is_some() { consider::trust(w, speaker, q.about) } else { 0.5 };
    match q.topic {
        QTopic::Transfer { story, player } => {
            let grounded = w.media.stories[story].grounded;
            let wants_out = w.market.requests.contains_key(&player) || w.market.listed.contains_key(&player);
            match style {
                Some(MediaStyle::Candid) if wants_out => Stance::Ambition,
                _ if !grounded => Stance::Deny,
                Some(MediaStyle::Charming) | Some(MediaStyle::Candid) if trust > 0.55 => Stance::Loyalty,
                _ => Stance::Deflect,
            }
        }
        QTopic::Trouble { .. } => match style {
            Some(MediaStyle::Combative) if temper > 0.6 => Stance::Criticise,
            Some(MediaStyle::Guarded) => Stance::Deny,
            _ if trust > 0.6 => Stance::Support,
            _ => Stance::Deflect,
        },
        QTopic::Pressure => match style {
            Some(MediaStyle::Combative) => Stance::Complain,
            Some(MediaStyle::Candid) => Stance::Criticise,
            _ => Stance::Deflect,
        },
        QTopic::Rival { .. } => {
            if matches!(style, Some(MediaStyle::Combative)) && temper > 0.5 { Stance::Criticise } else if matches!(style, Some(MediaStyle::Charming)) { Stance::Praise } else { Stance::Deflect }
        }
        QTopic::Injury { .. } => if trust > 0.5 { Stance::Support } else { Stance::Deflect },
        QTopic::EarlierQuote { quote } => {
            let prev = w.pressroom.quotes.get(quote as usize).map(|q| q.stance);
            match prev {
                Some(Stance::Criticise) if trust < 0.4 => Stance::Criticise,
                Some(Stance::Criticise) => Stance::Support,
                _ => Stance::Deflect,
            }
        }
        QTopic::LastMatch { .. } | QTopic::Selection { .. } => {
            if trust > 0.6 { Stance::Praise } else if temper > 0.7 && trust < 0.35 { Stance::Criticise } else { Stance::Deflect }
        }
    }
}

/// Record an answer (AI or human) and let it land.
pub fn answer(w: &mut World, conf: u32, qi: usize, stance: Stance) {
    let (speaker, q) = {
        let c = &w.pressroom.conferences[conf as usize];
        (c.speaker, c.questions[qi])
    };
    if w.pressroom.conferences[conf as usize].answers[qi].is_some() {
        return;
    }
    let Some((_, quote)) = crate::press::speak(w, speaker, q.about, stance) else { return };
    if let Some(r) = w.pressroom.quotes.get_mut(quote as usize) {
        r.topic = Some(q.topic);
        r.conference = conf;
    }
    w.pressroom.conferences[conf as usize].answers[qi] = Some(quote);
    // An evasive answer to a hard question invites a follow-up.
    let hard = matches!(q.topic, QTopic::Trouble { .. } | QTopic::Transfer { .. } | QTopic::Pressure);
    let pushy = w.media.journalist_profiles.get(&q.journalist).map_or(0.5, |p| f32::from(p.risk) / 100.0);
    if !q.follow_up && hard && matches!(stance, Stance::Deflect) && w.roll(stream::PRESS, &[u64::from(conf), qi as u64, 0xf0]) < pushy * 0.6 {
        let fq = Question { journalist: q.journalist, topic: q.topic, about: q.about, follow_up: true };
        let c = &mut w.pressroom.conferences[conf as usize];
        c.questions.push(fq);
        c.answers.push(None);
        let idx = c.questions.len() - 1;
        answer_or_ask(w, conf, idx);
    }
}

/// A human's answer from a decision.
pub fn decide(w: &mut World, conf: u32, question: u8, choice: Choice) {
    if let Choice::Say(s) = choice {
        answer(w, conf, usize::from(question), s);
    }
}

