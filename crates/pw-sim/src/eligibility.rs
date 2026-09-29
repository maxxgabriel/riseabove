//! Who may play for a representative side, judged against rules that are data (locked design; the India brief, item 12).
//!
//! Two bodies are judged here with the same shape of answer (`pw_world::eligibility::Judgement`: rule, evidence, outcome, reason):
//!
//! * **state sides**: the world's `Ecosystem::eligibility` (loaded from the pack): an age window, which grounds count (birth,
//!   club, institution, residence), how long residence must be, whether top-division players are available, one state a year;
//! * **national sides**: the nation's own `RuleProfile` (`national_bases`, residence years, when a player tied by caps may switch).
//!
//! Nothing in the code names a country. A pack changes who qualifies by changing its data.

use pw_core::{NationId, PlayerId, RegionId};
use pw_world::ecosystem::Basis;
use pw_world::eligibility::{Body, Evidence, Judgement, Reason, Rule};
use pw_world::{PlayerStatus, World};
use smallvec::SmallVec;

// ------------------------------------------------------------------------------------------------------------------ state sides

fn tier_of(w: &World, club: pw_core::ClubId) -> u8 {
    let l = w.clubs[club].league;
    if l.is_some() { w.comps[l].tier } else { 9 }
}

fn rule_of(b: Basis) -> Rule {
    match b {
        Basis::Birth => Rule::Birth,
        Basis::Club => Rule::Club,
        Basis::Institution => Rule::Institution,
        Basis::Residence => Rule::Residence,
    }
}

/// The state that `basis` ties a player to, and the number behind it (years of residence, else 0). `NONE` when it does not apply.
fn ground(w: &World, p: PlayerId, basis: Basis) -> (RegionId, i32) {
    let eco = &w.ext.ecosystem;
    match basis {
        Basis::Birth => eco.story.get(&p).map_or((RegionId::NONE, 0), |s| (eco.state_of(s.home), 0)),
        Basis::Club => {
            let club = w.players.hot[p].club;
            if club.is_some() { (eco.state_of(eco.region_of_club(club)), 0) } else { (RegionId::NONE, 0) }
        }
        Basis::Institution => match w.minor.member_of.get(&p).and_then(|i| eco.inst.get(i)) {
            Some(prof) => (eco.state_of(prof.region), 0),
            None => (RegionId::NONE, 0),
        },
        Basis::Residence => match eco.story.get(&p) {
            Some(s) => {
                let st = eco.state_of(s.dev);
                let since = eco.route(p).iter().find(|x| eco.state_of(x.region) == st).map(|x| x.date);
                let years = since.map_or(0, |d| d.days_until(w.date) / 365);
                (if years >= i32::from(eco.eligibility.residence_years) { st } else { RegionId::NONE }, years)
            }
            None => (RegionId::NONE, 0),
        },
    }
}

/// Every state a player may represent on the world's grounds, in the order of preference, with the ground. (The player's own state is
/// the first that applies.)
pub fn state_grounds(w: &World, p: PlayerId) -> Vec<(RegionId, Basis)> {
    let mut out: Vec<(RegionId, Basis)> = Vec::new();
    for &basis in &w.ext.ecosystem.eligibility.bases {
        let (r, _) = ground(w, p, basis);
        if r.is_some() && !out.iter().any(|x| x.0 == r) {
            out.push((r, basis));
        }
    }
    out
}

/// May this player play for this state's side in `year`, and why or why not. `fill`: the state is short of players and may call anyone who
/// qualifies for it; otherwise only players for whom it is their own (first) state.
pub fn judge_state(w: &World, p: PlayerId, state: RegionId, year: i32, fill: bool) -> Judgement {
    let body = Body::State(state);
    let rules = &w.ext.ecosystem.eligibility;
    let mut ev: SmallVec<[Evidence; 6]> = SmallVec::new();
    let age = w.age(p) as i32;
    ev.push(Evidence { rule: Rule::Age, holds: age >= i32::from(rules.min_age) && age <= i32::from(rules.max_age), detail: age });
    if age < i32::from(rules.min_age) {
        return Judgement::deny(body, Reason::TooYoung, ev);
    }
    if age > i32::from(rules.max_age) {
        return Judgement::deny(body, Reason::TooOld, ev);
    }
    if rules.one_state_per_year {
        let other = w.ext.ecosystem.represented.get(&p).is_some_and(|&(y, r)| y == year && r != state);
        ev.push(Evidence { rule: Rule::OneStatePerYear, holds: !other, detail: 0 });
        if other {
            return Judgement::deny(body, Reason::RepresentedAnother, ev);
        }
    }
    let club = w.players.hot[p].club;
    if rules.exclude_top_division {
        let top = club.is_some() && tier_of(w, club) == 1;
        ev.push(Evidence { rule: Rule::TopDivision, holds: !top, detail: 0 });
        if top {
            return Judgement::deny(body, Reason::TopDivision, ev);
        }
    }
    let mut first: Option<Rule> = None;
    let mut own_first = None;
    for &basis in &rules.bases {
        let (r, detail) = ground(w, p, basis);
        let holds = r == state;
        ev.push(Evidence { rule: rule_of(basis), holds, detail });
        if r.is_some() && own_first.is_none() {
            own_first = Some(r);
        }
        if holds && first.is_none() {
            first = Some(rule_of(basis));
        }
    }
    match first {
        Some(rule) if fill || own_first == Some(state) => Judgement::allow(body, rule, ev),
        // Qualifies, but another state has the first claim and this one is not short of players.
        Some(_) => Judgement::deny(body, Reason::NoGround, ev),
        None => Judgement::deny(body, Reason::NoGround, ev),
    }
}

