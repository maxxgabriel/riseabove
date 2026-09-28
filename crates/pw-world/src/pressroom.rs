//! Press conferences and the quotes they produce (M in the brief).
//!
//! Questions are drawn only from what the asking journalist could know:
//! public results and records, published stories, public injuries, the next
//! opponent, earlier quotes. A private matter is asked about only once it
//! has been reported. Answers are stances chosen by the speaker (their mind
//! or a human); every answer becomes a quote that people remember, that the
//! press can throw back later, and that fans read differently.

use pw_core::{ClubId, Date, PersonId, PlayerId, StoryId};
use serde::{Deserialize, Serialize};

use crate::media::Stance;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum QTopic {
    /// The last match (fixture uid).
    LastMatch {
        uid: u64,
    },
    /// A published transfer story about a player.
    Transfer {
        story: StoryId,
        player: PlayerId,
    },
    /// A published story about trouble (discipline, unrest, an incident).
    Trouble {
        story: StoryId,
    },
    Injury {
        player: PlayerId,
    },
    /// A run of poor results.
    Pressure,
    /// The next opponent (a rivalry).
    Rival {
        club: ClubId,
    },
    /// Something the speaker said before (quote id).
    EarlierQuote {
        quote: u32,
    },
    /// A player's form or selection.
    Selection {
        player: PlayerId,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Question {
    pub journalist: PersonId,
    pub topic: QTopic,
    pub about: PersonId,
    /// A follow-up to the previous question.
    pub follow_up: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conference {
    pub id: u32,
    pub date: Date,
    pub club: ClubId,
    pub speaker: PersonId,
    pub questions: Vec<Question>,
    /// Quote id per answered question.
    pub answers: Vec<Option<u32>>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct QuoteRecord {
    pub id: u32,
    pub speaker: PersonId,
    pub about: PersonId,
    pub stance: Stance,
    pub topic: Option<QTopic>,
    pub date: Date,
    pub conference: u32,
    pub story: StoryId,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Pressroom {
    pub conferences: Vec<Conference>,
    pub quotes: Vec<QuoteRecord>,
}

impl Pressroom {
    pub fn quotes_by(&self, p: PersonId) -> impl DoubleEndedIterator<Item = &QuoteRecord> + '_ {
        self.quotes.iter().filter(move |q| q.speaker == p)
    }
}
