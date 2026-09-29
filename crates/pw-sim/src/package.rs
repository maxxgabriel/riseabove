//! Contract packages (locked design 5.1-5.13).
//!
//! A contract is a package, not a wage. What a club offers follows its finances, its wage structure, how it reads the player, the role
//! it has in mind, the risks it sees (age, injuries, settling in) and its own strategy; what a player accepts follows what he and his
//! agent care about, and the same gross pay is not the same money in every country. Negotiation trades one dimension for another: a
//! lower wage for status, a signing fee for a release clause the club will not give, a shorter deal for an option. Each side moves
//! on the levers that give it most for what they cost the other.

use pw_core::{ClubId, Hidden, Money, PlayerId, PosGroup};
use pw_world::contract::{Options, SquadStatus, Trigger};
use pw_world::governance::TransferStyle;
use pw_world::negotiation::{Lever, Priority, Terms};
use pw_world::{PlayerStatus, World};

use crate::{adaptation, consider, market};

const GAMES: f32 = 38.0;

// ------------------------------------------------------------------ what the club can expect of a season

/// The chances a club has of the things bonuses are paid for, from where its reputation puts it among its league's clubs.
#[derive(Clone, Copy, Debug)]
pub struct Odds {
    pub title: f32,
    pub promotion: f32,
    pub continental: f32,
    pub relegation: f32,
}

pub fn odds(w: &World, club: ClubId) -> Odds {
    let league = w.clubs[club].league;
    if league.is_none() {
        return Odds { title: 0.0, promotion: 0.0, continental: 0.0, relegation: 0.1 };
    }
    let comp = &w.comps[league];
    let mut reps: Vec<u16> = comp.state.entrants.iter().map(|&t| w.clubs[w.teams[t].club].reputation).collect();
    reps.sort_by(|a, b| b.cmp(a));
    let mine = w.clubs[club].reputation;
    let rank = reps.iter().position(|&r| r <= mine).unwrap_or(reps.len()) + 1;
    let n = reps.len().max(2);
    let places = comp.continental.first().map_or(0, |&(_, p)| usize::from(p));
    let top = comp.tier <= 1;
    Odds {
        title: if top { (0.55 * (-0.9 * (rank as f32 - 1.0)).exp()).clamp(0.0, 0.6) } else { 0.0 },
        promotion: if !top && comp.promote > 0 { if rank <= usize::from(comp.promote) + 1 { 0.35 } else { 0.08 } } else { 0.0 },
        continental: if places > 0 { if rank <= places { 0.6 } else if rank <= places + 2 { 0.25 } else { 0.03 } } else { 0.0 },
        relegation: if comp.relegate > 0 { if rank + usize::from(comp.relegate) + 2 > n { 0.35 } else { 0.04 } } else { 0.0 },
    }
}

/// What a player of this kind can be expected to do in a season at a given share of minutes: (goals, assists, clean sheets).
fn output(w: &World, p: PlayerId, share: f32) -> (f32, f32, f32) {
    let apps = GAMES * share;
    match w.players.cold[p].best_pos.group() {
        PosGroup::Att => (apps * 0.35, apps * 0.20, 0.0),
        PosGroup::Mid => (apps * 0.10, apps * 0.15, 0.0),
        PosGroup::Def => (apps * 0.03, apps * 0.05, apps * 0.30),
        PosGroup::Gk => (0.0, 0.0, apps * 0.32),
    }
}

// ------------------------------------------------------------------ the player's side

