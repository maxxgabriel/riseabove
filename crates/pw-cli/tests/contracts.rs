//! Contract architecture (locked design 5.1-5.19): packages shaped by club strategy, priorities that differ by player, trade-offs
//! between dimensions, options and clauses that do something, wage-hierarchy ripples, and contracts that keep their story.

use pw_core::{ClubId, PlayerId};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::{Sim, clauses, market, package};
use pw_world::boardroom::{ContractFile, Verdict, Voice};
use pw_world::contract::{Options, SquadStatus, Trigger};
use pw_world::negotiation::{Lever, Priority, Terms};
use pw_world::{EventKind, World};

fn world() -> Sim {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 71, Scale::SMALL));
    sim.run(45);
    sim
}

fn base_terms(w: &World, club: ClubId, p: PlayerId) -> Terms {
    let c = market::new_contract(w, p, club, 1.0);
    Terms { signing_fee: c.wage * 2, ..Terms::from_contract(&c, 4) }
}

/// A club and a player who is not its own.
fn club_and_target(w: &World) -> (ClubId, PlayerId) {
    let club = w.clubs.ids().find(|&c| w.clubs[c].manager.is_some() && w.governance.contains_key(&c)).unwrap();
    let other = w.clubs.ids().find(|&c| c != club && w.clubs[c].nation == w.clubs[club].nation).unwrap();
    (club, w.teams[w.clubs[other].first_team()].squad[4])
}

fn set_age(w: &mut World, p: PlayerId, years: i32) {
    let who = w.players.cold[p].person;
    w.people[who].dob = w.date.add_days(-years * 365 - 100);
}

#[test]
fn clubs_shape_the_package_from_what_they_know_and_want() {
    let mut sim = world();
    let (club, p) = club_and_target(&sim.world);
    let w = &mut sim.world;
    let base = base_terms(w, club, p);

    // A young player with room to grow: a long deal, the club's option, steady rises.
    set_age(w, p, 19);
    w.players.cold[p].pa = w.players.cold[p].ca.saturating_add(35);
    w.dossiers.map.remove(&(club, p));
    let (young, shape) = package::opening(w, club, p, base, 0.3, 3, 0.4);
    assert_eq!(shape, package::Shape::YouthPathway);
    assert!(young.years >= 4 && young.options.club_years >= 1 && young.yearly_rise >= 6, "{young:?}");

    // An older one: a short guarantee, the option, and pay tied to playing, with an extension that playing earns.
    set_age(w, p, 33);
    w.players.cold[p].pa = w.players.cold[p].ca;
    let (old, shape) = package::opening(w, club, p, base, 0.3, 3, 0.4);
    assert_eq!(shape, package::Shape::Veteran);
    assert!(old.years <= 2 && old.options.club_years >= 1 && matches!(old.options.auto, Some((Trigger::Appearances(_), 1))), "{old:?}");
    assert!(old.appearance_bonus > young.appearance_bonus, "older players' pay leans on appearances");

    // Someone the club cannot do without and cannot replace: long, rich, no strings.
    set_age(w, p, 26);
    let (desperate, shape) = package::opening(w, club, p, base, 1.0, 0, 0.9);
    assert_eq!(shape, package::Shape::DesperateTarget);
    assert!(desperate.options == Options::default() && desperate.years >= 4 && desperate.signing_fee > base.signing_fee, "{desperate:?}");

    // Cash decides how much is paid now and how much later.
    let revenue = pw_sim::finance::season_revenue(w, club);
    w.clubs[club].finance.balance = revenue * 3;
    let (rich, _) = package::opening(w, club, p, base, 0.3, 3, 0.4);
    w.clubs[club].finance.balance = 0;
    let (poor, _) = package::opening(w, club, p, base, 0.3, 3, 0.4);
    assert!(rich.signing_fee > poor.signing_fee * 2, "{} vs {}", rich.signing_fee, poor.signing_fee);
    assert!(poor.appearance_bonus >= rich.appearance_bonus, "a club short of cash shifts pay to bonuses");
}

