//! Social media as a simulation. See `pw_world::socialnet` for the layers.
//!
//! Every day: real events become frames; the accounts that care may see
//! each frame (activity, allegiance, rivalry, hour, fatigue); those who see
//! it update their opinions (slowly, stubbornly, remembering highs and lows)
//! and some post — choosing a concept from the frame, their opinion and
//! memories and their persona; a few reply, quote, disagree or call out
//! earlier posts that really exist. Engagement gives reach; reach makes
//! trends, virality, memes, and stories for journalists who watch the
//! supporters. Supporter groups carry the aggregate moods and act when
//! the state justifies it; chants are born from real moments and remembered.

use pw_core::rng::{period, stream};
use pw_core::{ClubId, Date, EventId, Hidden, NationId, PersonId, PlayerId};
use pw_world::event::{EventKind, Visibility};
use pw_world::media::{ClaimType, Stance};
use pw_world::socialnet::{
    AccountId, AccountKind, Age, Chant, ChantKind, Concept, Dim, Frame, GroupAction, GroupKind, Knew, Meme, MemeSource, MomentKind, N_DIMS, NO_POST, Opinion, Persona, Post, Remembered, SocialAccount,
    SupporterGroup, TopicKey, Trend,
};
use pw_world::{FanReason, FxHashMap, FxHashSet, MemoryKind, StoryKind, World};
use smallvec::SmallVec;

use crate::consider;

/// Posts are kept this long (plus the ones worth remembering).
const RETENTION_DAYS: i32 = 45;

// ---------------------------------------------------------------------------
// Population
// ---------------------------------------------------------------------------

fn p100(rng: &mut pw_core::Rng, mean: f32, sd: f32) -> u8 {
    rng.normal_ms(mean, sd).clamp(0.0, 100.0) as u8
}

fn persona(rng: &mut pw_core::Rng, kind: AccountKind) -> Persona {
    let (hostile, stats, youth, know) = match kind {
        AccountKind::Ultra | AccountKind::Hardcore => (60.0, 30.0, 45.0, 60.0),
        AccountKind::Provocateur => (85.0, 20.0, 30.0, 40.0),
        AccountKind::Stats => (25.0, 90.0, 45.0, 80.0),
        AccountKind::AcademyWatcher => (30.0, 50.0, 90.0, 75.0),
        AccountKind::FanNews | AccountKind::RumourMill => (35.0, 40.0, 45.0, 60.0),
        AccountKind::Casual | AccountKind::International => (30.0, 25.0, 35.0, 35.0),
        _ => (40.0, 35.0, 45.0, 50.0),
    };
    Persona {
        optimism: p100(rng, 50.0, 20.0),
        patience: p100(rng, 45.0, 20.0),
        tribalism: p100(rng, if matches!(kind, AccountKind::Ultra | AccountKind::Hardcore) { 80.0 } else { 55.0 }, 18.0),
        humour: p100(rng, 45.0, 22.0),
        hostility: p100(rng, hostile, 15.0),
        loyalty: p100(rng, 55.0, 20.0),
        nostalgia: p100(rng, 45.0, 22.0),
        stats: p100(rng, stats, 15.0),
        youth: p100(rng, youth, 15.0),
        local: p100(rng, 50.0, 20.0),
        celebrity: p100(rng, 40.0, 20.0),
        stubbornness: p100(rng, 50.0, 22.0),
        knowledge: p100(rng, know, 15.0),
        credulity: p100(rng, if kind == AccountKind::RumourMill { 80.0 } else { 45.0 }, 18.0),
    }
}

/// A handle and display name from the nation's own name pool.
fn names(w: &World, nation: NationId, club: ClubId, kind: AccountKind, rng: &mut pw_core::Rng) -> (String, String) {
    let n = &w.nations[nation];
    let first = if n.first_names.is_empty() { "Fan".to_string() } else { w.names.get(n.first_names[rng.index(n.first_names.len())]).to_string() };
    let last = if n.last_names.is_empty() { "Supporter".to_string() } else { w.names.get(n.last_names[rng.index(n.last_names.len())]).to_string() };
    let short: String = if club.is_some() { w.clubs[club].short_name.chars().filter(|c| c.is_alphanumeric()).collect() } else { String::new() };
    let clean = |s: &str| -> String { s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase() };
    let year = 60 + rng.below(40);
    let (handle, display) = match kind {
        AccountKind::FanNews => (format!("{}_news", clean(&short)), format!("{short} News")),
        AccountKind::Stats => (format!("{}_numbers", clean(&short)), format!("{short} in Numbers")),
        AccountKind::AcademyWatcher => (format!("{}_academy_watch", clean(&short)), format!("{short} Academy Watch")),
        AccountKind::RumourMill => (format!("{}_itk", clean(&short)), format!("{short} Transfer Talk")),
        AccountKind::Ultra => (format!("{}_{}_{}", clean(&short), clean(&last), rng.below(99)), format!("{last} ({short} ultras)")),
        _ => match rng.below(5) {
            0 => (format!("{}{}", clean(&first), year), format!("{first} {last}")),
            1 => (format!("{}_{}", clean(&first), clean(&short)), format!("{first} {}", last.chars().next().map_or(String::new(), |c| format!("{c}.")))),
            2 => (format!("{}{}{}", clean(&first).chars().take(3).collect::<String>(), clean(&last), rng.below(1000)), format!("{first} {last}")),
            3 => (format!("{}_til_{:02}", clean(&short), rng.below(100)), first.clone()),
            _ => (format!("the_real_{}", clean(&last)), last.to_string()),
        },
    };
    (handle, display)
}

fn new_account(w: &mut World, kind: AccountKind, nation: NationId, club: ClubId, person: PersonId, key: u64) -> AccountId {
    let today = w.date;
    let mut rng = w.rng(stream::SUPPORTERS, &[key, kind as u64]);
    let (handle, display) = if person.is_some() {
        let name = w.people[person].display_name(&w.names).into_owned();
        (name.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase(), name)
    } else {
        names(w, nation, club, kind, &mut rng)
    };
    let age = match rng.below(10) {
        0..=1 => Age::Teen,
        2..=5 => Age::Young,
        6..=8 => Age::Middle,
        _ => Age::Older,
    };
    let rival = if club.is_some() {
        w.culture
            .rivalries
            .of(pw_world::culture::Side::Club(club))
            .max_by_key(|r| r.intensity)
            .and_then(|r| match r.other(pw_world::culture::Side::Club(club)) {
                pw_world::culture::Side::Club(c) => Some(c),
                _ => None,
            })
            .unwrap_or(ClubId::NONE)
    } else {
        ClubId::NONE
    };
    let followers = match kind {
        AccountKind::FanNews | AccountKind::RumourMill => 20_000 + rng.below(200_000),
        AccountKind::Stats | AccountKind::AcademyWatcher => 5_000 + rng.below(40_000),
        AccountKind::Person => 1_000,
        _ => 50 + (rng.f32().powi(4) * 20_000.0) as u32,
    };
    let id = w.net.accounts.len() as AccountId;
    let persona = persona(&mut rng, kind);
    w.net.accounts.push(SocialAccount {
        id,
        handle,
        display,
        kind,
        person,
        age,
        nation,
        club,
        rival,
        intensity: p100(&mut rng, if matches!(kind, AccountKind::Ultra | AccountKind::Hardcore) { 85.0 } else { 55.0 }, 20.0),
        persona,
        activity: p100(&mut rng, if matches!(kind, AccountKind::FanNews | AccountKind::RumourMill) { 85.0 } else { 40.0 }, 20.0),
        peak_hour: (12 + rng.below(12)) as u8,
        followers,
        credibility: 40,
        created: today,
        last_post: Date(0),
        today: 0,
        active: true,
    });
    if club.is_some() {
        w.net.by_club.entry(club).or_default().push(id);
    }
    w.net.by_nation.entry(nation).or_default().push(id);
    if person.is_some() {
        w.net.by_person.insert(person, id);
    }
    id
}

/// Generate the population for clubs, nations, journalists and the famous.
pub fn ensure(w: &mut World) {
    let clubs: Vec<ClubId> = w.clubs.ids().filter(|&c| w.clubs[c].league.is_some() && !w.net.by_club.contains_key(&c)).collect();
    for club in clubs {
        let rep = w.clubs[club].reputation;
        let nation = w.clubs[club].nation;
        let tribal = w.culture.club(club).tribalism;
        let n = (4 + rep / 900).min(16) as u64;
        let key = u64::from(club.0) << 8;
        for i in 0..n {
            let kind = match i % 6 {
                0 | 3 => AccountKind::Supporter,
                1 => AccountKind::Casual,
                2 => AccountKind::Hardcore,
                4 => AccountKind::Local,
                _ => {
                    if rep >= 5000 {
                        AccountKind::International
                    } else {
                        AccountKind::Supporter
                    }
                }
            };
            new_account(w, kind, nation, club, PersonId::NONE, key | i);
        }
        if rep >= 3000 {
            new_account(w, AccountKind::FanNews, nation, club, PersonId::NONE, key | 0x40);
            new_account(w, AccountKind::AcademyWatcher, nation, club, PersonId::NONE, key | 0x41);
        }
        if rep >= 5000 {
            new_account(w, AccountKind::Stats, nation, club, PersonId::NONE, key | 0x42);
            new_account(w, AccountKind::RumourMill, nation, club, PersonId::NONE, key | 0x43);
        }
        if tribal >= 70 {
            new_account(w, AccountKind::Ultra, nation, club, PersonId::NONE, key | 0x44);
        }
        ensure_groups(w, club);
    }
    let nations: Vec<NationId> = w.nations.ids().filter(|&n| !w.nations[n].leagues.is_empty() && !w.net.by_nation.contains_key(&n)).collect();
    for n in nations {
        for i in 0..4u64 {
            let kind = if i == 3 { AccountKind::Provocateur } else { AccountKind::Neutral };
            new_account(w, kind, n, ClubId::NONE, PersonId::NONE, (u64::from(n.0) << 20) | i);
        }
    }
    // Real people (players, journalists, the famous) get their account the
    // first time they post (`person_post`): identity is stable once made, and
    // nobody pays for accounts that never speak.
}