/// What a player cares about, normalised to sum to one. Young and ambitious: a path and minutes. Older: years and money in hand. A
/// family pulls towards stability (locked design 5.10).
pub fn priorities(w: &World, p: PlayerId) -> [f32; 13] {
    let c = &w.players.cold[p];
    let hid = &w.people[c.person].hidden;
    let age = w.age_years(p);
    let ambition = hid.f(Hidden::Ambition) / 20.0;
    let loyalty = hid.f(Hidden::Loyalty) / 20.0;
    let adapt = hid.f(Hidden::Adaptability) / 20.0;
    let prof = hid.f(Hidden::Professionalism) / 20.0;
    let older = ((age - 26.0) / 9.0).clamp(0.0, 1.0);
    let young = ((23.0 - age) / 6.0).clamp(0.0, 1.0);
    let family = if w.lives[c.person].partner().is_some() || w.lives[c.person].household.children > 0 { 1.0 } else { 0.0 };
    let fame = f32::from(c.rep.world) / 10_000.0;
    let mut x = [0.0f32; 13];
    x[Priority::GuaranteedMoney.idx()] = 0.8 + 0.6 * older + 0.3 * (1.0 - ambition);
    x[Priority::MaxWage.idx()] = 0.8 + 0.5 * (1.0 - loyalty) + 0.3 * (1.0 - prof);
    x[Priority::Security.idx()] = 0.5 + 0.9 * older + 0.3 * family;
    x[Priority::PlayingTime.idx()] = 0.6 + 0.8 * ambition * (0.5 + 0.5 * (1.0 - older));
    x[Priority::ChampionsLeague.idx()] = 0.4 + 0.9 * ambition;
    x[Priority::Prestige.idx()] = 0.4 + 0.6 * ambition + 0.3 * fame;
    x[Priority::ReleaseClause.idx()] = 0.3 + 0.8 * ambition * (1.0 - loyalty);
    x[Priority::Flexibility.idx()] = 0.4 + 0.6 * ambition;
    x[Priority::Family.idx()] = 0.3 + 1.2 * family;
    x[Priority::City.idx()] = 0.4;
    x[Priority::Language.idx()] = 0.3 + 0.6 * (1.0 - adapt);
    x[Priority::Title.idx()] = 0.4 + 0.7 * ambition;
    x[Priority::Development.idx()] = 0.2 + 1.2 * young * ambition;
    let sum: f32 = x.iter().sum();
    for v in &mut x {
        *v /= sum;
    }
    x
}

/// The two things that matter most to him.
pub fn top_priorities(x: &[f32; 13]) -> [Priority; 2] {
    let mut ix: Vec<usize> = (0..13).collect();
    ix.sort_by(|&a, &b| x[b].total_cmp(&x[a]).then(a.cmp(&b)));
    [Priority::ALL[ix[0]], Priority::ALL[ix[1]]]
}

/// What an agent gets out of each lever, for himself, 0..1: commission follows wages, signing fees and bonuses, and a release clause or
/// a short deal means another deal sooner (locked design 5.13). Not the same as what the player gets.
fn agent_stake(lever: Lever) -> f32 {
    match lever {
        Lever::Wage => 0.8,
        Lever::SigningFee => 1.0,
        Lever::ReleaseClause => 0.7,
        Lever::Years => 0.4,
        Lever::Bonuses => 0.6,
        Lever::Loyalty => 0.3,
        Lever::PlayerOption => 0.5,
        Lever::YearlyRise => 0.4,
        Lever::Status => 0.2,
        Lever::ClubOption => 0.3,
        Lever::RelegationProtection => 0.2,
    }
}

/// How much of the going rate for a gross wage the player really keeps and can spend where the club is: tax and cost of living.
fn purchasing_power(w: &World, club: ClubId) -> f64 {
    let e = &w.nations[w.clubs[club].nation].env;
    ((1.0 - f64::from(e.tax) / 100.0) / (f64::from(e.living) / 100.0)) / 0.7
}

