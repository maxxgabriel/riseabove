//! QA: media, social opinion and attention invariants over several seeds and three seasons.
//!
//! Media: the truth category of every story is consistent with whether it was grounded and with the fidelity of what the journalist
//! held; bonds, feuds and fan standings stay in range; corrections point backwards. Social: every opinion is within range and its own
//! history brackets it; attention waves are proper probabilities; nicknames and myths are bounded.

mod qa_common;

use pw_core::PersonId;
use pw_import::synthetic::Scale;
use pw_world::info::Fidelity;
use pw_world::media::{ClaimType, Party, Truth};
use pw_world::socialnet::{Dim, N_DIMS, NO_POST};
use pw_world::{StoryKind, World};
use qa_common::*;

fn media_problems(w: &World) -> Vec<String> {
    let mut out = Vec::new();
    let mut last_date = pw_core::Date(i32::MIN);
    let n_people = w.people.len();
    for s in w.media.stories.iter() {
        let mut bad = |m: String| out.push(format!("story {:?} ({:?}, {:?}, {:?}): {m}", s.id, s.kind, s.claim_type, s.truth));
        if s.date.0 < last_date.0 {
            bad(format!("published on {:?}, before the story ahead of it ({:?})", s.date, last_date));
        }
        last_date = s.date;
        // The four categories.
        match s.truth {
            Truth::False => {
                // A story is false because it was not so, or because what the writer held was garbled or out of date; the public record
                // it came from must then not be presented as grounded, unless it was drawn from the public record itself.
                if s.grounded && s.info != u32::MAX {
                    let f = w.grapevine.get(s.info).knower(s.journalist).map(|k| k.fidelity);
                    if !matches!(f, Some(Fidelity::Garbled | Fidelity::Outdated) | None) {
                        bad(format!("false yet grounded and its item is held as {f:?}"));
                    }
                } else if s.grounded {
                    bad("false yet grounded, with no item behind it".into());
                }
            }
            Truth::Accurate | Truth::AccurateAtTime | Truth::Misleading => {
                if !s.grounded {
                    bad("called true-in-substance but was not grounded".into());
                }
                if s.info != u32::MAX && let Some(k) = w.grapevine.get(s.info).knower(s.journalist) {
                    if matches!(k.fidelity, Fidelity::Planted) {
                        bad("the journalist held a planted item, yet the story is not called manipulated".into());
                    }
                }
            }
            Truth::Manipulated => {
                if s.leaker.is_none() && s.info == u32::MAX {
                    bad("manipulated but nobody planted it".into());
                }
            }
        }
        if s.claim_type == ClaimType::Fact && !s.grounded {
            bad("stated as fact but not grounded".into());
        }
        if s.claim > 100 || s.news > 100 || s.verification.confidence > 100 || !(-100..=100).contains(&s.tone) {
            bad(format!("a number left its range: claim {} news {} confidence {} tone {}", s.claim, s.news, s.verification.confidence, s.tone));
        }
        if s.verification.confirmed > s.verification.asked || s.verification.denied > s.verification.asked {
            bad(format!("{:?} confirmed/denied more than were asked", s.verification));
        }
        if matches!(s.kind, StoryKind::Correction | StoryKind::Denial) || matches!(s.claim_type, ClaimType::Correction | ClaimType::Denial) {
            for r in &s.refs {
                if r.0 >= s.id.0 || w.media.stories.get(*r).is_some_and(|o| o.date.0 > s.date.0) {
                    bad(format!("refers to {r:?}, which is not earlier"));
                }
            }
        }
        if s.journalist.is_some() && !w.media.journalists.contains_key(&s.journalist) && s.journalist.0 as usize >= n_people {
            bad("written by nobody".into());
        }
    }
    for ((a, b), bond) in &w.media.bonds {
        let mut bad = |m: String| out.push(format!("bond {a:?}->{b:?}: {m}"));
        for (name, v) in [("respect", bond.respect), ("warmth", bond.warmth), ("trust", bond.trust)] {
            if !(-100..=100).contains(&v) {
                bad(format!("{name} {v}"));
            }
        }
        if bond.grudge > 100 {
            bad(format!("grudge {}", bond.grudge));
        }
        if bond.last.0 < bond.since.0 {
            bad("last touched before it began".into());
        }
        if a == b {
            bad("a party has a bond with itself".into());
        }
        for party in [a, b] {
            match party {
                Party::Person(p) if p.0 as usize >= n_people => bad("unknown person".into()),
                Party::Club(c) if c.0 as usize >= w.clubs.len() => bad("unknown club".into()),
                Party::Outlet(o) if o.0 as usize >= w.media.outlets.len() => bad("unknown outlet".into()),
                _ => {}
            }
        }
        for r in &bond.history {
            if r.date.0 < bond.since.0 - 1 || r.date.0 > w.date.0 {
                bad(format!("a reason dated {:?} outside {:?}..{:?}", r.date, bond.since, w.date));
            }
        }
    }
    for f in &w.media.feuds {
        if f.heat > 100 || f.a == f.b || f.last.0 < f.since.0 {
            out.push(format!("feud {:?}/{:?}: heat {} since {:?} last {:?}", f.a, f.b, f.heat, f.since, f.last));
        }
        if !w.media.journalists.contains_key(&f.a) || !w.media.journalists.contains_key(&f.b) {
            out.push(format!("feud {:?}/{:?} between people who are not journalists", f.a, f.b));
        }
    }
    for ((club, p), fan) in &w.media.fans {
        if !(-1000..=1000).contains(&fan.score) {
            out.push(format!("fans of {club:?} rate {p:?} at {}", fan.score));
        }
    }
    for (p, v) in w.media.image.iter().chain(w.media.insider.iter()) {
        if !(-1000..=1000).contains(v) {
            out.push(format!("{p:?} has an image/insider score {v}"));
        }
    }
    for r in &w.media.reactions {
        if !(-100..=100).contains(&r.sentiment) || r.volume == 0 || r.volume > 20 {
            out.push(format!("reaction with sentiment {} volume {}", r.sentiment, r.volume));
        }
    }
    out
}

