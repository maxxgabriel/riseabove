//! Turning world events into sentences the viewer is allowed to read.

use pw_core::{ClubId, CompId, PlayerId};
use pw_world::event::{AwardKind, Event, EventKind as E, Visibility};

use crate::ctx::Ctx;
use crate::model::{Part, Ref};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Transfers,
    Career,
    Health,
    Club,
    Competition,
}

impl Group {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "transfers" => Group::Transfers,
            "career" => Group::Career,
            "health" => Group::Health,
            "club" => Group::Club,
            "competition" => Group::Competition,
            _ => return None,
        })
    }
}

pub fn group_of(k: &E) -> Group {
    use E::*;
    match k {
        Transfer { .. } | LoanMove { .. } | LoanReturn { .. } | ContractSigned { .. } | Released { .. } | Interest { .. } | BidRejected { .. } | BidAccepted { .. } | TransferListed { .. } => Group::Transfers,
        Retired { .. } | Debut { .. } | FirstGoal { .. } | Award { .. } | CallUp { .. } => Group::Career,
        Injured { .. } | Recovered { .. } | Suspended { .. } => Group::Health,
        ManagerSacked { .. } | ManagerAppointed { .. } | YouthIntake { .. } => Group::Club,
        Champion { .. } | Promoted { .. } | Relegated { .. } => Group::Competition,
    }
}

pub fn label(k: &E) -> &'static str {
    use E::*;
    match k {
        Transfer { .. } => "Transfer",
        LoanMove { .. } => "Loan",
        LoanReturn { .. } => "Loan return",
        ContractSigned { renewal: true, .. } => "Renewal",
        ContractSigned { .. } => "Signing",
        Released { .. } => "Release",
        Retired { .. } => "Retirement",
        Injured { .. } => "Injury",
        Recovered { .. } => "Recovery",
        Suspended { .. } => "Suspension",
        ManagerSacked { .. } => "Dismissal",
        ManagerAppointed { .. } => "Appointment",
        YouthIntake { .. } => "Youth intake",
        Debut { .. } => "Debut",
        FirstGoal { .. } => "First goal",
        Champion { .. } => "Title",
        Promoted { .. } => "Promotion",
        Relegated { .. } => "Relegation",
        Interest { .. } => "Interest",
        BidRejected { .. } | BidAccepted { .. } => "Bid",
        TransferListed { .. } => "Listing",
        Award { .. } => "Award",
        CallUp { .. } => "Call-up",
    }
}

pub fn clubs_of(k: &E) -> Vec<ClubId> {
    use E::*;
    match *k {
        Transfer { from, to, .. } | LoanMove { from, to, .. } => vec![from, to],
        LoanReturn { to, .. } => vec![to],
        ContractSigned { club, .. } | Released { club, .. } | Interest { club, .. } | BidRejected { club, .. } | BidAccepted { club, .. } | TransferListed { club, .. } => vec![club],
        ManagerSacked { club, .. } | ManagerAppointed { club, .. } | YouthIntake { club, .. } => vec![club],
        _ => vec![],
    }
}

pub fn comp_of(k: &E) -> Option<CompId> {
    match *k {
        E::Debut { comp, .. } | E::FirstGoal { comp, .. } | E::Champion { comp, .. } | E::Promoted { comp, .. } | E::Relegated { comp, .. } | E::Award { comp, .. } => Some(comp),
        _ => None,
    }
}

/// May this viewer read this event?
pub fn visible(c: &Ctx, e: &Event) -> bool {
    let vis = match e.vis {
        Visibility::Public => true,
        Visibility::Club(cl) => c.observer() || c.same_club(cl),
        Visibility::Person(p) => c.observer() || Some(p) == c.me(),
    };
    vis && !spoils(c, e)
}

/// Events that would give away a result the viewer has chosen not to see yet.
fn spoils(c: &Ctx, e: &Event) -> bool {
    if c.s.meta.concealed.is_empty() {
        return false;
    }
    let team_of_event = match e.kind {
        E::Debut { team, .. } | E::FirstGoal { team, .. } => Some(team),
        E::Injured { player, .. } | E::Suspended { player, .. } => Some(c.w.players.hot[player].team).filter(|t| t.is_some()),
        _ => None,
    };
    let Some(team) = team_of_event else { return false };
    c.concealed_fixtures().iter().any(|f| f.date == e.date && f.involves(team))
}

fn pl(c: &Ctx, p: PlayerId) -> Part {
    Part::l(c.player_ref(p), c.player_name(p))
}

fn cl(c: &Ctx, x: ClubId) -> Part {
    if x.is_none() { Part::t("no club") } else { Part::l(Ref::club(x), c.club_name(x)) }
}

fn cp(c: &Ctx, x: CompId) -> Part {
    Part::l(Ref::comp(x), c.comp_name(x))
}

fn t(s: &str) -> Part {
    Part::t(s)
}

