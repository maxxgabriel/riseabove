//! Information as people hold it. `what` states the truth of an item (for
//! audits); `version` renders what a particular holder believes, shaped by
//! how faithfully it reached them — vaguer, louder or plainly wrong, but
//! always framed as what they heard, never as fact.

use pw_core::PersonId;
use pw_world::info::{Fidelity, InfoKind, Learned};
use pw_world::World;

use crate::fmt::{club, money, person, player};
use crate::lexicon;

/// The item as it really is.
pub fn what(w: &World, info: u32) -> String {
    let Some(it) = w.grapevine.items.get(info as usize) else { return "something".into() };
    describe(w, &it.kind, Fidelity::Accurate)
}

fn describe(w: &World, kind: &InfoKind, f: Fidelity) -> String {
    let vague = matches!(f, Fidelity::Partial | Fidelity::Garbled);
    let loud = matches!(f, Fidelity::Exaggerated | Fidelity::Planted);
    match *kind {
        InfoKind::Incident { .. } => "that something happened at the club".into(),
        InfoKind::Interest { club: c, player: p } => {
            if vague {
                format!("that a club has been asking about {}", player(w, p))
            } else if loud {
                format!("that {} are determined to sign {} and others are circling", club(w, c), player(w, p))
            } else {
                format!("that {} are interested in {}", club(w, c), player(w, p))
            }
        }
        InfoKind::Bid { club: c, player: p, fee } => {
            if vague {
                format!("that there has been a bid for {}", player(w, p))
            } else if loud {
                format!("that {} have bid big for {} — more than {}", club(w, c), player(w, p), money(fee))
            } else {
                format!("that {} bid {} for {}", club(w, c), money(fee), player(w, p))
            }
        }
        InfoKind::Unhappy { player: p, with } => {
            if vague {
                format!("that {} isn't happy", player(w, p))
            } else if loud {
                format!("that {} has had enough of {} and wants out", player(w, p), person(w, with))
            } else {
                format!("that {} is unhappy with {}", player(w, p), person(w, with))
            }
        }
        InfoKind::JobInDanger { club: c, manager } => {
            if loud {
                format!("that {} is one result from the sack at {}", person(w, manager), club(w, c))
            } else {
                format!("that the {} board is losing patience with {}", club(w, c), person(w, manager))
            }
        }
        InfoKind::Discipline { player: p, club: c } => {
            if vague {
                format!("that {} is in some kind of trouble", player(w, p))
            } else {
                format!("that {} was disciplined by {}", player(w, p), club(w, c))
            }
        }
        InfoKind::InjuryWorse { player: p, days } => {
            if vague {
                format!("that {}'s injury is worse than they're saying", player(w, p))
            } else {
                format!("that {} will be out for {}, not what was announced", player(w, p), crate::fmt::duration_days(days))
            }
        }
        InfoKind::ContractTalks { player: p, club: c, stalling } => {
            if stalling {
                format!("that contract talks between {} and {} have stalled", player(w, p), club(w, c))
            } else {
                format!("that {} has agreed a new deal with {}", player(w, p), club(w, c))
            }
        }
        InfoKind::Private { person: x, what } => format!("something private about {} ({})", person(w, x), lexicon::life_event(what)),
        InfoKind::DressingRoom { club: c, leader } => {
            if loud {
                format!("that the {} dressing room is in open revolt, led by {}", club(w, c), player(w, leader))
            } else {
                format!("that some {} players, around {}, have lost faith in the manager", club(w, c), player(w, leader))
            }
        }
        InfoKind::Exploring { player: p, agent } => format!("that {}'s agent {} is sounding out clubs", player(w, p), person(w, agent)),
    }
}

/// What `holder` believes about an item, and how they came by it.
pub fn version(w: &World, info: u32, holder: PersonId) -> Option<String> {
    let it = w.grapevine.items.get(info as usize)?;
    let k = it.knower(holder)?;
    let how = match k.how {
        Learned::Involved => "they were involved".to_string(),
        Learned::Witnessed => "they saw it".to_string(),
        Learned::Told { by } => format!("{} told them", person(w, by)),
        Learned::Read { .. } => "they read it".to_string(),
        Learned::Official => "they were told officially".to_string(),
        Learned::Inferred => "they worked it out".to_string(),
    };
    Some(format!("{} heard {} ({}; {}% sure)", person(w, holder), describe(w, &it.kind, k.fidelity), how, k.confidence))
}