/// What a package is worth to the player, in money over the whole deal (locked design 5.3, 5.10, 5.12).
pub fn player_utility(w: &World, p: PlayerId, club: ClubId, t: &Terms, pr: &[f32; 13]) -> f64 {
    let years = f64::from(t.years.max(1));
    let wage_year = t.wage as f64 * 52.0;
    let weight = |k: Priority| f64::from(pr[k.idx()]) * 13.0;
    let (security, flex, guaranteed) = (weight(Priority::Security), weight(Priority::Flexibility), weight(Priority::GuaranteedMoney));
    let o = odds(w, club);
    let status = t.status.unwrap_or(SquadStatus::Squad);
    let share = f64::from(status.expected_minutes()).max(0.25);
    let (goals, assists, sheets) = output(w, p, share as f32);
    let apps = f64::from(GAMES) * share;
    let caps = (crate::intl::standing(w, p) * 6.0) as f64;

    // Guaranteed pay, with later years worth more to someone who wants security and less to someone who wants to move on.
    let mut wages = 0.0;
    for y in 0..t.years.max(1) {
        let rise = (1.0 + f64::from(t.yearly_rise) / 100.0).powi(i32::from(y));
        let disc = (1.0 - 0.06 * f64::from(y) * (flex / 3.0).min(1.5) + 0.04 * f64::from(y) * (security / 3.0).min(1.5)).clamp(0.6, 1.1);
        wages += wage_year * rise * disc;
    }
    // Relegation takes a cut, or opens a way out.
    wages *= 1.0 - f64::from(o.relegation) * f64::from(t.relegation_cut) / 100.0;
    let fee = t.signing_fee as f64 * (1.0 + 0.3 * (guaranteed / 3.0).min(1.0));
    // Conditional pay is worth less to someone who wants it guaranteed.
    let conditional = (t.appearance_bonus as f64 * apps + t.goal_bonus as f64 * f64::from(goals) + t.assist_bonus as f64 * f64::from(assists) + t.clean_sheet_bonus as f64 * f64::from(sheets)
        + t.title_bonus as f64 * f64::from(o.title)
        + t.promotion_bonus as f64 * f64::from(o.promotion)
        + t.continental_bonus as f64 * f64::from(o.continental)
        + t.cap_bonus as f64 * caps)
        * years;
    let conditional = conditional * (1.0 - 0.15 * (guaranteed / 3.0).min(2.0)).clamp(0.4, 1.0) * 0.9;
    let loyalty = t.loyalty_bonus as f64 * (years - 1.0).max(0.0) * 0.75;
    let mut money = (wages + fee + conditional + loyalty) * purchasing_power(w, club);

    // Rights: a release clause is a way out, an option is who holds the key.
    if t.release_clause > 0 {
        let fair = w.players.cold[p].value.max(1) as f64;
        let usable = if (t.release_clause as f64) <= fair * 4.0 { 1.0 } else { 0.3 };
        money += wage_year * 0.9 * (weight(Priority::ReleaseClause) / 2.0).min(1.5) * usable;
    }
    if t.relegation_release > 0 {
        money += wage_year * f64::from(o.relegation) * 1.5 * (flex / 3.0).min(1.5);
    }
    money += wage_year * 0.15 * f64::from(t.options.player_years) * ((security + flex) / 4.0).min(1.5);
    money -= wage_year * 0.12 * f64::from(t.options.club_years) * (security / 2.5).min(1.5);
    if let Some((_, extra)) = t.options.auto {
        money += wage_year * 0.05 * f64::from(extra) * (security / 3.0).min(1.5);
    }

    // What he was promised, and what the place offers beyond money.
    money += wage_year * 1.2 * weight(Priority::PlayingTime).min(3.0) / 2.0 * (share - 0.35);
    let e_c = w.clubs[club].reputation as f64 / 10_000.0;
    money += wage_year * 0.6 * (weight(Priority::Prestige) / 2.0).min(2.0) * e_c;
    money += wage_year * 0.7 * (weight(Priority::ChampionsLeague) / 2.0).min(2.0) * f64::from(o.continental);
    money += wage_year * 0.9 * (weight(Priority::Title) / 2.0).min(2.0) * f64::from(o.title);
    let who = w.players.cold[p].person;
    let to_nation = w.clubs[club].nation;
    let move_cost = f64::from(consider::household_move_cost(w, who, to_nation).min(1.0));
    money -= wage_year * 0.5 * (weight(Priority::Family) / 2.0).min(2.0) * move_cost;
    let fluent = f64::from(w.lives[who].fluency(to_nation)) / 100.0;
    money += wage_year * 0.25 * (weight(Priority::Language) / 2.0).min(2.0) * (fluent - 0.5);
    let young = w.age(p) <= 23;
    if young {
        let path = f64::from(w.clubs[club].facilities.youth) / 20.0;
        money += wage_year * 0.4 * (weight(Priority::Development) / 2.0).min(2.0) * (path + 0.3 * f64::from(status.expected_minutes()));
    }
    money
}

