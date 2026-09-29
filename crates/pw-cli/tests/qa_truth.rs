//! QA: the truth firewall, tested by behaviour rather than by reading source.
//!
//! Locked design 1.15: a decision module must act on what its club believes. A club's belief about a player is held in a dossier
//! (`w.dossiers`), so a decision taken from a fresh dossier must be *identical* if the player's hidden ability is changed and nothing the
//! club has seen is changed. Each check runs the same decision on two clones of one world, one with the hidden `ca`/`pa` and every
//! attribute of the subject wrecked or inflated, and compares the outputs bit for bit.
//!
//! A sensitivity check makes sure the perturbation is real: quantities that legitimately follow the truth (the public market's noisy
//! reading, the audit-only `true_worth`) do move.

mod qa_common;

use pw_core::{ClubId, PlayerId};
use pw_import::synthetic::Scale;
use pw_sim::{Sim, bargaining, boardroom, dossier, market, package, scouting};
use pw_world::boardroom::Voice;
use pw_world::{PlayerStatus, World};
use qa_common::*;

fn world() -> Sim {
    ran(Scale::SMALL, 91, 75)
}

/// Pairs (club, player) where the club holds a fresh dossier on a player it does not employ, and pairs for its own squad.
fn pairs(w: &World) -> Vec<(ClubId, PlayerId)> {
    let mut out = Vec::new();
    let mut keys: Vec<_> = w.dossiers.map.keys().copied().collect();
    keys.sort();
    for (club, p) in keys {
        if w.players.hot[p].status != PlayerStatus::Active || dossier::reading(w, club, p).is_none() {
            continue;
        }
        if out.iter().filter(|x: &&(ClubId, PlayerId)| x.0 == club).count() >= 3 {
            continue;
        }
        out.push((club, p));
        if out.len() >= 48 {
            break;
        }
    }
    out
}

/// Wreck (or inflate) what the player truly is, without touching anything a club could have seen.
fn perturb(w: &mut World, p: PlayerId, up: bool, attrs: bool) {
    let c = &mut w.players.cold[p];
    if up {
        c.ca = c.ca.saturating_add(45).min(200);
        c.pa = c.pa.saturating_add(50).min(200);
        if attrs {
            for a in c.attrs.0.iter_mut() {
                *a = a.saturating_add(4);
            }
        }
    } else {
        c.ca = (c.ca / 3).max(1);
        c.pa = (c.pa / 3).max(1);
        if attrs {
            for a in c.attrs.0.iter_mut() {
                *a = a.saturating_sub(5);
            }
        }
    }
}

/// The stances that are formed from dossiers, scouting reports, the club's own books and the owner's tastes: everything except the
/// voices that read the *public* market view (the captain's comparison, the supporters' fame), which is a noisy reading of the truth
/// by design (locked design 3.25) and so moves when the truth moves.
const BELIEF_VOICES: [Voice; 4] = [Voice::Director, Voice::Recruitment, Voice::Owner, Voice::Analyst];

#[test]
fn belief_accessors_and_dossier_stances_do_not_move_with_hidden_ability() {
    let sim = world();
    let w = &sim.world;
    let pairs = pairs(w);
    assert!(pairs.len() >= 20, "enough (club, player) pairs with fresh dossiers: {}", pairs.len());
    let mut leaks: Vec<String> = Vec::new();
    for (i, &(club, p)) in pairs.iter().enumerate() {
        // Accessors of a club's belief: wrecking or inflating the truth, attributes included, must change nothing.
        let mut v = w.clone();
        perturb(&mut v, p, i % 2 == 0, true);
        let mut note = |what: &str, same: bool| {
            if !same {
                leaks.push(format!("{what} for club {club:?} on {p:?} changed when only his hidden ability changed"));
            }
        };
        note("scouting::view", scouting::view(w, club, p) == scouting::view(&v, club, p));
        note("market::fair_value", market::fair_value(w, club, p) == market::fair_value(&v, club, p));
        note("package::want", package::want(w, club, p) == package::want(&v, club, p));
        note("dossier::worth", dossier::worth(w, club, p, 1.0) == dossier::worth(&v, club, p, 1.0));
        note("scouting::disagreement", scouting::disagreement(w, club, p) == scouting::disagreement(&v, club, p));

        // The boardroom's dossier-driven voices, on ability alone (attributes are read through the club's own noisy perception by
        // `dossier::system_fit`, which is a belief that follows the truth by design).
        let mut v = w.clone();
        perturb(&mut v, p, i % 2 == 0, false);
        let opening = market::fair_value(w, club, p);
        match (boardroom::decide(w, club, p, None, opening), boardroom::decide(&v, club, p, None, opening)) {
            (Some(x), Some(y)) => {
                for voice in BELIEF_VOICES {
                    let (a, b) = (x.stances.iter().find(|s| s.voice == voice), y.stances.iter().find(|s| s.voice == voice));
                    note(&format!("boardroom {voice:?} stance"), a.map(|s| (s.support, s.why)) == b.map(|s| (s.support, s.why)));
                }
                note("boardroom dossier", x.dossier == y.dossier);
                note("boardroom risks and alternatives", (&x.known, &x.unknown, x.alternatives, x.bar, x.fair) == (&y.known, &y.unknown, y.alternatives, y.bar, y.fair));
            }
            (None, None) => {}
            _ => note("boardroom::decide (whether a view could be formed)", false),
        }
    }
    assert!(leaks.is_empty(), "{} leaks of hidden ability into decisions:\n{}", leaks.len(), leaks.iter().take(15).cloned().collect::<Vec<_>>().join("\n"));
}