fn ensure_groups(w: &mut World, club: ClubId) {
    let today = w.date;
    let rep = u32::from(w.clubs[club].reputation);
    let cult = w.culture.club(club);
    let mut kinds = vec![GroupKind::SeasonTicket, GroupKind::Online, GroupKind::Trust];
    if rep >= 5000 {
        kinds.push(GroupKind::International);
        kinds.push(GroupKind::Numbers);
    }
    if cult.identity.youth >= 50 {
        kinds.push(GroupKind::Academy);
    }
    if cult.tribalism >= 70 {
        kinds.push(GroupKind::Ultras);
    }
    for kind in kinds {
        let id = w.net.groups.len() as u32;
        let size = match kind {
            GroupKind::SeasonTicket => w.clubs[club].capacity / 2,
            GroupKind::Online => rep * 20,
            GroupKind::International => rep * 50,
            GroupKind::Ultras => 500 + rep / 10,
            _ => 200 + rep / 20,
        };
        let voice = match kind {
            GroupKind::Ultras | GroupKind::Trust => 70,
            GroupKind::SeasonTicket => 50,
            _ => 30,
        };
        w.net.groups.push(SupporterGroup { id, club, kind, size, manager: 10, board: 0, team: 20, voice, with_board: 0, founded: today, last_action: Date(0) });
    }
}

// ---------------------------------------------------------------------------
// Opinions and memories
// ---------------------------------------------------------------------------

/// How much each dimension counts for this account (locked design 6.11): a stats account weighs football and value, an ultra effort and
/// identification, a casual fan form and charm, an academy watcher whether he is one of ours. Resentment counts against.
fn dim_weights(acc: &pw_world::socialnet::SocialAccount) -> [f32; N_DIMS] {
    let p = acc.persona;
    let f = |x: u8| f32::from(x) / 100.0;
    let ultra = if matches!(acc.kind, AccountKind::Ultra | AccountKind::Hardcore) { 1.0 } else { 0.0 };
    let academy = if acc.kind == AccountKind::AcademyWatcher { 0.5 } else { 0.0 };
    let stats = if acc.kind == AccountKind::Stats { 0.3 } else { 0.0 };
    let mut x = [0.0f32; N_DIMS];
    x[Dim::Football.idx()] = 0.30 + 0.5 * f(p.stats) + stats;
    x[Dim::Form.idx()] = 0.20 + 0.25 * (1.0 - f(p.stats));
    x[Dim::Affection.idx()] = 0.15 + 0.40 * f(p.celebrity) + 0.15 * (1.0 - f(p.stats));
    x[Dim::Trust.idx()] = 0.10 + 0.40 * f(p.loyalty);
    x[Dim::Effort.idx()] = 0.10 + 0.30 * f(p.tribalism) + 0.15 * ultra;
    x[Dim::Identification.idx()] = 0.10 + 0.45 * f(p.tribalism) * f(p.local) + 0.4 * f(p.youth) + academy;
    x[Dim::Value.idx()] = 0.05 + 0.45 * f(p.stats);
    x[Dim::Resentment.idx()] = -(0.15 + 0.40 * f(p.hostility));
    x
}

/// What the dimensions add up to for this account: a summary, kept for the places that need one number. Never the whole of what they think.
pub fn summarise(acc: &pw_world::socialnet::SocialAccount, dims: &[i16; N_DIMS]) -> i16 {
    let w = dim_weights(acc);
    let total: f32 = w.iter().map(|x| x.abs()).sum();
    (w.iter().zip(dims).map(|(w, &d)| w * f32::from(d)).sum::<f32>() / total.max(1e-3)).clamp(-1000.0, 1000.0) as i16
}

/// How much is expected of a player by those who watch him (locked design 6.9): what he cost, what he earns, how famous he is, what he
/// was promised, how much has been said about him. A young player from the academy is expected to be raw. 0..1.
pub fn expectation(w: &World, p: PlayerId) -> f32 {
    let c = &w.players.cold[p];
    let club = w.players.hot[p].club;
    let fame = f32::from(c.rep.world) / 10_000.0;
    let (wage_rank, fee_norm) = if club.is_some() {
        let team = w.clubs[club].first_team();
        let mine = c.contract.current_wage(w.date);
        let n = w.teams[team].squad.len().max(1);
        let below = w.teams[team].squad.iter().filter(|&&q| w.players.cold[q].contract.current_wage(w.date) < mine).count();
        let fee = w.history.spells.get(&p).and_then(|s| s.last()).map_or(0, |s| s.fee);
        let revenue = crate::finance::season_revenue(w, club).max(1);
        (below as f32 / n as f32, (fee as f32 / (revenue as f32 * 0.25)).min(1.0))
    } else {
        (0.3, 0.0)
    };
    let status = c.contract.promised_status.map_or(0.0, |s| s.expected_minutes());
    let hype = (f32::from(w.media.image.get(&c.person).copied().unwrap_or(0)) / 1000.0).clamp(0.0, 1.0);
    let raw = if w.age(p) < 20 && c.youth_club == club { 0.35 } else { 0.0 };
    (0.25 * wage_rank + 0.25 * fee_norm + 0.20 * status + 0.20 * fame + 0.10 * hype - raw).clamp(0.0, 1.0)
}

/// What an event does to how someone is seen, dimension by dimension (locked design 6.2): a hat-trick lifts form and football and barely
/// touches trust; a transfer request is about trust and belonging, not ability; an expensive signing raises the question of value.
/// `feel` is the event's valence for this account (+1 good for them, -1 bad); `own` whether they support the club it happened at.
pub fn impact_for(w: &World, f: Frame, own: bool, feel: i8, fw: f32, derby: bool) -> [i16; N_DIMS] {
    let s = 85.0 * fw;
    let mut d = [0.0f32; N_DIMS];
    let mut set = |dim: Dim, v: f32| d[dim.idx()] = v * s;
    match f {
        Frame::HatTrick { .. } | Frame::LateWinner { .. } => {
            if feel > 0 {
                set(Dim::Form, 1.0);
                set(Dim::Football, 0.6);
                set(Dim::Affection, 0.3);
                set(Dim::Effort, 0.2);
                set(Dim::Identification, if derby { 0.6 } else { 0.3 });
                set(Dim::Value, 0.2);
                set(Dim::Resentment, -0.2);
            } else if feel < 0 {
                // The other side's man: grudging respect for the player, more grievance against him.
                set(Dim::Form, 0.3);
                set(Dim::Football, 0.3);
                set(Dim::Affection, -0.5);
                set(Dim::Resentment, if derby { 0.9 } else { 0.6 });
                set(Dim::Trust, -0.1);
            }
        }
        Frame::RedCard { .. } => {
            if own && feel < 0 {
                set(Dim::Form, -0.6);
                set(Dim::Trust, -0.4);
                set(Dim::Effort, -0.2);
                set(Dim::Football, -0.1);
                set(Dim::Resentment, 0.3);
            } else if !own && feel > 0 {
                // "Dirty as always": the same act, read through allegiance (locked design 6.6).
                set(Dim::Resentment, 0.3);
                set(Dim::Trust, -0.3);
            }
        }
        Frame::TransferRequest { .. } => {
            if own {
                set(Dim::Trust, -1.0);
                set(Dim::Identification, -1.0);
                set(Dim::Resentment, 0.8);
                set(Dim::Affection, -0.3);
                set(Dim::Effort, -0.2);
            } else {
                set(Dim::Football, 0.2);
            }
        }
        Frame::Departure { .. } => {
            if own && feel < 0 {
                set(Dim::Trust, -0.6);
                set(Dim::Identification, -0.8);
                set(Dim::Affection, -0.2);
            }
        }
        Frame::Signing { player, .. } => {
            if own && feel > 0 {
                let club = w.players.hot[player].club;
                let ideal = if club.is_some() { crate::market::ideal_ca(w.clubs[club].reputation) } else { 100.0 };
                let public = crate::market::public_view(w, player).0;
                set(Dim::Football, 0.6 * ((public - ideal) / 40.0).clamp(-1.0, 1.0));
                // Whether he is worth it: what was paid against what the market says he is.
                let fee = w.history.spells.get(&player).and_then(|s| s.last()).map_or(0, |s| s.fee) as f32;
                let value = crate::market::value_of(w, player).max(1) as f32;
                if fee > 0.0 {
                    set(Dim::Value, -0.8 * (fee / value - 1.0).clamp(-1.0, 1.0));
                }
                set(Dim::Affection, 0.2);
                set(Dim::Identification, 0.05);
            }
        }
        Frame::Award { .. } => {
            set(Dim::Football, 0.8 * f32::from(feel.max(0)));
            set(Dim::Identification, 0.4 * f32::from(feel.max(0)));
            set(Dim::Affection, 0.2 * f32::from(feel.max(0)));
        }
        Frame::Record { .. } | Frame::Milestone { .. } => {
            set(Dim::Football, 0.6 * f32::from(feel.max(0)));
            set(Dim::Identification, 0.4 * f32::from(feel.max(0)));
        }
        Frame::Injury { .. } => {
            if own {
                set(Dim::Affection, 0.1);
                set(Dim::Form, -0.2);
            }
        }
        _ => {
            let k = f32::from(feel);
            set(Dim::Football, 0.5 * k);
            set(Dim::Form, 0.3 * k);
            set(Dim::Affection, 0.2 * k);
        }
    }
    d.map(|x| x as i16)
}

