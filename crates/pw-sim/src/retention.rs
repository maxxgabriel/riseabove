//! Retention: what the world keeps, and for how long, so that a save grows with the football being played and not with the years
//! that have passed.
//!
//! The rule is the same everywhere: **old detail is summarised or dropped; identity and headline history stay.** People, honours,
//! records, season lines, transfers, titles and debuts are never touched here. What goes is the working detail around them: a
//! meeting's round-by-round log, an information item nobody is passing on, an incident from years ago that nothing waits on, the
//! recent-appearance list of a player who retired long ago. Nothing is invented and no kept row is altered; where a row is kept its
//! outcome is exactly what it was.
//!
//! Monthly: `monthly` (the event log). Yearly: `yearly` (everything else, called from `compact`). The grapevine keeps its own monthly
//! rule (`grapevine::compact`). Every window below is a named constant; the save-size diagnostics (`pathway-sim growth`) and
//! `crates/pw-cli/tests/qa_save_bounds.rs` watch that they hold.

use pw_core::{Date, PersonId, PlayerId};
use pw_world::event::EventKind as E;
use pw_world::negotiation::{Negotiation, TalkLine};
use pw_world::{LifeEventKind, PlayerStatus, World};

const CLOSED_TALK_DETAIL_DAYS: i32 = 365;
/// A meeting is forgotten this long after it was held (longer than the events that point at it, `EVENT_LIFE_DAYS`), unless a decision still points at it.
const MEETING_DAYS: i32 = 3 * 365;
/// Events keep full detail this long. The readers that look back furthest (the league rule review, the youth-graduate count) read a year.
const EVENT_DETAIL_DAYS: i32 = 400;
/// People-facing event detail (a meeting, a press story, a promise, most of a person's life events) is kept this long.
const EVENT_LIFE_DAYS: i32 = 730;
/// An incident nothing waits on is forgotten after this long (its consequences are in the people, the club and the events that stay).
const INCIDENT_DAYS: i32 = 3 * 365;
/// A relationship nobody has touched for this long is forgotten if either person has left the game, and for ever after twice as long.
const TIE_DAYS: i32 = 3 * 365;
/// A player who retired (last played) this long ago is "gone": the working detail kept about them is dropped, the career stays.
const GONE_DAYS: i32 = 3 * 365;
/// An injury this long (days missed) is part of a career and is kept even for a player who is gone.
const SERIOUS_INJURY_DAYS: u16 = 90;

fn compact_talk(talk: &mut Negotiation) {
    let final_line = talk.log.iter().rev().find(|(_, line)| matches!(line, TalkLine::PlayerAccepted | TalkLine::PlayerRejected | TalkLine::ClubWalkedAway)).or_else(|| talk.log.last()).copied();
    talk.log.clear();
    talk.log.shrink_to_fit();
    if let Some(line) = final_line {
        talk.log.push(line);
    }
    talk.moves.clear();
    talk.moves.shrink_to_fit();
    talk.ask = None;
}

/// Events that are history for good: the headline of someone's career or of a club's.
fn headline(kind: &E) -> bool {
    matches!(
        kind,
        E::Transfer { .. }
            | E::Champion { .. }
            | E::Award { .. }
            | E::Debut { .. }
            | E::Retired { .. }
            | E::ManagerAppointed { .. }
            | E::JoinedStaff { .. }
            | E::Life { kind: LifeEventKind::Married { .. } | LifeEventKind::ChildBorn | LifeEventKind::Separated { .. } | LifeEventKind::Bereavement, .. }
    )
}

/// Events that people talk about for a while, and that stay addressable (an inbox line, a story) for two years.
fn talked_about(kind: &E) -> bool {
    matches!(
        kind,
        E::Meeting { .. } | E::PromiseMade { .. } | E::PromiseKept { .. } | E::PromiseBroken { .. } | E::TransferRequested { .. } | E::Published { .. } | E::Life { .. }
    )
}