/// What does follow the truth, by design: the public market's noisy reading and everything built on it. Pinned so that a change that
/// cuts it (or one that makes club beliefs start following it) is noticed.
#[test]
fn the_public_market_reading_is_where_truth_enters_pricing() {
    let sim = world();
    let w = &sim.world;
    let mut moved = 0;
    let mut checked = 0;
    for &(club, p) in pairs(w).iter().take(24) {
        let seller = w.players.hot[p].club;
        if seller == club || seller.is_none() {
            continue;
        }
        let mut v = w.clone();
        perturb(&mut v, p, false, false);
        checked += 1;
        let (a1, _) = bargaining::initial_limits(w, club, seller, p);
        let (b1, _) = bargaining::initial_limits(&v, club, seller, p);
        if (a1.lo, a1.hi) != (b1.lo, b1.hi) {
            moved += 1;
        }
    }
    assert!(checked >= 8, "{checked}");
    assert!(moved * 4 >= checked * 3, "the opening ranges follow the public reading of a wrecked player in only {moved} of {checked} cases");
}

/// The perturbation must be real: things that legitimately follow the truth do move, so the test above cannot pass vacuously.
#[test]
fn the_perturbation_is_visible_to_things_that_may_see_the_truth() {
    let sim = world();
    let w = &sim.world;
    let pairs = pairs(w);
    let (mut moved_public, mut moved_true, mut moved_truth_worth) = (0, 0, 0);
    for &(_, p) in pairs.iter().take(16) {
        let mut v = w.clone();
        perturb(&mut v, p, false, true);
        if market::value_of(w, p) != market::value_of(&v, p) {
            moved_public += 1;
        }
        if market::true_worth(w, p) != market::true_worth(&v, p) {
            moved_truth_worth += 1;
        }
        if w.players.cold[p].ca != v.players.cold[p].ca {
            moved_true += 1;
        }
    }
    assert!(moved_true >= 12, "the hidden ability really changed ({moved_true})");
    assert!(moved_public >= 12, "the public market's reading follows the truth, noisily ({moved_public})");
    assert!(moved_truth_worth >= 12, "true_worth follows the truth ({moved_truth_worth})");
}

/// A club's belief about a player it has never seen closely is a noisy reading, so it must differ from the truth for most players;
/// and a club's belief about its own players is close to it on average (the firewall does not make clubs blind).
#[test]
fn beliefs_are_neither_blind_nor_omniscient() {
    let sim = world();
    let w = &sim.world;
    let (mut n, mut exact, mut err) = (0u32, 0u32, 0.0f32);
    for &(club, p) in &pairs(w) {
        let (ca, _, _, _) = scouting::view(w, club, p);
        let truth = f32::from(w.players.cold[p].ca);
        n += 1;
        err += (ca - truth).abs();
        if (ca - truth).abs() < 0.01 {
            exact += 1;
        }
    }
    assert!(n > 20);
    assert!(exact * 5 < n, "beliefs equal the truth for {exact} of {n} pairs: clubs are omniscient");
    assert!(err / (n as f32) < 30.0, "mean belief error {} is so large that clubs are blind", err / n as f32);
}
