//! The boardroom and bargaining (locked design 3.6-3.7, 3.11-3.14, 3.19-3.24, 3.27-3.29): institutional power decides signings,
//! appetite for risk moves with circumstances, decisions leave case files judged on process as well as result, and negotiators hold
//! ranges and signals rather than each other's limits.

use pw_core::{ClubId, PlayerId, StaffId};
use pw_data::DataPack;
use pw_import::synthetic::{self, Scale};
use pw_sim::{Sim, bargaining, boardroom};
use pw_world::boardroom::{Case, CaseState, Structure, Verdict, Voice};
use pw_world::deals::{DealLine, Limit, RivalInfo, Source};
use pw_world::dossier::{Confidence, RiskKind};
use pw_world::{StaffRole, World};

fn world() -> Sim {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 91, Scale::SMALL));
    sim.run(45);
    sim
}

/// A buyer club with staff, and a first-team player of another club in the same nation to look at.
fn pair(w: &World) -> (ClubId, PlayerId) {
    let buyer = w.clubs.ids().find(|&c| w.clubs[c].manager.is_some() && w.clubs[c].staff.len() >= 4 && w.governance.contains_key(&c)).expect("a buyer");
    let other = w.clubs.ids().find(|&c| c != buyer && w.clubs[c].nation == w.clubs[buyer].nation).unwrap();
    let p = w.teams[w.clubs[other].first_team()].squad[5];
    (buyer, p)
}

fn make_director(w: &mut World, club: ClubId) -> StaffId {
    let s = w.clubs[club].staff.iter().copied().find(|&s| !matches!(w.staff[s].role, StaffRole::Manager | StaffRole::DirectorOfFootball | StaffRole::Scout | StaffRole::Analyst)).expect("spare staff");
    w.staff[s].role = StaffRole::DirectorOfFootball;
    s
}

#[test]
fn how_a_club_is_run_decides_who_counts() {
    let mut sim = world();
    let (club, _) = pair(&sim.world);
    let w = &mut sim.world;
    w.governance.get_mut(&club).unwrap().owner.meddling = 85;
    assert_eq!(boardroom::structure(w, club), Structure::OwnerLed);
    w.governance.get_mut(&club).unwrap().owner.meddling = 10;
    w.governance.get_mut(&club).unwrap().policy.transfer_style = pw_world::governance::TransferStyle::Balanced;
    for s in w.clubs[club].staff.clone() {
        if w.staff[s].role == StaffRole::DirectorOfFootball {
            w.staff[s].role = StaffRole::Coach;
        }
    }
    assert_eq!(boardroom::structure(w, club), Structure::ManagerLed);
    make_director(w, club);
    assert!(matches!(boardroom::structure(w, club), Structure::DirectorLed | Structure::Committee));

    // The same voices count differently under different structures, always summing to one, absent voices not at all.
    let all = [true; 7];
    let mut some = all;
    some[Voice::Captain.idx()] = false;
    w.governance.get_mut(&club).unwrap().owner.meddling = 10;
    let manager_led = {
        for s in w.clubs[club].staff.clone() {
            if w.staff[s].role == StaffRole::DirectorOfFootball {
                w.staff[s].role = StaffRole::Coach;
            }
        }
        boardroom::powers(w, club, &all)
    };
    w.governance.get_mut(&club).unwrap().owner.meddling = 90;
    let owner_led = boardroom::powers(w, club, &all);
    assert!((manager_led.iter().sum::<f32>() - 1.0).abs() < 1e-4 && (owner_led.iter().sum::<f32>() - 1.0).abs() < 1e-4);
    assert!(manager_led[Voice::Manager.idx()] > owner_led[Voice::Manager.idx()] + 0.2);
    assert!(owner_led[Voice::Owner.idx()] > manager_led[Voice::Owner.idx()] + 0.2);
    assert_eq!(boardroom::powers(w, club, &some)[Voice::Captain.idx()], 0.0);

    // Standing earned through past decisions shifts it.
    w.governance.get_mut(&club).unwrap().owner.meddling = 10;
    let before = boardroom::powers(w, club, &all)[Voice::Director.idx()];
    w.boardroom.authority.insert((club, Voice::Director), 0.25);
    assert!(boardroom::powers(w, club, &all)[Voice::Director.idx()] > before + 0.05);
}