/// What relocating costs him beyond the wage: the compensation a player asks for a move that takes him a long way (locked design 5.11).
pub fn relocation_demand(w: &World, p: PlayerId, from: ClubId, to: ClubId, wage_year: f64) -> f64 {
    if from.is_some() && w.clubs[from].nation == w.clubs[to].nation {
        return 0.0;
    }
    let d = adaptation::distances(w, p, from, to);
    let x = (d[pw_world::adaptation::Channel::Social.idx()] + d[pw_world::adaptation::Channel::Environment.idx()] + d[pw_world::adaptation::Channel::Mental.idx()]) / 3.0;
    wage_year * 0.15 * f64::from(x)
}

// ------------------------------------------------------------------ the club's side

/// How bad a guarantee looks to a club that has doubts about age, injuries and settling in, 0..1.
fn doubt(w: &World, club: ClubId, p: PlayerId) -> f32 {
    let age = w.age_years(p);
    let age_risk = ((age - 29.0) / 6.0).clamp(0.0, 1.0);
    let from = w.players.hot[p].club;
    let settle = if from.is_some() && from != club { (adaptation::readiness(w, club, p).weeks_to_useful / 26.0).min(1.0) } else { 0.0 };
    let injury = w.dossiers.get(club, p).map_or(0.0, |d| d.risks.iter().filter(|r| r.kind == pw_world::dossier::RiskKind::Injuries).map(|r| r.level).fold(0.0, f32::max));
    (0.4 * age_risk + 0.35 * settle + 0.25 * injury).clamp(0.0, 1.0)
}

/// The band a wage sits in at this club, as a share of the top wage the structure allows for someone of his standing, and how far above
/// the structure it is (0 when within it), for the wage-hierarchy effects (locked design 5.6).
pub fn hierarchy_excess(w: &World, club: ClubId, wage: Money) -> f32 {
    let team = w.clubs[club].first_team();
    let mut wages: Vec<Money> = w.teams[team].squad.iter().map(|&q| w.players.cold[q].contract.current_wage(w.date)).filter(|&x| x > 0).collect();
    if wages.len() < 5 {
        return 0.0;
    }
    wages.sort_unstable();
    let top = wages[(wages.len() * 9 / 10).min(wages.len() - 1)] as f32;
    ((wage as f32 - top * 1.15) / top.max(1.0)).max(0.0)
}