#[test]
fn what_a_club_puts_in_the_package_follows_its_ambitions() {
    let mut sim = world();
    let (club, p) = club_and_target(&sim.world);
    let w = &mut sim.world;
    let base = base_terms(w, club, p);
    w.clubs[club].reputation = 9500;
    let (grand, _) = package::opening(w, club, p, base, 0.3, 3, 0.7);
    assert!(grand.continental_bonus > 0, "a big club promises European money");
    w.clubs[club].reputation = 700;
    let (small, _) = package::opening(w, club, p, base, 0.3, 3, 0.7);
    assert_eq!(small.continental_bonus, 0);
    // A club in danger of the drop protects itself.
    let o = package::odds(w, club);
    assert!((0.0..=1.0).contains(&o.relegation) && (0.0..=1.0).contains(&o.title));
    assert!(if o.relegation > 0.25 { small.relegation_cut >= 20 } else { true });
}

#[test]
fn what_a_player_cares_about_changes_with_who_he_is() {
    let mut sim = world();
    let (club, p) = club_and_target(&sim.world);
    let w = &mut sim.world;
    let who = w.players.cold[p].person;
    for (age, ambition) in [(19, 19u8), (34, 3u8)] {
        set_age(w, p, age);
        w.people[who].hidden.set(pw_core::Hidden::Ambition, ambition);
        let x = package::priorities(w, p);
        assert!((x.iter().sum::<f32>() - 1.0).abs() < 1e-4);
        let top = package::top_priorities(&x);
        assert_ne!(top[0], top[1]);
        if age == 19 {
            assert!(x[Priority::PlayingTime.idx()] > x[Priority::Security.idx()] && x[Priority::Development.idx()] > x[Priority::GuaranteedMoney.idx()] * 0.4, "{top:?}");
        } else {
            assert!(x[Priority::Security.idx()] > x[Priority::PlayingTime.idx()] && x[Priority::GuaranteedMoney.idx()] > x[Priority::Development.idx()], "{top:?}");
        }
    }
    let _ = club;
}

#[test]
fn the_same_gross_wage_is_not_the_same_money_everywhere() {
    let mut sim = world();
    let (club, p) = club_and_target(&sim.world);
    let w = &mut sim.world;
    let t = base_terms(w, club, p);
    let pr = package::priorities(w, p);
    let nation = w.clubs[club].nation;
    w.nations[nation].env.tax = 10;
    w.nations[nation].env.living = 70;
    let cheap_and_untaxed = package::player_utility(w, p, club, &t, &pr);
    w.nations[nation].env.tax = 48;
    w.nations[nation].env.living = 140;
    let dear_and_taxed = package::player_utility(w, p, club, &t, &pr);
    assert!(cheap_and_untaxed > dear_and_taxed * 1.5, "{cheap_and_untaxed} vs {dear_and_taxed}");
}

#[test]
fn packages_are_traded_a_lower_wage_for_status_and_refused_clauses_are_paid_for_another_way() {
    let mut sim = world();
    let (club, p) = club_and_target(&sim.world);
    let w = &mut sim.world;
    let who = w.players.cold[p].person;
    let mut t = base_terms(w, club, p);
    t.status = None;
    t.release_clause = 0;

    // An ambitious young player weighs minutes and a path; an older one wants years and money in hand.
    set_age(w, p, 21);
    w.people[who].hidden.set(pw_core::Hidden::Ambition, 19);
    let young = package::priorities(w, p);
    let mut with_status = t;
    with_status.status = Some(SquadStatus::Important);
    with_status.wage = (t.wage as f64 * 0.94) as i64;
    assert!(package::player_utility(w, p, club, &with_status, &young) > package::player_utility(w, p, club, &t, &young), "he takes 6% less for a place in the side");

    set_age(w, p, 34);
    w.people[who].hidden.set(pw_core::Hidden::Ambition, 3);
    let old = package::priorities(w, p);
    let mut with_fee = t;
    with_fee.signing_fee = t.wage * 20;
    with_fee.wage = (t.wage as f64 * 0.97) as i64;
    assert!(package::player_utility(w, p, club, &with_fee, &old) > package::player_utility(w, p, club, &t, &old), "an older player takes money now for slightly less a week");

    // Asked to close a gap, he goes for what costs the club least per unit of his value; with a lever forbidden, another is used.
    let gap = package::player_utility(w, p, club, &t, &old) * 0.05;
    let (open, used_open) = package::close_gap(w, p, club, &t, &old, gap, &|_| true);
    assert!(!used_open.is_empty() && package::player_utility(w, p, club, &open, &old) >= package::player_utility(w, p, club, &t, &old) + gap * 0.9);
    let (_, used_without) = package::close_gap(w, p, club, &t, &old, gap, &|l| l != used_open[0]);
    assert!(!used_without.contains(&used_open[0]) && !used_without.is_empty(), "{used_open:?} vs {used_without:?}");

    // A club that will not give a release clause pays for the refusal in something it will give.
    let value = w.players.cold[p].value.max(1_000_000);
    let mut asked = t;
    asked.release_clause = value * 3;
    let refused = package::player_utility(w, p, club, &asked, &young) - package::player_utility(w, p, club, &t, &young);
    assert!(refused > 0.0, "a release clause is worth something to an ambitious player");
    let (paid, used) = package::compensate(w, p, club, &t, &young, refused * 0.8, &|l| l != Lever::ReleaseClause && l != Lever::Wage);
    assert!(paid.release_clause == 0 && !used.is_empty());
    assert!(package::player_utility(w, p, club, &paid, &young) > package::player_utility(w, p, club, &t, &young) + refused * 0.5);
    let mut style = w.governance[&club].policy.clone();
    style.transfer_style = pw_world::governance::TransferStyle::Develop;
    w.governance.get_mut(&club).unwrap().policy = style;
    assert!(!package::club_allows(w, club, p, Lever::ReleaseClause, 0.5, 0.3), "a club that trades on resale refuses a clause");
    assert!(package::club_allows(w, club, p, Lever::ReleaseClause, 0.9, 0.3), "unless it wants him badly enough");
}