/// Move what an account thinks of a person. Different events move different dimensions; expectation decides how much a performance
/// impresses or disappoints; those who see him as one of their own are quicker to be pleased; stubbornness slows all of it.
pub fn apply(w: &mut World, a: AccountId, about: PersonId, mut delta: [i16; N_DIMS]) {
    if about.is_none() || delta.iter().all(|&x| x == 0) {
        return;
    }
    let today = w.date;
    let acc = w.net.accounts[a as usize].clone();
    let stub = f32::from(acc.persona.stubbornness) / 150.0;
    let player = w.people[about].player;
    let expect = if player.is_some() { expectation(w, player) } else { 0.3 };
    let existing = w.net.opinion(a, about).map_or([0i16; N_DIMS], |o| o.dims);
    let ours = existing[Dim::Identification.idx()] > 300;
    for dim in Dim::ALL {
        let i = dim.idx();
        let mut x = f32::from(delta[i]);
        if matches!(dim, Dim::Football | Dim::Form) {
            // Against what was expected: a modest game from an expensive signing disappoints, from a young academy player excites.
            x *= if x > 0.0 { 1.2 - 0.9 * expect } else { 0.8 + 0.9 * expect };
        }
        if ours && x > 0.0 && matches!(dim, Dim::Football | Dim::Form | Dim::Affection) {
            x *= 1.25;
        }
        delta[i] = (x * (1.0 - stub)) as i16;
    }
    let v = w.net.opinions.entry(a).or_default();
    let idx = match v.iter().position(|o| o.about == about) {
        Some(i) => i,
        None => {
            if v.len() >= 12
                && let Some(i) = v.iter().enumerate().min_by_key(|(_, o)| o.score.unsigned_abs()).map(|(i, _)| i)
            {
                // Forget the mildest view.
                v.remove(i);
            }
            v.push(Opinion { about, dims: [0; N_DIMS], score: 0, since: today, low: 0, high: 0, voiced: NO_POST });
            v.len() - 1
        }
    };
    let o = &mut v[idx];
    let before = o.score;
    for dim in Dim::ALL {
        o.dims[dim.idx()] = (o.dims[dim.idx()] + delta[dim.idx()]).clamp(-1000, 1000);
    }
    o.score = summarise(&acc, &o.dims);
    o.low = o.low.min(o.score);
    o.high = o.high.max(o.score);
    let after = o.score;
    // Turning is rare and worth a record; drifting is not. A view that was warm and is now cold (or the reverse) is an event, caused by
    // the newest thing that happened to him; one such turn per person, direction and month is enough, so a crowd turning at once is one
    // event and not thousands.
    let turned = if before >= 0 && after <= -TURN { Some(false) } else if before <= 0 && after >= TURN { Some(true) } else { None };
    if let Some(up) = turned {
        let seen = w.events.latest_where(today, 31, |e| matches!(e.kind, EventKind::OpinionTurned { about: x, up: u, .. } if x == about && u == up));
        let fresh = seen.is_none_or(|id| w.events.get(id).is_none_or(|e| e.date.days_until(today) > 30));
        if fresh {
            let player = w.people[about].player;
            let cause = w.events.latest_where(today, 30, |e| e.kind.people().contains(&about) || (player.is_some() && e.kind.player() == Some(player)));
            let because = cause.map_or_else(Default::default, |id| pw_world::causes![pw_world::event::Cause::Event(id)]);
            w.events.push_caused(today, pw_world::event::Visibility::Public, EventKind::OpinionTurned { about, account: a as u32, up }, because);
        }
    }
}

/// How far an account's score must swing past neutral for its view of someone to count as turned.
const TURN: i16 = 300;

/// A group of accounts whose view of someone can be read as one (locked design 6.11).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Audience {
    OwnSupporters(ClubId),
    RivalSupporters(ClubId),
    Neutral,
    Stats,
    Ultras,
    Casual,
    AcademyWatchers,
}

fn in_audience(acc: &pw_world::socialnet::SocialAccount, aud: Audience) -> bool {
    match aud {
        Audience::OwnSupporters(c) => acc.club == c,
        Audience::RivalSupporters(c) => acc.rival == c,
        Audience::Neutral => acc.club.is_none() && acc.kind != AccountKind::Person,
        Audience::Stats => acc.kind == AccountKind::Stats,
        Audience::Ultras => matches!(acc.kind, AccountKind::Ultra | AccountKind::Hardcore),
        Audience::Casual => acc.kind == AccountKind::Casual,
        Audience::AcademyWatchers => acc.kind == AccountKind::AcademyWatcher,
    }
}

/// What one audience thinks of someone: the mean reading on each dimension among those who hold a view, what it adds up to for them,
/// and how many hold one. `None` when nobody in it has an opinion.
pub fn audience_view(w: &World, about: PersonId, aud: Audience) -> Option<([i16; N_DIMS], i16, usize)> {
    let mut sum = [0i64; N_DIMS];
    let (mut n, mut score) = (0usize, 0i64);
    for (&a, ops) in &w.net.opinions {
        let acc = &w.net.accounts[a as usize];
        if !in_audience(acc, aud) {
            continue;
        }
        if let Some(o) = ops.iter().find(|o| o.about == about) {
            for i in 0..N_DIMS {
                sum[i] += i64::from(o.dims[i]);
            }
            score += i64::from(o.score);
            n += 1;
        }
    }
    (n > 0).then(|| {
        let mut dims = [0i16; N_DIMS];
        for i in 0..N_DIMS {
            dims[i] = (sum[i] / n as i64) as i16;
        }
        (dims, (score / n as i64) as i16, n)
    })
}

fn remember(w: &mut World, a: AccountId, kind: MomentKind, about: PersonId, event: EventId) {
    let today = w.date;
    let v = w.net.memories.entry(a).or_default();
    if v.iter().any(|m| m.kind == kind && m.about == about && m.date.days_until(today) < 30) {
        return;
    }
    v.push(Remembered { kind, about, date: today, event });
    if v.len() > 8 {
        v.remove(0);
    }
}

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

/// Today's frames from what happened.
fn frames(w: &World) -> Vec<(Frame, SmallVec<[ClubId; 2]>, PersonId, EventId)> {
    let today = w.date;
    let mut v: Vec<(Frame, SmallVec<[ClubId; 2]>, PersonId, EventId)> = Vec::new();
    let pp = |p: PlayerId| if p.is_some() { w.players.cold[p].person } else { PersonId::NONE };
    for m in w.recent_matches.on(today) {
        let clubs: SmallVec<[ClubId; 2]> = [m.home, m.away].into_iter().collect();
        if !w.net.by_club.contains_key(&m.home) && !w.net.by_club.contains_key(&m.away) {
            continue;
        }
        v.push((Frame::Result { uid: m.uid }, clubs.clone(), pp(m.pom), EventId::NONE));
        if let Some(g) = m.late_winner {
            v.push((Frame::LateWinner { uid: m.uid, player: g.player }, clubs.clone(), pp(g.player), EventId::NONE));
        }
        for &p in &m.hat_tricks {
            v.push((Frame::HatTrick { uid: m.uid, player: p }, clubs.clone(), pp(p), EventId::NONE));
        }
        for &(p, _) in &m.reds {
            v.push((Frame::RedCard { uid: m.uid, player: p }, clubs.clone(), pp(p), EventId::NONE));
        }
    }
    for e in w.events.since(today) {
        if !matches!(e.vis, Visibility::Public) {
            continue;
        }
        let f = match e.kind {
            EventKind::Transfer { player, from, to, .. } => {
                let clubs: SmallVec<[ClubId; 2]> = [to, from].into_iter().filter(|c| c.is_some()).collect();
                if from.is_some() {
                    v.push((Frame::Departure { player, from, to }, clubs.clone(), pp(player), e.id));
                }
                (Frame::Signing { player, club: to }, clubs, pp(player))
            }
            EventKind::TransferRequested { player, club } => (Frame::TransferRequest { player }, [club].into_iter().collect(), pp(player)),
            EventKind::ManagerSacked { club, staff } => (Frame::ManagerSacked { club }, [club].into_iter().collect(), w.staff[staff].person),
            EventKind::ManagerAppointed { club, staff } => (Frame::ManagerAppointed { club }, [club].into_iter().collect(), w.staff[staff].person),
            EventKind::Award { player, .. } => (Frame::Award { player }, [w.players.hot[player].club].into_iter().collect(), pp(player)),
            EventKind::Milestone { player, club, .. } => (Frame::Milestone { player }, [club].into_iter().collect(), pp(player)),
            EventKind::RecordBroken { player, club, .. } if player.is_some() => (Frame::Record { player }, [club].into_iter().collect(), pp(player)),
            EventKind::Injured { player, days, .. } if days >= 21 => (Frame::Injury { player }, [w.players.hot[player].club].into_iter().collect(), pp(player)),
            EventKind::Incident { incident, .. } => {
                let Some(i) = w.incidents.get(incident) else { continue };
                (Frame::Incident { incident }, [i.club].into_iter().filter(|c| c.is_some()).collect(), i.parties.first().copied().unwrap_or(PersonId::NONE))
            }
            EventKind::RefereeControversy { controversy } => {
                let Some(c) = w.officials.controversies.get(controversy as usize) else { continue };
                (Frame::Controversy { controversy }, [c.against, c.benefited].into_iter().collect(), crate::officials::referee_person(w, c.referee))
            }
            EventKind::Published { story } => {
                let s = &w.media.stories[story];
                if !matches!(
                    s.kind,
                    StoryKind::TransferRumour
                        | StoryKind::Leak
                        | StoryKind::Unhappy
                        | StoryKind::Discipline
                        | StoryKind::IncidentNews
                        | StoryKind::Interview
                        | StoryKind::Criticism
                        | StoryKind::Praise
                ) {
                    continue;
                }
                let clubs: SmallVec<[ClubId; 2]> = [s.club, s.other_club].into_iter().filter(|c| c.is_some()).collect();
                let f = if s.kind == StoryKind::Interview {
                    w.pressroom.quotes.iter().rev().take(50).find(|q| q.story == story).map_or(Frame::Story { story }, |q| Frame::Quote { quote: q.id })
                } else {
                    Frame::Story { story }
                };
                (f, clubs, s.person)
            }
            _ => continue,
        };
        let (frame, clubs, about) = f;
        v.push((frame, clubs, about, e.id));
    }
    v
}

/// A stable key for a frame (its kind and the ids in it).
pub fn frame_key(f: Frame) -> u64 {
    let (k, a, b): (u64, u64, u64) = match f {
        Frame::Result { uid } => (1, uid, 0),
        Frame::LateWinner { uid, player } => (2, uid, u64::from(player.0)),
        Frame::HatTrick { uid, player } => (3, uid, u64::from(player.0)),
        Frame::RedCard { uid, player } => (4, uid, u64::from(player.0)),
        Frame::Signing { player, club } => (5, u64::from(player.0), u64::from(club.0)),
        Frame::Departure { player, from, to } => (6, u64::from(player.0), (u64::from(from.0) << 32) | u64::from(to.0)),
        Frame::TransferRequest { player } => (7, u64::from(player.0), 0),
        Frame::Story { story } => (8, u64::from(story.0), 0),
        Frame::Quote { quote } => (9, u64::from(quote), 0),
        Frame::ManagerSacked { club } => (10, u64::from(club.0), 0),
        Frame::ManagerAppointed { club } => (11, u64::from(club.0), 0),
        Frame::Award { player } => (12, u64::from(player.0), 0),
        Frame::Milestone { player } => (13, u64::from(player.0), 0),
        Frame::Record { player } => (14, u64::from(player.0), 0),
        Frame::Injury { player } => (15, u64::from(player.0), 0),
        Frame::Incident { incident } => (16, u64::from(incident), 0),
        Frame::Controversy { controversy } => (18, u64::from(controversy), 0),
        Frame::Post { post } => (17, u64::from(post), 0),
    };
    pw_core::rng::hash_key(&[k, a, b])
}