/// What a package costs the club in all: guaranteed pay with a premium for doubt, cash now, expected bonuses, what a clause or an option
/// costs or is worth, and what breaking its own wage structure will cost it later (locked design 5.2, 5.6, 5.17).
pub fn club_cost(w: &World, club: ClubId, p: PlayerId, t: &Terms) -> f64 {
    let years = f64::from(t.years.max(1));
    let wage_year = t.wage as f64 * 52.0;
    let o = odds(w, club);
    let d = f64::from(doubt(w, club, p));
    let status = t.status.unwrap_or(SquadStatus::Squad);
    let share = status.expected_minutes();
    let (goals, assists, sheets) = output(w, p, share.max(0.25));
    let apps = f64::from(GAMES * share.max(0.25));
    let revenue = crate::finance::season_revenue(w, club).max(1) as f64;
    let balance = w.clubs[club].finance.balance as f64;
    let tight = if balance < revenue * 0.3 { 0.25 } else { 0.0 };

    let mut wages = 0.0;
    for y in 0..t.years.max(1) {
        wages += wage_year * (1.0 + f64::from(t.yearly_rise) / 100.0).powi(i32::from(y));
    }
    // A long guarantee on a doubtful player is a risk the club prices in.
    wages *= 1.0 + 0.04 * (years - 1.0) * d * 3.0;
    wages *= 1.0 - f64::from(o.relegation) * f64::from(t.relegation_cut) / 100.0;
    let fee = t.signing_fee as f64 * (1.0 + tight);
    let risk_averse = 1.0 - f64::from(w.boardroom.appetite_of(club)) * 0.4;
    let conditional = (t.appearance_bonus as f64 * apps + t.goal_bonus as f64 * f64::from(goals) + t.assist_bonus as f64 * f64::from(assists) + t.clean_sheet_bonus as f64 * f64::from(sheets)
        + t.title_bonus as f64 * f64::from(o.title)
        + t.promotion_bonus as f64 * f64::from(o.promotion)
        + t.continental_bonus as f64 * f64::from(o.continental)
        + t.cap_bonus as f64 * 3.0)
        * years
        * 0.9
        * risk_averse;
    let loyalty = t.loyalty_bonus as f64 * (years - 1.0).max(0.0);
    let mut cost = wages + fee + conditional + loyalty;

    if t.release_clause > 0 {
        let fair = w.players.cold[p].value.max(1) as f64;
        cost += (fair * 1.5 - t.release_clause as f64).max(0.0) * 0.2 + wage_year * 0.2;
    }
    if t.relegation_release > 0 {
        cost += wage_year * f64::from(o.relegation) * 1.2;
    }
    cost += wage_year * 0.12 * f64::from(t.options.player_years);
    cost -= wage_year * 0.10 * f64::from(t.options.club_years) * (0.6 + d);
    if let Some((_, extra)) = t.options.auto {
        cost += wage_year * 0.03 * f64::from(extra);
    }
    // A promise of status, and a wage above the structure, are commitments to everyone else at the club too.
    cost += wage_year * 0.1 * f64::from((share - 0.35).max(0.0)) * years;
    cost += wage_year * f64::from(hierarchy_excess(w, club, t.wage)) * 0.3 * years;
    cost
}

/// Whether this club will put this dimension on the table at all (locked design 5.15: a club may refuse a clause and pay in other ways).
pub fn club_allows(w: &World, club: ClubId, p: PlayerId, lever: Lever, want: f32, leverage: f32) -> bool {
    let style = w.governance.get(&club).map_or(TransferStyle::Balanced, |g| g.policy.transfer_style);
    match lever {
        Lever::ReleaseClause => {
            let resale_minded = matches!(style, TransferStyle::Develop | TransferStyle::Homegrown | TransferStyle::Value);
            if resale_minded { want > 0.75 } else { want > 0.45 || leverage > 0.6 }
        }
        Lever::PlayerOption => want > 0.7 || leverage > 0.7,
        Lever::Status => want > 0.1,
        Lever::Years => w.age(p) <= 34,
        _ => true,
    }
}

// ------------------------------------------------------------------ levers