#[test]
fn an_agent_pushes_what_pays_the_agent() {
    let mut sim = world();
    let w = &mut sim.world;
    // A player with an agent.
    let p = *w.agents.of_player.keys().min().expect("someone has an agent");
    let club = w.clubs.ids().find(|&c| c != w.players.hot[p].club).unwrap();
    let a = w.agents.agent_of(p).unwrap();
    let t = base_terms(w, club, p);
    let pr = package::priorities(w, p);
    let gap = package::player_utility(w, p, club, &t, &pr) * 0.08;
    w.agents.list[a].greed = 1;
    let (_, modest) = package::close_gap(w, p, club, &t, &pr, gap, &|_| true);
    w.agents.list[a].greed = 20;
    let (_, greedy) = package::close_gap(w, p, club, &t, &pr, gap, &|_| true);
    let pays_agent = |v: &[Lever]| v.iter().filter(|l| matches!(l, Lever::SigningFee | Lever::ReleaseClause | Lever::Bonuses)).count();
    assert!(pays_agent(&greedy) >= pays_agent(&modest), "{greedy:?} vs {modest:?}");
}

#[test]
fn a_wage_above_the_structure_ripples_through_the_squad() {
    let mut sim = world();
    let (club, p) = club_and_target(&sim.world);
    let w = &mut sim.world;
    let team = w.clubs[club].first_team();
    let top = w.teams[team].squad.iter().map(|&q| w.players.cold[q].contract.current_wage(w.date)).max().unwrap();
    assert_eq!(package::hierarchy_excess(w, club, top / 3), 0.0, "a wage inside the structure breaks nothing");
    let outrageous = top * 3;
    assert!(package::hierarchy_excess(w, club, outrageous) > 1.0);
    w.market.envy.clear();
    package::wage_ripples(w, club, p, outrageous);
    assert!(!w.market.envy.is_empty(), "those paid less and worth as much notice");
    assert!(w.market.envy.values().all(|&e| e > 0.0 && e <= 0.5));
    let before: f32 = w.market.envy.values().sum();
    package::monthly(w);
    assert!(w.market.envy.values().sum::<f32>() < before, "resentment fades");
    // Signing within the structure does nothing of the kind.
    w.market.envy.clear();
    package::wage_ripples(w, club, p, top / 2);
    assert!(w.market.envy.is_empty());
}

// ---------------------------------------------------------------- clauses in force

fn signed_player(w: &World) -> (ClubId, PlayerId) {
    let club = w.clubs.ids().find(|&c| w.clubs[c].manager.is_some()).unwrap();
    let p = w.teams[w.clubs[club].first_team()].squad[3];
    (club, p)
}

