//! Social media as a simulation (D–H in the media brief; 2–4, 8–10 in the
//! final brief).
//!
//! Layers, from cheapest to richest:
//! 1. **Supporter groups** per club — season-ticket regulars, online
//!    supporters, international fans, academy watchers, ultras where the
//!    club's culture has them, numbers people, the supporters' trust —
//!    each with a size and moods toward the manager, the board and owner and
//!    the team, and relationships of their own.
//! 2. **Persistent accounts** — a representative population per club and
//!    nation, generated from the world seed with names from that nation's
//!    own name pool, each with a persona, opinions that move slowly and
//!    remember, memories of moments, and a record of what they posted.
//! 3. **Important accounts** — fan news, stats, rumour and academy accounts,
//!    journalists, and the famous people themselves.
//! 4. **Posts** — materialised only around real events (frames), as
//!    semantic structures (concept, stance, references) rendered to words
//!    by narration. Replies, quote-posts, reposts and likes are posts or
//!    counters on posts; threads decay by attention, relevance and fatigue.
//!
//! Nothing here is text. Every post records what its author knew and how,
//! and why they took the stance they did.

use pw_core::{ClubId, Date, EventId, NationId, PersonId, PlayerId, StoryId};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::FxHashMap;
use crate::media::ClaimType;

pub type AccountId = u32;
pub const NO_POST: u32 = u32::MAX;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum AccountKind {
    Supporter,
    Hardcore,
    Ultra,
    Casual,
    Local,
    International,
    Stats,
    FanNews,
    AcademyWatcher,
    RumourMill,
    Neutral,
    Provocateur,
    /// A real person in the world (player, manager, journalist, pundit).
    Person,
    ClubOfficial,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Age {
    Teen,
    Young,
    Middle,
    Older,
}

/// Who an account is, 0–100 each.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Persona {
    pub optimism: u8,
    pub patience: u8,
    pub tribalism: u8,
    pub humour: u8,
    pub hostility: u8,
    pub loyalty: u8,
    pub nostalgia: u8,
    pub stats: u8,
    pub youth: u8,
    pub local: u8,
    pub celebrity: u8,
    /// Resists changing their mind.
    pub stubbornness: u8,
    pub knowledge: u8,
    /// Credulity: believes what they read.
    pub credulity: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SocialAccount {
    pub id: AccountId,
    pub handle: String,
    pub display: String,
    pub kind: AccountKind,
    /// The real person behind it, if any.
    pub person: PersonId,
    pub age: Age,
    pub nation: NationId,
    pub club: ClubId,
    /// The club they love to hate.
    pub rival: ClubId,
    /// Supporter intensity, 0–100.
    pub intensity: u8,
    pub persona: Persona,
    /// Posting appetite, 0–100.
    pub activity: u8,
    /// Hour of the day they are most active.
    pub peak_hour: u8,
    pub followers: u32,
    /// Earned standing with other accounts, 0–100.
    pub credibility: u8,
    pub created: Date,
    pub last_post: Date,
    /// Posts today (attention fatigue).
    pub today: u8,
    pub active: bool,
}

/// An account's view of someone (a player, manager, owner, journalist).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Opinion {
    pub about: PersonId,
    /// −1000..1000.
    pub score: i16,
    pub since: Date,
    /// The lowest (and highest) it has been — for "I was wrong".
    pub low: i16,
    pub high: i16,
    /// The last post that voiced it.
    pub voiced: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MomentKind {
    DerbyGoal,
    LateWinner,
    HatTrick,
    Mistake,
    RedCard,
    TransferRequest,
    JoinedRival,
    Loyalty,
    Interview,
    Trophy,
    AcademyDebut,
    Betrayal,
    Record,
}

/// Something an account remembers about someone.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Remembered {
    pub kind: MomentKind,
    pub about: PersonId,
    pub date: Date,
    pub event: EventId,
}