fn apply(t: &mut Terms, lever: Lever, amount: f64) {
    let up = |x: Money, k: f64| ((x as f64 * (1.0 + k)) as Money).max(x + 1);
    match lever {
        Lever::Wage => t.wage = up(t.wage, amount),
        Lever::SigningFee => t.signing_fee = if t.signing_fee == 0 { (t.wage as f64 * 4.0 * amount.max(0.5)) as Money } else { up(t.signing_fee, amount) },
        Lever::Status => {
            t.status = Some(match t.status {
                None | Some(SquadStatus::Squad | SquadStatus::ImpactSub | SquadStatus::Fringe | SquadStatus::Backup | SquadStatus::Youngster | SquadStatus::NotNeeded) => SquadStatus::Regular,
                Some(SquadStatus::Regular) => SquadStatus::Important,
                Some(s) => s,
            })
        }
        Lever::ReleaseClause => {}
        Lever::PlayerOption => t.options.player_years = t.options.player_years.max(1),
        Lever::ClubOption => t.options.club_years = 0,
        Lever::Bonuses => {
            t.appearance_bonus = up(t.appearance_bonus.max(t.wage / 20), amount.max(0.2));
            t.goal_bonus = up(t.goal_bonus.max(t.wage / 20), amount.max(0.2));
            t.clean_sheet_bonus = up(t.clean_sheet_bonus.max(t.wage / 20), amount.max(0.2));
        }
        Lever::Loyalty => t.loyalty_bonus = up(t.loyalty_bonus.max(t.wage * 2), amount.max(0.3)),
        Lever::YearlyRise => t.yearly_rise = (t.yearly_rise + 2).min(12),
        Lever::Years => t.years = (t.years + 1).min(6),
        Lever::RelegationProtection => {
            t.relegation_cut = t.relegation_cut.saturating_sub(15);
            if t.relegation_release == 0 {
                t.relegation_release = t.release_clause.max(1_000_000);
            }
        }
    }
}

fn bump(t: &Terms, w: &World, p: PlayerId, lever: Lever) -> Terms {
    let mut x = *t;
    if lever == Lever::ReleaseClause {
        x.release_clause = (w.players.cold[p].value.max(1) as f64 * 2.5) as Money;
    } else {
        apply(&mut x, lever, 0.05);
    }
    x
}

/// Move `t` towards giving the player `gap` more in his own terms, using the levers that give him most per unit the club pays, weighted by
/// what he and his agent care about. Returns the new terms and the levers used, in order.
pub fn close_gap(w: &World, p: PlayerId, club: ClubId, t: &Terms, pr: &[f32; 13], gap: f64, allowed: &dyn Fn(Lever) -> bool) -> (Terms, Vec<Lever>) {
    let mut cur = *t;
    let mut used: Vec<Lever> = Vec::new();
    let agent = w.agents.agent_of(p).map(|a| f64::from(w.agents.list[a].greed) / 20.0 * 0.5).unwrap_or(0.0);
    let mut remaining = gap;
    for _ in 0..5 {
        if remaining <= 0.0 {
            break;
        }
        let u0 = player_utility(w, p, club, &cur, pr);
        let c0 = club_cost(w, club, p, &cur);
        let mut best: Option<(Lever, f64, f64)> = None;
        for lever in Lever::ALL {
            if !allowed(lever) || (used.contains(&lever) && !matches!(lever, Lever::Wage | Lever::SigningFee)) {
                continue;
            }
            let next = bump(&cur, w, p, lever);
            if next == cur {
                continue;
            }
            let du = player_utility(w, p, club, &next, pr) - u0;
            let dc = (club_cost(w, club, p, &next) - c0).max(1.0);
            // The agent leans towards what pays the agent.
            let ratio = du / dc * (1.0 - agent + agent * f64::from(agent_stake(lever)) * 2.0);
            if du > 0.0 && best.is_none_or(|b| ratio > b.1) {
                best = Some((lever, ratio, du));
            }
        }
        let Some((lever, _, du)) = best else { break };
        // Take as many steps on this lever as close the gap (bounded), then move on to the next.
        let steps = (remaining / du.max(1.0)).ceil().clamp(1.0, 40.0) as u32;
        let cap = match lever {
            Lever::Wage => 24,
            Lever::SigningFee => 30,
            _ => 1,
        };
        for _ in 0..steps.min(cap) {
            cur = bump(&cur, w, p, lever);
        }
        used.push(lever);
        remaining = gap - (player_utility(w, p, club, &cur, pr) - player_utility(w, p, club, t, pr));
    }
    (cur, used)
}

/// Give up on a dimension the club will not offer and pay for it in others: the club's answer to a request it refuses (locked design 5.4).
pub fn compensate(w: &World, p: PlayerId, club: ClubId, offer: &Terms, pr: &[f32; 13], value_refused: f64, allowed: &dyn Fn(Lever) -> bool) -> (Terms, Vec<Lever>) {
    close_gap(w, p, club, offer, pr, value_refused, allowed)
}