#[test]
fn bonuses_are_paid_by_the_club_to_the_player_when_their_conditions_are_met() {
    let mut sim = world();
    let (club, p) = signed_player(&sim.world);
    let w = &mut sim.world;
    let group = w.players.cold[p].best_pos.group();
    let c = &mut w.players.cold[p].contract;
    c.club = club;
    c.appearance_bonus = 1_000;
    c.goal_bonus = 5_000;
    c.assist_bonus = 2_000;
    c.clean_sheet_bonus = 3_000;
    c.title_bonus = 50_000;
    c.promotion_bonus = 40_000;
    let who = w.players.cold[p].person;
    let (bal, saved) = (w.clubs[club].finance.balance, w.lives[who].finances.savings);
    clauses::match_bonuses(w, p, 90, 2, 1, false);
    let paid = bal - w.clubs[club].finance.balance;
    assert_eq!(paid, 1_000 + 2 * 5_000 + 2_000, "appearance, two goals and an assist");
    assert!(w.lives[who].finances.savings > saved, "and it reaches him, less tax");
    // A clean sheet counts for a defender or keeper who played most of the match.
    let before = w.clubs[club].finance.balance;
    clauses::match_bonuses(w, p, 90, 0, 0, true);
    let extra = before - w.clubs[club].finance.balance;
    assert_eq!(extra, 1_000 + if matches!(group, pw_core::PosGroup::Def | pw_core::PosGroup::Gk) { 3_000 } else { 0 });
    // The title: everyone in the squad who has it in his contract.
    let team = w.clubs[club].first_team();
    let comp = w.clubs[club].league;
    w.events.push(w.date, pw_world::Visibility::Public, EventKind::Champion { comp, team, season: 2030 });
    let before = w.clubs[club].finance.balance;
    clauses::weekly(w);
    assert!(before - w.clubs[club].finance.balance >= 50_000, "the title bonus was paid");
}

#[test]
fn relegation_cuts_wages_once_and_opens_a_way_out() {
    let mut sim = world();
    let (club, p) = signed_player(&sim.world);
    let w = &mut sim.world;
    let team = w.clubs[club].first_team();
    let comp = w.clubs[club].league;
    let c = &mut w.players.cold[p].contract;
    c.wage = 100_000;
    c.relegation_cut = 25;
    c.relegation_release = 12_000_000;
    c.release_clause = 0;
    w.events.push(w.date, pw_world::Visibility::Public, EventKind::Relegated { comp, team });
    clauses::weekly(w);
    let c = &w.players.cold[p].contract;
    assert_eq!(c.wage, 75_000, "a quarter off");
    assert_eq!(c.release_clause, 12_000_000, "the clause that only exists after relegation is now live");
    assert_eq!((c.relegation_cut, c.relegation_release), (0, 0), "used once");
    clauses::weekly(w);
    assert_eq!(w.players.cold[p].contract.wage, 75_000, "and not again");
}

