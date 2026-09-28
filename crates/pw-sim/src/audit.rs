//! Semantic truth guards (20 in the final brief).
//!
//! The world's communications must not contradict its state. `audit`
//! walks the records that text is rendered from and reports anything that
//! would let text say something untrue: a story stated as fact that was not
//! grounded, a post referring to a post that does not exist, a call-out
//! that does not quote a real earlier post, a record "broken" by a worse
//! mark, an inbox message without a source, a generated past figure
//! sharing a real person's name, a school membership out of step. Tests run
//! it after long simulations; it returns data, it never repairs.

use pw_world::inbox::MsgSource;
use pw_world::media::ClaimType;
use pw_world::socialnet::{Concept, Frame, NO_POST};
use pw_world::{FxHashSet, World};

#[derive(Clone, Debug, PartialEq)]
pub enum Violation {
    /// A story stated as fact that rests on nothing true.
    UngroundedFact { story: u32 },
    DanglingPostRef { post: u32, missing: u32 },
    /// A call-out that does not point at the called-out author's own post.
    CallOutWithoutPrior { post: u32 },
    /// A relayed rumour without a story behind it.
    RelayWithoutStory { post: u32 },
    /// A record "broken" by a mark that is not better.
    RecordNotBetter { broken: u32 },
    InboxWithoutSource { message: u32 },
    /// A generated figure of the past shares a real person's name.
    PastFigureNamedLikeReal { figure: u32 },
    /// Minor football membership lists disagree.
    MembershipMismatch { player: u32 },
    /// A vote's points do not add up to its ballots.
    VoteTally { vote: u32 },
    /// A referee recorded for a fixture is not the one the fixture draws.
    RefereeAssignment { uid: u64 },
    /// A rule value outside its sane range.
    RuleOutOfRange { change: u32 },
}

pub fn audit(w: &World) -> Vec<Violation> {
    let mut v = Vec::new();
    stories(w, &mut v);
    posts(w, &mut v);
    records(w, &mut v);
    inbox(w, &mut v);
    past(w, &mut v);
    minor(w, &mut v);
    votes(w, &mut v);
    rules(w, &mut v);
    v
}

fn stories(w: &World, v: &mut Vec<Violation>) {
    for s in w.media.stories.iter() {
        let public = matches!(s.source, pw_world::event::Cause::Event(_));
        if s.claim_type == ClaimType::Fact && !s.grounded && !public {
            v.push(Violation::UngroundedFact { story: s.id.0 });
        }
    }
}

fn posts(w: &World, v: &mut Vec<Violation>) {
    for p in &w.net.posts {
        for r in p.refs.iter().copied().chain([p.reply_to, p.quote_of]) {
            if r != NO_POST && w.net.post(r).is_none() {
                v.push(Violation::DanglingPostRef { post: p.id, missing: r });
            }
        }
        if p.concept == Concept::CallOut {
            // The quoted earlier post must be by the author of the post being answered.
            let answered = w.net.post(if p.reply_to != NO_POST { p.reply_to } else { p.quote_of }).map(|x| x.author);
            let ok = p.refs.first().and_then(|&r| w.net.post(r)).is_some_and(|e| Some(e.author) == answered || e.author == p.author);
            if !ok && !p.refs.is_empty() {
                v.push(Violation::CallOutWithoutPrior { post: p.id });
            }
        }
        if p.concept == Concept::Relay && !matches!(p.frame, Frame::Story { .. }) {
            v.push(Violation::RelayWithoutStory { post: p.id });
        }
    }
}

fn records(w: &World, v: &mut Vec<Violation>) {
    for (i, b) in w.records.broken.iter().enumerate() {
        if let Some(old) = b.old {
            let better = if b.key.stat.lower_is_better() { b.new.value < old.value } else { b.new.value > old.value };
            if !better {
                v.push(Violation::RecordNotBetter { broken: i as u32 });
            }
        }
    }
}

fn inbox(w: &World, v: &mut Vec<Violation>) {
    for m in &w.inbox.messages {
        let exists = match m.source {
            MsgSource::Decision { decision } => w.decisions.all.get(decision).is_some(),
            MsgSource::Tell { info, .. } => (info as usize) < w.grapevine.items.len(),
            MsgSource::Story { story } => w.media.stories.get(story).is_some(),
            MsgSource::Mention { post } => w.net.post(post).is_some() || post < w.net.post_base,
            MsgSource::Question { conference, .. } => (conference as usize) < w.pressroom.conferences.len(),
            // Events may have been compacted away; the id must at least have existed.
            MsgSource::Meeting { event } | MsgSource::Private { event } => event.0 < w.events.last_id().0.saturating_add(1),
        };
        if !exists {
            v.push(Violation::InboxWithoutSource { message: m.id });
        }
    }
}

fn past(w: &World, v: &mut Vec<Violation>) {
    let real: FxHashSet<String> = w.people.iter().map(|p| p.display_name(&w.names).into_owned()).collect();
    for f in &w.backfill.figures {
        if f.provenance == pw_world::backfill::Provenance::Generated && real.contains(&f.name) {
            v.push(Violation::PastFigureNamedLikeReal { figure: f.id });
        }
    }
}

fn minor(w: &World, v: &mut Vec<Violation>) {
    for (&p, &i) in &w.minor.member_of {
        if !w.minor.institutions.get(i as usize).is_some_and(|x| x.members.contains(&p)) {
            v.push(Violation::MembershipMismatch { player: p.0 });
        }
    }
}

fn votes(w: &World, v: &mut Vec<Violation>) {
    for vote in &w.acclaim.votes {
        if vote.casts.is_empty() || vote.result.len() >= 30 {
            continue;
        }
        let expected: u32 = vote
            .casts
            .iter()
            .map(|c| {
                let pts: &[u32] = if c.kind == pw_world::awards::VoterKind::Committee { &[1, 1, 1] } else { &[5, 3, 1] };
                pts.iter().take(c.picks.len()).sum::<u32>()
            })
            .sum();
        let got: u32 = vote.result.iter().map(|r| r.1).sum();
        if expected != got {
            v.push(Violation::VoteTally { vote: vote.id });
        }
    }
}

fn rules(w: &World, v: &mut Vec<Violation>) {
    use pw_world::evolution::RuleKey;
    for c in &w.evolution.changes {
        let ok = match c.key {
            RuleKey::Subs => (3..=5).contains(&c.new),
            RuleKey::RedBan => (1..=6).contains(&c.new),
            RuleKey::HomegrownMin => (0..=10).contains(&c.new),
            RuleKey::AwayGoals => c.new == 0,
        };
        if !ok {
            v.push(Violation::RuleOutOfRange { change: c.id });
        }
    }
}

/// The referee recorded for each recent fixture is the one it draws.
pub fn referees(w: &World) -> Vec<Violation> {
    let mut v = Vec::new();
    for m in &w.recent_matches.list {
        let fx = w.fixtures.get(m.fixture);
        if let Some(&r) = w.officials.assigned.get(&m.uid) {
            if crate::officials::referee_for(w, fx) != Some(r) {
                v.push(Violation::RefereeAssignment { uid: m.uid });
            }
        }
    }
    v
}
