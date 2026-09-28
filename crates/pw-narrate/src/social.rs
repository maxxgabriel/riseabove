//! Rendering social posts and chants.
//!
//! A post is a semantic structure (`pw_world::socialnet::Post`): a concept,
//! a subject, the real frame it reacts to, what the author knew and the
//! earlier posts it refers to. Words come from the author's voice (age band,
//! kind of account, humour, hedging). Every noun in the output is taken from
//! the frame or the post's references; nothing is added that the world does
//! not hold.

use pw_world::World;
use pw_world::socialnet::{AccountKind, Age, Chant, ChantKind, Concept, Frame, Post, SocialAccount};

use crate::fmt::{club_short, person, player};
use crate::lexicon::{AgeBand, Locale, Register, Slot, Voice, emoji, loud, word};
use crate::pick;

/// The voice of an account: its kind, age band and persona, where it
/// lives (vocabulary only) and when it is.
pub fn voice(w: &World, a: &SocialAccount) -> Voice {
    let register = match a.kind {
        AccountKind::Stats => Register::Analytical,
        AccountKind::FanNews | AccountKind::ClubOfficial => Register::Neutral,
        AccountKind::RumourMill => Register::Tabloid,
        AccountKind::Ultra | AccountKind::Hardcore => Register::Terrace,
        AccountKind::Person => Register::Neutral,
        _ => Register::Casual,
    };
    let age = match a.age {
        Age::Teen | Age::Young => AgeBand::Young,
        Age::Middle => AgeBand::Middle,
        Age::Older => AgeBand::Older,
    };
    Voice {
        locale: Locale::En,
        dialect: if a.nation.is_some() { crate::lexicon::Dialect::for_code(&w.nations[a.nation].code) } else { crate::lexicon::Dialect::International },
        era: crate::lexicon::Era::of_year(w.date.year()),
        platform: crate::lexicon::Platform::Social,
        register,
        age,
        emoji: matches!(register, Register::Casual | Register::Terrace) && age == AgeBand::Young,
        humour: a.persona.humour,
        hedging: 100 - a.persona.knowledge,
    }
}

/// What the frame is, in a few words (only facts the frame holds).
fn frame_words(w: &World, v: &Voice, f: Frame, about_club: pw_core::ClubId) -> String {
    match f {
        Frame::Result { uid } => {
            w.recent_matches.by_uid(uid).map_or_else(String::new, |m| format!("{} {} {}", club_short(w, m.home), crate::grammar::score(v, m.hg, m.ag, uid), club_short(w, m.away)))
        }
        Frame::LateWinner { player: p, .. } => format!("{}'s late winner", player(w, p)),
        Frame::HatTrick { player: p, .. } => format!("{}'s hat-trick", player(w, p)),
        Frame::RedCard { player: p, .. } => format!("{}'s red card", player(w, p)),
        Frame::Signing { player: p, club } => format!("{} to {}", player(w, p), club_short(w, club)),
        Frame::Departure { player: p, to, .. } => {
            if to.is_some() {
                format!("{} leaving for {}", player(w, p), club_short(w, to))
            } else {
                format!("{} leaving", player(w, p))
            }
        }
        Frame::TransferRequest { player: p } => format!("{} handing in a transfer request", player(w, p)),
        Frame::Story { story } => crate::press::headline(w, &w.media.stories[story]),
        Frame::Quote { quote } => w.pressroom.quotes.get(quote as usize).map_or_else(String::new, |q| format!("what {} said", person(w, q.speaker))),
        Frame::ManagerSacked { club } => format!("{} sacking the manager", club_short(w, club)),
        Frame::ManagerAppointed { club } => format!("{}'s new manager", club_short(w, club)),
        Frame::Award { player: p } => format!("{}'s award", player(w, p)),
        Frame::Milestone { player: p } => format!("{}'s milestone", player(w, p)),
        Frame::Record { player: p } => format!("{}'s record", player(w, p)),
        Frame::Injury { player: p } => format!("{}'s injury", player(w, p)),
        Frame::Incident { incident } => {
            // A post says the thing, not "that" the thing.
            if w.incidents.get(incident).is_some() {
                let t = crate::incidents::summary(w, incident, false, false);
                t.strip_prefix("that ").map_or(t.clone(), str::to_string)
            } else {
                String::new()
            }
        }
        Frame::Controversy { controversy } => w.officials.controversies.get(controversy as usize).map_or_else(String::new, |c| crate::officiating::call(w, c)),
        Frame::Post { .. } => {
            if about_club.is_some() {
                club_short(w, about_club)
            } else {
                String::new()
            }
        }
    }
}