/// The main thing the event is about, for row navigation.
pub fn primary(c: &Ctx, k: &E) -> Option<Ref> {
    if let Some(p) = k.player() {
        return Some(c.player_ref(p));
    }
    match *k {
        E::Retired { person } => Some(Ref::person(person)),
        E::ManagerSacked { staff, .. } | E::ManagerAppointed { staff, .. } => Some(Ref::person(c.w.staff[staff].person)),
        E::YouthIntake { club, .. } => Some(Ref::club(club)),
        E::Champion { team, .. } | E::Promoted { team, .. } | E::Relegated { team, .. } => Some(c.team_ref(team)),
        _ => None,
    }
}

pub fn describe(c: &Ctx, k: &E) -> Vec<Part> {
    use E::*;
    match *k {
        Transfer { player, from, to, fee } => {
            let mut v = vec![pl(c, player), t(" joined "), cl(c, to)];
            if from.is_some() {
                v.push(t(" from "));
                v.push(cl(c, from));
            }
            if fee > 0 {
                v.push(t(" for "));
                v.push(Part::money(fee));
            } else {
                v.push(t(if from.is_some() { " on a free transfer" } else { " as a free agent" }));
            }
            v
        }
        LoanMove { player, from, to, until } => {
            vec![pl(c, player), t(" went on loan to "), cl(c, to), t(" from "), cl(c, from), t(" until "), Part::date(until)]
        }
        LoanReturn { player, to } => vec![pl(c, player), t(" returned from loan to "), cl(c, to)],
        ContractSigned { player, club, wage, until, renewal } => {
            let mut v = vec![pl(c, player), t(if renewal { " signed a new contract with " } else { " signed with " }), cl(c, club), t(" until "), Part::date(until)];
            if c.sees_contract(player) {
                v.push(t(" on "));
                v.push(Part::money(wage));
                v.push(t(" a week"));
            }
            v
        }
        Released { player, club } => vec![pl(c, player), t(" left "), cl(c, club), t(" when the contract ended")],
        Retired { person } => vec![Part::l(Ref::person(person), c.person_name(person)), t(" retired from playing")],
        Injured { player, injury, days } => {
            let name = pw_sim::health::injury_name(c.w, injury);
            vec![pl(c, player), t(&format!(" was injured ({}), out for about {} days", name.to_lowercase(), days))]
        }
        Recovered { player } => vec![pl(c, player), t(" recovered from injury")],
        Suspended { player, matches } => vec![pl(c, player), t(&format!(" was suspended for {matches} match{}", if matches == 1 { "" } else { "es" }))],
        ManagerSacked { staff, club } => {
            let person = c.w.staff[staff].person;
            vec![cl(c, club), t(" dismissed manager "), Part::l(Ref::person(person), c.person_name(person))]
        }
        ManagerAppointed { staff, club } => {
            let person = c.w.staff[staff].person;
            vec![cl(c, club), t(" appointed "), Part::l(Ref::person(person), c.person_name(person)), t(" as manager")]
        }
        YouthIntake { club, count } => vec![cl(c, club), t(&format!(" took in {count} new youth players"))],
        Debut { player, team, comp } => vec![pl(c, player), t(" made a first appearance for "), Part::l(c.team_ref(team), c.team_short(team)), t(" in "), cp(c, comp)],
        FirstGoal { player, team, comp } => vec![pl(c, player), t(" scored a first goal for "), Part::l(c.team_ref(team), c.team_short(team)), t(" in "), cp(c, comp)],
        Champion { comp, team, season } => vec![Part::l(c.team_ref(team), c.team_name(team)), t(" won "), cp(c, comp), t(&format!(" ({})", c.season_label(comp, season)))],
        Promoted { comp, team } => vec![Part::l(c.team_ref(team), c.team_name(team)), t(" earned promotion from "), cp(c, comp)],
        Relegated { comp, team } => vec![Part::l(c.team_ref(team), c.team_name(team)), t(" were relegated from "), cp(c, comp)],
        Interest { player, club } => vec![cl(c, club), t(" showed interest in "), pl(c, player)],
        BidRejected { player, club, fee } => vec![cl(c, club), t(" had a bid of "), Part::money(fee), t(" for "), pl(c, player), t(" rejected")],
        BidAccepted { player, club, fee } => vec![cl(c, club), t(" had a bid of "), Part::money(fee), t(" for "), pl(c, player), t(" accepted")],
        TransferListed { player, club } => vec![cl(c, club), t(" listed "), pl(c, player), t(" for transfer")],
        Award { player, comp, award, season } => {
            let a = match award {
                AwardKind::PlayerOfSeason => "Player of the Season",
                AwardKind::YoungPlayerOfSeason => "Young Player of the Season",
                AwardKind::TopScorer => "top scorer",
                AwardKind::TeamOfSeason => "the Team of the Season",
                AwardKind::PlayerOfMonth => "Player of the Month",
            };
            vec![pl(c, player), t(&format!(" won {a} in ")), cp(c, comp), t(&format!(" ({})", c.season_label(comp, season)))]
        }
        CallUp { player } => vec![pl(c, player), t(" was called up by the national team")],
    }
}