// ------------------------------------------------------------------ the club's opening offer

/// How a club shapes its opening offer, and why.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    Standard,
    /// A young player with room to grow: a long deal, the club's option, protection of value.
    YouthPathway,
    /// An older player: a short guarantee, an option, pay tied to playing.
    Veteran,
    /// Doubts about settling or health: shorter, an option, more of the pay conditional.
    RiskShifted,
    /// Someone the club badly needs and cannot replace: long, rich, no strings.
    DesperateTarget,
    /// A club short of cash: less up front, more conditional.
    CashTight,
}

impl Shape {
    pub const fn label(self) -> &'static str {
        match self {
            Shape::Standard => "an ordinary package",
            Shape::YouthPathway => "a long deal with the club's option to protect its investment",
            Shape::Veteran => "a short guarantee with pay tied to playing",
            Shape::RiskShifted => "a shorter deal with more pay at risk, because of doubts about health or settling in",
            Shape::DesperateTarget => "a long, rich guarantee for someone it cannot do without",
            Shape::CashTight => "little up front and more pay conditional, for want of cash",
        }
    }
}

/// The package a club opens with. Built from what the club knows and wants, not from a wage alone.
pub fn opening(w: &World, club: ClubId, p: PlayerId, base: Terms, urgency: f32, alternatives: usize, want: f32) -> (Terms, Shape) {
    let age = w.age_years(p);
    let (ca, _, pa, _) = crate::scouting::view(w, club, p);
    let d = doubt(w, club, p);
    let revenue = crate::finance::season_revenue(w, club).max(1) as f64;
    let balance = w.clubs[club].finance.balance as f64;
    let rich = balance > revenue * 1.2;
    let tight = balance < revenue * 0.3;
    let g = w.governance.get(&club);
    let o = odds(w, club);
    let rep = w.clubs[club].reputation;
    let mut t = base;
    let max_years = pw_world::rules::max_contract_years_for(w, w.clubs[club].nation, w.age(p));
    let mut shape = Shape::Standard;

    if age <= 22.0 && pa - ca > 12.0 {
        t.years = max_years.min(5);
        t.options.club_years = 1;
        t.yearly_rise = 8;
        t.signing_fee /= 2;
        t.appearance_bonus = t.wage / 20;
        shape = Shape::YouthPathway;
    } else if age >= 31.0 {
        t.years = t.years.min(2);
        t.options.club_years = 1;
        t.options.auto = Some((Trigger::Appearances(25), 1));
        t.appearance_bonus = t.wage / 6;
        t.wage = (t.wage as f32 * 0.96) as Money;
        t.yearly_rise = 0;
        shape = Shape::Veteran;
    } else if d > 0.45 {
        t.years = t.years.min(3);
        t.options.club_years = 1;
        t.wage = (t.wage as f32 * 0.93) as Money;
        t.appearance_bonus = t.wage / 6;
        t.goal_bonus = t.goal_bonus.max(t.wage / 6);
        t.clean_sheet_bonus = t.wage / 8;
        t.signing_fee = t.signing_fee * 6 / 10;
        shape = Shape::RiskShifted;
    }
    if urgency >= 0.8 && alternatives == 0 && want >= 0.6 && t.years >= 3 {
        // No strings, and the money to make him sign.
        t.years = max_years.min(5);
        t.options = Options::default();
        t.wage = (t.wage as f32 * 1.08) as Money;
        t.signing_fee *= 2;
        shape = Shape::DesperateTarget;
    } else if tight {
        t.signing_fee = t.signing_fee * 3 / 10;
        t.appearance_bonus = t.appearance_bonus.max(t.wage / 8);
        t.goal_bonus = t.goal_bonus.max(t.wage / 8);
        if shape == Shape::Standard {
            shape = Shape::CashTight;
        }
    } else if rich && shape == Shape::Standard {
        t.signing_fee = t.signing_fee * 8 / 5;
    }
    // What a club's ambitions put in the package.
    if o.continental > 0.3 || rep >= 6000 {
        t.continental_bonus = t.wage * 4;
    }
    if o.promotion > 0.2 {
        t.promotion_bonus = t.wage * 6;
    }
    if want > 0.55 && o.title > 0.15 {
        t.title_bonus = t.wage * 8;
    }
    let loyal_owner = g.is_some_and(|g| matches!(g.owner.kind, pw_world::club::Ownership::Benefactor | pw_world::club::Ownership::MemberOwned));
    if want > 0.5 && (loyal_owner || w.people[w.players.cold[p].person].hidden.f(Hidden::Loyalty) >= 14.0) {
        t.loyalty_bonus = t.wage * 6;
    }
    // A club in danger of the drop protects itself.
    t.relegation_cut = if o.relegation > 0.25 { 25 } else if o.relegation > 0.1 { 10 } else { 0 };
    if crate::intl::standing(w, p) > 0.4 {
        t.cap_bonus = t.wage / 4;
    }
    (t, shape)
}