/// +1 good for the subject's supporters, −1 bad, 0 neutral.
fn valence(w: &World, f: Frame, club: ClubId) -> i8 {
    match f {
        Frame::Result { uid } => w.recent_matches.by_uid(uid).map_or(0, |m| match m.winner() {
            Some(c) if c == club => 1,
            Some(_) => -1,
            None => 0,
        }),
        Frame::LateWinner { player, .. } | Frame::HatTrick { player, .. } => {
            if w.players.hot[player].club == club {
                1
            } else {
                -1
            }
        }
        Frame::RedCard { player, .. } => {
            if w.players.hot[player].club == club {
                -1
            } else {
                1
            }
        }
        Frame::Signing { club: c, .. } => {
            if c == club {
                1
            } else {
                -1
            }
        }
        Frame::Departure { from, .. } => {
            if from == club {
                -1
            } else {
                1
            }
        }
        Frame::TransferRequest { .. } | Frame::Injury { .. } | Frame::ManagerSacked { .. } | Frame::Incident { .. } => -1,
        Frame::Award { .. } | Frame::Milestone { .. } | Frame::Record { .. } | Frame::ManagerAppointed { .. } => 1,
        Frame::Story { story } => {
            let s = &w.media.stories[story];
            if (s.kind == StoryKind::TransferRumour && s.club == club) || s.tone < -20 {
                -1
            } else if s.tone > 20 {
                1
            } else {
                0
            }
        }
        Frame::Quote { quote } => w.pressroom.quotes.get(quote as usize).map_or(0, |q| match q.stance {
            Stance::Praise | Stance::Support | Stance::Loyalty => 1,
            Stance::Criticise | Stance::Complain | Stance::Ambition => -1,
            _ => 0,
        }),
        // A call that went against you is bad news, whoever was right.
        Frame::Controversy { controversy } => w.officials.controversies.get(controversy as usize).map_or(0, |c| {
            if c.against == club {
                -1
            } else if c.benefited == club {
                1
            } else {
                0
            }
        }),
        Frame::Post { .. } => 0,
    }
}

/// How much a frame matters (0–1).
fn weight(w: &World, f: Frame) -> f32 {
    match f {
        Frame::Result { uid } => w.recent_matches.by_uid(uid).map_or(0.3, |m| 0.3 + f32::from(m.significance) / 200.0),
        Frame::LateWinner { .. } | Frame::HatTrick { .. } => 0.8,
        Frame::RedCard { .. } => 0.5,
        Frame::Signing { player, .. } | Frame::Departure { player, .. } => 0.3 + f32::from(w.players.cold[player].rep.current) / 15_000.0,
        Frame::TransferRequest { .. } | Frame::ManagerSacked { .. } => 0.8,
        Frame::Story { story } => 0.2 + f32::from(w.media.stories[story].news) / 150.0,
        Frame::Quote { .. } => 0.4,
        Frame::Controversy { controversy } => w.officials.controversies.get(controversy as usize).map_or(0.3, |c| 0.3 + f32::from(c.grievance) / 200.0),
        _ => 0.4,
    }
}

/// Whether an account believes a story, and how much (contextual trust:
/// the outlet's record with them, their credulity and knowledge, what they
/// want to be true, and corroboration).
pub fn believes(w: &World, a: AccountId, story: pw_core::StoryId) -> f32 {
    let acc = &w.net.accounts[a as usize];
    let s = &w.media.stories[story];
    let outlet_cred = if s.outlet.is_some() { f32::from(w.media.outlets[s.outlet].credibility) / 100.0 } else { 0.4 };
    // Trust in this journalist on this club and kind of story, from what the public has seen of their record there; with little
    // on record it leans on the outlet (locked design §2.7, §2.9).
    let source_cred = w.media.journalist_profiles.get(&s.journalist).and_then(|j| j.public_trust(s.club, pw_world::media::topic_of(s.kind))).map_or(outlet_cred, |(t, n)| {
        let weight = (n as f32 / 4.0).min(1.0);
        outlet_cred * (1.0 - weight) + t * weight
    });
    let own = w.net.outlet_trust.get(&(a, s.outlet.0)).map_or(0.0, |&t| f32::from(t) / 100.0);
    let credulity = f32::from(acc.persona.credulity) / 100.0;
    let know = f32::from(acc.persona.knowledge) / 100.0;
    // Nobody wants to believe their best player is leaving; everyone wants
    // to believe the rival's is.
    let desirable = if s.club == acc.club && s.tone < 0 {
        -0.2
    } else if s.club == acc.rival && s.tone < 0 {
        0.2
    } else {
        0.0
    };
    let corroborated = if s.thread != u32::MAX { (w.media.threads[s.thread as usize].stories.len() as f32 - 1.0).min(3.0) * 0.08 } else { 0.0 };
    let claim = match s.claim_type {
        ClaimType::Fact => 0.4,
        ClaimType::Report => 0.15,
        ClaimType::Rumour => 0.0,
        ClaimType::Speculation => -0.1,
        _ => 0.0,
    };
    // What they already think of the person: someone they distrust is easily believed to have done wrong, someone they trust is not
    // (locked design 6.12). It shifts belief; strong evidence still carries.
    let prior = if s.person.is_some() {
        w.net.opinion(a, s.person).map_or(0.0, |o| {
            let trust = f32::from(o.dims[Dim::Trust.idx()]) / 1000.0;
            if s.tone < 0 { -0.2 * trust } else if s.tone > 0 { 0.15 * trust } else { 0.0 }
        })
    } else {
        0.0
    };
    (0.3 + source_cred * (0.3 + know * 0.3) + own + credulity * 0.25 + desirable + corroborated + claim + prior).clamp(0.0, 1.0)
}

/// How likely an account is to pass a story on, whatever it believes of it (locked design §2.10). People share what they doubt because
/// it is funny or hurts a rival, and say nothing about what they believe. Belief adds to the chance without deciding it.
pub fn pass_on(w: &World, a: AccountId, story: pw_core::StoryId) -> f32 {
    let acc = &w.net.accounts[a as usize];
    if acc.kind == AccountKind::RumourMill {
        return 1.0;
    }
    let p = acc.persona;
    let s = &w.media.stories[story];
    let rival_hurt = s.club == acc.rival && s.tone < 0;
    let fun = f32::from(p.humour) / 100.0 * f32::from(s.news) / 100.0;
    let spite = if rival_hurt { f32::from(p.hostility) / 100.0 } else { 0.0 };
    (0.08 + 0.30 * spite + 0.35 * fun + 0.35 * (believes(w, a, story) - 0.5).max(0.0)).clamp(0.0, 1.0)
}

/// Choose what an account says about a frame, from the frame's valence for
/// them, their opinion and memories, and who they are.
pub fn concept(w: &World, a: AccountId, f: Frame, about: PersonId, club_val: i8, own: bool, rival: bool, roll: f32) -> Option<(Concept, PersonId, SmallVec<[u32; 2]>)> {
    let acc = &w.net.accounts[a as usize];
    let p = acc.persona;
    let prior = w.net.opinion(a, about).map_or(0, |o| o.score);
    let low = w.net.opinion(a, about).map_or(0, |o| o.low);
    let mut refs: SmallVec<[u32; 2]> = SmallVec::new();
    if let Frame::Story { story } = f {
        let belief = believes(w, a, story);
        let s = &w.media.stories[story];
        let rival_hurt = s.club == acc.rival && s.tone < 0;
        let own_hurt = s.club == acc.club && s.tone < 0;
        if roll < pass_on(w, a, story) {
            let concept = if belief >= 0.5 || acc.kind == AccountKind::RumourMill {
                Concept::Relay
            } else if p.humour > 60 {
                Concept::Sarcasm
            } else if rival_hurt {
                Concept::Mock
            } else {
                Concept::Question
            };
            return Some((concept, PersonId::NONE, refs));
        }
        // Not passing it on: a worried fan of the club says so; someone who believes it and is not touched by it says nothing.
        if belief > 0.6 && !own_hurt && roll > 0.6 {
            return None;
        }
        return Some((if club_val < 0 { Concept::Worry } else { Concept::Question }, PersonId::NONE, refs));
    }
    if about.is_some() && !matches!(f, Frame::Story { .. }) {
        // How he looks (locked design 6.16, 6.36-6.38): read by taste, and sometimes used to score a point.
        if roll < 0.30 && crate::attention::looks_reaction(w, acc, about).is_some() {
            return Some((Concept::Looks, PersonId::NONE, refs));
        }
        // Hype and anti-hype together (locked design 6.26, 6.27): numbers people say the noise is out of proportion.
        let pl = w.people[about].player;
        if p.stats >= 60 && pl.is_some() && club_val >= 0 && roll < 0.5 && crate::attention::overhyped(w, pl) > 0.25 && w.net.opinion(a, about).is_none_or(|o| o.dims[Dim::Football.idx()] < 250) {
            return Some((Concept::Overrated, PersonId::NONE, refs));
        }
        // The nostalgic retell what everyone remembers.
        if p.nostalgia >= 65 && own && roll < 0.25 && w.net.myths.iter().any(|m| m.about == about) {
            return Some((Concept::Folklore, PersonId::NONE, refs));
        }
    }
    if rival && !own {
        // The other lot.
        return if club_val < 0 && p.hostility > 40 { Some((if p.humour > 60 { Concept::Sarcasm } else { Concept::Mock }, PersonId::NONE, refs)) } else { None };
    }
    if !own {
        return None;
    }
    // An old episode, brought back by a new one (locked design 6.3): the memory faded in intensity, not away.
    if about.is_some() && club_val < 0 && roll < 0.65 {
        let kind = match f {
            Frame::TransferRequest { .. } => Some(MomentKind::TransferRequest),
            Frame::RedCard { .. } => Some(MomentKind::Mistake),
            Frame::Departure { .. } => Some(MomentKind::JoinedRival),
            _ => None,
        };
        if let Some(k) = kind
            && w.net.memories.get(&a).is_some_and(|v| v.iter().any(|m| m.kind == k && m.about == about && m.date.days_until(w.date) > 45))
        {
            return Some((Concept::Recall, PersonId::NONE, refs));
        }
    }
    let positive_for_subject = club_val > 0;
    if positive_for_subject && about.is_some() && low <= -300 && prior < 200 {
        // They did not rate this person. Now what?
        if let Some(earlier) = w.net.posts_by(a).rev().find(|x| x.about == about && matches!(x.concept, Concept::Criticise | Concept::DoubleDown | Concept::Mock)) {
            refs.push(earlier.id);
        }
        return Some((
            if p.stubbornness > 70 {
                Concept::DoubleDown
            } else if p.humour > 60 {
                Concept::ReluctantPraise
            } else {
                Concept::ConcedeWrong
            },
            PersonId::NONE,
            refs,
        ));
    }
    // Those who trust him, or feel he is one of theirs, give the benefit of the doubt (locked design 6.12).
    let loyalty_to = w.net.opinion(a, about).map_or(0, |o| o.dims[Dim::Trust.idx()] + o.dims[Dim::Identification.idx()]);
    if !positive_for_subject && about.is_some() && (prior >= 400 || loyalty_to >= 700) {
        return Some((Concept::Defend, PersonId::NONE, refs));
    }
    // A legend comparison for the nostalgic after a big moment.
    if positive_for_subject && p.nostalgia > 65 && matches!(f, Frame::LateWinner { .. } | Frame::HatTrick { .. } | Frame::Record { .. }) {
        let legend = w.honours.clubs.get(&acc.club).and_then(|r| r.legends.first().copied());
        if let Some(l) = legend {
            return Some((Concept::CompareLegend, l, refs));
        }
    }
    let c = match (club_val, f) {
        (1, Frame::Result { .. }) => Concept::Celebrate,
        (-1, Frame::Result { .. }) => {
            if p.optimism < 35 {
                Concept::Criticise
            } else {
                Concept::Lament
            }
        }
        (1, _) => {
            if roll < 0.5 {
                Concept::Praise
            } else {
                Concept::Celebrate
            }
        }
        (-1, Frame::Injury { .. }) => Concept::Worry,
        (-1, Frame::Departure { .. }) => {
            if p.loyalty > 60 {
                Concept::Criticise
            } else {
                Concept::Lament
            }
        }
        (-1, _) => {
            if p.hostility > 55 {
                Concept::Criticise
            } else {
                Concept::Lament
            }
        }
        _ => Concept::Question,
    };
    Some((c, PersonId::NONE, refs))
}

