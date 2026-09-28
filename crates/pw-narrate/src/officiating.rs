//! Referees, big calls, appeals and charges in words. Whether a call was
//! right is hidden state: text reports what was given and how it was
//! received, never the truth, until an appeal panel rules.

use pw_world::officials::{Appeal, AppealOutcome, CallKind, Charge, ChargeKind, Controversy};
use pw_world::World;

use crate::fmt::{club, money, person, player};

pub fn referee(w: &World, id: u32) -> String {
    w.officials.referees.get(id as usize).map_or_else(|| "the referee".into(), |r| format!("referee {}", person(w, r.person)))
}

/// "the late penalty against Riverton", "Smith's red card".
pub fn call(w: &World, c: &Controversy) -> String {
    match c.kind {
        CallKind::Penalty => format!("the {}penalty against {}", if c.minute >= 80 { "late " } else { "" }, club(w, c.against)),
        CallKind::RedCard => format!("{}'s red card", player(w, c.player)),
        CallKind::SecondYellow => format!("{}'s second yellow", player(w, c.player)),
    }
}

pub fn controversy(w: &World, c: &Controversy) -> String {
    let what = call(w, c);
    let mut s = format!("{} gave {what} in the {}th minute; {} supporters were furious.", capital(&referee(w, c.referee)), c.minute, club(w, c.against));
    if w.officials.perceived_bias(c.against, c.referee) {
        s.push_str(&format!(" Many of them now believe {} is against them.", referee(w, c.referee)));
    }
    s
}

pub fn appeal(w: &World, a: &Appeal) -> String {
    let who = player(w, a.player);
    match a.outcome {
        Some(AppealOutcome::Rescinded) => format!("{}'s appeal was upheld: {who}'s red card was rescinded.", club(w, a.club)),
        Some(AppealOutcome::Upheld) => format!("{}'s appeal was rejected: {who} serves the ban.", club(w, a.club)),
        Some(AppealOutcome::Extended) => format!("{}'s appeal was ruled frivolous: {who}'s ban was extended by a match.", club(w, a.club)),
        None => format!("{} appealed {who}'s red card.", club(w, a.club)),
    }
}

pub fn charge(w: &World, c: &Charge) -> String {
    match c.kind {
        ChargeKind::FailingToControl => format!("{} were fined {} for failing to control their players.", club(w, c.club), money(c.fine)),
        ChargeKind::RefereeComments { person: p } => format!("{} were fined {} over {}'s comments about the referee.", club(w, c.club), money(c.fine), person(w, p)),
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
}
