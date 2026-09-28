//! Headlines, articles and fan reactions. The claim strength and the outlet's
//! style shape the words; what the story rests on is always a real source.

use pw_world::event::{MilestoneKind, RecordKind};
use pw_world::media::{OutletKind, Reaction, Stance, Story, StoryLink};
use pw_world::perf::Label;
use pw_world::{FanReason, StoryKind, World};

use crate::fmt::{club, club_short, money, person, player};
use crate::pick;

fn subject(w: &World, s: &Story) -> String {
    if s.player.is_some() { player(w, s.player) } else if s.person.is_some() { person(w, s.person) } else { club(w, s.club) }
}

fn tabloid(w: &World, s: &Story) -> bool {
    s.outlet.is_some() && matches!(w.media.outlets[s.outlet].kind, OutletKind::Tabloid | OutletKind::FanChannel)
}

pub fn outlet_name(w: &World, s: &Story) -> String {
    if s.outlet.is_some() { w.media.outlets[s.outlet].name.clone() } else { "The press".into() }
}

pub fn headline(w: &World, s: &Story) -> String {
    let key = u64::from(s.id.0);
    let who = subject(w, s);
    let loud = tabloid(w, s);
    match s.kind {
        StoryKind::TransferRumour => {
            let verb = match s.claim {
                0..=29 => pick(key, &["monitoring", "keeping tabs on", "aware of"]),
                30..=54 => pick(key, &["keen on", "tracking", "interested in"]),
                55..=74 => pick(key, &["preparing a move for", "lining up", "ready to test the waters for"]),
                _ => pick(key, &["set to bid for", "closing in on", "poised to swoop for"]),
            };
            let fee = if s.fee > 0 && s.claim >= 55 { format!(" in {} deal", money(s.fee)) } else { String::new() };
            let base = format!("{} {verb} {who}{fee}", club(w, s.other_club), );
            if loud { format!("{}!", base.to_uppercase()) } else { base }
        }
        StoryKind::TransferNews => format!("{who} completes move to {}", club(w, s.club)),
        StoryKind::ManagerPressure => format!("{}: {}", club(w, s.club), pick(key, &["board patience wearing thin", "pressure mounts on the manager", "crisis talks expected"])),
        StoryKind::ManagerChange => format!("{}: {}", club(w, s.club), pick(key, &["change in the dugout", "new era begins", "manager news"])),
        StoryKind::Unhappy => {
            if loud { format!("{who} WANTS OUT") } else { format!("{who} {}", pick(key, &["unsettled at", "frustrated at", "considering their future at"])) + &format!(" {}", club_short(w, s.club)) }
        }
        StoryKind::Discipline => format!("{who} {}", pick(key, &["disciplined by club", "in hot water", "fined after internal row"])),
        StoryKind::Injury => format!("Blow for {}: {who} {}", club_short(w, s.club), pick(key, &["faces spell out", "sidelined for weeks", "out injured"])),
        StoryKind::Praise => format!("{who}: {}", pick(key, &["the talk of the league", "a star in the making", "in the form of their life", "one to watch"])),
        StoryKind::Criticism => format!("{who} {}", pick(key, &["under fire after poor run", "must improve, say critics", "struggling for form"])),
        StoryKind::Contract => format!("{who} contract talks {}", pick(key, &["stall", "continue", "hit a snag"])),
        StoryKind::Personal => format!("{who} {}", pick(key, &["ties the knot", "celebrates at home", "happy off the pitch"])),
        StoryKind::Season => format!("{} {}", club(w, s.club), pick(key, &["are champions", "lift the title", "crowned champions"])),
        StoryKind::Interview => match w.media.links.get(&s.id) {
            Some(StoryLink::Quote(q)) => {
                let speaker = person(w, q.speaker);
                let target = if q.about.is_some() { person(w, q.about) } else { String::new() };
                match q.stance {
                    Stance::Praise => format!("{speaker} hails {target}"),
                    Stance::Criticise => if loud { format!("{speaker} SLAMS {}", target.to_uppercase()) } else { format!("{speaker} criticises {target}") },
                    Stance::Support => format!("{speaker} backs {target}"),
                    Stance::Deny => format!("{speaker} denies the reports"),
                    Stance::Complain => format!("{speaker} speaks out over role"),
                    Stance::Ambition => format!("{speaker}: \"I want to win the biggest trophies\""),
                    Stance::Loyalty => format!("{speaker}: \"I am happy here\""),
                    Stance::Deflect => format!("{speaker} gives little away"),
                }
            }
            _ => format!("{who} speaks"),
        },
        StoryKind::MatchReport => match w.media.links.get(&s.id) {
            Some(StoryLink::Fixture { home, away, hg, ag, star, .. }) => {
                let base = format!("{} {hg}-{ag} {}", club_short(w, *home), club_short(w, *away));
                if star.is_some() { format!("{base}: {} {}", player(w, *star), pick(key, &["stars", "shines", "makes the difference", "steals the show"])) } else { base }
            }
            _ => format!("Report: {}", club(w, s.club)),
        },
        StoryKind::Feature | StoryKind::Analysis => match w.media.links.get(&s.id) {
            Some(StoryLink::Reading(l)) => reading_headline(&who, *l, key),
            _ => format!("The rise of {who}"),
        },
        StoryKind::WonderkidList => format!("{}: the {} young players to watch", pick(key, &["Ranked", "Revealed", "The list"]), match w.media.links.get(&s.id) {
            Some(StoryLink::List(v)) => v.len(),
            _ => 10,
        }),
        StoryKind::SeasonReview => format!("Season review: {} on top", club(w, s.club)),
        StoryKind::Retrospective => match w.media.links.get(&s.id) {
            Some(StoryLink::Honour(i)) => {
                let h = &w.history.honours[*i as usize];
                format!("{} years on: when {} won the {}", w.date.year() - h.season, club(w, h.club), w.comps[h.comp].name)
            }
            _ => format!("{who}: a career remembered"),
        },
        StoryKind::AwardNews => match w.media.links.get(&s.id) {
            Some(StoryLink::Award(a)) => format!("{who} wins {}", a.label()),
            _ => format!("Honour for {who}"),
        },
        StoryKind::International => match w.media.links.get(&s.id) {
            Some(StoryLink::Tournament(t)) => {
                let t = &w.intl.tournaments[*t as usize];
                format!("{} win the {} {}", crate::fmt::nation(w, t.winner), t.kind.label(), t.year)
            }
            _ => "International football".into(),
        },
        StoryKind::Leak => format!("{}: {}", club_short(w, s.club), pick(key, &["trouble behind closed doors", "all is not well", "revealed: the story inside the club"])),
        StoryKind::IncidentNews => match w.grapevine.items.get(s.info as usize) {
            Some(it) => match it.kind {
                pw_world::info::InfoKind::Incident { incident } => {
                    let t = crate::incidents::summary(w, incident, false, loud);
                    let t = t.strip_prefix("that ").unwrap_or(&t).to_string();
                    crate::lexicon::loud(&voice(w, s), &t)
                }
                _ => format!("Incident at {}", club_short(w, s.club)),
            },
            None => format!("Incident at {}", club_short(w, s.club)),
        },
        StoryKind::Denial => format!("{who}: {}", pick(key, &["reports denied", "\"no truth\" in the story", "camp plays down speculation"])),
        StoryKind::Correction => format!("Correction: {who}"),
        StoryKind::FanReaction => format!("{} fans {}", club_short(w, s.club), pick(key, &["react", "have their say", "are divided"])),
        StoryKind::Milestone => match w.media.links.get(&s.id) {
            Some(StoryLink::Milestone(k, n)) => match k {
                MilestoneKind::ClubApps => format!("{who} reaches {n} games for {}", club_short(w, s.club)),
                MilestoneKind::CareerGoals => format!("{n} and counting for {who}"),
                MilestoneKind::SeniorApps => format!("{who} brings up {n} appearances"),
                MilestoneKind::Caps => format!("{who} wins cap number {n}"),
            },
            Some(StoryLink::Record(k, v)) => match k {
                RecordKind::ClubTopScorer => format!("{who} becomes {}'s record scorer", club_short(w, s.club)),
                RecordKind::ClubMostApps => format!("{who} breaks {} appearance record", club_short(w, s.club)),
                RecordKind::ClubRecordSigning => format!("{who} becomes {} record signing", club_short(w, s.club)),
                RecordKind::ClubRecordSale => format!("{} record sale: {who}", club_short(w, s.club)),
                RecordKind::ClubBiggestWin => format!("Record win for {}", club_short(w, s.club)),
                RecordKind::LeagueGoalsInSeason => format!("{who} breaks the scoring record ({v})"),
                RecordKind::NationMostCaps => format!("{who} becomes most-capped ever"),
                RecordKind::NationTopScorer => format!("{who} becomes national record scorer"),
                RecordKind::WorldRecordFee => format!("World record: {who} for {}", money(*v)),
            },
            _ => format!("Landmark for {who}"),
        },
    }
}