/// One post, as its author would write it.
pub fn post(w: &World, p: &Post) -> String {
    let Some(a) = w.net.accounts.get(p.author as usize) else { return String::new() };
    let v = voice(w, a);
    let key = u64::from(p.id);
    let subj = if p.about.is_some() { person(w, p.about) } else { String::new() };
    let what = frame_words(w, &v, p.frame, p.club);
    // A manager's football, in the words of someone who watches it.
    let style = {
        let s = if p.about.is_some() { w.people[p.about].staff } else { pw_core::StaffId::NONE };
        s.get().filter(|&s| w.staff[s].role == pw_world::StaffRole::Manager).map(|s| w.staff[s].philosophy)
    };
    let hedge =
        if v.hedging > 60 && matches!(p.claim, pw_world::media::ClaimType::Rumour | pw_world::media::ClaimType::Speculation) { format!("{} ", word(&v, Slot::Hedge, key)) } else { String::new() };
    let target = if subj.is_empty() { what.clone() } else { subj.clone() };
    let s = match p.concept {
        Concept::Praise => match style {
            Some(ph) if key % 2 == 0 => format!("{target} {}: {}", word(&v, Slot::PraiseAdj, key), crate::grammar::style(&v, ph.press, ph.tempo, ph.directness, key)),
            _ => format!("{} {} {}", target, pick(key, &["is", "was", "looked"]), word(&v, Slot::PraiseAdj, key)),
        },
        Concept::ReluctantPraise => format!("{} — {} {}", word(&v, Slot::Concede, key), target, word(&v, Slot::PraiseAdj, key)),
        Concept::ConcedeWrong => format!("{}. {}", word(&v, Slot::Concede, key), target),
        Concept::DoubleDown => format!("{}. {}", target, word(&v, Slot::DoubleDown, key)),
        Concept::Criticise => match style.and_then(|ph| crate::grammar::style_complaint(&v, ph.press, ph.tempo, ph.directness, key)) {
            Some(c) => format!("{target}: {c}"),
            None => format!("{} {} {}", target, pick(key, &["is", "was", "looked"]), word(&v, Slot::CriticAdj, key)),
        },
        Concept::Mock => {
            if a.persona.humour >= 60 || a.persona.hostility >= 60 {
                format!("{} {}", what, crate::grammar::banter(&v, a.persona.humour, a.persona.hostility, key))
            } else {
                format!("{} {}", what, word(&v, Slot::Mock, key))
            }
        }
        Concept::Celebrate => format!("{}! {}", what, word(&v, Slot::Celebrate, key)),
        Concept::Lament => format!("{}. {}", what, word(&v, Slot::Lament, key)),
        Concept::Worry => format!("{} {}", word(&v, Slot::Worry, key), target),
        Concept::CompareLegend => format!("{} {} {}", target, word(&v, Slot::Nostalgia, key), person(w, p.about2)),
        Concept::Relay => format!("{hedge}{what}"),
        Concept::Question => format!("{} {}?", word(&v, Slot::Question, key), target),
        Concept::Defend => format!("{} {}. {}", target, word(&v, Slot::Loyal, key), word(&v, Slot::Patience, key)),
        Concept::Sarcasm => format!("{}. {}", what, word(&v, Slot::Sarcasm, key)),
        Concept::CallOut => callout(w, p, &v, key),
        Concept::Agree => pick(key, &["this", "exactly", "said it better than I could", "100%"]).to_string(),
        Concept::Disagree => pick(key, &["not having that", "completely wrong", "strongly disagree", "no chance"]).to_string(),
        Concept::Chant => w.net.chants.get(p.extra as usize).map_or_else(String::new, |c| chant(w, c)),
        Concept::Meme => w.net.memes.get(p.extra as usize).map_or_else(String::new, |m| meme(w, m)),
        Concept::Statement => {
            if target.is_empty() {
                what
            } else {
                target
            }
        }
    };
    let positive = matches!(p.concept, Concept::Praise | Concept::Celebrate | Concept::ConcedeWrong | Concept::Defend | Concept::ReluctantPraise);
    let s = if matches!(p.concept, Concept::Celebrate | Concept::Mock) && v.register == Register::Terrace { loud(&Voice { register: Register::Tabloid, ..v }, &s) } else { s };
    format!("{s}{}", emoji(&v, positive, key))
}

/// "You wanted them gone two weeks ago" — only when that post exists.
fn callout(w: &World, p: &Post, v: &Voice, key: u64) -> String {
    let Some(&r) = p.refs.first() else { return pick(key, &["that's not what you said before", "funny how that changes"]).to_string() };
    let Some(earlier) = w.net.post(r) else { return String::new() };
    let days = earlier.date.days_until(p.date).max(0);
    let when = match days {
        0 => "this morning".to_string(),
        1 => "yesterday".to_string(),
        2..=13 => format!("{days} days ago"),
        14..=59 => format!("{} weeks ago", days / 7),
        _ => format!("{} months ago", days / 30),
    };
    let who = if earlier.about.is_some() { person(w, earlier.about) } else { "them".to_string() };
    let said = match earlier.concept {
        Concept::Criticise => format!("you called {who} {}", word(v, Slot::CriticAdj, u64::from(earlier.id))),
        Concept::DoubleDown => format!("you said {who} wasn't good enough"),
        Concept::Mock => format!("you were mocking {who}"),
        _ => format!("you said the opposite about {who}"),
    };
    format!("{said} {when}")
}