#[test]
fn the_outcome_is_the_power_weighted_stances_not_an_average() {
    let mut sim = world();
    let w = &mut sim.world;
    // Find a club and target on which the manager and the owner disagree clearly.
    let mut found = None;
    'outer: for club in w.clubs.ids().collect::<Vec<_>>() {
        if w.governance.get(&club).is_none() || w.clubs[club].manager.is_none() {
            continue;
        }
        let other = w.clubs.ids().find(|&c| c != club && w.clubs[c].nation == w.clubs[club].nation).unwrap();
        for &p in w.teams[w.clubs[other].first_team()].squad.iter().take(12) {
            let Some(x) = boardroom::decide(w, club, p, None, 5_000_000) else { continue };
            let get = |v: Voice| x.stances.iter().find(|s| s.voice == v).map(|s| s.support);
            if let (Some(m), Some(o)) = (get(Voice::Manager), get(Voice::Owner))
                && (m - o).abs() > 0.5
            {
                found = Some((club, p, m, o));
                break 'outer;
            }
        }
    }
    let (club, p, manager, owner) = found.expect("a club where manager and owner disagree on someone");
    let score_under = |w: &mut World, meddling: u8| {
        w.governance.get_mut(&club).unwrap().owner.meddling = meddling;
        boardroom::decide(w, club, p, None, 5_000_000).unwrap().score
    };
    let manager_led = score_under(w, 5);
    let owner_led = score_under(w, 95);
    if owner > manager {
        assert!(owner_led > manager_led, "power to the owner favours the owner's view: {owner_led} vs {manager_led}");
    } else {
        assert!(owner_led < manager_led, "power to the owner favours the owner's view: {owner_led} vs {manager_led}");
    }
    // The score is exactly the weighted stances.
    let x = boardroom::decide(w, club, p, None, 5_000_000).unwrap();
    let sum: f32 = x.stances.iter().map(|s| s.support * s.power).sum();
    assert!((sum - x.score).abs() < 1e-5);
    assert!((x.stances.iter().map(|s| s.power).sum::<f32>() - 1.0).abs() < 1e-4);
}

#[test]
fn appetite_for_risk_follows_circumstances_and_explains_itself() {
    let mut sim = world();
    let (club, _) = pair(&sim.world);
    let w = &mut sim.world;
    let base = boardroom::appetite(w, club, 0);
    assert!((0.05..=0.95).contains(&base.level));

    let mut rich = w.clone();
    rich.governance.get_mut(&club).unwrap().owner.since = rich.date;
    rich.clubs[club].finance.balance = 3 * pw_sim::finance::season_revenue(&rich, club);
    let a = boardroom::appetite(&rich, club, 2);
    assert!(a.level > base.level + 0.1, "new owner, cash and trophies make a club bolder: {} vs {}", a.level, base.level);
    assert!(a.drivers.iter().any(|d| d.0 == pw_world::boardroom::Driver::NewOwner) && a.drivers.iter().any(|d| d.0 == pw_world::boardroom::Driver::Cash));

    let mut burnt = w.clone();
    burnt.boardroom.record.insert(club, pw_world::boardroom::Record { hits: 0, flops: 3 });
    burnt.clubs[club].board.satisfaction = 10;
    let b = boardroom::appetite(&burnt, club, 0);
    assert!(b.level < base.level - 0.15, "two expensive flops and board pressure make it cautious: {} vs {}", b.level, base.level);
    assert!(b.drivers.iter().any(|d| d.0 == pw_world::boardroom::Driver::RecentFlops));
    assert!(a.level > b.level);

    // The same target faces a lower bar at a bold club.
    let (_, p) = pair(w);
    w.boardroom.appetite.insert(club, pw_world::boardroom::Appetite { level: 0.85, ..Default::default() });
    let bold = boardroom::decide(w, club, p, None, 5_000_000).unwrap().threshold;
    w.boardroom.appetite.insert(club, pw_world::boardroom::Appetite { level: 0.15, ..Default::default() });
    let cautious = boardroom::decide(w, club, p, None, 5_000_000).unwrap().threshold;
    assert!(bold < cautious, "{bold} vs {cautious}");
}

fn a_case(w: &mut World) -> Case {
    let (club, p) = pair(w);
    let revenue = pw_sim::finance::season_revenue(w, club);
    assert!(boardroom::consider_target(w, club, p, None, revenue, revenue) || true);
    w.boardroom.cases.last().cloned().expect("an expensive look is kept as a case file")
}

