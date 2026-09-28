//! The inhabited person's feed: everything that reached them, and nothing
//! that didn't (P3). An item exists only because an event did; its text is
//! rendered from that event.

use pw_core::{Date, EventId, PersonId};
use pw_world::World;
use pw_world::event::{Event, EventKind, Visibility};

#[derive(Clone, Debug)]
pub struct Item {
    pub date: Date,
    pub event: EventId,
    pub text: String,
    pub important: bool,
}

/// Does this event concern `me` (directly, or through my club), and could I know it?
pub fn concerns(w: &World, me: PersonId, e: &Event) -> bool {
    let club = w.club_of_person(me);
    if !e.vis.reaches(me, club) {
        return false;
    }
    let my_player = w.people[me].player;
    if my_player.is_some() && e.kind.player() == Some(my_player) {
        return true;
    }
    if e.kind.people().contains(&me) {
        return true;
    }
    if let EventKind::Published { story } = e.kind {
        let s = &w.media.stories[story];
        return s.person == me || (club.is_some() && (s.club == club || s.other_club == club));
    }
    match e.vis {
        Visibility::Person(p) | Visibility::Between(p, _) if p == me => return true,
        Visibility::Between(_, p) if p == me => return true,
        _ => {}
    }
    // Club news I'd hear at the training ground.
    if club.is_some() && e.kind.clubs().contains(&club) {
        return matches!(
            e.kind,
            EventKind::ManagerSacked { .. }
                | EventKind::ManagerAppointed { .. }
                | EventKind::Transfer { .. }
                | EventKind::LoanMove { .. }
                | EventKind::TransferListed { .. }
                | EventKind::TransferRequested { .. }
                | EventKind::Unrest { .. }
                | EventKind::Fined { .. }
                | EventKind::Captaincy { .. }
        );
    }
    false
}

pub fn is_important(w: &World, me: PersonId, e: &Event) -> bool {
    let my_player = w.people[me].player;
    let mine = my_player.is_some() && e.kind.player() == Some(my_player);
    match e.kind {
        EventKind::Meeting { .. } | EventKind::PromiseMade { .. } | EventKind::PromiseBroken { .. } | EventKind::PromiseKept { .. } => true,
        EventKind::TalksOpened { .. } | EventKind::TalksCollapsed { .. } => mine,
        EventKind::Transfer { .. } | EventKind::LoanMove { .. } | EventKind::Injured { .. } | EventKind::Suspended { .. } | EventKind::Fined { .. } => mine,
        EventKind::TransferListed { .. } | EventKind::StatusChanged { .. } | EventKind::Debut { .. } | EventKind::Award { .. } | EventKind::Captaincy { .. } => mine,
        EventKind::ManagerSacked { club, .. } | EventKind::ManagerAppointed { club, .. } => club == w.club_of_person(me),
        EventKind::Life { person, .. } => person == me,
        EventKind::Published { story } => w.media.stories[story].person == me,
        _ => false,
    }
}

pub fn important_since(w: &World, me: PersonId, after: EventId) -> bool {
    w.events.after(after).iter().any(|e| concerns(w, me, e) && is_important(w, me, e))
}

/// Feed items after `after` (newest last).
pub fn since(w: &World, me: PersonId, after: EventId) -> Vec<Item> {
    w.events
        .after(after)
        .iter()
        .filter(|e| concerns(w, me, e))
        .filter_map(|e| {
            let text = pw_narrate::events::line(w, e, me)?;
            Some(Item { date: e.date, event: e.id, text, important: is_important(w, me, e) })
        })
        .collect()
}

/// The last `n` items that concerned me, from everything still on record.
pub fn recent(w: &World, me: PersonId, n: usize) -> Vec<Item> {
    let mut v: Vec<Item> = w
        .events
        .all()
        .iter()
        .rev()
        .filter(|e| concerns(w, me, e))
        .take(n)
        .filter_map(|e| Some(Item { date: e.date, event: e.id, text: pw_narrate::events::line(w, e, me)?, important: is_important(w, me, e) }))
        .collect();
    v.reverse();
    v
}