/// A meme, referred to by what it is built on.
pub fn meme(w: &World, m: &pw_world::socialnet::Meme) -> String {
    use pw_world::socialnet::MemeSource;
    let key = u64::from(m.id);
    match m.source {
        MemeSource::Mistake { player: p, .. } => format!("{} {}", pick(key, &["do a", "pulling a", "classic"]), player(w, p)),
        MemeSource::Hero { player: p, .. } => format!("{} {}", pick(key, &["remember", "never forget", "still thinking about"]), player(w, p)),
        MemeSource::Quote { quote } => w.pressroom.quotes.get(quote as usize).map_or_else(String::new, |q| format!("\"{}\" energy", crate::press::stance_label(q.stance))),
        MemeSource::Post { post } => w.net.post(post).map_or_else(String::new, |p| format!("{} again", pick(key ^ u64::from(p.id), &["this", "here we go", "that post"]))),
        MemeSource::Saga { .. } => pick(key, &["the saga continues", "episode whatever of this"]).to_string(),
    }
}

// ---------------------------------------------------------------------------
// Chants: original structures built from the words of a real moment. No
// existing song's lyrics or melodies are used or referenced.
// ---------------------------------------------------------------------------

/// Lines from a chant's shape and seed, filled with names from the world.
pub fn chant(w: &World, c: &Chant) -> String {
    let name = if c.about.is_some() { surname(w, c.about) } else { String::new() };
    let club = club_short(w, c.club);
    let target = if c.target.is_some() { club_short(w, c.target) } else { String::new() };
    let s = c.seed;
    let adj = pick(s, &["magic", "golden", "mighty", "fearless", "electric", "loyal"]);
    let verb = pick(s >> 8, &["score", "run", "fight", "shine", "win"]);
    let place = pick(s >> 16, &["the terraces", "the town", "the north stand", "our end", "the ground"]);
    let lines: Vec<String> = match (c.kind, c.shape % 3) {
        (ChantKind::PlayerPraise | ChantKind::PlayerName, 0) => vec![format!("{name}, {name}"), format!("the {adj} one from {club}"), format!("watch {name} {verb}")],
        (ChantKind::PlayerPraise | ChantKind::PlayerName, 1) => vec![format!("oh {name}, {name}"), format!("{place} sing your name")],
        (ChantKind::PlayerPraise | ChantKind::PlayerName, _) => vec![format!("{name} will {verb} for {club}"), format!("{name} will {verb} again")],
        (ChantKind::Rivalry, 0) => vec![format!("we are {club}"), format!("{target} fear the {adj} ones")],
        (ChantKind::Rivalry, _) => vec![format!("{name} sank {target}"), format!("and {place} will never forget")],
        (ChantKind::Mocking, _) => vec![format!("{name}, {name}, where did you go?"), format!("gone to {target}, we won't miss you so")],
        (ChantKind::Protest, _) => vec![format!("we want our {club} back"), if name.is_empty() { "the board out".to_string() } else { format!("{name} out") }],
        (ChantKind::Manager, _) => vec![format!("{name}'s {adj} army"), format!("marching on with {club}")],
        (ChantKind::Identity, _) => vec![format!("{adj} {club}"), format!("from {place} to the grave")],
    };
    lines.join(" / ")
}

fn surname(w: &World, p: pw_core::PersonId) -> String {
    let full = person(w, p);
    full.rsplit(' ').next().unwrap_or(&full).to_string()
}

/// A supporter group acting, in a sentence.
pub fn group_action(w: &World, club: pw_core::ClubId, group: u32, action: pw_world::socialnet::GroupAction) -> String {
    use pw_world::socialnet::{GroupAction as A, GroupKind as K};
    let g = w.net.groups.get(group as usize);
    let who = match g.map(|g| g.kind) {
        Some(K::SeasonTicket) => "season-ticket holders",
        Some(K::Online) => "supporters online",
        Some(K::International) => "overseas supporters",
        Some(K::Academy) => "academy followers",
        Some(K::Ultras) => "the ultras",
        Some(K::Numbers) => "the numbers crowd",
        Some(K::Trust) => "the supporters' trust",
        None => "supporters",
    };
    let c = club_short(w, club);
    match action {
        A::Protest => format!("{c} {who} protested against the club's owners."),
        A::Petition => format!("{c} {who} launched a petition against the board."),
        A::Banner => format!("{c} {who} unfurled a banner criticising the manager."),
        A::Applause => format!("{c} {who} gave the team a standing ovation."),
        A::Booing => format!("{c} {who} booed the team off."),
        A::Campaign => format!("{c} {who} started a campaign."),
        A::Tribute => format!("{c} {who} paid tribute."),
    }
}