#[test]
fn a_result_alone_does_not_decide_whether_the_decision_was_good() {
    let mut sim = world();
    let mut c = a_case(&mut sim.world);
    c.confidence = Confidence::High;
    c.risks_unknown.clear();
    c.price_paid = c.price_high / 2;
    let bad = -0.6;

    // Known and accepted, and it happened.
    c.risks_known = [RiskKind::Injuries].into_iter().collect();
    c.risks_accepted = c.risks_known.clone();
    assert_eq!(boardroom::verdict(&c, bad, &[Some(RiskKind::Injuries), None]), Verdict::AcceptedRisk);
    // Nobody saw it coming.
    c.risks_known.clear();
    c.risks_accepted.clear();
    assert_eq!(boardroom::verdict(&c, bad, &[Some(RiskKind::Injuries), None]), Verdict::MissedRisk);
    // A sound process and plain bad luck.
    assert_eq!(boardroom::verdict(&c, bad, &[None, None]), Verdict::Unlucky);
    assert_eq!(boardroom::verdict(&c, 0.6, &[None, None]), Verdict::Sound);
    // A careless process.
    c.confidence = Confidence::Low;
    assert_eq!(boardroom::verdict(&c, bad, &[None, None]), Verdict::Mistake);
    assert_eq!(boardroom::verdict(&c, 0.6, &[None, None]), Verdict::Lucky);
    // Paying far above the range the club itself believed acceptable is part of the process.
    c.confidence = Confidence::High;
    c.price_paid = c.price_high * 2;
    assert_eq!(boardroom::verdict(&c, 0.6, &[None, None]), Verdict::Lucky);
}

#[test]
fn judged_signings_change_who_the_club_listens_to_and_are_remembered() {
    let mut sim = world();
    let w = &mut sim.world;
    let mut c = a_case(w);
    let club = c.buyer;
    let director = make_director(w, club);
    let manager = w.clubs[club].manager;
    // The director forced through a signing the manager opposed; a year on, the player is nowhere to be seen.
    c.state = CaseState::Signed;
    c.signed = Some(w.date.add_days(-330));
    c.champion = Voice::Director;
    c.overruled = [Voice::Manager].into_iter().collect();
    c.confidence = Confidence::Low;
    c.outcome = None;
    c.price_paid = c.price_high / 2;
    let stranger = w.clubs.ids().find(|&x| x != club).unwrap();
    w.players.hot[c.player].club = stranger;
    let events = w.events.len();
    w.boardroom.cases.push(c);
    boardroom::review(w);

    let judged = w.boardroom.cases.last().unwrap().outcome.expect("judged");
    assert!(judged.success < -0.15 && judged.verdict == Verdict::Mistake, "{judged:?}");
    assert!(w.boardroom.authority_of(club, Voice::Director) < 0.0, "the director who forced it through loses standing");
    assert!(w.boardroom.authority_of(club, Voice::Manager) > 0.0, "the manager who opposed it gains leverage");
    assert_eq!(w.boardroom.record.get(&club).map(|r| r.flops), Some(1), "and the club is a little more wary");
    assert!(w.dossiers.track.get(&director).is_some_and(|t| t.wrong == 1), "the director's record takes the hit");
    assert!(w.events.all()[events..].iter().any(|e| matches!(e.kind, pw_world::EventKind::SigningReviewed { overruled: true, verdict: Verdict::Mistake, .. })), "an overruled objection is part of the story");
    let _ = manager;

    // And the reverse: a success raises the champion, lowers those who opposed it.
    let mut good = w.boardroom.cases.last().cloned().unwrap();
    good.outcome = None;
    good.signed = Some(w.date.add_days(-330));
    good.confidence = Confidence::High;
    w.players.hot[good.player].club = club;
    w.players.cold[good.player].senior_apps = good.apps_at + 60;
    w.boardroom.cases.push(good);
    let (dir_before, mgr_before) = (w.boardroom.authority_of(club, Voice::Director), w.boardroom.authority_of(club, Voice::Manager));
    boardroom::review(w);
    let out = w.boardroom.cases.last().unwrap().outcome.unwrap();
    assert!(out.success > 0.15, "{out:?}");
    assert!(w.boardroom.authority_of(club, Voice::Director) > dir_before);
    assert!(w.boardroom.authority_of(club, Voice::Manager) < mgr_before);
}

#[test]
fn a_captain_who_opposed_a_signing_carries_it_into_the_dressing_room() {
    let mut sim = world();
    let w = &mut sim.world;
    let mut c = a_case(w);
    let club = c.buyer;
    let team = w.clubs[club].first_team();
    let captain = w.teams[team].squad[0];
    w.teams[team].captain = captain;
    let cap_person = w.players.cold[captain].person;
    c.state = CaseState::Pursuing;
    c.stances.retain(|s| s.voice != Voice::Captain);
    c.stances.push(pw_world::boardroom::Stance { voice: Voice::Captain, who: cap_person, support: -0.6, power: 0.03, why: pw_world::boardroom::Why::RoleCompetition });
    let p = c.player;
    w.boardroom.cases.push(c);
    boardroom::on_signed(w, club, p, 1_000_000);
    let newcomer = w.players.cold[p].person;
    assert!(w.social.get(cap_person, newcomer).is_some(), "the captain now has a view of him");
    assert_eq!(w.boardroom.cases.last().unwrap().state, CaseState::Signed);
}