fn reading_headline(who: &str, l: Label, key: u64) -> String {
    match l {
        Label::BigGamePlayer => format!("{who}: {}", pick(key, &["the player for the big occasions", "made for the biggest nights"])),
        Label::FlatTrackBully => format!("Does {who} disappear when it matters?"),
        Label::InForm => format!("{who} {}", pick(key, &["in the form of their life", "cannot stop performing"])),
        Label::InSlump => format!("What has happened to {who}?"),
        Label::Underrated => format!("The numbers say {who} is {}", pick(key, &["underrated", "far better than people think"])),
        Label::Overrated => format!("Is {who} overrated? The data suggests so"),
        Label::GoalThreat => format!("{who}: {}", pick(key, &["goals, goals, goals", "the most dangerous forward around"])),
        Label::Workhorse => format!("{who}, the engine nobody notices"),
        Label::Unreliable => format!("Brilliant or baffling: the two sides of {who}"),
        Label::Breakthrough => format!("{who}: {}", pick(key, &["the breakthrough of the season", "a star is born"])),
        Label::FrozenOut => format!("Why has {who} disappeared from the side?"),
        Label::Durable => format!("{who}, never injured, always there"),
        Label::InjuryProne => format!("The body keeps failing {who}"),
    }
}

/// The voice an outlet writes in.
pub fn voice(w: &World, s: &Story) -> crate::lexicon::Voice {
    use crate::lexicon::{AgeBand, Locale, Register, Voice};
    let register = if s.outlet.is_some() {
        match w.media.outlets[s.outlet].kind {
            OutletKind::Tabloid => Register::Tabloid,
            OutletKind::FanChannel => Register::Casual,
            OutletKind::DataSite => Register::Analytical,
            OutletKind::National => Register::Formal,
            _ => Register::Neutral,
        }
    } else {
        Register::Neutral
    };
    Voice { locale: Locale::En, register, age: AgeBand::Middle, emoji: false, humour: 20, hedging: 40 }
}