/// Everything a new post is made of.
struct NewPost {
    frame: Frame,
    concept: Concept,
    about: PersonId,
    about2: PersonId,
    club: ClubId,
    intensity: u8,
    claim: ClaimType,
    reply_to: u32,
    quote_of: u32,
    refs: SmallVec<[u32; 2]>,
    knew: Knew,
    minute: u16,
    extra: u32,
}

impl NewPost {
    fn new(frame: Frame, concept: Concept, about: PersonId, club: ClubId, knew: Knew, minute: u16) -> Self {
        NewPost { frame, concept, about, about2: PersonId::NONE, club, intensity: 50, claim: ClaimType::Opinion, reply_to: NO_POST, quote_of: NO_POST, refs: SmallVec::new(), knew, minute, extra: 0 }
    }
}

fn create_post(w: &mut World, author: AccountId, n: NewPost) -> u32 {
    let NewPost { frame, concept: c, about, about2, club, intensity, claim, reply_to, quote_of, refs, knew, minute, extra } = n;
    let today = w.date;
    w.net.sync_index();
    let id = w.net.next_post_id();
    let prior = w.net.opinion(author, about).map_or(0, |o| o.score);
    let depth = w.net.post(reply_to).map_or(0, |p| p.depth + 1);
    w.net.posts.push(Post {
        id,
        author,
        date: today,
        minute,
        frame,
        concept: c,
        about,
        about2,
        club,
        extra,
        intensity,
        claim,
        reply_to,
        quote_of,
        refs,
        likes: 0,
        reposts: 0,
        replies: 0,
        depth,
        knew,
        prior,
    });
    w.net.sync_index();
    if let Some(parent) = w.net.post_mut(reply_to) {
        parent.replies = parent.replies.saturating_add(1);
    }
    let a = &mut w.net.accounts[author as usize];
    a.last_post = today;
    a.today = a.today.saturating_add(1);
    if about.is_some()
        && let Some(o) = w.net.opinions.get_mut(&author).and_then(|v| v.iter_mut().find(|o| o.about == about))
    {
        o.voiced = id;
    }
    id
}

// ---------------------------------------------------------------------------
// The daily cycle
// ---------------------------------------------------------------------------

pub fn daily(w: &mut World) {
    let today = w.date;
    for a in w.net.accounts.iter_mut() {
        a.today = 0;
    }
    let mut rivals_of: FxHashMap<ClubId, Vec<AccountId>> = FxHashMap::default();
    for a in &w.net.accounts {
        if a.rival.is_some() && a.active {
            rivals_of.entry(a.rival).or_default().push(a.id);
        }
    }
    let todays = frames(w);
    let mut new_posts: Vec<u32> = Vec::new();
    for (f, clubs, about, ev) in todays {
        new_posts.extend(react(w, f, &clubs, about, ev, 0, &rivals_of));
        // Later waves: next-day reflection for the weighty ones.
        if weight(w, f) >= 0.7 {
            w.net.waves.push((f, today.add_days(1), 1));
        }
    }
    let due: Vec<(Frame, u8)> = w.net.waves.iter().filter(|x| x.1 <= today).map(|x| (x.0, x.2)).collect();
    w.net.waves.retain(|x| x.1 > today);
    for (f, wave) in due {
        let (clubs, about) = frame_subjects(w, f);
        new_posts.extend(react(w, f, &clubs, about, EventId::NONE, wave, &rivals_of));
    }
    threads(w, &new_posts);
    engagement(w, &new_posts);
    trends(w, &new_posts);
    virality(w, &new_posts);
    seen_by_subjects(w, &new_posts);
    memes(w, &new_posts);
    compact(w);
}

fn frame_subjects(w: &World, f: Frame) -> (SmallVec<[ClubId; 2]>, PersonId) {
    let pp = |p: PlayerId| if p.is_some() { w.players.cold[p].person } else { PersonId::NONE };
    match f {
        Frame::Result { uid } | Frame::LateWinner { uid, .. } | Frame::HatTrick { uid, .. } | Frame::RedCard { uid, .. } => {
            let m = w.recent_matches.by_uid(uid);
            let clubs = m.map_or(SmallVec::new(), |m| [m.home, m.away].into_iter().collect());
            let p = match f {
                Frame::LateWinner { player, .. } | Frame::HatTrick { player, .. } | Frame::RedCard { player, .. } => pp(player),
                _ => m.map_or(PersonId::NONE, |m| pp(m.pom)),
            };
            (clubs, p)
        }
        Frame::Signing { player, club } => ([club].into_iter().collect(), pp(player)),
        Frame::Departure { player, from, to } => ([from, to].into_iter().collect(), pp(player)),
        Frame::TransferRequest { player } | Frame::Award { player } | Frame::Milestone { player } | Frame::Record { player } | Frame::Injury { player } => {
            ([w.players.hot[player].club].into_iter().collect(), pp(player))
        }
        Frame::ManagerSacked { club } | Frame::ManagerAppointed { club } => ([club].into_iter().collect(), PersonId::NONE),
        Frame::Story { story } => {
            let s = &w.media.stories[story];
            ([s.club].into_iter().filter(|c| c.is_some()).collect(), s.person)
        }
        Frame::Quote { quote } => w.pressroom.quotes.get(quote as usize).map_or((SmallVec::new(), PersonId::NONE), |q| ([w.club_of_person(q.speaker)].into_iter().collect(), q.about)),
        Frame::Incident { incident } => w.incidents.get(incident).map_or((SmallVec::new(), PersonId::NONE), |i| ([i.club].into_iter().collect(), i.parties.first().copied().unwrap_or(PersonId::NONE))),
        Frame::Post { post } => w.net.post(post).map_or((SmallVec::new(), PersonId::NONE), |p| ([p.club].into_iter().collect(), p.about)),
        Frame::Controversy { controversy } => w
            .officials
            .controversies
            .get(controversy as usize)
            .map_or((SmallVec::new(), PersonId::NONE), |c| ([c.against, c.benefited].into_iter().collect(), crate::officials::referee_person(w, c.referee))),
    }
}

/// Accounts that care see a frame; some post.
fn react(w: &mut World, f: Frame, clubs: &[ClubId], about: PersonId, ev: EventId, wave: u8, rivals_of: &FxHashMap<ClubId, Vec<AccountId>>) -> Vec<u32> {
    let today = w.date;
    let fw = weight(w, f);
    if wave == 0 {
        crate::attention::on_frame(w, f, about, fw, ev);
    }
    let fkey = pw_core::rng::hash_key(&[frame_key(f), u64::from(ev.0), u64::from(about.0), today.0 as u64]);
    let mut audience: Vec<(AccountId, bool)> = Vec::new();
    for &c in clubs {
        if c.is_none() {
            continue;
        }
        if let Some(v) = w.net.by_club.get(&c) {
            audience.extend(v.iter().map(|&a| (a, true)));
        }
        if let Some(v) = rivals_of.get(&c) {
            audience.extend(v.iter().map(|&a| (a, false)));
        }
    }
    // Neutrals notice the famous.
    let fame = if about.is_some() { f32::from(w.renown.of(about).fame) / 10_000.0 } else { 0.0 };
    if fame > 0.5
        && let Some(&c) = clubs.first()
        && c.is_some()
        && let Some(v) = w.net.by_nation.get(&w.clubs[c].nation)
    {
        audience.extend(v.iter().filter(|&&a| w.net.accounts[a as usize].club.is_none()).map(|&a| (a, false)));
    }
    audience.sort();
    audience.dedup_by_key(|x| x.0);
    let mut posted = Vec::new();
    for (a, own) in audience {
        let acc = w.net.accounts[a as usize].clone();
        // Real people (players, journalists, the famous, and any human) post
        // about their own lives, not as supporters reacting to everything.
        if !acc.active || acc.kind == AccountKind::Person {
            continue;
        }
        let club = if own { acc.club } else { acc.rival };
        let val = valence(w, f, club);
        let see = (0.25 + f32::from(acc.activity) / 250.0 + f32::from(acc.intensity) / 400.0) * if own { 1.0 } else { 0.4 } * if wave > 0 { 0.6 } else { 1.0 };
        let r1 = w.roll(stream::SOCIAL_ACTIVITY, &[u64::from(a), fkey, u64::from(wave), 1]);
        if r1 >= see {
            continue;
        }
        // Seeing moves opinions whether or not they post: which dimensions depends on what happened and to whom, and rivals form views too.
        if about.is_some() && wave == 0 {
            let feel = if own { val } else { -val };
            let derby = matches!(f, Frame::LateWinner { uid, .. } | Frame::HatTrick { uid, .. } if w.recent_matches.by_uid(uid).is_some_and(|m| m.derby));
            let impact = impact_for(w, f, own, feel, fw, derby);
            apply(w, a, about, impact);
            if own {
                let mem = match f {
                    Frame::LateWinner { uid, .. } if w.recent_matches.by_uid(uid).is_some_and(|m| m.derby) => Some(MomentKind::DerbyGoal),
                    Frame::LateWinner { .. } => Some(MomentKind::LateWinner),
                    Frame::HatTrick { .. } => Some(MomentKind::HatTrick),
                    Frame::RedCard { .. } => Some(MomentKind::Mistake),
                    Frame::TransferRequest { .. } => Some(MomentKind::TransferRequest),
                    Frame::Departure { to, .. } if to == acc.rival => Some(MomentKind::JoinedRival),
                    Frame::Record { .. } => Some(MomentKind::Record),
                    Frame::Award { .. } => Some(MomentKind::Trophy),
                    _ => None,
                };
                if let Some(k) = mem {
                    remember(w, a, k, about, ev);
                    if k == MomentKind::JoinedRival {
                        // Going to the other lot: belonging and trust gone at once, and a grievance in their place.
                        let mut hit = [0i16; N_DIMS];
                        hit[Dim::Identification.idx()] = -500;
                        hit[Dim::Trust.idx()] = -300;
                        hit[Dim::Affection.idx()] = -100;
                        hit[Dim::Resentment.idx()] = 400;
                        apply(w, a, about, hit);
                    }
                }
            }
        }
        let fatigue = 1.0 - f32::from(acc.today) * 0.35;
        let post_p = f32::from(acc.activity) / 100.0 * (0.25 + fw * 0.6) * fatigue.max(0.0);
        let r2 = w.roll(stream::SOCIAL_ACTIVITY, &[u64::from(a), fkey, u64::from(wave), 2]);
        if r2 >= post_p {
            continue;
        }
        let r3 = w.roll(stream::SOCIAL_ACTIVITY, &[u64::from(a), fkey, 3]);
        let Some((c, about2, refs)) = concept(w, a, f, about, val, own, !own, r3) else { continue };
        let knew = match f {
            Frame::Story { story } => Knew::Read { story },
            _ => Knew::Watched,
        };
        let claim = match c {
            Concept::Relay => ClaimType::Rumour,
            Concept::Celebrate | Concept::Lament if matches!(f, Frame::Result { .. }) => ClaimType::Fact,
            _ => ClaimType::Opinion,
        };
        let intensity = (fw * 70.0 + f32::from(acc.intensity) * 0.3) as u8;
        let minute = if wave > 0 { 480 + (r3 * 600.0) as u16 } else { (u16::from(acc.peak_hour) * 60).min(1380) + (r3 * 50.0) as u16 };
        let extra = if c == Concept::Looks { crate::attention::looks_reaction(w, &acc, about).unwrap_or(1) } else { 0 };
        let id = create_post(w, a, NewPost { about2, intensity, claim, refs, extra, ..NewPost::new(f, c, about, club, knew, minute) });
        posted.push(id);
    }
    posted
}

