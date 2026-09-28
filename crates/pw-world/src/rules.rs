//! Explainable rule checks (02 §3): every refusal carries reasons the UI can
//! show ("why can't I play/sign?").

use pw_core::{ClubId, CompId, Date, Money, NationId, PlayerId};
use pw_data::RuleProfile;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::comp::{CompKind, CompRules};
use crate::player::PlayerStatus;
use crate::world::World;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Reason {
    WindowClosed,
    SquadFull { max: u8 },
    Suspended { matches: u8 },
    Injured { days: u16 },
    Retired,
    TooYoung { min: u8 },
    MinorAbroad,
    AlreadyAtClub,
    WorkPermit { points: u8, needed: u8 },
    HomegrownQuota { min: u8 },
    ForeignQuota { max: u8 },
    LoanLimit { max: u8 },
    ForeignLoan,
    TooManyRegistrations { max: u8 },
    ContractTooLong { max: u8 },
    CupTied,
    NotEligibleAge,
}

impl Reason {
    pub fn text(&self) -> String {
        match *self {
            Reason::WindowClosed => "The registration window is closed.".into(),
            Reason::SquadFull { max } => format!("The squad is at its limit of {max} players."),
            Reason::Suspended { matches } => format!("Suspended for {matches} more match(es)."),
            Reason::Injured { days } => format!("Injured, about {days} days out."),
            Reason::Retired => "Retired.".into(),
            Reason::TooYoung { min } => format!("Players must be {min} to sign a professional contract."),
            Reason::MinorAbroad => "International transfers of players under 18 are not permitted.".into(),
            Reason::AlreadyAtClub => "Already registered with this club.".into(),
            Reason::WorkPermit { points, needed } => format!("A work permit was refused ({points} points, {needed} needed)."),
            Reason::HomegrownQuota { min } => format!("The squad must keep at least {min} homegrown players."),
            Reason::ForeignQuota { max } => format!("The squad already has the maximum of {max} foreign players."),
            Reason::LoanLimit { max } => format!("The club already has the maximum of {max} loans."),
            Reason::ForeignLoan => "Loans are only allowed between clubs in the same country.".into(),
            Reason::TooManyRegistrations { max } => format!("A player may only register with {max} clubs in a season."),
            Reason::ContractTooLong { max } => format!("Contracts may not run longer than {max} years."),
            Reason::CupTied => "Cup-tied: already played in this competition for another club.".into(),
            Reason::NotEligibleAge => "Not eligible for this age group.".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RuleOutcome {
    pub reasons: SmallVec<[Reason; 2]>,
}

impl RuleOutcome {
    #[inline]
    pub fn allowed(&self) -> bool {
        self.reasons.is_empty()
    }

    #[inline]
    fn deny(&mut self, r: Reason) {
        self.reasons.push(r);
    }
}

pub const MIN_PRO_AGE: u8 = 16;
pub const MIN_ACADEMY_AGE: u8 = 14;

pub fn max_contract_years(age: u32) -> u8 {
    if age < 18 { 3 } else { 5 }
}

pub fn can_play(w: &World, p: PlayerId, _rules: &CompRules) -> RuleOutcome {
    let h = &w.players.hot[p];
    let mut o = RuleOutcome::default();
    if h.status == PlayerStatus::Retired {
        o.deny(Reason::Retired);
    }
    if h.injury != 0 {
        o.deny(Reason::Injured { days: h.injury_days });
    }
    if h.ban > 0 {
        o.deny(Reason::Suspended { matches: h.ban });
    }
    o
}

/// Can `club` register `p` today (permanent or loan move)?
pub fn can_sign(w: &World, club: ClubId, p: PlayerId, today: Date) -> RuleOutcome {
    let mut o = RuleOutcome::default();
    let h = &w.players.hot[p];
    let c = &w.players.cold[p];
    let person = &w.people[c.person];
    let buyer = &w.clubs[club];
    if h.club == club {
        o.deny(Reason::AlreadyAtClub);
    }
    if h.status == PlayerStatus::Retired {
        o.deny(Reason::Retired);
    }
    let free_agent = h.status == PlayerStatus::FreeAgent;
    if !free_agent && !w.nations[buyer.nation].season.window_open(today) {
        o.deny(Reason::WindowClosed);
    }
    let age = person.age(today);
    if age < u32::from(MIN_PRO_AGE) {
        o.deny(Reason::TooYoung { min: MIN_PRO_AGE });
    }
    if age < 18 && person.nation != buyer.nation {
        let from_nation = if h.club.is_some() { w.clubs[h.club].nation } else { person.nation };
        if from_nation != buyer.nation {
            o.deny(Reason::MinorAbroad);
        }
    }
    let max = w.data.tuning.squad.first_team_max;
    let size = w.clubs[club].teams.first().map_or(0, |&t| w.teams[t].squad.len());
    if size >= usize::from(max) + 4 {
        o.deny(Reason::SquadFull { max });
    }
    registration(w, club, p, today, &mut o);
    o
}

// ------------------------------------------------------------------ profiles

/// The rules profile a nation's football runs under.
pub fn profile(w: &World, nation: NationId) -> RuleProfile {
    if nation.is_none() {
        return RuleProfile::default();
    }
    let n = &w.nations[nation];
    let i = w.data.rules.profile_for(&n.code, n.confed.code());
    w.data.rules.get(i)
}

fn bloc(w: &World, nation: NationId) -> Option<String> {
    if nation.is_none() {
        return None;
    }
    let n = &w.nations[nation];
    if let Some(b) = w.data.rules.bloc_of(&n.code) {
        return Some(b.to_string());
    }
    let prof = profile(w, nation);
    (!prof.bloc.is_empty()).then_some(prof.bloc)
}

/// Is `p` a foreigner in `nation`'s football (nationality and blocs)?
pub fn is_foreign(w: &World, p: PlayerId, nation: NationId) -> bool {
    let person = &w.people[w.players.cold[p].person];
    if person.nation == nation || (person.nation2.is_some() && person.nation2 == nation) {
        return false;
    }
    match bloc(w, nation) {
        Some(b) => bloc(w, person.nation).as_deref() != Some(b.as_str()) && bloc(w, person.nation2).as_deref() != Some(b.as_str()),
        None => true,
    }
}

/// Years a player spent registered with `club` (or any club in `nation`) between
/// the homegrown ages, from their recorded spells.
pub fn homegrown_years(w: &World, p: PlayerId, club: ClubId, nation: NationId, prof: &RuleProfile) -> f32 {
    let dob = w.people[w.players.cold[p].person].dob;
    let from = dob.add_months(12 * i32::from(prof.homegrown_age_from));
    let to = dob.add_months(12 * i32::from(prof.homegrown_age_to) + 12);
    let mut days = 0;
    if let Some(spells) = w.history.spells.get(&p) {
        for s in spells {
            let counts = if club.is_some() { s.club == club } else { s.club.is_some() && w.clubs[s.club].nation == nation };
            if !counts {
                continue;
            }
            let a = s.from.max(from);
            let b = s.to.unwrap_or(w.date).min(to);
            days += a.days_until(b).max(0);
        }
    }
    if days == 0 && w.players.cold[p].youth_club == club && club.is_some() {
        // Imported worlds without spell history: trust the recorded youth club.
        return f32::from(prof.homegrown_years);
    }
    days as f32 / 365.0
}

pub fn is_homegrown(w: &World, p: PlayerId, club: ClubId) -> bool {
    let nation = w.clubs[club].nation;
    let prof = profile(w, nation);
    homegrown_years(w, p, ClubId::NONE, nation, &prof) >= f32::from(prof.homegrown_years)
}

/// Points toward a work permit (a generic points model, 02 §3): international
/// standing, fee and wage level, the strength of the league played in.
pub fn work_permit_points(w: &World, p: PlayerId, fee: Money, wage: Money) -> u8 {
    let c = &w.players.cold[p];
    let person = &w.people[c.person];
    let nation_rep = if person.nation.is_some() { f32::from(w.nations[person.nation].reputation) } else { 0.0 };
    let caps = f32::from(c.caps.min(40));
    let intl = (caps / 2.0).min(10.0) * (nation_rep / 8000.0).min(1.0) * 1.5;
    let fee_p = if fee >= 20_000_000 { 6.0 } else if fee >= 8_000_000 { 4.0 } else if fee >= 2_000_000 { 2.0 } else { 0.0 };
    let wage_p = if wage >= 60_000 { 6.0 } else if wage >= 25_000 { 4.0 } else if wage >= 8_000 { 2.0 } else { 0.0 };
    let club = w.players.hot[p].club;
    let league_p = if club.is_some() && w.clubs[club].league.is_some() {
        let comp = &w.comps[w.clubs[club].league];
        (f32::from(comp.reputation) / 1000.0).min(8.0) / f32::from(comp.tier.max(1))
    } else {
        0.0
    };
    (intl + fee_p + wage_p + league_p).round().clamp(0.0, 40.0) as u8
}

fn registration(w: &World, club: ClubId, p: PlayerId, today: Date, o: &mut RuleOutcome) {
    let nation = w.clubs[club].nation;
    let prof = profile(w, nation);
    let person = &w.people[w.players.cold[p].person];
    let age = person.age(today);
    let foreign = is_foreign(w, p, nation);
    // Minimum professional age for this nation (replaces the generic constant).
    if age < u32::from(prof.min_pro_age) && !o.reasons.iter().any(|r| matches!(r, Reason::TooYoung { .. })) {
        o.deny(Reason::TooYoung { min: prof.min_pro_age });
    }
    // Minors: bloc exception lets 16–17-year-olds move within the bloc.
    if age < u32::from(prof.minor_age) && foreign {
        let same_bloc = bloc(w, nation).is_some() && bloc(w, nation) == bloc(w, person.nation);
        let min = if same_bloc { prof.bloc_minors_min_age } else { prof.minors_abroad_min_age };
        if age < u32::from(min) && !o.reasons.contains(&Reason::MinorAbroad) {
            o.deny(Reason::MinorAbroad);
        }
    }
    // Work permits for foreigners outside the bloc.
    if prof.work_permit && foreign {
        let value = w.players.cold[p].value;
        let wage = w.players.cold[p].contract.current_wage(today);
        let pts = work_permit_points(w, p, value, wage);
        if pts < prof.work_permit_min_points {
            o.deny(Reason::WorkPermit { points: pts, needed: prof.work_permit_min_points });
        }
    }
    let first = w.clubs[club].teams.first().copied();
    let squad: Vec<PlayerId> = first.map(|t| w.teams[t].squad.clone()).unwrap_or_default();
    let senior = |x: &PlayerId| w.age(*x) > u32::from(prof.u21_exempt_age);
    if prof.foreign_squad_max > 0 && foreign {
        let n = squad.iter().filter(|x| senior(x) && is_foreign(w, **x, nation)).count();
        if n >= usize::from(prof.foreign_squad_max) {
            o.deny(Reason::ForeignQuota { max: prof.foreign_squad_max });
        }
    }
    if prof.homegrown_min > 0 && age > u32::from(prof.u21_exempt_age) && !is_homegrown(w, p, club) {
        let non_hg = squad.iter().filter(|x| senior(x) && !is_homegrown(w, **x, club)).count();
        let room = usize::from(prof.squad_max.saturating_sub(prof.homegrown_min));
        if non_hg >= room {
            o.deny(Reason::HomegrownQuota { min: prof.homegrown_min });
        }
    }
    // Registrations this season (a season is the buyer nation's current one).
    let season_start = w.nations[nation].season.start.add_days(-60);
    let regs = w.history.spells.get(&p).map_or(0, |s| s.iter().filter(|x| x.from >= season_start).count());
    if regs >= usize::from(prof.registrations_per_season) {
        o.deny(Reason::TooManyRegistrations { max: prof.registrations_per_season });
    }
}

/// Extra checks for a loan into `club` from `parent`.
pub fn can_loan(w: &World, club: ClubId, parent: ClubId, p: PlayerId, today: Date) -> RuleOutcome {
    let mut o = can_sign(w, club, p, today);
    let nation = w.clubs[club].nation;
    let prof = profile(w, nation);
    if prof.loans_domestic_only && parent.is_some() && w.clubs[parent].nation != nation {
        o.deny(Reason::ForeignLoan);
    }
    let loans_in = w.clubs[club].teams.iter().flat_map(|&t| w.teams[t].squad.iter()).filter(|&&x| w.players.cold[x].loan.as_ref().is_some_and(|l| l.club == club)).count();
    if prof.max_loans_in > 0 && loans_in >= usize::from(prof.max_loans_in) {
        o.deny(Reason::LoanLimit { max: prof.max_loans_in });
    }
    // Remove the "already at club" style duplicate the parent check can't cause.
    o
}

pub fn max_contract_years_for(w: &World, nation: NationId, age: u32) -> u8 {
    let prof = profile(w, nation);
    if age < u32::from(prof.minor_age) { prof.max_contract_years_minor } else { prof.max_contract_years }
}

/// Has the player already played in this knockout competition for another club this season?
pub fn cup_tied(w: &World, p: PlayerId, comp: CompId, club: ClubId) -> bool {
    if comp.is_none() {
        return false;
    }
    let c = &w.comps[comp];
    if !matches!(c.kind, CompKind::Cup | CompKind::Continental) {
        return false;
    }
    let nation = if c.nation.is_some() { c.nation } else { w.clubs[club].nation };
    if !profile(w, nation).cup_tied {
        return false;
    }
    w.stats.for_player(p).any(|l| l.comp == comp && l.club != club && l.apps > 0)
}

/// May `p` play for `club` in `comp` today?
pub fn match_eligible(w: &World, p: PlayerId, comp: CompId, club: ClubId) -> bool {
    !cup_tied(w, p, comp, club)
}