fn social_problems(w: &World) -> Vec<String> {
    let mut out = Vec::new();
    let n_people = w.people.len();
    let n_acc = w.net.accounts.len() as u32;
    for (&acc, ops) in &w.net.opinions {
        if acc >= n_acc {
            out.push(format!("opinions held by unknown account {acc}"));
            continue;
        }
        let mut seen = std::collections::HashSet::new();
        for o in ops {
            let mut bad = |m: String| out.push(format!("account {acc} about {:?}: {m}", o.about));
            if !seen.insert(o.about) {
                bad("two opinions about the same person".into());
            }
            if o.about.0 as usize >= n_people {
                bad("about nobody".into());
            }
            for (i, d) in o.dims.iter().enumerate() {
                if !(-1000..=1000).contains(d) {
                    bad(format!("dimension {i} ({}) is {d}", Dim::ALL[i].label()));
                }
            }
            if !(-1000..=1000).contains(&o.score) {
                bad(format!("score {}", o.score));
            }
            if o.low > o.high {
                bad(format!("low {} above high {}", o.low, o.high));
            }
            if o.score < o.low || o.score > o.high {
                bad(format!("score {} outside its own history {}..{}", o.score, o.low, o.high));
            }
            if o.since.0 > w.date.0 {
                bad("held since the future".into());
            }
        }
    }
    let _ = N_DIMS;
    for (&who, a) in &w.net.attention {
        let mut bad = |m: String| out.push(format!("attention on {who:?}: {m}"));
        if who.0 as usize >= n_people {
            bad("on nobody".into());
        }
        if !(0.0..=1.0).contains(&a.general_share) {
            bad(format!("general share {}", a.general_share));
        }
        if a.last_spark.0 > w.date.0 {
            bad("sparked in the future".into());
        }
        for wave in &a.waves {
            if !(0.0..=1.0001).contains(&wave.peak) || wave.start.0 > w.date.0 {
                bad(format!("wave peak {} start {:?}", wave.peak, wave.start));
            }
            let l = wave.level(w.date);
            if !(0.0..=1.0001).contains(&l) {
                bad(format!("wave level {l}"));
            }
        }
        let total = pw_sim::attention::level(w, who);
        if !(0.0..=1.0001).contains(&total) || !total.is_finite() {
            bad(format!("total level {total} is not a probability"));
        }
    }
    for n in &w.net.nicknames {
        if n.person.0 as usize >= n_people || n.last_used.0 < n.born.0 || n.born.0 > w.date.0 {
            out.push(format!("nickname {n:?} is malformed"));
        }
    }
    for m in &w.net.myths {
        if m.embellishment > 100 || m.about.0 as usize >= n_people || m.last.0 < m.born.0 {
            out.push(format!("myth {m:?} is malformed"));
        }
    }
    for p in &w.net.posts {
        let mut bad = |m: String| out.push(format!("post {}: {m}", p.id));
        if p.author >= n_acc {
            bad("by an unknown account".into());
        }
        if p.reply_to != NO_POST && p.reply_to >= p.id {
            bad(format!("replies to {}, which is not earlier", p.reply_to));
        }
        if p.quote_of != NO_POST && p.quote_of >= p.id {
            bad(format!("quotes {}, which is not earlier", p.quote_of));
        }
        if p.refs.iter().any(|&r| r != NO_POST && r >= p.id) {
            bad("cites a later post".into());
        }
        if p.intensity > 100 || !(-1000..=1000).contains(&p.prior) {
            bad(format!("intensity {} prior {}", p.intensity, p.prior));
        }
        if p.about.is_some() && p.about.0 as usize >= n_people {
            bad("about nobody".into());
        }
    }
    let _ = PersonId::NONE;
    out
}

fn assert_clean(_w: &World, what: &str, mut problems: Vec<String>) {
    let n = problems.len();
    problems.truncate(12);
    assert!(n == 0, "{what}: {n} problems, first {problems:#?}");
}

#[test]
fn media_categories_and_relationships_hold_up_across_seeds() {
    for seed in [121u64, 122, 123] {
        let s = ran(Scale::TINY, seed, 3 * 365);
        let w = &s.world;
        assert!(w.media.stories.len() > 300, "seed {seed}: the press was busy ({})", w.media.stories.len());
        assert_clean(w, &format!("media, seed {seed}"), media_problems(w));
    }
}

#[test]
fn social_opinion_and_attention_stay_in_range_across_seeds() {
    for seed in [131u64, 132, 133] {
        let s = ran(Scale::TINY, seed, 3 * 365);
        let w = &s.world;
        assert!(w.net.opinions.values().map(|v| v.len()).sum::<usize>() > 100, "seed {seed}: accounts hold opinions");
        assert_clean(w, &format!("social, seed {seed}"), social_problems(w));
    }
}

/// The checks hold at intermediate dates too, not only at the end: every 150 days of a small world.
#[test]
#[ignore = "heavy: small world, checks every 150 days for three seasons"]
fn media_and_social_hold_at_every_stage_on_the_small_world() {
    let mut s = sim(Scale::SMALL, 141);
    for _ in 0..7 {
        s.run(150);
        assert_clean(&s.world, "media", media_problems(&s.world));
        assert_clean(&s.world, "social", social_problems(&s.world));
    }
}