/// Replies, disagreements, call-outs and quote-posts. Threads decay with
/// depth, age and fatigue.
fn threads(w: &mut World, new_posts: &[u32]) {
    let mut frontier: Vec<u32> = new_posts.to_vec();
    for depth in 0..3u8 {
        let mut next = Vec::new();
        for &pid in &frontier {
            let Some(post) = w.net.post(pid).cloned() else { continue };
            let club = post.club;
            let mut pool: Vec<AccountId> = w.net.by_club.get(&club).cloned().unwrap_or_default();
            if let Some(r) = w.net.accounts.get(post.author as usize).map(|a| a.rival)
                && r.is_some()
            {
                pool.extend(w.net.by_club.get(&r).cloned().unwrap_or_default());
            }
            pool.retain(|&a| a != post.author && !w.net.is_muted(a, post.author));
            let momentum = (f32::from(post.intensity) / 100.0) * (1.0 + (w.net.accounts[post.author as usize].followers as f32).log10() / 6.0);
            for &a in pool.iter().take(10) {
                let acc = w.net.accounts[a as usize].clone();
                if acc.today >= 4 || acc.kind == AccountKind::Person {
                    continue;
                }
                let theirs = w.net.opinion(a, post.about).map_or(0, |o| o.score);
                let disagree = post.about.is_some() && ((post.prior >= 0) != (theirs >= 0)) && theirs.abs() > 150;
                let callout_of = if matches!(post.concept, Concept::Praise | Concept::Celebrate | Concept::ConcedeWrong | Concept::ReluctantPraise) && post.about.is_some() {
                    w.net
                        .posts_by(post.author)
                        .rev()
                        .find(|x| x.id != post.id && x.about == post.about && matches!(x.concept, Concept::Criticise | Concept::DoubleDown) && x.date.days_until(w.date) <= 60)
                        .map(|x| x.id)
                } else {
                    None
                };
                let p = 0.06
                    * momentum
                    * (0.5 + f32::from(acc.persona.hostility.max(acc.persona.humour)) / 100.0)
                    * if disagree || callout_of.is_some() { 2.5 } else { 1.0 }
                    * 0.5f32.powi(i32::from(depth));
                let r = w.roll(stream::SOCIAL_ACTIVITY, &[u64::from(a), u64::from(pid), 0x9e]);
                if r >= p {
                    continue;
                }
                let own = acc.club == club;
                let (c, refs): (Concept, SmallVec<[u32; 2]>) = if let Some(e) = callout_of {
                    (Concept::CallOut, [e].into_iter().collect())
                } else if !own {
                    (if acc.persona.humour > 55 { Concept::Sarcasm } else { Concept::Mock }, SmallVec::new())
                } else if disagree {
                    (Concept::Disagree, SmallVec::new())
                } else {
                    (Concept::Agree, SmallVec::new())
                };
                // Rivals quote rather than reply.
                let (reply_to, quote_of) = if !own && depth == 0 { (NO_POST, pid) } else { (pid, NO_POST) };
                let n = NewPost {
                    intensity: post.intensity.saturating_sub(10),
                    reply_to,
                    quote_of,
                    refs,
                    ..NewPost::new(Frame::Post { post: pid }, c, post.about, if own { club } else { acc.club }, Knew::Saw { post: pid }, post.minute.saturating_add(20 + (r * 200.0) as u16).min(1439))
                };
                let id = create_post(w, a, n);
                next.push(id);
                // Hostile back-and-forth ends in mutes.
                if c == Concept::Disagree && acc.persona.hostility > 80 && r < p * 0.2 {
                    w.net.muted.entry(a).or_default().push(post.author);
                }
            }
        }
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }
}

/// Likes and reposts as aggregates of the unmaterialised audience.
fn engagement(w: &mut World, new_posts: &[u32]) {
    for &pid in new_posts {
        let Some(p) = w.net.post(pid).cloned() else { continue };
        let author = &w.net.accounts[p.author as usize];
        let reach = author.followers as f32;
        let club_mood = if p.club.is_some() { f32::from(w.clubs[p.club].fan_mood) / 100.0 } else { 0.5 };
        // Posts that say what the crowd feels travel further.
        let resonance = match p.concept {
            Concept::Celebrate | Concept::Praise => 0.4 + club_mood,
            Concept::Criticise | Concept::Lament => 1.4 - club_mood,
            Concept::Mock | Concept::Sarcasm | Concept::CallOut => 1.2,
            Concept::Relay => 1.1,
            _ => 0.7,
        };
        let fame = if p.about.is_some() { 1.0 + f32::from(w.renown.of(p.about).fame) / 5000.0 } else { 1.0 };
        let noise = 0.5 + w.roll(stream::SOCIAL_ACTIVITY, &[u64::from(pid), 0xe9]);
        let likes = (reach * 0.02 * resonance * fame * noise * (0.5 + f32::from(p.intensity) / 100.0)) as u32;
        let reposts = likes / 12;
        if let Some(q) = w.net.post_mut(pid) {
            q.likes += likes;
            q.reposts += reposts;
        }
        // Big engagement brings followers.
        if reposts > 50 {
            let a = &mut w.net.accounts[p.author as usize];
            a.followers = a.followers.saturating_add(reposts / 5);
        }
    }
}

fn trends(w: &mut World, new_posts: &[u32]) {
    let today = w.date;
    let mut counts: FxHashMap<(TopicKey, NationId), u32> = FxHashMap::default();
    for &pid in new_posts {
        let Some(p) = w.net.post(pid) else { continue };
        let nation = w.net.accounts[p.author as usize].nation;
        let key = match p.frame {
            Frame::Result { uid } | Frame::LateWinner { uid, .. } | Frame::HatTrick { uid, .. } => TopicKey::Match(uid),
            _ if p.about.is_some() => TopicKey::Person(p.about),
            _ => TopicKey::Club(p.club),
        };
        *counts.entry((key, nation)).or_default() += 1 + p.reposts / 20;
    }
    w.net.trends.retain(|t| t.date.days_until(today) < 7);
    let mut v: Vec<((TopicKey, NationId), u32)> = counts.into_iter().filter(|x| x.1 >= 5).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(format!("{:?}", a.0).cmp(&format!("{:?}", b.0))));
    for ((key, nation), posts) in v.into_iter().take(20) {
        w.net.trends.push(Trend { key, nation, date: today, posts });
    }
}

/// Viral posts become news; journalists watch the supporters.
fn virality(w: &mut World, new_posts: &[u32]) {
    for &pid in new_posts {
        let Some(p) = w.net.post(pid).cloned() else { continue };
        if p.reposts < 400 || p.club.is_none() {
            continue;
        }
        w.net.kept.insert(pid, p.clone());
        crate::newsroom::fan_reaction(w, p.club, p.about, pid);
    }
}

/// People who read what is said about them feel it.
fn seen_by_subjects(w: &mut World, new_posts: &[u32]) {
    let mut tally: FxHashMap<PersonId, (u32, u32)> = FxHashMap::default();
    for &pid in new_posts {
        let Some(p) = w.net.post(pid) else { continue };
        if p.about.is_none() {
            continue;
        }
        let e = tally.entry(p.about).or_default();
        match p.concept {
            Concept::Praise | Concept::Celebrate | Concept::Defend | Concept::ConcedeWrong | Concept::CompareLegend => e.0 += 1 + p.reposts / 50,
            Concept::Criticise | Concept::Mock | Concept::Sarcasm | Concept::DoubleDown => e.1 += 1 + p.reposts / 50,
            _ => {}
        }
    }
    for (who, (good, bad)) in tally {
        let media = w.lives.get(who).map_or(1, |l| l.routine.media);
        if media < 2 {
            continue;
        }
        let thick = consider::hid(w, who, Hidden::Pressure) / 20.0;
        let p = w.people[who].player;
        if bad >= 3 {
            if let Some(l) = w.lives.get_mut(who) {
                l.stress = l.stress.saturating_add(((1.0 - thick) * 6.0) as u8).min(100);
            }
            if p.is_some() {
                let h = &mut w.players.hot[p];
                h.morale = h.morale.saturating_sub(((1.0 - thick) * 4.0) as u8);
            }
        }
        if good >= 3 && p.is_some() {
            let h = &mut w.players.hot[p];
            h.confidence = (h.confidence + 2).min(100);
        }
    }
}