/// A short article body: who says it, what it rests on (as the paper frames it).
pub fn body(w: &World, s: &Story) -> String {
    let key = u64::from(s.id.0) ^ 0xabcd;
    let outlet = outlet_name(w, s);
    let who = subject(w, s);
    let source = if s.leaker.is_some() {
        pick(key, &["a source close to the club", "people familiar with the situation", "sources inside the club", "a well-placed insider"])
    } else {
        pick(key, &["club sources", "those at the club", "observers"])
    };
    match s.kind {
        StoryKind::TransferRumour => format!(
            "{outlet} understands {} have been watching {who} closely, according to {source}. {}",
            club(w, s.other_club),
            if s.claim >= 70 { "A formal approach could come soon." } else { "No formal approach has been made." }
        ),
        StoryKind::Unhappy => format!("{who} is understood to be unhappy, {outlet} has been told by {source}."),
        StoryKind::Discipline => format!("{who} has been disciplined internally, according to {source}."),
        StoryKind::Criticism => format!("{who}'s recent performances have drawn criticism, with {outlet} questioning their place in the side."),
        StoryKind::Interview => match w.media.links.get(&s.id) {
            Some(StoryLink::Quote(q)) => format!("{} spoke to {outlet}: \"{}\"", person(w, q.speaker), quote_line(w, q.about, q.stance, key)),
            _ => headline(w, s),
        },
        StoryKind::WonderkidList => match w.media.links.get(&s.id) {
            Some(StoryLink::List(v)) => {
                let names: Vec<String> = v.iter().enumerate().map(|(i, &p)| format!("{}. {}", i + 1, player(w, p))).collect();
                format!("{outlet} ranks the best young players: {}", names.join("; "))
            }
            _ => headline(w, s),
        },
        StoryKind::SeasonReview => match w.media.links.get(&s.id) {
            Some(StoryLink::List(v)) if !v.is_empty() => {
                let names: Vec<String> = v.iter().map(|&p| player(w, p)).collect();
                format!("{} are champions. {outlet}'s team of the season: {}.", club(w, s.club), names.join(", "))
            }
            _ => headline(w, s),
        },
        StoryKind::Praise => format!("{who} continues to impress, {outlet} reports."),
        _ => headline(w, s),
    }
}

/// What a stance sounds like in someone's mouth.
fn quote_line(w: &World, about: pw_core::PersonId, stance: Stance, key: u64) -> String {
    let name = if about.is_some() { person(w, about) } else { String::new() };
    match stance {
        Stance::Praise => format!("{name} {}", pick(key, &["has been outstanding.", "deserves everything coming their way.", "is a joy to work with."])),
        Stance::Criticise => format!("{name} {}", pick(key, &["has to do much better.", "knows it has not been good enough.", "needs to look at themselves."])),
        Stance::Support => format!("{name} {}", pick(key, &["has my full support.", "will come good, I have no doubt.", "is going through a spell every player has."])),
        Stance::Deny => pick(key, &["There is no truth in it.", "I have heard nothing about that.", "It is not something we recognise."]).to_string(),
        Stance::Complain => pick(key, &["I want to play. I am not here to sit and watch.", "Nobody has explained my role to me.", "I deserve more minutes than I am getting."]).to_string(),
        Stance::Ambition => pick(key, &["I want to play at the very highest level.", "Every player dreams of the biggest stage.", "I have ambitions I need to fulfil."]).to_string(),
        Stance::Loyalty => pick(key, &["I am happy here. This club is my home.", "I have no intention of leaving.", "The fans know how much this club means to me."]).to_string(),
        Stance::Deflect => pick(key, &["We take it game by game.", "I will not comment on that.", "The focus is on the next match."]).to_string(),
    }
}

