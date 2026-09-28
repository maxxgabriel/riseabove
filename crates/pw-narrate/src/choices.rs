//! Decisions and their options as text.

use pw_world::decision::{Choice, Decision, DecisionKind};
use pw_world::World;

use crate::fmt::{club, money, person, wage};

pub fn title(w: &World, d: &Decision) -> String {
    match &d.kind {
        DecisionKind::TransferTalks { club: c, fee } => format!("{} want to talk about a {} move", club(w, *c), money(*fee)),
        DecisionKind::ContractOffer { club: c, contract, renewal } => {
            format!("{} {} offer: {} until {}", club(w, *c), if *renewal { "renewal" } else { "contract" }, wage(contract.wage), contract.end)
        }
        DecisionKind::LoanOffer { loan } => format!("{} propose a loan to {} until {}", club(w, loan.parent), club(w, loan.club), loan.end),
        DecisionKind::FreeAgentOffer { club: c, contract } => format!("{} offer a contract: {}", club(w, *c), wage(contract.wage)),
        DecisionKind::Negotiation { talk } => {
            let t = &w.talks[*talk];
            format!("{} — {}: {}", club(w, t.club), t.kind.label(), crate::talk::terms(&t.offer))
        }
        DecisionKind::Meeting { meeting } => {
            let m = &w.meetings.list[*meeting];
            format!("{} wants to see you about {}", person(w, m.initiator), m.topic.label())
        }
        DecisionKind::Partner { partner, ask } => {
            let what = match ask {
                pw_world::PartnerAsk::MoveIn => "wants to move in together",
                pw_world::PartnerAsk::Marry => "has asked you to marry them",
                pw_world::PartnerAsk::Separate => "thinks you should separate",
            };
            format!("{} {what}", person(w, *partner))
        }
        DecisionKind::Trial { club: c, days } => format!("{} invite you for a {days}-day trial", club(w, *c)),
        DecisionKind::NationChoice { nation: n, other } => format!(
            "{} have called you up. Accepting commits you to them; refusing commits you to {}",
            crate::fmt::nation(w, *n),
            crate::fmt::nation(w, *other)
        ),
    }
}

pub fn option(c: &Choice) -> String {
    match c {
        Choice::Accept => "Accept".into(),
        Choice::Reject => "Reject".into(),
        Choice::Decline => "Decline".into(),
        Choice::Respond(t) => format!("Respond — {}", t.label()),
        Choice::Counter { wage: x, status, release_clause, .. } => {
            let mut s = format!("Counter: {}", wage(*x));
            if let Some(st) = status {
                s += &format!(", status {}", st.label());
            }
            if *release_clause > 0 {
                s += &format!(", release clause {}", money(*release_clause));
            }
            s
        }
    }
}