/// Monthly: the event log keeps everything for a little over a year, headline events for good, and what people talk about for two years.
pub fn monthly(w: &mut World) {
    let today = w.date;
    let (full, talk) = (today.add_days(-EVENT_DETAIL_DAYS), today.add_days(-EVENT_LIFE_DAYS));
    w.events.compact(full, |e| headline(&e.kind) || (e.date >= talk && talked_about(&e.kind)));
}

/// Has this player left the game for good: retired, and not seen on a pitch for a long time, with no job in it?
pub fn gone(w: &World, p: PlayerId, today: Date) -> bool {
    let h = &w.players.hot[p];
    if h.status != PlayerStatus::Retired || h.last_match.days_until(today) <= GONE_DAYS {
        return false;
    }
    let s = w.people[w.players.cold[p].person].staff;
    s.is_none() || !w.staff[s].employed()
}

/// Has this person left the game: a computer-run player who is `gone`? Their life stops being simulated month by month (no new
/// finances, family events, private news): without this a world's population of retired people grows for ever and each of them keeps
/// producing events, rumours and ties. What their life held when they left stays as it was; a person a human inhabits is never "left".
pub fn left_the_game(w: &World, who: PersonId, today: Date) -> bool {
    let person = &w.people[who];
    person.player.is_some() && person.mind == pw_world::MindKind::Ai && gone(w, person.player, today)
}

/// Drop round-by-round detail from old closed talks; keep ids, parties, final terms, private limits, causes, and outcome.
/// Yearly: forget what nothing needs any more. Returns how many closed talks were compacted.
pub fn yearly(w: &mut World) -> usize {
    let today = w.date;
    let mut compacted = 0;
    for talk in w.talks.iter_mut() {
        if !talk.is_open() && talk.opened.days_until(today) > CLOSED_TALK_DETAIL_DAYS && (talk.log.len() > 1 || !talk.moves.is_empty() || talk.ask.is_some()) {
            compact_talk(talk);
            compacted += 1;
        }
    }
    w.incidents.forget_before(today.add_days(-INCIDENT_DAYS));
    let pinned: std::collections::HashSet<pw_core::MeetingId> = w.decisions.all.iter().filter_map(|d| if let pw_world::DecisionKind::Meeting { meeting } = d.kind { Some(meeting) } else { None }).collect();
    w.meetings.forget_before(today.add_days(-MEETING_DAYS), &pinned);
    gone_players(w, today);
    compacted
}

/// People who have left the game: the working detail about them goes (who they knew in passing, their last ten appearances, their
/// development curve, minor knocks, what their club's scouts once thought). Their identity, honours, records, season lines, career
/// history, serious injuries and every event that is history stay.
fn gone_players(w: &mut World, today: Date) {
    let gone_players: Vec<PlayerId> = w.players.ids().filter(|&p| gone(w, p, today)).collect();
    let is_gone: pw_world::FxHashSet<PersonId> = gone_players.iter().map(|&p| w.players.cold[p].person).collect();
    // Ties and memories: nobody has touched them for three years and one end is gone; any tie untouched for six.
    let (stale, very_stale) = (today.add_days(-TIE_DAYS), today.add_days(-2 * TIE_DAYS));
    w.social.forget_ties(stale, very_stale, |p| is_gone.contains(&p));
    for &p in &gone_players {
        w.perf.recent.remove(&p);
        w.perf.readings.remove(&p);
        w.growth.records.remove(&p);
        w.medical.fragile.remove(&p);
        w.medical.chronic.remove(&p);
        w.medical.willing_to_rush.remove(&p);
        if let Some(h) = w.medical.history.get_mut(&p) {
            h.retain(|c| c.actual >= SERIOUS_INJURY_DAYS);
            h.shrink_to_fit();
        }
    }
    w.medical.history.retain(|_, h| !h.is_empty());
}
