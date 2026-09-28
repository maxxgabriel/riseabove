//! Incidents in words: what happened, where, to whom, and what was done.
//! Every sentence is built from the incident record; the pressures behind it
//! are rendered by `why` from the event's causes, not invented here.

use pw_core::PersonId;
use pw_world::incident::{IncidentKind, Location, Response};
use pw_world::World;

use crate::fmt::{club, nation, person};
use crate::pick;

pub fn strength(level: u8) -> &'static str {
    match level {
        0..=39 => "a little",
        40..=89 => "noticeably",
        90..=149 => "strongly",
        _ => "overwhelmingly",
    }
}

pub fn place(l: Location) -> &'static str {
    match l {
        Location::TrainingGround => "at training",
        Location::DressingRoom => "in the dressing room",
        Location::Stadium => "at the ground",
        Location::Travel => "on the way to a match",
        Location::Home => "at home",
        Location::Office => "behind the scenes",
        Location::City => "in town",
        Location::Nationwide => "across the country",
    }
}

/// What happened, as a clause ("that X and Y clashed at training"). `vague`
/// and `loud` shape a second-hand version (never adding facts).
pub fn summary(w: &World, id: u32, vague: bool, loud: bool) -> String {
    describe(w, id, vague, loud, PersonId::NONE)
}

/// The same, from `viewer`'s point of view: they are "you" when they are one of the people involved.
pub fn summary_for(w: &World, id: u32, vague: bool, loud: bool, viewer: PersonId) -> String {
    describe(w, id, vague, loud, viewer)
}

fn describe(w: &World, id: u32, vague: bool, loud: bool, viewer: PersonId) -> String {
    let Some(i) = w.incidents.get(id) else { return "something happened".into() };
    let name = |p: PersonId| if p == viewer && p.is_some() { "you".to_string() } else { person(w, p) };
    let a = i.parties.first().map_or("someone".to_string(), |&p| name(p));
    let b = i.parties.get(1).map_or("someone".to_string(), |&p| name(p));
    let c = club(w, i.club);
    // "you" takes a different verb and possessive from a name.
    let first_is_viewer = viewer.is_some() && i.parties.first() == Some(&viewer);
    let (is, was, has) = if first_is_viewer { ("are", "were", "have") } else { ("is", "was", "has") };
    let a_poss = if first_is_viewer { "your".to_string() } else { format!("{a}'s") };
    match i.kind {
        IncidentKind::TrainingConfrontation => {
            if vague {
                format!("that there was a bust-up at {c} training")
            } else if loud || i.severity >= 75 {
                format!("that {a} and {b} came to blows at training")
            } else {
                format!("that {a} and {b} clashed at training")
            }
        }
        IncidentKind::TacticalDisagreement => {
            if vague { format!("that {a} fell out with the coaching staff") } else { format!("that {a} argued with {b} over tactics") }
        }
        IncidentKind::StormedOut => format!("that {a} stormed out of training"),
        IncidentKind::LateArrival => format!("that {a} turned up late"),
        IncidentKind::EquipmentProblem => format!("that {c} lost a training session to equipment failures"),
        IncidentKind::PitchDamage => format!("that the {c} pitch was damaged"),
        IncidentKind::TravelDelay => format!("that {c} were badly delayed travelling to a match"),
        IncidentKind::Postponement => format!("that a {c} match was postponed"),
        IncidentKind::VisaProblem => format!("that {a} {is} held up by a visa problem"),
        IncidentKind::RegistrationError => format!("that {c} failed to register {a} properly"),
        IncidentKind::PaperworkProblem => format!("that paperwork problems are holding up {c}'s business"),
        IncidentKind::CoachResigned => format!("that {a} walked out on {c}"),
        IncidentKind::StaffPoached => format!("that {a} {was} lured away from {c}"),
        IncidentKind::FamilyEmergency => {
            if vague { format!("that {a} {has} something going on at home") } else { format!("that {a} {has} a family emergency") }
        }
        IncidentKind::RelationshipConflict => format!("that {a} {is} having trouble at home"),
        IncidentKind::Pregnancy => format!("that {a} and {b} are expecting a child"),
        IncidentKind::MovingProblem => format!("that {a_poss} move has been a headache"),
        IncidentKind::Burglary => format!("that {a_poss} home was burgled"),
        IncidentKind::ExamClash => format!("that {a_poss} exams clash with football"),
        IncidentKind::ChildcareClash => format!("that {a} {is} struggling with childcare"),
        IncidentKind::UnexpectedBill => format!("that {a} {was} hit with an unexpected bill"),
        IncidentKind::OwnershipControversy => format!("that {c}'s owners are embroiled in controversy"),
        IncidentKind::SponsorCollapse => format!("that a sponsor of {c} has collapsed"),
        IncidentKind::EconomicDownturn => format!("that the economy in {} has turned down", nation(w, i.nation)),
        IncidentKind::FacilityDamage => format!("that {c}'s facilities were damaged"),
        IncidentKind::TransportDisruption => format!("that transport in {} is disrupted", nation(w, i.nation)),
        IncidentKind::SevereWeather => format!("that severe weather has hit {}", nation(w, i.nation)),
        IncidentKind::FederationDispute => format!("that the federation in {} is at odds with the clubs", nation(w, i.nation)),
        IncidentKind::Investigation => format!("that {c} are under investigation"),
        IncidentKind::SupporterUnrest => format!("that {c} supporters are protesting"),
    }
}

/// The event line ("X and Y clashed at training. Several teammates saw it.").
pub fn sentence(w: &World, id: u32, viewer: PersonId) -> String {
    let Some(i) = w.incidents.get(id) else { return "Something happened.".into() };
    let d = describe(w, id, false, false, viewer);
    let body = capitalise(d.strip_prefix("that ").unwrap_or(&d));
    let seen = !i.witnesses.is_empty() && matches!(i.location, Location::TrainingGround | Location::DressingRoom);
    let tail = if seen { pick(u64::from(id), &[" Several teammates saw it.", " It did not go unnoticed.", " Others were there."]) } else { "" };
    format!("{body}.{tail}")
}

/// What was done about it.
pub fn response(w: &World, id: u32, by: PersonId, r: Response, viewer: PersonId) -> String {
    let who = if by == viewer { "You".to_string() } else { person(w, by) };
    let what = summary(w, id, false, false);
    let what = what.strip_prefix("that ").unwrap_or(&what).to_string();
    match r {
        Response::Protect(p) => format!("{who} took {}'s side after {what}.", person(w, p)),
        other => format!("{who} {} after {what}.", other.label()),
    }
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