// ------------------------------------------------------------------------------------------------------------------ national sides

fn person_of(w: &World, p: PlayerId) -> pw_core::PersonId {
    w.players.cold[p].person
}

/// Whether a competitive senior cap for another nation still lets him play for `nation` under its federation's rules.
fn may_switch(w: &World, p: PlayerId, nation: NationId, tied_to: NationId) -> bool {
    let prof = w.ext.scenario.national_rules(&w.nations[nation].code);
    let Some(cap) = w.intl.caps.get(&p).and_then(|v| v.iter().find(|c| c.nation == tied_to && c.level == pw_world::intl::Level::Senior)) else { return false };
    let years_since = cap.last.days_until(w.date) / 365;
    cap.competitive <= u16::from(prof.switch_max_caps) && years_since >= i32::from(prof.switch_wait_years)
}

/// May this player be picked by this nation's national sides, and on what ground.
pub fn judge_national(w: &World, p: PlayerId, nation: NationId) -> Judgement {
    let body = Body::National(nation);
    let mut ev: SmallVec<[Evidence; 6]> = SmallVec::new();
    if let Some(tied) = w.intl.locked_to(p) {
        let switch = tied != nation && may_switch(w, p, nation, tied);
        ev.push(Evidence { rule: Rule::CapTied, holds: tied == nation, detail: i32::from(w.intl.senior_caps(p)) });
        if tied == nation {
            return Judgement::allow(body, Rule::CapTied, ev);
        }
        if !switch {
            return Judgement::deny(body, Reason::CapTied, ev);
        }
    }
    if let Some(&d) = w.intl.declared.get(&p) {
        ev.push(Evidence { rule: Rule::Declared, holds: d == nation, detail: 0 });
        if d == nation {
            return Judgement::allow(body, Rule::Declared, ev);
        }
        return Judgement::deny(body, Reason::Declared, ev);
    }
    let who = person_of(w, p);
    let person = &w.people[who];
    let prof = w.ext.scenario.national_rules(&w.nations[nation].code);
    let mut first: Option<Rule> = None;
    for b in &prof.bases {
        let (rule, holds, detail) = match b.as_str() {
            "nationality" => (Rule::Nationality, person.nation == nation || person.nation2 == nation, 0),
            "birth" => {
                let born = w.ext.ecosystem.story.get(&p).is_some_and(|s| s.home.is_some() && w.ext.ecosystem.regions[s.home].nation == nation);
                (Rule::Birth, born, 0)
            }
            "parent" => (Rule::Parent, w.lives.get(who).is_some_and(|l| l.household.parents.nation == nation), 0),
            "residence" => {
                let years = w.lives.get(who).filter(|l| l.home == nation).map_or(0, |l| l.years_here(w.date) as i32);
                (Rule::Residence, years >= i32::from(prof.residence_years), years)
            }
            _ => continue,
        };
        ev.push(Evidence { rule, holds, detail });
        if holds && first.is_none() {
            first = Some(rule);
        }
    }
    match first {
        Some(rule) => Judgement::allow(body, rule, ev),
        None => Judgement::deny(body, Reason::NoGround, ev),
    }
}

/// Nations that could pick him now: the sporting nations he is tied to or has declared for, else every nation whose rules he satisfies,
/// the nation of his passport first.
pub fn eligible_nations(w: &World, p: PlayerId) -> SmallVec<[NationId; 2]> {
    let mut out: SmallVec<[NationId; 2]> = SmallVec::new();
    // A declaration binds to one nation. A tie by caps is judged below: it allows the nation he is tied to and denies the others, unless
    // his federation's rules let him switch.
    if let Some(&n) = w.intl.declared.get(&p) {
        out.push(n);
        return out;
    }
    let who = person_of(w, p);
    let person = &w.people[who];
    let mut candidates: SmallVec<[NationId; 4]> = SmallVec::new();
    if let Some(n) = w.intl.locked_to(p) {
        candidates.push(n);
    }
    for n in [person.nation, person.nation2] {
        if n.is_some() && !candidates.contains(&n) {
            candidates.push(n);
        }
    }
    if let Some(l) = w.lives.get(who) {
        for n in [l.household.parents.nation, l.home] {
            if n.is_some() && !candidates.contains(&n) {
                candidates.push(n);
            }
        }
    }
    if let Some(s) = w.ext.ecosystem.story.get(&p)
        && s.home.is_some()
    {
        let n = w.ext.ecosystem.regions[s.home].nation;
        if n.is_some() && !candidates.contains(&n) {
            candidates.push(n);
        }
    }
    for n in candidates {
        if !out.contains(&n) && judge_national(w, p, n).eligible {
            out.push(n);
        }
    }
    out
}

/// A player who has retired or been released is not available to any side.
pub fn available(w: &World, p: PlayerId) -> bool {
    !matches!(w.players.hot[p].status, PlayerStatus::Retired)
}
