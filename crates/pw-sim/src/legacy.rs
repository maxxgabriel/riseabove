//! Bringing a loaded world's new state up to date (locked design 10.3, 10.4, 10.8).
//!
//! `pw_world::ext` upgrades the *bytes* of the extension state (new domains start empty). What cannot be done on bytes is anything
//! that needs the rest of the world: a present baseline derived from current state, say. That is this module's job, and it runs
//! exactly once, after load, when `Extensions::migrated_from` says the state came from an older layout.
//!
//! Rules: deterministic (no fresh randomness), derived only from existing state, never inventing history (no fake past sightings,
//! results or reasons), and anything derived is marked as legacy-derived wherever the state has provenance to mark it with.
//! What an old save simply never recorded stays unknown, not zero.

use pw_core::{Date, PlayerId};
use pw_world::World;
use pw_world::pathway::{Creation, Draw};
use pw_world::recog::{Regard, Segment, Source, Vouch, VouchBasis};

/// Run the initialisation for every layout the world may have come from. Idempotent: clears the marker.
pub fn finish(w: &mut World) {
    let Some(from) = w.ext.migrated_from.take() else { return };
    if from < 2 {
        from_layout_1(w);
    }
    // Layout 2 to 3 (the scenario's known derbies and reference report) derives nothing: both start empty, and stay empty until a
    // world is built from reference data. There is no present baseline to compute for them.
}

/// Layout 1 had a single `sponsor` count per player instead of recommendations with causes, one `export` number instead of regard by
/// market and kind of football, and no record of how players came to exist.
fn from_layout_1(w: &mut World) {
    let today = w.date;
    let mut players: Vec<PlayerId> = w.ext.ecosystem.repute.keys().copied().collect();
    players.sort();

    // Someone used to vouch, but not who or why. Keep the fact, mark it, and give it the weakest footing: it is never a causal
    // recommendation, so a scout gives it little weight (`VouchBasis::Legacy`).
    for &p in &players {
        let sponsor = w.ext.ecosystem.repute[&p].sponsor;
        if sponsor == 0 {
            continue;
        }
        let Some(s) = w.ext.ecosystem.story.get(&p) else { continue };
        let from = Source::District(w.ext.ecosystem.state_of(s.dev));
        w.ext.recog.vouch.insert(p, Vouch { from, basis: VouchBasis::Legacy, strength: f32::from(sponsor.min(3)) / 3.0, credibility: 0.4, date: today });
    }

    // One number for how the world abroad sees the country: it becomes the starting regard of every market for the football of its
    // domestic league (the one part an outsider could have seen), and nothing for the rest. Marked legacy so nothing mistakes it for
    // an earned record. Markets are the scenario's; a save that predates them has the one anonymous market 0.
    let base = w.ext.ecosystem.export.max(0.0);
    if base > 0.0 {
        let markets = w.ext.scenario.markets.len().max(1);
        for m in 0..markets {
            w.ext.recog.export.insert((m as u8, Segment::League), Regard { level: base, legacy: true, ..Regard::default() });
        }
    }

    // How each ecosystem player came to exist: what the player's own story says, and nothing that it does not. The age and the
    // institution were not kept, so they are unknown.
    let mut stories: Vec<PlayerId> = w.ext.ecosystem.story.keys().copied().collect();
    stories.sort();
    for p in stories {
        let s = w.ext.ecosystem.story[&p];
        let first_env = w.ext.ecosystem.route(p).first().map_or(pw_world::ecosystem::StageKind::Grassroots, |x| x.kind);
        w.ext.pathway.created.insert(
            p,
            Creation { date: Date(0), region: s.home, provider: s.provider, institution: None, age: None, first_env, first_finder: s.found_by, why: Draw::Unrecorded, legacy: true },
        );
    }
}
