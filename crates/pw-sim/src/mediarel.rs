//! Media relationships and rivalries (locked design 2.13-2.21).
//!
//! Relationships in the media's world are persistent, directional and remember why. A player and a journalist do not feel the same
//! about each other, and neither is a single number: respect (professional regard) is separate from warmth (liking), and both from
//! trust and from a grudge that outlasts the mood. What a story does to its subject and to the subject's club leaves a mark on both;
//! what the marks add up to changes who gets access, how the journalist writes about them, and who they talk to. Two journalists on the
//! same ground can fall into a feud that cools with time and flares again.

use pw_core::rng::stream;
use pw_core::{ClubId, Date, PersonId, StoryId};
use pw_world::media::{BondCause, BondReason, Feud, FeudCause, Intent, MediaBond, Party, SourceAim, Story, Truth};
use pw_world::World;

/// Bonds kept in a world; beyond this the least significant are forgotten.
const MAX_BONDS: usize = 40_000;

/// A grudge this deep does not go away with the mood.
const GRUDGE_LINE: u8 = 30;

pub fn bond(w: &World, from: Party, to: Party) -> Option<&MediaBond> {
    w.media.bonds.get(&(from, to))
}

/// Move a relationship, recording why. A conflict that has happened before, and recently, is worse the next time: old grievances
/// reignite (locked design 2.19).
#[allow(clippy::too_many_arguments)]
pub fn adjust(w: &mut World, from: Party, to: Party, cause: BondCause, story: StoryId, respect: i8, warmth: i8, trust: i8, grudge: i8) {
    if from == to {
        return;
    }
    let today = w.date;
    let b = w.media.bonds.entry((from, to)).or_insert_with(|| MediaBond::new(today));
    let prior = b.history.iter().filter(|r| r.cause == cause && r.date.days_until(today) < 730).count().min(3) as f32;
    let hurts = warmth < 0 || grudge > 0;
    let k = if hurts { 1.0 + 0.4 * prior } else { 1.0 };
    let scale = |v: i8| (f32::from(v) * k).round() as i16;
    b.respect = (i16::from(b.respect) + scale(respect)).clamp(-100, 100) as i8;
    b.warmth = (i16::from(b.warmth) + scale(warmth)).clamp(-100, 100) as i8;
    b.trust = (i16::from(b.trust) + scale(trust)).clamp(-100, 100) as i8;
    b.grudge = (i16::from(b.grudge) + scale(grudge)).clamp(0, 100) as u8;
    b.last = today;
    if b.history.len() >= 4 {
        b.history.remove(0);
    }
    b.history.push(BondReason { cause, date: today, story });
    if w.media.bonds.len() > MAX_BONDS {
        prune(w);
    }
}

fn weight(b: &MediaBond) -> i32 {
    i32::from(b.respect).abs() + i32::from(b.warmth).abs() + i32::from(b.trust).abs() + i32::from(b.grudge)
}

fn prune(w: &mut World) {
    let mut ws: Vec<i32> = w.media.bonds.values().map(weight).collect();
    ws.sort_unstable();
    let cut = ws[ws.len() / 10];
    w.media.bonds.retain(|_, b| weight(b) > cut);
}