/// A real thing that happened, as supporters experience it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Frame {
    Result {
        uid: u64,
    },
    LateWinner {
        uid: u64,
        player: PlayerId,
    },
    HatTrick {
        uid: u64,
        player: PlayerId,
    },
    RedCard {
        uid: u64,
        player: PlayerId,
    },
    Signing {
        player: PlayerId,
        club: ClubId,
    },
    Departure {
        player: PlayerId,
        from: ClubId,
        to: ClubId,
    },
    TransferRequest {
        player: PlayerId,
    },
    Story {
        story: StoryId,
    },
    Quote {
        quote: u32,
    },
    ManagerSacked {
        club: ClubId,
    },
    ManagerAppointed {
        club: ClubId,
    },
    Award {
        player: PlayerId,
    },
    Milestone {
        player: PlayerId,
    },
    Record {
        player: PlayerId,
    },
    Injury {
        player: PlayerId,
    },
    Incident {
        incident: u32,
    },
    /// A disputed refereeing call (`World::officials.controversies`).
    Controversy {
        controversy: u32,
    },
    /// A person's own post (the human's, or an AI person's).
    Post {
        post: u32,
    },
}

/// What a post does, semantically.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Concept {
    Praise,
    /// Praise from someone who did not rate them.
    ReluctantPraise,
    /// "Fair enough, I was wrong."
    ConcedeWrong,
    /// "One good game changes nothing."
    DoubleDown,
    Criticise,
    Mock,
    Celebrate,
    Lament,
    Worry,
    /// Compare to a club legend (the legend's person id is the `about2`).
    CompareLegend,
    /// Relay a rumour from a story.
    Relay,
    Question,
    Defend,
    Sarcasm,
    /// "You wanted them gone two weeks ago" (refers to the author's post).
    CallOut,
    Agree,
    Disagree,
    /// Sing (a chant id is the `extra`).
    Chant,
    /// Use a meme (meme id is the `extra`).
    Meme,
    /// A person's own statement.
    Statement,
}