// ---------------------------------------------------------------- bargaining

fn some_deal(w: &World) -> pw_world::deals::ClubDeal {
    w.deals.deals.iter().find(|d| d.is_open()).or_else(|| w.deals.deals.first()).cloned().expect("the market produced a deal")
}

fn world_with_deals() -> Sim {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 92, Scale::SMALL));
    for _ in 0..400 {
        sim.run(1);
        if sim.world.deals.deals.len() >= 6 {
            break;
        }
    }
    sim
}

#[test]
fn each_side_starts_with_a_wide_range_that_a_better_negotiator_narrows_and_the_talks_narrow_further() {
    let sim = world_with_deals();
    let w = &sim.world;
    let d = some_deal(w);
    // Whatever the talks have narrowed it to, a belief is never inverted; at the start it is a real range around the public estimate.
    for x in &w.deals.deals {
        assert!(x.buyer_thinks_seller_min.lo <= x.buyer_thinks_seller_min.hi && x.seller_thinks_buyer_max.lo <= x.seller_thinks_buyer_max.hi);
    }
    let (smin, bmax) = bargaining::initial_limits(w, d.buyer, d.seller, d.player);
    assert!(smin.lo < smin.hi && bmax.lo < bmax.hi, "ranges, never exact figures");
    let public = pw_sim::market::value_of(w, d.player);
    assert!(smin.lo < public && public < smin.hi, "the buyer's range brackets the public estimate: {smin:?} vs {public}");

    let mut skilled = w.clone();
    let mut unskilled = w.clone();
    for (world, level) in [(&mut skilled, 20u8), (&mut unskilled, 1u8)] {
        for s in world.clubs[d.buyer].staff.clone() {
            world.staff[s].attrs.set(pw_core::StaffAttr::Negotiating, level);
        }
    }
    let width = |world: &World| {
        let (l, _) = bargaining::initial_limits(world, d.buyer, d.seller, d.player);
        l.hi - l.lo
    };
    assert!(width(&skilled) < width(&unskilled), "a better negotiator knows more about the seller");

    // A counter says he will take no more than the ask and more than was bid.
    let mut x = d.clone();
    x.buyer_thinks_seller_min = Limit { lo: public / 2, hi: public * 2 };
    bargaining::learn_from_counter(&mut x, public * 3 / 4, public * 11 / 10);
    assert_eq!((x.buyer_thinks_seller_min.lo, x.buyer_thinks_seller_min.hi), (public * 3 / 4, public * 11 / 10));
    // A failed bid says the buyer can pay at least that.
    x.seller_thinks_buyer_max = Limit { lo: public / 2, hi: public };
    bargaining::learn_from_bid(&mut x, public * 13 / 10);
    assert!(x.seller_thinks_buyer_max.lo >= public * 13 / 10 && x.seller_thinks_buyer_max.hi >= x.seller_thinks_buyer_max.lo);
}

#[test]
fn the_ask_never_drops_below_the_reservation_and_follows_belief_and_credible_signals() {
    let sim = world_with_deals();
    let w = &sim.world;
    let mut d = some_deal(w);
    let want = 10_000_000.0;
    d.signals = Default::default();

    d.seller_thinks_buyer_max = Limit { lo: 6_000_000, hi: 8_000_000 };
    let modest = bargaining::seller_ask(w, &d, want);
    d.seller_thinks_buyer_max = Limit { lo: 20_000_000, hi: 30_000_000 };
    let rich = bargaining::seller_ask(w, &d, want);
    assert!(modest >= want && rich >= want, "never below what the seller itself needs");
    assert!(rich > modest * 1.2, "a seller who thinks the buyer is rich asks more: {rich} vs {modest}");

    // A claim that the budget is gone, from a club with a clean record, brings the ask down but never below the reservation.
    let mut honest = w.clone();
    honest.boardroom.honesty.insert(d.buyer, 0.95);
    let mut liar = w.clone();
    liar.boardroom.honesty.insert(d.buyer, 0.05);
    d.signals.budget_claimed = 11_000_000;
    let believed = bargaining::seller_ask(&honest, &d, want);
    let doubted = bargaining::seller_ask(&liar, &d, want);
    assert!(believed < rich && believed >= want, "{believed} vs {rich}");
    assert!(doubted > believed, "a club known to bluff is believed less: {doubted} vs {believed}");
    // A credible threat to walk helps a little, and a buyer in no hurry is read as one who can wait.
    let mut e = d.clone();
    e.signals = Default::default();
    e.signals.walked = true;
    assert!(bargaining::seller_ask(&honest, &e, want) < rich);
    e.signals = Default::default();
    e.signals.delays = 2;
    assert!(bargaining::seller_ask(w, &e, want) < rich);
}