/// After signing on terms above the structure, those paid less notice: they will ask for more, and some remember it (locked design 5.6).
pub fn wage_ripples(w: &mut World, club: ClubId, newcomer: PlayerId, wage: Money) {
    let excess = hierarchy_excess(w, club, wage);
    if excess <= 0.0 {
        return;
    }
    let today = w.date;
    let team = w.clubs[club].first_team();
    let n_ca = market::public_view(w, newcomer).0;
    let who = w.players.cold[newcomer].person;
    let squad = w.teams[team].squad.clone();
    for q in squad {
        if q == newcomer {
            continue;
        }
        let c = &w.players.cold[q];
        let earns = c.contract.current_wage(today);
        if earns == 0 || (earns as f32) > wage as f32 * 0.7 || c.status > SquadStatus::Regular || w.players.hot[q].status != PlayerStatus::Active {
            continue;
        }
        // Someone at least as good in the public eye, paid clearly less, minds.
        if market::public_view(w, q).0 + 4.0 < n_ca {
            continue;
        }
        let e = w.market.envy.entry(q).or_insert(0.0);
        *e = (*e + (0.06 + excess * 0.2)).min(0.5);
        let person = w.players.cold[q].person;
        if w.people[person].mind == pw_world::MindKind::Ai {
            w.social.remember(person, who, pw_world::MemoryKind::Rivalry, today, pw_core::EventId::NONE, false, 0.3, 0);
        }
    }
}

/// Monthly: resentment about pay fades.
pub fn monthly(w: &mut World) {
    for v in w.market.envy.values_mut() {
        *v *= 0.95;
    }
    w.market.envy.retain(|_, v| *v > 0.01);
}

/// The player's own valuation of what he would have to be paid to sign the going terms, plus what the move costs him.
pub fn reservation_utility(w: &World, p: PlayerId, club: ClubId, res_wage: Money, years: u8, pr: &[f32; 13]) -> f64 {
    let mut market_terms = Terms::from_contract(&market::new_contract(w, p, club, 1.0), years);
    market_terms.wage = res_wage;
    market_terms.signing_fee = 0;
    market_terms.status = None;
    let from = w.players.hot[p].club;
    player_utility(w, p, club, &market_terms, pr) + relocation_demand(w, p, from, club, res_wage as f64 * 52.0)
}

/// Whether the dossier says this player is a real target (used to size the club's want).
pub fn want(w: &World, club: ClubId, p: PlayerId) -> f32 {
    let (ca, _, pa, _) = crate::scouting::view(w, club, p);
    let ideal = market::ideal_ca(w.clubs[club].reputation);
    ((ca - ideal) / 20.0 + (pa - ca).max(0.0) / 60.0 + consider::club_need_for(w, club, p) * 0.3).clamp(-0.5, 1.0)
}