#[test]
fn options_and_extensions_belong_to_whoever_holds_them() {
    let mut sim = world();
    let (club, p) = signed_player(&sim.world);
    let w = &mut sim.world;
    let today = w.date;
    // The club's option, on a player it wants.
    {
        let c = &mut w.players.cold[p].contract;
        c.club = club;
        c.end = today.add_days(60);
        c.options = Options { club_years: 2, ..Options::default() };
    }
    w.players.cold[p].status = SquadStatus::Important;
    w.clubs[club].finance.wage_budget = w.clubs[club].finance.wage_bill * 2;
    let end = w.players.cold[p].contract.end;
    let events = w.events.len();
    clauses::weekly(w);
    let c = &w.players.cold[p].contract;
    assert!(c.options.used && c.end > end.add_days(600), "two years added");
    assert!(w.events.all()[events..].iter().any(|e| matches!(e.kind, EventKind::ContractOption { taken: true, kind: pw_world::event::OptionKind::Club, .. })));

    // The club passes on a player it does not want: the option is spent and the deal runs out.
    let q = w.teams[w.clubs[club].first_team()].squad[5];
    {
        let c = &mut w.players.cold[q].contract;
        c.club = club;
        c.end = today.add_days(60);
        c.options = Options { club_years: 1, ..Options::default() };
    }
    w.players.cold[q].status = SquadStatus::NotNeeded;
    let qend = w.players.cold[q].contract.end;
    w.players.cold[q].ca = 20;
    let events = w.events.len();
    clauses::weekly(w);
    assert_eq!(w.players.cold[q].contract.end, qend, "not extended");
    assert!(w.events.all()[events..].iter().any(|e| matches!(e.kind, EventKind::ContractOption { taken: false, player, .. } if player == q)) || w.players.cold[q].contract.options.used);

    // An automatic extension on appearances, and one on a title.
    let r = w.teams[w.clubs[club].first_team()].squad[6];
    {
        let cold = &mut w.players.cold[r];
        cold.contract.club = club;
        cold.contract.end = today.add_days(300);
        cold.contract.apps_base = cold.senior_apps;
        cold.contract.options = Options { auto: Some((Trigger::Appearances(20), 1)), ..Options::default() };
    }
    let rend = w.players.cold[r].contract.end;
    clauses::weekly(w);
    assert_eq!(w.players.cold[r].contract.end, rend, "not yet");
    w.players.cold[r].senior_apps += 20;
    clauses::weekly(w);
    assert!(w.players.cold[r].contract.end > rend.add_days(300), "twenty appearances earned another year");

    let s = w.teams[w.clubs[club].first_team()].squad[7];
    {
        let c = &mut w.players.cold[s].contract;
        c.club = club;
        c.end = today.add_days(300);
        c.options = Options { auto: Some((Trigger::Title, 2)), ..Options::default() };
    }
    let send = w.players.cold[s].contract.end;
    let team = w.clubs[club].first_team();
    let comp = w.clubs[club].league;
    w.events.push(w.date, pw_world::Visibility::Public, EventKind::Champion { comp, team, season: 2030 });
    clauses::weekly(w);
    assert!(w.players.cold[s].contract.end > send.add_days(600));
}

#[test]
fn a_release_clause_lets_a_club_with_the_need_and_the_money_go_straight_to_the_player() {
    let mut sim = world();
    let w = &mut sim.world;
    let today = w.date;
    // The best player anywhere, at a club with a clause that is affordable elsewhere.
    let p = w.players.ids().filter(|&p| w.players.hot[p].club.is_some() && w.players.hot[p].status == pw_world::PlayerStatus::Active).max_by_key(|&p| w.players.cold[p].ca).unwrap();
    let seller = w.players.hot[p].club;
    let group = w.players.cold[p].best_pos.group();
    let buyer = w.clubs.ids().filter(|&c| c != seller).max_by_key(|&c| w.clubs[c].reputation).unwrap();
    for n in w.nations.ids().collect::<Vec<_>>() {
        w.nations[n].season.windows = [(today.add_days(-5), today.add_days(20))].into_iter().collect();
    }
    let fair = market::fair_value(w, buyer, p);
    w.players.cold[p].contract.release_clause = fair;
    w.players.cold[p].loan = None;
    let f = &mut w.clubs[buyer].finance;
    f.transfer_budget = fair * 3;
    f.balance = fair * 5;
    w.clubs[buyer].market.needs = [pw_world::club::Need { group, pos: w.players.cold[p].best_pos, min_ability: 50, max_age: 40, urgency: 3 }].into_iter().collect();
    w.market.cooldown.clear();
    w.market.talking.remove(&p);
    // Far above what the buyer thinks he is worth: nobody pays that just because they can.
    w.players.cold[p].contract.release_clause = fair * 4;
    clauses::release_clause_bids(w);
    assert!(!w.market.talking.contains_key(&p), "nobody pays four times what he is worth");
    w.players.cold[p].contract.release_clause = fair;
    w.market.cooldown.clear();
    w.clubs[buyer].market.needs = [pw_world::club::Need { group, pos: w.players.cold[p].best_pos, min_ability: 50, max_age: 40, urgency: 3 }].into_iter().collect();
    clauses::release_clause_bids(w);
    // Whether the club's own people approved it is their call; when they did, the seller had no say in it.
    if let Some(t) = w.market.talking.get(&p) {
        assert_eq!(w.talks[*t].fee, fair);
        assert_eq!(w.talks[*t].club, buyer);
        assert_eq!(w.talks[*t].seller, seller);
    }
}