/// Memes are born from real moments and die when nobody uses them.
fn memes(w: &mut World, new_posts: &[u32]) {
    let today = w.date;
    // Births: a red card in a derby, a derby late winner, a viral post.
    for m in w.recent_matches.on(today).cloned().collect::<Vec<_>>() {
        if !m.derby {
            continue;
        }
        let born = if let Some(&(p, _)) = m.reds.first() {
            Some((MemeSource::Mistake { player: p, uid: m.uid }, p))
        } else {
            m.late_winner.map(|g| (MemeSource::Hero { player: g.player, uid: m.uid }, g.player))
        };
        if let Some((source, p)) = born {
            let id = w.net.memes.len() as u32;
            let club = w.players.hot[p].club;
            w.net.memes.push(Meme { id, source, about: w.players.cold[p].person, club, born: today, recognition: 30, uses: 0, variants: 1, peak: today, alive: true });
        }
    }
    for &pid in new_posts {
        // A viral jibe can become a meme; at most one new meme per club a month.
        if let Some(p) = w.net.post(pid)
            && p.reposts >= 800
            && matches!(p.concept, Concept::Mock | Concept::Sarcasm | Concept::CallOut | Concept::Criticise)
            && !w.net.memes.iter().rev().take(200).any(|m| m.source == MemeSource::Post { post: pid } || (m.club == p.club && m.born.days_until(today) < 30))
        {
            let id = w.net.memes.len() as u32;
            let (about, club) = (p.about, p.club);
            w.net.kept.insert(pid, p.clone());
            w.net.memes.push(Meme { id, source: MemeSource::Post { post: pid }, about, club, born: today, recognition: 25, uses: 0, variants: 1, peak: today, alive: true });
        }
    }
    // Rival accounts reuse a club's best-known memes against it — on the days
    // it plays, when there is something to hang them on.
    let playing: FxHashSet<ClubId> = w.recent_matches.on(today).flat_map(|m| [m.home, m.away]).collect();
    let mut live: Vec<Meme> = w.net.memes.iter().filter(|m| m.alive && playing.contains(&m.club)).copied().collect();
    live.sort_by(|a, b| a.club.cmp(&b.club).then(b.recognition.cmp(&a.recognition)).then(a.id.cmp(&b.id)));
    let mut per_club: FxHashMap<ClubId, u8> = FxHashMap::default();
    live.retain(|m| {
        let n = per_club.entry(m.club).or_default();
        *n += 1;
        *n <= 3
    });
    for m in live {
        let mockers: Vec<AccountId> = w.net.accounts.iter().filter(|a| a.rival == m.club && a.active && a.today < 3 && a.persona.humour > 50).map(|a| a.id).take(6).collect();
        for a in mockers {
            let r = w.roll(stream::SOCIAL_ACTIVITY, &[u64::from(a), u64::from(m.id), period::day(today)]);
            if r < f32::from(m.recognition) / 400.0 {
                let id = create_post(w, a, NewPost::new(Frame::Post { post: NO_POST }, Concept::Meme, m.about, m.club, Knew::Watched, 1200));
                if let Some(p) = w.net.post_mut(id) {
                    p.extra = m.id;
                }
                let mm = &mut w.net.memes[m.id as usize];
                mm.uses += 1;
                // Each use spreads it a little less.
                mm.recognition = (mm.recognition + (100 - mm.recognition.min(100)) / 25).min(100);
                if mm.uses.is_multiple_of(10) {
                    mm.variants = mm.variants.saturating_add(1);
                }
                mm.peak = today;
            }
        }
    }
}

/// Monthly: memes fade; supporters' trust in outlets follows how stories
/// they believed turned out.
pub fn monthly(w: &mut World) {
    let today = w.date;
    for m in w.net.memes.iter_mut() {
        // Everything fades; what nobody uses fades fast, and old jokes go stale.
        m.recognition = m.recognition.saturating_sub(4);
        if m.peak.days_until(today) > 30 {
            m.recognition = m.recognition.saturating_sub(10);
        }
        if m.born.days_until(today) > 365 {
            m.recognition = m.recognition.saturating_sub(5);
        }
        if m.recognition < 5 {
            m.alive = false;
        }
    }
    // Relays of stories whose thread resolved: believers learn.
    let resolved: Vec<(u32, bool)> = w
        .media
        .threads
        .iter()
        .filter(|t| t.closed.is_some_and(|d| d.days_until(today) <= 31))
        .flat_map(|t| {
            let right = t.state == pw_world::media::ThreadState::Happened;
            t.stories.iter().map(move |&s| (s.0, right))
        })
        .collect();
    // Who relayed each story: one pass over the window, not one per story.
    let mut relayed: FxHashMap<u32, Vec<AccountId>> = FxHashMap::default();
    if !resolved.is_empty() {
        for p in w.net.posts.iter().filter(|p| p.concept == Concept::Relay) {
            if let Frame::Story { story } = p.frame {
                relayed.entry(story.0).or_default().push(p.author);
            }
        }
    }
    for (sid, right) in resolved {
        let outlet = w.media.stories[pw_core::StoryId(sid)].outlet;
        let relayers: Vec<AccountId> = relayed.get(&sid).cloned().unwrap_or_default();
        for a in relayers {
            let t = w.net.outlet_trust.entry((a, outlet.0)).or_insert(0);
            *t = (*t + if right { 8 } else { -12 }).clamp(-60, 60);
            let acc = &mut w.net.accounts[a as usize];
            acc.credibility = if right { (acc.credibility + 3).min(100) } else { acc.credibility.saturating_sub(4) };
        }
    }
}

fn compact(w: &mut World) {
    let today = w.date;
    let cut = w.net.posts.iter().take_while(|p| p.date.days_until(today) > RETENTION_DAYS).count();
    if cut == 0 {
        return;
    }
    // Keep what someone still refers to.
    let referenced: Vec<u32> = w.net.posts[cut..].iter().flat_map(|p| p.refs.iter().copied().chain([p.reply_to, p.quote_of])).filter(|&r| r != NO_POST).collect();
    let old: Vec<Post> = w.net.posts.drain(..cut).collect();
    w.net.post_base += cut as u32;
    w.net.trim_index();
    for p in old {
        if referenced.contains(&p.id) || p.reposts >= 400 {
            w.net.kept.insert(p.id, p);
        }
    }
    if w.net.kept.len() > 50_000 {
        let mut ids: Vec<u32> = w.net.kept.keys().copied().collect();
        ids.sort();
        for id in ids.into_iter().take(10_000) {
            w.net.kept.remove(&id);
        }
    }
}

// ---------------------------------------------------------------------------
// Supporter groups, chants and actions (weekly)
// ---------------------------------------------------------------------------

pub fn weekly(w: &mut World) {
    ensure(w);
    let groups: Vec<u32> = (0..w.net.groups.len() as u32).collect();
    for gid in groups {
        let g = w.net.groups[gid as usize].clone();
        let club = g.club;
        let cult = w.culture.club(club);
        let results: Vec<i8> = w
            .recent_matches
            .of_club(club)
            .rev()
            .take(5)
            .map(|m| {
                if m.winner() == Some(club) {
                    1
                } else if m.winner().is_none() {
                    0
                } else {
                    -1
                }
            })
            .collect();
        let form = results.iter().map(|&r| i32::from(r)).sum::<i32>() as f32 / 5.0;
        let expect = f32::from(cult.expectations) / 100.0;
        let derby_loss = w.recent_matches.of_club(club).rev().take(5).any(|m| m.derby && m.winner().is_some_and(|x| x != club));
        let academy_minutes = {
            let t = w.clubs[club].first_team();
            w.teams[t].squad.iter().filter(|&&p| w.players.cold[p].youth_club == club && w.players.hot[p].minutes_4w > 90).count() as f32
        };
        let meddling = w.governance.get(&club).map_or(0.0, |x| f32::from(x.owner.meddling) / 100.0);
        let red = w.governance.get(&club).map_or(0.0, |x| f32::from(x.red_months) / 12.0);
        let (wf, wd, wy, wb) = match g.kind {
            GroupKind::SeasonTicket => (1.0, 0.8, 0.3, 0.5),
            GroupKind::Online => (1.4, 0.6, 0.2, 0.3),
            GroupKind::International => (1.2, 0.2, 0.1, 0.2),
            GroupKind::Academy => (0.5, 0.4, 1.5, 0.3),
            GroupKind::Ultras => (0.8, 1.6, 0.4, 1.0),
            GroupKind::Numbers => (1.0, 0.2, 0.3, 0.3),
            GroupKind::Trust => (0.4, 0.3, 0.3, 1.5),
        };
        let team_target = (form - (expect - 0.5)) * 60.0 * wf - if derby_loss { 25.0 * wd } else { 0.0 } + (academy_minutes - 1.0) * 6.0 * wy;
        let board_target = -(meddling * 50.0 + red * 60.0) * wb + form * 20.0;
        let x = &mut w.net.groups[gid as usize];
        x.team = (f32::from(x.team) * 0.7 + team_target.clamp(-100.0, 100.0) * 0.3) as i8;
        x.manager = (f32::from(x.manager) * 0.8 + team_target.clamp(-100.0, 100.0) * 0.2) as i8;
        x.board = (f32::from(x.board) * 0.8 + board_target.clamp(-100.0, 100.0) * 0.2) as i8;
        act(w, gid);
    }
    // The club's single fan mood is kept as the size-weighted view of its groups.
    let mut by_club: FxHashMap<ClubId, (f32, f32)> = FxHashMap::default();
    for g in &w.net.groups {
        let e = by_club.entry(g.club).or_default();
        e.0 += f32::from(g.team) * g.size as f32;
        e.1 += g.size as f32;
    }
    for (club, (sum, n)) in by_club {
        if n > 0.0 {
            let target = 50.0 + sum / n / 2.0;
            let m = &mut w.clubs[club].fan_mood;
            *m = (f32::from(*m) * 0.7 + target * 0.3).clamp(0.0, 100.0) as u8;
        }
    }
    chants(w);
}