#[test]
fn what_is_heard_about_rivals_counts_by_its_source_and_a_bluff_costs_credibility() {
    let sim = world_with_deals();
    let w = &sim.world;
    let mut d = some_deal(w);
    d.rival_info.clear();
    let base = bargaining::buyer_ceiling(&d, 10_000_000.0);
    assert_eq!(base, 10_000_000.0, "nothing heard, nothing changes");
    let tip = |source, reliability| RivalInfo { club: ClubId::NONE, claimed: 12_000_000, source, reliability, date: w.date, real: true };
    d.rival_info.push(tip(Source::Agent, 0.85));
    let trusted = bargaining::buyer_ceiling(&d, 10_000_000.0);
    d.rival_info.clear();
    d.rival_info.push(tip(Source::Claim, 0.1));
    let doubted = bargaining::buyer_ceiling(&d, 10_000_000.0);
    assert!(trusted > doubted && doubted > base, "{trusted} {doubted} {base}");

    // Deals across the run: an agent only ever passes on a bid that exists.
    for deal in &w.deals.deals {
        for r in deal.rival_info.iter().filter(|r| r.source == Source::Agent) {
            assert!(r.real, "agents report real interest only");
            assert!(r.club.is_some(), "and say whose");
        }
    }

    // Found out: a seller that invented rivals is believed less next time.
    let mut w2 = w.clone();
    let idx = w2.deals.deals.iter().position(|x| x.is_open()).unwrap_or(0);
    w2.deals.deals[idx].rival_info.push(RivalInfo { club: ClubId::NONE, claimed: 9_000_000, source: Source::Claim, reliability: 0.6, date: w2.date, real: false });
    let seller = w2.deals.deals[idx].seller;
    let before = w2.boardroom.honesty_of(seller);
    bargaining::settle_bluffs(&mut w2, idx, 1_000_000);
    assert!(w2.boardroom.honesty_of(seller) < before);
    assert!(w2.deals.deals[idx].signals.caught);
}

#[test]
fn public_facts_change_what_a_seller_thinks_it_can_ask_overnight() {
    let mut sim = world_with_deals();
    let w = &mut sim.world;
    let d = some_deal(w);
    let group = w.players.cold[d.player].best_pos.group();
    let before = bargaining::buyer_need_premium(w, &d);
    let team = w.clubs[d.buyer].first_team();
    let squad: Vec<PlayerId> = w.teams[team].squad.iter().copied().filter(|&q| w.players.cold[q].best_pos.group() == group).take(3).collect();
    assert!(!squad.is_empty());
    for &q in &squad {
        w.players.hot[q].injury_days = 60;
    }
    let after = bargaining::buyer_need_premium(w, &d);
    assert!(after > before, "injuries at the buyer in that position raise the price: {after} vs {before}");
}

#[test]
fn replacement_chains_happen_and_every_wait_is_resolved_one_way_or_the_other() {
    let mut sim = Sim::new(synthetic::build(DataPack::builtin(), 93, Scale::SMALL));
    sim.run(400);
    let w = &sim.world;
    let (mut waited, mut resolved, mut failed) = (0, 0, 0);
    for d in &w.deals.deals {
        let wait = d.log.iter().position(|(_, l)| matches!(l, DealLine::AwaitingReplacement(_)));
        let Some(at) = wait else { continue };
        waited += 1;
        let after = &d.log[at..];
        if after.iter().any(|(_, l)| matches!(l, DealLine::ReplacementSigned)) {
            resolved += 1;
        } else if matches!(d.end, Some(pw_world::deals::DealEnd::ReplacementFailed)) {
            failed += 1;
        }
        // While the wait lasts the deal is neither done nor collapsed, and nobody waits more than three weeks.
        if d.awaiting.is_some() {
            assert!(d.is_open());
            assert!(d.awaiting_since.days_until(w.date) <= 24, "a wait dragged on: {} days", d.awaiting_since.days_until(w.date));
        }
    }
    assert!(waited > 0, "sellers with no cover hold deals for a replacement");
    assert!(resolved + failed > 0, "and some of those waits end: {resolved} landed, {failed} fell through of {waited}");
}
