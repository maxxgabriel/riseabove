//! Attention, virality, brand and folklore (locked design 6.16-6.40).
//!
//! Football reputation, fame, attention and commercial appeal are kept apart. Attention comes in waves: a goal, a quote, a look, a row, a
//! comeback, a joke. Each has its own half-life and its own reach beyond football, so some fade in days and some spill into audiences
//! that never watch a match. What the market makes of it (brand) is a bounded premium beside football value, never in place of it. Some
//! attention turns into nicknames that spread, mutate and die; some moments become folklore, retold more often than checked. How someone
//! looks is read differently by different audiences, and it is generated, never a fact about anyone.

use std::collections::HashMap;

use pw_core::rng::hash_key;
use pw_core::{PersonId, PlayerId};
use pw_world::attention::{Cause, Myth, NickKind, NickState, Nickname, Wave};
use pw_world::socialnet::{AccountKind, Frame, MomentKind, SocialAccount};
use pw_world::World;

/// Who is looking: taste in looks is not one thing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Taste {
    General,
    Celebrity,
    Football,
}

/// How strong the attention on a person is right now, 0..1.
pub fn level(w: &World, who: PersonId) -> f32 {
    w.net.attention.get(&who).map_or(0.0, |a| a.waves.iter().map(|x| x.level(w.date)).sum::<f32>().min(1.0))
}

/// The strength of one kind of wave.
pub fn level_of(w: &World, who: PersonId, cause: Cause) -> f32 {
    w.net.attention.get(&who).map_or(0.0, |a| a.waves.iter().filter(|x| x.cause == cause).map(|x| x.level(w.date)).sum::<f32>().min(1.0))
}

/// Something sets attention on someone. A new wave of the same kind builds on what is left of the old one; beyond football, the more
/// a kind of wave reaches, the more it widens who follows him and how famous he is outside the game.
pub fn spark(w: &mut World, who: PersonId, cause: Cause, mag: f32) {
    let today = w.date;
    let mag = mag.clamp(0.0, 1.0);
    if who.is_none() || mag < 0.15 {
        return;
    }
    let a = w.net.attention.entry(who).or_default();
    match a.waves.iter_mut().find(|x| x.cause == cause) {
        Some(x) => {
            let left = x.level(today);
            x.peak = (left + mag * 0.7).min(1.0);
            x.start = today;
        }
        None => {
            if a.waves.len() >= 3
                && let Some(i) = a.waves.iter().enumerate().min_by(|x, y| x.1.level(today).total_cmp(&y.1.level(today))).map(|(i, _)| i)
            {
                a.waves.remove(i);
            }
            a.waves.push(Wave { cause, start: today, peak: mag });
        }
    }
    a.general_share = (a.general_share + (1.0 - a.general_share) * cause.reach() * mag * 0.2).clamp(0.0, 1.0);
    a.last_spark = today;
    if cause.reach() >= 0.4 {
        // It has left football: people who never watch a match now know his name.
        let r = w.renown.people.entry(who).or_default();
        r.fame = (f32::from(r.fame) + 300.0 * mag * cause.reach()).min(10_000.0) as u16;
        r.followers = r.followers.saturating_add((20_000.0 * mag * cause.reach()) as u32);
    }
    // What people say and do reaches him, and he reads it in his own way (locked design 7.49).
    crate::lifestate::on_attention(w, who, cause, mag);
}