/// What one story does to the people in it and the club it is about.
pub fn on_story(w: &mut World, id: StoryId) {
    let s: Story = w.media.stories[id].clone();
    let grudge_before = if s.person.is_some() { bond(w, Party::Person(s.person), Party::Person(s.journalist)).map_or(0, |b| b.grudge) } else { 0 };
    let j = Party::Person(s.journalist);
    let harsh = s.tone <= -30;
    let warm = s.tone >= 30;
    if !(harsh || warm) {
        return;
    }
    let false_ish = matches!(s.truth, Truth::False | Truth::Manipulated);
    if s.person.is_some() && s.person != s.journalist && !w.media.journalists.contains_key(&s.person) {
        let subject = Party::Person(s.person);
        if harsh {
            match s.truth {
                Truth::False | Truth::Manipulated => adjust(w, subject, j, BondCause::FalseStory, id, -10, -8, -6, 12),
                Truth::Misleading => adjust(w, subject, j, BondCause::MisleadingFraming, id, -6, -6, -4, 8),
                // Harsh and true: disliked, and respected for it (section 2.20).
                Truth::Accurate | Truth::AccurateAtTime => adjust(w, subject, j, BondCause::HarshButTrue, id, 3, -6, 0, 3),
            }
        } else if warm {
            let mouthpiece = false_ish || s.intent == Intent::Favour;
            adjust(w, subject, j, BondCause::Praised, id, if mouthpiece { -1 } else { 1 }, 6, 3, 0);
        }
    }
    // A grievance that has become lasting is an event of its own, caused by the story, and the person lives through it.
    if s.person.is_some() && s.person != s.journalist {
        let now = bond(w, Party::Person(s.person), Party::Person(s.journalist)).map_or(0, |b| b.grudge);
        if grudge_before < GRUDGE_LINE && now >= GRUDGE_LINE {
            let cause = bond(w, Party::Person(s.person), Party::Person(s.journalist)).and_then(|b| b.history.last().map(|r| r.cause)).unwrap_or(BondCause::FalseStory);
            let mut causes = pw_world::Causes::new();
            if s.event.is_some() {
                causes.push(pw_world::Cause::Event(s.event));
            }
            let vis = pw_world::event::Visibility::Between(s.person, s.journalist);
            w.events.push_caused(w.date, vis, pw_world::EventKind::MediaGrudge { subject: s.person, journalist: s.journalist, cause }, causes);
        }
    }
    // The club feels it about the outlet.
    if s.club.is_some() && s.outlet.is_some() {
        let (club, outlet) = (Party::Club(s.club), Party::Outlet(s.outlet));
        if harsh {
            let (cause, wm, gr) = match s.truth {
                Truth::False | Truth::Manipulated => (BondCause::FalseStory, -8, 6),
                Truth::Misleading => (BondCause::MisleadingFraming, -6, 4),
                _ => (BondCause::HarshButTrue, -2, 0),
            };
            adjust(w, club, outlet, cause, id, 0, wm, 0, gr);
        } else {
            adjust(w, club, outlet, BondCause::Praised, id, 0, 3, 0, 0);
        }
    }
}

/// A person speaking to a journalist: whether they will. A grudge, a cold relationship, or a club that has frozen an outlet out, all
/// make refusal likely; a warm one makes it rare. AI speakers only; a human chooses for themselves.
pub fn grants_access(w: &World, speaker: PersonId, journalist: PersonId) -> bool {
    let today = w.date;
    let mut refuse = 0.0f32;
    if let Some(b) = bond(w, Party::Person(speaker), Party::Person(journalist)) {
        refuse += f32::from(b.grudge) / 100.0 * 0.9 + f32::from((-i16::from(b.warmth)).max(0) as u8) / 200.0;
        refuse -= f32::from(b.warmth.max(0) as u8) / 300.0;
    }
    let club = w.club_of_person(speaker);
    let outlet = w.media.journalists.get(&journalist).map_or(pw_core::OutletId::NONE, |j| j.outlet);
    if club.is_some()
        && outlet.is_some()
        && let Some(b) = bond(w, Party::Club(club), Party::Outlet(outlet))
        && b.warmth <= -50
    {
        refuse += 0.5;
    }
    let roll = w.roll(stream::MEDIA, &[u64::from(speaker.0), u64::from(journalist.0), today.0 as u64, 0xacc]);
    roll >= refuse.clamp(0.0, 0.95)
}

/// Called when someone speaks (or declines to): the journalist and the speaker each remember it.
pub fn spoke(w: &mut World, speaker: PersonId, journalist: PersonId, story: StoryId, granted: bool) {
    let (s, j) = (Party::Person(speaker), Party::Person(journalist));
    if granted {
        adjust(w, j, s, BondCause::GaveInterview, story, 1, 2, 2, 0);
    } else {
        adjust(w, j, s, BondCause::RefusedInterview, story, -1, -3, -2, 3);
    }
}