/// Groups act only when the state justifies it.
fn act(w: &mut World, gid: u32) {
    let today = w.date;
    let g = w.net.groups[gid as usize].clone();
    if g.last_action.days_until(today) < 21 && g.last_action.0 != 0 {
        return;
    }
    let action = if g.board <= -60 && g.voice >= 50 {
        Some(if g.kind == GroupKind::Trust { GroupAction::Petition } else { GroupAction::Protest })
    } else if g.manager <= -70 && g.voice >= 50 {
        Some(GroupAction::Banner)
    } else if g.team >= 70 && g.kind == GroupKind::Ultras {
        Some(GroupAction::Applause)
    } else {
        None
    };
    let Some(action) = action else { return };
    let club = g.club;
    w.net.groups[gid as usize].last_action = today;
    w.events.push(today, Visibility::Public, EventKind::SupporterAction { club, group: gid, action });
    match action {
        GroupAction::Protest | GroupAction::Petition => {
            let sens = w.governance.get(&club).map_or(0.5, |x| f32::from(x.owner.fan_sensitivity) / 100.0);
            let b = &mut w.clubs[club].board;
            b.satisfaction = b.satisfaction.saturating_sub((2.0 + sens * 6.0) as u8);
            if let Some(owner) = w.governance.get(&club).map(|x| x.owner.person) {
                w.media.nudge_image(owner, -15);
            }
        }
        GroupAction::Banner => {
            if let Some(m) = w.clubs[club].manager.get() {
                let mp = w.staff[m].person;
                if let Some(l) = w.lives.get_mut(mp) {
                    l.stress = l.stress.saturating_add(5).min(100);
                }
                w.media.move_fans(club, mp, -40, FanReason::Performances, today);
            }
        }
        _ => {}
    }
}

/// Chants are born from real moments at the club, and live on.
fn chants(w: &mut World) {
    let today = w.date;
    let week: Vec<pw_world::matchfacts::MatchFacts> = w.recent_matches.list.iter().filter(|m| m.date.days_until(today) <= 7).cloned().collect();
    for m in week {
        let heroes: SmallVec<[PlayerId; 3]> = m.late_winner.map(|g| g.player).into_iter().chain(m.hat_tricks.iter().copied()).collect();
        for p in heroes {
            let club = w.players.hot[p].club;
            let person = w.players.cold[p].person;
            if club.is_none() || w.net.chants.iter().any(|c| c.about == person && c.club == club) {
                continue;
            }
            let big = m.derby || m.significance >= 50 || !m.hat_tricks.is_empty();
            if !big {
                continue;
            }
            let id = w.net.chants.len() as u32;
            let seed = pw_core::rng::hash_key(&[w.seed, stream::CULTURE, u64::from(person.0), u64::from(club.0)]);
            let target = if m.home == club { m.away } else { m.home };
            w.net.chants.push(Chant {
                id,
                club,
                kind: if m.derby { ChantKind::Rivalry } else { ChantKind::PlayerPraise },
                about: person,
                target,
                shape: (seed % 6) as u8,
                seed,
                born: today,
                moment: EventId::NONE,
                popularity: 40,
                last_sung: today,
            });
        }
    }
    // Players who crossed to a rival get a mocking chant at their old club.
    let crossed: Vec<(ClubId, PersonId, ClubId)> = w
        .events
        .since(today.add_days(-7))
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Transfer { player, from, to, .. } if from.is_some() && w.media.rivalry(from, to) >= 50 => Some((from, w.players.cold[player].person, to)),
            _ => None,
        })
        .collect();
    for (club, person, target) in crossed {
        if w.net.chants.iter().any(|c| c.about == person && c.club == club && c.kind == ChantKind::Mocking) {
            continue;
        }
        let id = w.net.chants.len() as u32;
        let seed = pw_core::rng::hash_key(&[w.seed, stream::CULTURE, u64::from(person.0), 0x6d]);
        w.net.chants.push(Chant { id, club, kind: ChantKind::Mocking, about: person, target, shape: (seed % 6) as u8, seed, born: today, moment: EventId::NONE, popularity: 50, last_sung: today });
    }
    // Protest chants when the board has lost the terraces.
    let angry: Vec<ClubId> = w.net.groups.iter().filter(|g| g.board <= -60 && matches!(g.kind, GroupKind::SeasonTicket | GroupKind::Ultras)).map(|g| g.club).collect();
    for club in angry {
        if w.net.chants.iter().any(|c| c.club == club && c.kind == ChantKind::Protest && c.born.days_until(today) < 180) {
            continue;
        }
        let owner = w.governance.get(&club).map_or(PersonId::NONE, |g| g.owner.person);
        let id = w.net.chants.len() as u32;
        let seed = pw_core::rng::hash_key(&[w.seed, stream::CULTURE, u64::from(club.0), today.0 as u64]);
        w.net.chants.push(Chant {
            id,
            club,
            kind: ChantKind::Protest,
            about: owner,
            target: ClubId::NONE,
            shape: (seed % 6) as u8,
            seed,
            born: today,
            moment: EventId::NONE,
            popularity: 60,
            last_sung: today,
        });
    }
    // Popularity fades unless sung (atmosphere sings them on matchdays).
    for c in w.net.chants.iter_mut() {
        if c.last_sung.days_until(today) > 60 {
            c.popularity = c.popularity.saturating_sub(2);
        }
    }
}

// ---------------------------------------------------------------------------
// People posting (humans through intents; AI persons through the same path)
// ---------------------------------------------------------------------------

/// A real person posts. Same consequences whoever decides it.
pub fn person_post(w: &mut World, who: PersonId, about: PersonId, concept: Concept, reply_to: u32, quote_of: u32) -> Option<u32> {
    let today = w.date;
    let a = match w.net.account_of(who) {
        Some(a) => a,
        None => {
            let club = w.club_of_person(who);
            let nation = w.lives.get(who).map_or(w.people[who].nation, |l| l.home);
            new_account(w, AccountKind::Person, nation, club, who, (u64::from(who.0) << 24) | 0x77)
        }
    };
    if let Some(p) = w.net.post(reply_to)
        && w.net.is_muted(p.author, a)
    {
        return None;
    }
    let club = w.club_of_person(who);
    let frame = if reply_to != NO_POST {
        Frame::Post { post: reply_to }
    } else if quote_of != NO_POST {
        Frame::Post { post: quote_of }
    } else {
        Frame::Post { post: NO_POST }
    };
    let id = create_post(w, a, NewPost { intensity: 70, reply_to, quote_of, ..NewPost::new(frame, concept, about, club, Knew::Own, 720) });
    // What saying it publicly does.
    if about.is_some() && about != who {
        let compat = consider::compat(w, about, who);
        match concept {
            Concept::Praise | Concept::Defend | Concept::Agree | Concept::Celebrate => w.social.remember(about, who, MemoryKind::PublicPraise, today, EventId::NONE, true, 0.6, compat),
            Concept::Criticise | Concept::Mock | Concept::Sarcasm | Concept::CallOut | Concept::Disagree => {
                w.social.remember(about, who, MemoryKind::PublicCriticism, today, EventId::NONE, true, 0.7, compat);
                // Teammates close ranks; the manager notices.
                let p = w.people[about].player;
                if p.is_some()
                    && w.players.hot[p].club == club
                    && club.is_some()
                    && let Some(m) = w.clubs[club].manager.get()
                {
                    let mp = w.staff[m].person;
                    let c = consider::compat(w, mp, who);
                    w.social.remember(mp, who, MemoryKind::PoorAttitude, today, EventId::NONE, true, 0.5, c);
                }
                w.media.nudge_image(who, -8);
            }
            _ => {}
        }
    }
    let fan_delta = match concept {
        Concept::Mock | Concept::Sarcasm => 10,
        Concept::Celebrate | Concept::Praise => 8,
        Concept::Criticise => -12,
        _ => 0,
    };
    w.media.move_fans(club, who, fan_delta, FanReason::Interview, today);
    // Others react to it like to anything else.
    w.net.waves.push((Frame::Post { post: id }, today, 0));
    Some(id)
}

/// What a person would see in their feed: followed accounts, their club,
/// teammates, themselves, rivals, trending and viral posts — a thin,
/// personal slice of a world far bigger than any screen.
pub fn feed(w: &World, who: PersonId, n: usize) -> Vec<u32> {
    let today = w.date;
    let club = w.club_of_person(who);
    let me = w.net.account_of(who);
    let follows: SmallVec<[AccountId; 8]> = me.and_then(|a| w.net.follows.get(&a).cloned()).unwrap_or_default();
    let team = {
        let p = w.people[who].player;
        if p.is_some() { w.players.hot[p].team } else { pw_core::TeamId::NONE }
    };
    let mates: Vec<PersonId> = if team.is_some() { w.teams[team].squad.iter().map(|&p| w.players.cold[p].person).collect() } else { Vec::new() };
    let rival = me.map_or(ClubId::NONE, |a| w.net.accounts[a as usize].rival);
    let trending: Vec<TopicKey> = w.net.trends.iter().filter(|t| t.date.days_until(today) <= 2).map(|t| t.key).collect();
    let mut scored: Vec<(f32, u32)> = w
        .net
        .posts
        .iter()
        .rev()
        .take_while(|p| p.date.days_until(today) <= 3)
        .filter(|p| me.is_none_or(|m| !w.net.is_muted(m, p.author)))
        .map(|p| {
            let mut s = 0.0f32;
            if follows.contains(&p.author) {
                s += 3.0;
            }
            if p.club == club && club.is_some() {
                s += 2.0;
            }
            if p.about == who {
                s += 4.0;
            }
            if mates.contains(&p.about) {
                s += 1.5;
            }
            if p.club == rival && rival.is_some() {
                s += 0.8;
            }
            if w.net.accounts[p.author as usize].kind == AccountKind::Person {
                s += 1.0;
            }
            if trending.iter().any(|t| *t == TopicKey::Person(p.about) || *t == TopicKey::Club(p.club)) {
                s += 1.0;
            }
            s += ((p.likes + 1) as f32).log10() * 0.6;
            s -= p.date.days_until(today) as f32 * 0.8;
            (s, p.id)
        })
        .filter(|x| x.0 > 1.0)
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)));
    scored.into_iter().take(n).map(|x| x.1).collect()
}

/// AI people who like the attention post about their own moments.
pub fn persons_post(w: &mut World) {
    let today = w.date;
    let movers: Vec<(PersonId, PersonId, Concept)> = w
        .recent_matches
        .on(today)
        .flat_map(|m| m.late_winner.map(|g| g.player).into_iter().chain(m.hat_tricks.iter().copied()).collect::<Vec<_>>())
        .map(|p| w.players.cold[p].person)
        .filter(|&who| w.people[who].mind == pw_world::MindKind::Ai && w.lives.get(who).is_some_and(|l| l.routine.media >= 3))
        .map(|who| (who, PersonId::NONE, Concept::Celebrate))
        .collect();
    for (who, about, c) in movers {
        person_post(w, who, about, c, NO_POST, NO_POST);
    }
}