/// A frame everyone saw: what kind of attention it draws, and how much.
pub fn on_frame(w: &mut World, f: Frame, about: PersonId, fw: f32) {
    if about.is_none() {
        return;
    }
    match f {
        Frame::HatTrick { .. } | Frame::LateWinner { .. } | Frame::Record { .. } | Frame::Award { .. } => {
            spark(w, about, Cause::Football, fw);
            // A good-looking player's big night is also a look at him.
            let chance = w.roll(pw_core::rng::stream::SOCIAL_ACTIVITY, &[u64::from(about.0), w.date.0 as u64, 0xae57]);
            if appeal(w, about, Taste::Celebrity) > 0.5 && chance < 0.3 {
                spark(w, about, Cause::Aesthetic, 0.7 * fw);
            }
        }
        Frame::RedCard { .. } | Frame::TransferRequest { .. } | Frame::Incident { .. } => spark(w, about, Cause::Controversy, 0.9 * fw),
        Frame::Injury { .. } => spark(w, about, Cause::Emotional, 0.5 * fw),
        Frame::Quote { quote } => {
            use pw_world::media::Stance;
            if let Some(q) = w.pressroom.quotes.get(quote as usize) {
                let subject = if q.about.is_some() { q.about } else { q.speaker };
                match q.stance {
                    Stance::Ambition | Stance::Criticise | Stance::Complain => spark(w, subject, Cause::Controversy, 0.6),
                    Stance::Praise | Stance::Loyalty | Stance::Support => spark(w, subject, Cause::Personality, 0.5),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// How someone looks to one audience, -1..1. Generated from who the person is and who is looking, stable over time except for age, and
/// never a fact about anyone: the same person is read differently by different audiences (locked design 6.16, 6.17).
pub fn appeal(w: &World, who: PersonId, taste: Taste) -> f32 {
    let h = |salt: u64| (hash_key(&[w.seed, u64::from(who.0), taste as u64, salt]) % 2001) as f32 / 1000.0 - 1.0;
    // Two draws averaged: most people are unremarkable, a few strike a lot of people one way or the other.
    let base = 0.5 * (h(0xa1) + h(0xa2));
    let p = w.people[who].player;
    let age = if p.is_some() { w.age_years(p) } else { 30.0 };
    let youth = ((30.0 - age) / 12.0).clamp(-0.5, 1.0) * 0.2;
    (base + youth).clamp(-1.0, 1.0)
}

/// How much the market sees in him beyond football, 0..1: fame outside the game, whether his following is people who buy what football
/// sells, how his looks land, and how much is being said about him now.
pub fn commercial_appeal(w: &World, p: PlayerId) -> f32 {
    let who = w.players.cold[p].person;
    let fame = f32::from(w.renown.of(who).fame) / 10_000.0;
    let football = f32::from(w.players.cold[p].rep.world) / 10_000.0;
    (0.35 * football + 0.25 * fame + 0.20 * appeal(w, who, Taste::Celebrity).max(0.0) + 0.20 * level(w, who)).clamp(0.0, 1.0)
}

/// The share of his price the market adds for what he is worth to brands, at most a tenth: it sits beside football value and never in
/// place of it (locked design 6.24).
pub fn brand_premium(w: &World, p: PlayerId) -> f32 {
    0.10 * commercial_appeal(w, p)
}

/// How far the noise about him outruns what he is: attention now against what his standing as a footballer earns. Positive is hype.
pub fn overhyped(w: &World, p: PlayerId) -> f32 {
    let who = w.players.cold[p].person;
    level(w, who) + f32::from(w.renown.of(who).fame) / 20_000.0 - f32::from(w.players.cold[p].rep.world) / 10_000.0
}

/// Whether an account will say something about how a person looks, and in what spirit: 1 admiring, 2 genuinely unimpressed, 3 point-scoring.
pub fn looks_reaction(w: &World, acc: &SocialAccount, about: PersonId) -> Option<u32> {
    if about.is_none() || level_of(w, about, Cause::Aesthetic) < 0.25 || acc.persona.celebrity < 40 {
        return None;
    }
    let taste = if acc.persona.celebrity >= 70 {
        Taste::Celebrity
    } else if matches!(acc.kind, AccountKind::Casual | AccountKind::Neutral | AccountKind::International) {
        Taste::General
    } else {
        Taste::Football
    };
    let liked = appeal(w, about, taste);
    let their_club = {
        let p = w.people[about].player;
        if p.is_some() { w.players.hot[p].club } else { pw_core::ClubId::NONE }
    };
    if liked > 0.25 {
        Some(1)
    } else if their_club.is_some() && their_club == acc.rival && acc.persona.hostility >= 60 {
        Some(3)
    } else if liked < -0.25 && acc.persona.hostility >= 45 {
        Some(2)
    } else {
        None
    }
}

/// Weekly: waves fade, nicknames spread and mutate and die, folklore is retold.
pub fn weekly(w: &mut World) {
    let today = w.date;
    w.net.attention.retain(|_, a| {
        a.waves.retain(|x| x.level(today) >= 0.03);
        a.general_share *= 0.995;
        !(a.waves.is_empty() && a.general_share < 0.02)
    });
    nicknames(w);
    if today.day() <= 7 {
        folklore(w);
    }
}

fn nicknames(w: &mut World) {
    let today = w.date;
    // Someone with the world's attention and no name yet may get one.
    let hot: Vec<PersonId> = w.net.attention.iter().filter(|(_, a)| a.waves.iter().map(|x| x.level(today)).sum::<f32>() > 0.45).map(|(&p, _)| p).collect();
    for who in hot {
        if w.net.nicknames.iter().any(|n| n.person == who && n.state != NickState::Dead) {
            // Attention revives a name that was fading.
            if let Some(n) = w.net.nicknames.iter_mut().find(|n| n.person == who && n.state == NickState::Fading) {
                n.state = NickState::Rising;
                n.last_used = today;
            }
            continue;
        }
        if w.roll(pw_core::rng::stream::SOCIAL_ACTIVITY, &[u64::from(who.0), today.0 as u64, 0x21c4]) >= 0.05 {
            continue;
        }
        let dominant = w.net.attention[&who].waves.iter().max_by(|a, b| a.level(today).total_cmp(&b.level(today))).map(|x| x.cause);
        let remembered = w.net.memories.values().flat_map(|v| v.iter()).filter(|m| m.about == who).map(|m| m.kind).next();
        let kind = match (dominant, remembered) {
            (Some(Cause::Controversy), _) => NickKind::Mocking,
            (_, Some(k)) => NickKind::Moment(k),
            (Some(Cause::Meme), _) => NickKind::Mocking,
            _ if w.people[who].nation != w.people[who].nation2 && w.people[who].nation2.is_some() => NickKind::Origin,
            _ => NickKind::Diminutive,
        };
        if w.net.nicknames.len() < 6000 {
            let key = hash_key(&[w.seed, u64::from(who.0), today.0 as u64]) as u32;
            w.net.nicknames.push(Nickname { person: who, kind, born: today, users: 5, last_used: today, state: NickState::Rising, variants: 0, key });
        }
    }
    // Existing ones live their lives.
    let levels: HashMap<PersonId, f32> = w.net.nicknames.iter().map(|n| (n.person, level(w, n.person))).collect();
    for i in 0..w.net.nicknames.len() {
        let n = w.net.nicknames[i];
        if n.state == NickState::Dead {
            continue;
        }
        let lvl = levels.get(&n.person).copied().unwrap_or(0.0);
        let roll = w.roll(pw_core::rng::stream::SOCIAL_ACTIVITY, &[u64::from(n.person.0), u64::from(n.key), today.0 as u64, 0x21c5]);
        let x = &mut w.net.nicknames[i];
        if lvl > 0.15 {
            x.last_used = today;
        }
        match x.state {
            NickState::Rising => {
                x.users = x.users.saturating_add((lvl * 60.0) as u16 + 2);
                if x.users >= 120 || x.born.days_until(today) > 60 {
                    x.state = NickState::Established;
                }
            }
            NickState::Established => {
                x.users = x.users.saturating_add((lvl * 20.0) as u16);
                // A name in wide use gets bent.
                if roll < 0.02 && x.variants < 6 {
                    x.variants += 1;
                    x.key = hash_key(&[u64::from(x.key), u64::from(x.variants)]) as u32;
                }
                if lvl < 0.05 && x.last_used.days_until(today) > 90 {
                    x.state = NickState::Fading;
                }
            }
            NickState::Fading => {
                x.users = (f32::from(x.users) * 0.97) as u16;
                if x.last_used.days_until(today) > 270 {
                    x.state = NickState::Dead;
                }
            }
            NickState::Dead => {}
        }
    }
    let cutoff = today.add_days(-730);
    w.net.nicknames.retain(|n| n.state != NickState::Dead || n.last_used >= cutoff);
}

/// Monthly: a moment enough people remember becomes a story; stories are retold, and grow in the telling unless someone who knows
/// says otherwise.
fn folklore(w: &mut World) {
    let today = w.date;
    let mut counts: HashMap<(PersonId, MomentKind), (u32, pw_core::EventId, u32, u32)> = HashMap::new();
    for (&a, ms) in &w.net.memories {
        let knows = w.net.accounts[a as usize].persona.knowledge >= 60;
        for m in ms {
            let e = counts.entry((m.about, m.kind)).or_insert((0, m.event, 0, 0));
            e.0 += 1;
            e.2 += u32::from(knows);
            e.3 += u32::from(w.net.accounts[a as usize].persona.nostalgia >= 60);
        }
    }
    for (&(about, moment), &(n, event, knowing, nostalgic)) in &counts {
        match w.net.myths.iter().position(|m| m.about == about && m.moment == moment) {
            Some(i) => {
                // Retold in proportion to how many carry it, checked in proportion to how many of them know better.
                let retold = (nostalgic + n / 3).min(40) as u16;
                let corrected = (knowing.min(u32::from(retold)) / 2) as u16;
                let m = &mut w.net.myths[i];
                m.retellings = m.retellings.saturating_add(retold);
                m.corrected = m.corrected.saturating_add(corrected);
                let growth = (retold / 8).saturating_sub(corrected / 6);
                m.embellishment = (u16::from(m.embellishment) + growth).min(100) as u8;
                if corrected > 0 && m.embellishment > 0 && corrected / 6 > retold / 8 {
                    m.embellishment -= 1;
                }
                m.last = today;
            }
            None if n >= 5 && w.net.myths.len() < 2500 => w.net.myths.push(Myth { about, moment, event, born: today, retellings: 0, embellishment: 0, corrected: 0, last: today }),
            None => {}
        }
    }
}