/// How the journalist's own view of the subject bends the tone of a story: a grudge writes harsher, warmth softer. Points on the tone scale.
pub fn tone_bias(w: &World, journalist: PersonId, subject: PersonId) -> f32 {
    if subject.is_none() {
        return 0.0;
    }
    bond(w, Party::Person(journalist), Party::Person(subject)).map_or(0.0, |b| 0.15 * f32::from(b.warmth) - 0.25 * f32::from(b.grudge))
}

/// Why the journalist and outlet ran it as they did (locked design 2.4), from the relationship and the outlet, not from the facts.
pub fn intent_for(w: &World, s: &Story, framing_gap: f32) -> Intent {
    let j = Party::Person(s.journalist);
    if s.person.is_some()
        && let Some(b) = bond(w, j, Party::Person(s.person))
    {
        if s.tone < -20 && (b.grudge >= 30 || b.warmth <= -35) {
            return Intent::Punish;
        }
        if s.tone > 20 && b.warmth >= 35 {
            return Intent::Favour;
        }
    }
    if s.leaker.is_some()
        && s.tone > 20
        && let Some(b) = bond(w, j, Party::Person(s.leaker))
        && b.trust >= 30
    {
        return Intent::Favour;
    }
    let sens = s.outlet.is_some().then(|| w.media.outlets[s.outlet].sensationalism).unwrap_or(8);
    if sens >= 14 && framing_gap.abs() >= 10.0 {
        return Intent::Engage;
    }
    Intent::Inform
}

/// What the source wanted, from how the last hop of the information reached the journalist.
pub fn aim_of(w: &World, s: &Story) -> SourceAim {
    use pw_world::info::Motive;
    if s.info == u32::MAX || s.leaker.is_none() {
        return SourceAim::Genuine;
    }
    let motive = w.grapevine.chain(s.info, s.journalist).last().map(|t| t.motive);
    match motive {
        Some(Motive::AgentStrategy) => SourceAim::RaiseValue,
        Some(Motive::ClubStrategy) => SourceAim::ShapeNarrative,
        Some(Motive::PlayerStrategy) => SourceAim::ForcePlayer,
        Some(Motive::Revenge) => SourceAim::Damage,
        Some(Motive::PressFriendship) => SourceAim::Friendship,
        _ => SourceAim::Genuine,
    }
}

// ------------------------------------------------------------------ media against media

fn feud_index(w: &World, a: PersonId, b: PersonId) -> Option<usize> {
    w.media.feuds.iter().position(|f| (f.a == a && f.b == b) || (f.a == b && f.b == a))
}

/// `who` has a reason to resent `rival`.
pub fn heat(w: &mut World, who: PersonId, rival: PersonId, cause: FeudCause, by: u8) {
    if who == rival {
        return;
    }
    let today = w.date;
    match feud_index(w, who, rival) {
        Some(i) => {
            let f = &mut w.media.feuds[i];
            // A feud that had gone quiet flares faster than a new one starts (locked design 2.19).
            let flare = if f.last.days_until(today) > 120 { 2 } else { 1 };
            f.heat = f.heat.saturating_add(by * flare).min(100);
            f.last = today;
            if f.causes.len() >= 4 {
                f.causes.remove(0);
            }
            f.causes.push((cause, today));
        }
        None => {
            let mut causes = smallvec::SmallVec::new();
            causes.push((cause, today));
            w.media.feuds.push(Feud { a: who, b: rival, heat: by, since: today, last: today, causes });
        }
    }
}

/// After a story on a running matter: everyone who wrote about it after someone else was beaten to it.
pub fn scooped(w: &mut World, id: StoryId) {
    let s = &w.media.stories[id];
    let thread = s.thread;
    if thread == u32::MAX {
        return;
    }
    let (mine, date) = (s.journalist, s.date);
    let firsts: Vec<(PersonId, Date)> = w.media.threads[thread as usize].stories.iter().filter(|&&x| x != id).map(|&x| (w.media.stories[x].journalist, w.media.stories[x].date)).collect();
    // The first to publish on the thread scooped anyone who comes after within a few days from another outlet.
    if let Some(&(first, when)) = firsts.iter().min_by_key(|x| x.1)
        && first != mine
        && when.days_until(date) <= 3
        && when < date
    {
        let (a_out, b_out) = (w.media.journalists.get(&first).map(|j| j.outlet), w.media.journalists.get(&mine).map(|j| j.outlet));
        if a_out != b_out {
            heat(w, mine, first, FeudCause::Scooped, 6);
        }
    }
}

