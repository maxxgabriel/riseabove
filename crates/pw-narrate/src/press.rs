//! Headlines, articles and fan reactions. The claim strength and the outlet's
//! style shape the words; what the story rests on is always a real source.

use pw_world::media::{OutletKind, Reaction, Story};
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
            if loud { format!("{who} WANTS OUT") } else { format!("{who} {}", pick(key, &["unsettled at", "frustrated at", "considering his future at"])) + &format!(" {}", club_short(w, s.club)) }
        }
        StoryKind::Discipline => format!("{who} {}", pick(key, &["disciplined by club", "in hot water", "fined after internal row"])),
        StoryKind::Injury => format!("Blow for {}: {who} {}", club_short(w, s.club), pick(key, &["faces spell out", "sidelined for weeks", "out injured"])),
        StoryKind::Praise => format!("{who}: {}", pick(key, &["the talk of the league", "a star in the making", "form of his life", "one to watch"])),
        StoryKind::Criticism => format!("{who} {}", pick(key, &["under fire after poor run", "must improve, say critics", "struggling for form"])),
        StoryKind::Contract => format!("{who} contract talks {}", pick(key, &["stall", "continue", "hit a snag"])),
        StoryKind::Personal => format!("{who} {}", pick(key, &["ties the knot", "celebrates at home", "happy off the pitch"])),
        StoryKind::Season => format!("{} {}", club(w, s.club), pick(key, &["are champions", "lift the title", "crowned champions"])),
    }
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
        StoryKind::Criticism => format!("{who}'s recent performances have drawn criticism, with {outlet} questioning his place in the side."),
        StoryKind::Praise => format!("{who} continues to impress, {outlet} reports."),
        _ => headline(w, s),
    }
}

/// A fan's post, voiced from the reaction's sentiment and reason.
pub fn reaction(w: &World, r: &Reaction) -> String {
    let key = u64::from(r.event.0) ^ u64::from(r.about.0) << 8;
    let name = person(w, r.about);
    let fans = if r.club.is_some() { format!("{} fans", club_short(w, r.club)) } else { "Fans".into() };
    let line = match (r.reason, r.sentiment) {
        (FanReason::JoinedRival, _) => pick(key, &["Never forget what he did.", "Traitor. Simple as.", "Don't bother coming back."]),
        (FanReason::TransferRequest, _) => pick(key, &["Wants out? Let him go.", "After everything we gave him.", "Disappointing. Thought he was one of us."]),
        (FanReason::Leaving, s) if s < 0 => pick(key, &["Sad to see this.", "Worried about this one.", "Don't let him go."]),
        (FanReason::Loyalty, _) => pick(key, &["Thank you for everything.", "A proper servant of this club.", "Legend."]),
        (FanReason::Performances, s) if s > 20 => pick(key, &["What a signing.", "Get in!", "Can't wait to see him play."]),
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