/// How the author came to know what they post about.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Knew {
    /// Watched it (the match, a public event).
    Watched,
    Read {
        story: StoryId,
    },
    /// Saw another post.
    Saw {
        post: u32,
    },
    /// Heard it through people (an information item).
    Heard {
        info: u32,
    },
    /// Their own life.
    Own,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Post {
    pub id: u32,
    pub author: AccountId,
    pub date: Date,
    pub minute: u16,
    pub frame: Frame,
    pub concept: Concept,
    /// Main subject (person).
    pub about: PersonId,
    /// Secondary subject (a legend compared to, a person called out).
    pub about2: PersonId,
    pub club: ClubId,
    /// Chant/meme id, or a score, depending on concept.
    pub extra: u32,
    /// 0–100.
    pub intensity: u8,
    pub claim: ClaimType,
    pub reply_to: u32,
    pub quote_of: u32,
    /// Earlier posts this one refers to (the author's own, or others').
    pub refs: SmallVec<[u32; 2]>,
    pub likes: u32,
    pub reposts: u32,
    pub replies: u16,
    pub depth: u8,
    pub knew: Knew,
    /// The author's opinion of `about` before posting.
    pub prior: i16,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum TopicKey {
    Person(PersonId),
    Club(ClubId),
    Match(u64),
    Meme(u32),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Trend {
    pub key: TopicKey,
    pub nation: NationId,
    pub date: Date,
    pub posts: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum GroupKind {
    SeasonTicket,
    Online,
    International,
    Academy,
    Ultras,
    Numbers,
    /// A formal supporters' organisation.
    Trust,
}

/// A persistent community of supporters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SupporterGroup {
    pub id: u32,
    pub club: ClubId,
    pub kind: GroupKind,
    /// Members (thousands-scale proxy).
    pub size: u32,
    /// −100..100 moods.
    pub manager: i8,
    pub board: i8,
    pub team: i8,
    /// How organised and vocal, 0–100.
    pub voice: u8,
    /// Relationship with the club's board/owner (−100..100).
    pub with_board: i8,
    pub founded: Date,
    /// Last organised action.
    pub last_action: Date,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum GroupAction {
    Protest,
    Banner,
    Applause,
    Booing,
    Campaign,
    Petition,
    Tribute,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ChantKind {
    PlayerPraise,
    PlayerName,
    Rivalry,
    Mocking,
    Protest,
    Manager,
    Identity,
}

/// An original chant, built from a structure and the words of a real
/// moment; words are rendered by narration from these parts.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Chant {
    pub id: u32,
    pub club: ClubId,
    pub kind: ChantKind,
    pub about: PersonId,
    /// The opponent or target club (rivalry and mocking chants).
    pub target: ClubId,
    /// Structure template index.
    pub shape: u8,
    /// Word choices (stable per chant).
    pub seed: u64,
    pub born: Date,
    pub moment: EventId,
    pub popularity: u8,
    pub last_sung: Date,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MemeSource {
    Mistake { player: PlayerId, uid: u64 },
    Quote { quote: u32 },
    Post { post: u32 },
    Saga { thread: u32 },
    Hero { player: PlayerId, uid: u64 },
}

/// A world-born meme with a lifecycle.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Meme {
    pub id: u32,
    pub source: MemeSource,
    pub about: PersonId,
    pub club: ClubId,
    pub born: Date,
    pub recognition: u8,
    pub uses: u32,
    pub variants: u8,
    pub peak: Date,
    pub alive: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SocialNet {
    pub accounts: Vec<SocialAccount>,
    pub by_club: FxHashMap<ClubId, Vec<AccountId>>,
    pub by_nation: FxHashMap<NationId, Vec<AccountId>>,
    pub by_person: FxHashMap<PersonId, AccountId>,
    pub opinions: FxHashMap<AccountId, SmallVec<[Opinion; 6]>>,
    pub memories: FxHashMap<AccountId, SmallVec<[Remembered; 4]>>,
    /// Accounts each account follows (important ones only).
    pub follows: FxHashMap<AccountId, SmallVec<[AccountId; 8]>>,
    /// Muted accounts (they stop seeing each other's posts).
    pub muted: FxHashMap<AccountId, SmallVec<[AccountId; 2]>>,
    /// Posts in the retention window; `post_base` is the id of `posts[0]`.
    pub posts: Vec<Post>,
    pub post_base: u32,
    /// Posts kept beyond the window (viral, remembered, called out).
    pub kept: FxHashMap<u32, Post>,
    pub trends: Vec<Trend>,
    pub groups: Vec<SupporterGroup>,
    pub chants: Vec<Chant>,
    pub memes: Vec<Meme>,
    /// Frames waiting for later waves: (frame, due date, wave).
    pub waves: Vec<(Frame, Date, u8)>,
    /// What each account has learned about each outlet (account, outlet
    /// id) → −60..60: trust is contextual, earned story by story.
    pub outlet_trust: FxHashMap<(AccountId, u32), i8>,
}

impl SocialNet {
    pub fn post(&self, id: u32) -> Option<&Post> {
        if id == NO_POST {
            return None;
        }
        if id >= self.post_base { self.posts.get((id - self.post_base) as usize) } else { self.kept.get(&id) }
    }

    pub fn post_mut(&mut self, id: u32) -> Option<&mut Post> {
        if id == NO_POST {
            return None;
        }
        if id >= self.post_base { self.posts.get_mut((id - self.post_base) as usize) } else { self.kept.get_mut(&id) }
    }

    pub fn next_post_id(&self) -> u32 {
        self.post_base + self.posts.len() as u32
    }

    pub fn opinion(&self, a: AccountId, about: PersonId) -> Option<Opinion> {
        self.opinions.get(&a).and_then(|v| v.iter().find(|o| o.about == about).copied())
    }

    pub fn account_of(&self, p: PersonId) -> Option<AccountId> {
        self.by_person.get(&p).copied()
    }

    pub fn posts_by(&self, a: AccountId) -> impl DoubleEndedIterator<Item = &Post> + '_ {
        self.posts.iter().filter(move |p| p.author == a)
    }

    pub fn is_muted(&self, viewer: AccountId, author: AccountId) -> bool {
        self.muted.get(&viewer).is_some_and(|v| v.contains(&author))
    }
}
