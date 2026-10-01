//! Names, labels and descriptions a world takes from its scenario's reference data (`data/worlds/<nation>/**`), each with the record
//! it came from and that record's standing (`DataOrigin`).
//!
//! What lives here is identity and words: the real outlet a newspaper of the world is, the nickname supporters use for a club, the
//! association a state's football answers to, the programmes, partnerships, licences, representative sides and rules the reference
//! describes. Nothing here is a strength, a result or a history: the simulation starts from its own values and owns what happens.
//! Systems that read it (the newsroom, the narration, the views) use it for how things are called and what they are, never for how
//! good they are.

use pw_core::{ClubId, CompId, OutletId, RegionId};
use serde::{Deserialize, Serialize};

use crate::FxHashMap;
use crate::scenario::DataOrigin;

/// The reference record something came from, and how far it can be trusted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// The record's id (`media.ganashakti`).
    pub id: String,
    pub origin: DataOrigin,
}

/// A real newspaper, channel or site the world's outlet is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreOutlet {
    pub source: Source,
    /// The reference's kind of outlet, in its words (`regional`, `football_specialist`).
    pub kind: String,
    /// Languages it publishes in, as codes (`bn`, `ml`), in the order the record lists them.
    pub languages: Vec<String>,
    /// `national`, `multi_state`, `state` or `local`.
    pub reach: String,
    /// The state it is based in, when it has one.
    pub home: RegionId,
    pub city: String,
}

/// A broadcaster and the competitions of this world it holds rights to in the starting season.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreBroadcaster {
    pub source: Source,
    pub name: String,
    pub languages: Vec<String>,
    pub comps: Vec<CompId>,
}

/// Another name of a club: a nickname, a supporters' group, an abbreviation, a native-script spelling.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreAlias {
    pub text: String,
    /// `nickname`, `supporter_group`, `short_form`, `native_script` ...
    pub kind: String,
    /// Language code, when the record gives one.
    pub lang: String,
    pub origin: DataOrigin,
}

/// A real school, sports hostel or university the world's institution is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreInstitution {
    pub source: Source,
    /// The reference's kind (`sports_school`, `sai_centre`, `central`).
    pub kind: String,
    pub operator: String,
    /// What the record says it does for football, in one line.
    pub note: String,
}

/// The academy a club of the world runs, by its real name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreAcademy {
    pub source: Source,
    pub name: String,
    pub kind: String,
    pub residential: Option<bool>,
    pub age_groups: Vec<String>,
}

/// The association a state's football answers to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreAssociation {
    pub source: Source,
    pub name: String,
    pub abbr: String,
    pub hq: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreProgramme {
    pub source: Source,
    pub name: String,
    pub operator: String,
    pub kind: String,
    pub ages: String,
    /// The states it runs in; empty for a national programme.
    pub regions: Vec<RegionId>,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LorePartnership {
    pub source: Source,
    /// Clubs of this world that are party to it.
    pub clubs: Vec<ClubId>,
    /// The other parties, by name, with their nation code.
    pub foreign: Vec<(String, String)>,
    pub components: Vec<String>,
    pub purpose: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreTeam {
    pub source: Source,
    pub name: String,
    pub kind: String,
    pub gender: String,
    pub age: String,
    pub eligibility: String,
}

/// One step of the coaching ladder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreLicence {
    pub source: Source,
    pub name: String,
    pub body: String,
    pub order: u8,
    pub requirement: String,
}

/// One step of the referees' ladder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreGrade {
    pub source: Source,
    pub name: String,
    pub body: String,
    pub order: u8,
    pub scope: String,
}

/// A rule in words. Only words: numbers a rule may carry are not applied from here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreRule {
    pub source: Source,
    pub topic: String,
    pub statement: String,
    /// Competitions of this world it applies to.
    pub comps: Vec<CompId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreLanguage {
    pub code: String,
    pub name: String,
    pub script: String,
}

/// A football word in one language: `concept` is what it names (`match.goal`), `text` the word, `romanised` how it is written in the
/// Latin alphabet when the record gives that.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoreTerm {
    pub concept: String,
    pub lang: String,
    pub text: String,
    pub romanised: String,
    pub register: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Lore {
    pub outlets: FxHashMap<OutletId, LoreOutlet>,
    pub broadcasters: Vec<LoreBroadcaster>,
    pub aliases: FxHashMap<ClubId, Vec<LoreAlias>>,
    pub institutions: FxHashMap<u32, LoreInstitution>,
    pub academies: FxHashMap<ClubId, LoreAcademy>,
    pub associations: FxHashMap<RegionId, LoreAssociation>,
    /// Competitions of this world that are a real competition, and which.
    pub competitions: FxHashMap<CompId, Source>,
    pub programmes: Vec<LoreProgramme>,
    pub partnerships: Vec<LorePartnership>,
    pub teams: Vec<LoreTeam>,
    pub licences: Vec<LoreLicence>,
    pub grades: Vec<LoreGrade>,
    pub rules: Vec<LoreRule>,
    pub languages: Vec<LoreLanguage>,
    pub terms: Vec<LoreTerm>,
}

impl Lore {
    /// Nothing was taken from reference data (a world built without it, or saved before this existed).
    pub fn is_empty(&self) -> bool {
        self.outlets.is_empty() && self.aliases.is_empty() && self.institutions.is_empty() && self.associations.is_empty() && self.terms.is_empty()
    }

    /// The code of a language, by the name a scenario uses for it (`Bengali` is `bn`).
    pub fn language_code(&self, name: &str) -> Option<&str> {
        self.languages.iter().find(|l| l.name.eq_ignore_ascii_case(name)).map(|l| l.code.as_str())
    }

    /// The words for a concept in a language, most neutral first.
    pub fn terms(&self, concept: &str, lang: &str) -> impl Iterator<Item = &LoreTerm> {
        self.terms.iter().filter(move |t| t.concept == concept && t.lang == lang)
    }

    /// The nicknames of a club that supporters and headlines use.
    pub fn nicknames(&self, club: ClubId) -> impl Iterator<Item = &LoreAlias> {
        self.aliases.get(&club).into_iter().flatten().filter(|a| a.kind == "nickname")
    }
}