// ---------------------------------------------------------------- what a contract remembers

fn a_file(w: &mut World) -> ContractFile {
    let (club, p) = signed_player(w);
    let t = base_terms(w, club, p);
    let f = ContractFile {
        club,
        player: p,
        date: w.date.add_days(-320),
        terms: t,
        believed_worth: 20_000_000,
        role: SquadStatus::Important,
        risks: Default::default(),
        competing: 2,
        alternatives: 1,
        exception: Some(Voice::Director),
        concessions: [(Lever::SigningFee, false), (Lever::Status, true)].into_iter().collect(),
        priorities: [Priority::PlayingTime, Priority::Prestige],
        promised: Some(SquadStatus::Important),
        commitment: 90_000_000,
        burden_pct: 14,
        apps_at: w.players.cold[p].senior_apps,
        outcome: None,
    };
    f
}

#[test]
fn an_important_contract_is_judged_on_what_was_known_and_on_how_it_turned_out() {
    let mut sim = world();
    let w = &mut sim.world;
    let mut f = a_file(w);
    let club = f.club;
    // He never plays and the market has gone off him.
    let stranger = w.clubs.ids().find(|&c| c != club).unwrap();
    w.players.hot[f.player].club = stranger;
    f.believed_worth = market::value_of(w, f.player) * 4;
    let concern = |w: &World| w.governance[&club].concerns.iter().find(|(k, _)| *k == pw_world::governance::BoardConcern::Finances).map_or(0, |x| i32::from(x.1));
    let before = concern(w);
    let events = w.events.len();
    w.boardroom.contracts.push(f.clone());
    clauses::review_contracts(w);
    let judged = w.boardroom.contracts.last().unwrap().outcome.expect("judged");
    assert!(judged.success < -0.15 && judged.verdict == Verdict::Mistake, "{judged:?}");
    assert!(concern(w) < before, "the board notices the money");
    assert!(w.boardroom.authority_of(club, Voice::Director) < 0.0, "and who approved terms outside the structure");
    assert!(w.events.all()[events..].iter().any(|e| matches!(e.kind, EventKind::SigningReviewed { verdict: Verdict::Mistake, .. })));

    // A modest deal that goes wrong through bad luck is not a mistake: what was known at the time was sound.
    f.exception = None;
    f.burden_pct = 4;
    f.outcome = None;
    w.boardroom.contracts.push(f);
    clauses::review_contracts(w);
    assert_eq!(w.boardroom.contracts.last().unwrap().outcome.unwrap().verdict, Verdict::Unlucky);
}

#[test]
fn signed_packages_leave_files_promises_and_a_record_of_who_gave_way() {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 72, Scale::SMALL));
    sim.run(400);
    let w = &sim.world;
    assert!(!w.boardroom.contracts.is_empty(), "important contracts leave files");
    for f in &w.boardroom.contracts {
        assert!(f.burden_pct <= 100 && f.priorities[0] != f.priorities[1] && f.terms.years >= 1);
    }
    // Talks that ended in agreement: some of them traded one dimension for another, with each side giving something.
    let agreed: Vec<_> = w.talks.iter().filter(|t| t.end == Some(pw_world::negotiation::TalkEnd::Signed)).collect();
    assert!(!agreed.is_empty(), "contracts were signed");
    assert!(agreed.iter().any(|t| t.moves.iter().any(|m| m.1) && t.moves.iter().any(|m| !m.1)) || agreed.iter().any(|t| t.moves.len() >= 2), "packages were traded, not just priced");
    // Terms beyond the wage exist in the world: options, bonuses and clauses on real contracts.
    let contracts: Vec<_> = w.players.cold.iter().map(|c| &c.contract).filter(|c| c.club.is_some()).collect();
    assert!(contracts.iter().any(|c| c.options.any()), "options");
    assert!(contracts.iter().any(|c| c.continental_bonus > 0 || c.title_bonus > 0 || c.promotion_bonus > 0), "success bonuses");
    // A promised status is a promise the world remembers.
    let promised = agreed.iter().filter(|t| t.offer.status.is_some()).count();
    if promised > 0 {
        assert!(w.social.promises.iter().any(|p| matches!(p.kind, pw_world::social::PromiseKind::Status(_))), "a status promise was recorded");
    }
}
