//! Compact detail in closed negotiations while keeping the negotiation row and its final outcome.
use pw_world::World;
use pw_world::negotiation::{Negotiation, TalkLine};

const CLOSED_TALK_DETAIL_DAYS: i32 = 365;

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

/// Drop round-by-round detail from old closed talks; keep ids, parties, final terms, private limits, causes, and outcome.
pub fn yearly(w: &mut World) -> usize {
    let today = w.date;
    let mut compacted = 0;
    for talk in w.talks.iter_mut() {
        if !talk.is_open() && talk.opened.days_until(today) > CLOSED_TALK_DETAIL_DAYS && (talk.log.len() > 1 || !talk.moves.is_empty() || talk.ask.is_some()) {
            compact_talk(talk);
            compacted += 1;
        }
    }
    compacted
}