/// A story is shown to have been wrong; journalists who wrote the opposite have reason to say so.
pub fn exposed(w: &mut World, id: StoryId) {
    let s = w.media.stories[id].clone();
    if s.thread == u32::MAX {
        return;
    }
    let rivals: Vec<PersonId> = w.media.threads[s.thread as usize].stories.iter().map(|&x| &w.media.stories[x]).filter(|o| o.journalist != s.journalist && matches!(o.claim_type, pw_world::media::ClaimType::Denial | pw_world::media::ClaimType::Correction)).map(|o| o.journalist).collect();
    for r in rivals {
        heat(w, r, s.journalist, FeudCause::Discredited, 10);
        heat(w, s.journalist, r, FeudCause::Contradicted, 4);
    }
}

/// One journalist corrects a story, which is a small public defeat for whoever wrote the other side.
pub fn corrected(w: &mut World, id: StoryId) {
    exposed(w, id);
}

/// Weekly: feuds cool, and flare where the two still cover the same club; a hot one goes after the other's sources. Monthly: bonds fade,
/// grudges more slowly than moods.
pub fn weekly(w: &mut World) {
    let today = w.date;
    let feuds = std::mem::take(&mut w.media.feuds);
    let mut kept = Vec::with_capacity(feuds.len());
    for mut f in feuds {
        // Same ground: they keep running into each other.
        let ca: Vec<ClubId> = w.media.journalists.get(&f.a).map(|j| j.beat.to_vec()).unwrap_or_default();
        let overlap = w.media.journalists.get(&f.b).is_some_and(|j| j.beat.iter().any(|c| ca.contains(c)));
        if overlap && f.heat > 20 && w.roll(stream::MEDIA, &[u64::from(f.a.0), u64::from(f.b.0), today.0 as u64, 0xfe0d]) < 0.15 {
            f.heat = f.heat.saturating_add(2).min(100);
        }
        // A hot feud: one tries to take the other's best source.
        if f.heat >= 45 && w.roll(stream::MEDIA, &[u64::from(f.a.0), u64::from(f.b.0), today.0 as u64, 0x50c]) < 0.08 {
            poach(w, f.a, f.b);
            if f.causes.len() >= 4 {
                f.causes.remove(0);
            }
            f.causes.push((FeudCause::PoachedSource, today));
        }
        f.heat = (f32::from(f.heat) * 0.97) as u8;
        if f.heat >= 3 || f.since.days_until(today) < 90 {
            kept.push(f);
        }
    }
    w.media.feuds = kept;
    if today.day() <= 7 {
        for b in w.media.bonds.values_mut() {
            b.respect = (f32::from(b.respect) * 0.98) as i8;
            b.warmth = (f32::from(b.warmth) * 0.94) as i8;
            b.trust = (f32::from(b.trust) * 0.96) as i8;
            b.grudge = (f32::from(b.grudge) * 0.99) as u8;
        }
        w.media.bonds.retain(|_, b| weight(b) > 2);
    }
}

/// `a` tries to take `b`'s strongest source: where they have a tie to the same person it shifts.
fn poach(w: &mut World, a: PersonId, b: PersonId) {
    let Some(target) = w.media.journalist_profiles.get(&b).and_then(|p| p.ties.iter().max_by_key(|t| t.strength).map(|t| t.person)) else { return };
    let mine_has = w.media.journalist_profiles.get(&a).is_some_and(|p| p.tie(target).is_some());
    if let Some(p) = w.media.journalist_profiles.get_mut(&b)
        && let Some(t) = p.ties.iter_mut().find(|t| t.person == target)
    {
        t.strength = t.strength.saturating_sub(6);
    }
    if mine_has && let Some(p) = w.media.journalist_profiles.get_mut(&a) && let Some(t) = p.ties.iter_mut().find(|t| t.person == target) {
        t.strength = t.strength.saturating_add(6).min(100);
    }
}