/// A fan's post, voiced from the reaction's sentiment and reason.
pub fn reaction(w: &World, r: &Reaction) -> String {
    let key = u64::from(r.event.0) ^ u64::from(r.about.0) << 8;
    let name = person(w, r.about);
    let fans = if r.club.is_some() { format!("{} fans", club_short(w, r.club)) } else { "Fans".into() };
    let line = match (r.reason, r.sentiment) {
        (FanReason::JoinedRival, _) => pick(key, &["Never forget what they did.", "Traitor. Simple as.", "Don't bother coming back."]),
        (FanReason::TransferRequest, _) => pick(key, &["Wants out? Let them go.", "After everything we gave them.", "Disappointing. Thought they were one of us."]),
        (FanReason::Leaving, s) if s < 0 => pick(key, &["Sad to see this.", "Worried about this one.", "Don't let them go."]),
        (FanReason::Loyalty, _) => pick(key, &["Thank you for everything.", "A proper servant of this club.", "Legend."]),
        (FanReason::Performances, s) if s > 20 => pick(key, &["What a signing.", "Get in!", "Can't wait to see them play."]),
        (FanReason::Performances, s) if s < -10 => pick(key, &["Not good enough.", "Time for a change.", "Poor again."]),
        (_, s) if s >= 0 => pick(key, &["Good news.", "Happy with that.", "Fair enough."]),
        _ => pick(key, &["Not having it.", "Poor show.", "Unacceptable."]),
    };
    format!("[{fans}, {}] on {name}: \"{line}\"", volume(r.volume))
}

fn volume(v: u8) -> &'static str {
    match v {
        0..=3 => "a few voices",
        4..=9 => "many",
        _ => "trending",
    }
}

/// A stance as an answer option.
pub fn stance_label(s: Stance) -> &'static str {
    match s {
        Stance::Praise => "Praise",
        Stance::Criticise => "Criticise",
        Stance::Deflect => "Deflect",
        Stance::Ambition => "Talk up ambitions",
        Stance::Loyalty => "Declare loyalty",
        Stance::Complain => "Complain",
        Stance::Support => "Back them",
        Stance::Deny => "Deny it",
    }
}

/// A press-conference question as the journalist asks it.
pub fn question(w: &World, conference: u32, q: u8) -> String {
    use pw_world::pressroom::QTopic;
    let Some(c) = w.pressroom.conferences.get(conference as usize) else { return "A question.".into() };
    let Some(q) = c.questions.get(usize::from(q)) else { return "A question.".into() };
    let who = person(w, q.journalist);
    let outlet = w.media.journalists.get(&q.journalist).map_or("the press".to_string(), |j| w.media.outlets[j.outlet].name.clone());
    let lead = if q.follow_up { "Just to press you on that — " } else { "" };
    let body = match q.topic {
        QTopic::LastMatch { uid } => match w.recent_matches.by_uid(uid) {
            Some(m) => format!("what did you make of the {}-{} against {}?", m.hg, m.ag, club_short(w, if m.home == c.club { m.away } else { m.home })),
            None => "what did you make of the last game?".into(),
        },
        QTopic::Transfer { story, player: p } => format!("{} report {} — is {} leaving?", outlet_of(w, story), headline(w, &w.media.stories[story]).to_lowercase(), player(w, p)),
        QTopic::Trouble { story } => format!("there are reports that {} — what's going on?", headline(w, &w.media.stories[story]).to_lowercase()),
        QTopic::Injury { player: p } => format!("how long will {} be out?", player(w, p)),
        QTopic::Pressure => "after this run of results, are you worried about your job?".into(),
        QTopic::Rival { club: o } => format!("how big is this one against {}?", club(w, o)),
        QTopic::EarlierQuote { quote } => match w.pressroom.quotes.get(quote as usize) {
            Some(r) => format!("a few weeks ago you said {} — do you stand by that?", quote_line(w, r.about, r.stance, u64::from(quote)).trim_end_matches('.').to_lowercase()),
            None => "do you stand by what you said before?".into(),
        },
        QTopic::Selection { player: p } => format!("what's the latest on {}'s place in the side?", player(w, p)),
    };
    format!("{who} ({outlet}): {lead}{body}")
}

fn outlet_of(w: &World, s: pw_core::StoryId) -> String {
    let st = &w.media.stories[s];
    if st.outlet.is_some() { w.media.outlets[st.outlet].name.clone() } else { "Reports".into() }
}
