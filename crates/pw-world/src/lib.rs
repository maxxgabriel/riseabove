//! Complete world state. Plain data plus small, local invariants; the systems
//! that evolve it live in `pw-sim`.

pub mod agent;
pub mod beliefs;
pub mod calendar;
pub mod club;
pub mod comp;
pub mod contract;
pub mod decision;
pub mod event;
pub mod history;
pub mod intent;
pub mod interaction;
pub mod knowledge;
pub mod life;
pub mod media;
pub mod names;
pub mod negotiation;
pub mod nation;
pub mod person;
pub mod player;
pub mod rules;
pub mod social;
pub mod staff;
pub mod stats;
pub mod world;

pub use club::{Club, Team, TeamKind};
pub use comp::{CompKind, Competition, Fixture, Fixtures, Format, Score, TableRow};
pub use contract::{Contract, ContractKind, Loan, SquadStatus};
pub use decision::{Choice, Decision, DecisionKind, Decisions, MindKind};
pub use event::{Cause, Causes, Event, EventKind, EventLog, Fact, LifeEventKind, Visibility};
pub use intent::{Intent, PartnerAsk};
pub use interaction::{Meeting, MeetingState, Outcome, Tone, Topic};
pub use life::{Life, Lifestyle, MoodFactor, Occupation, Routine};
pub use media::{FanReason, Story, StoryKind};
pub use names::{NameId, Names};
pub use nation::{Confed, Nation};
pub use person::Person;
pub use negotiation::{Negotiation, TalkEnd, TalkKind, TalkLine, TalkState, Terms};
pub use player::{Focus, Intensity, PlayerCold, PlayerHot, PlayerStatus, Players, TrainingPlan};
pub use social::{MemoryKind, Promise, PromiseKind, PromiseState, Rel, Social};
pub use staff::{Archetype, Philosophy, Staff, StaffRole};
pub use world::World;

pub type FxHashMap<K, V> = rustc_hash::FxHashMap<K, V>;
pub type FxHashSet<K> = rustc_hash::FxHashSet<K>;
