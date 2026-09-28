//! The newsroom (K, L, N in the brief): how things become news.
//!
//! world event / information item
//!   → newsworthiness for a given outlet (importance, reputation, local
//!     relevance, rivalry, rarity, controversy, human interest, fame,
//!     timeliness)
//!   → which journalists know it (public record, or holders of the item)
//!   → verification (the journalist asks their sources at the club; each
//!     confirms, denies or knows nothing — by what they know and how discreet
//!     they are)
//!   → editorial decision (the outlet's accuracy, appetite and quota)
//!   → claim type (fact / report / rumour / speculation) and angle
//!   → publication (with its thread, back-references, time of day)
//!   → reactions and follow-ups (denials, developments, corrections,
//!     analysis the next morning).
//!
//! Journalists are people with knowledge, ambition, risk tolerance, biases,
//! a beat with a tenure, and ties to sources whose reliability they learn.
//! They cultivate sources on their beat, change jobs, and get sacked.

use pw_core::rng::{period, stream};
use pw_core::{ClubId, Date, EventId, Hidden, Money, NationId, OutletId, PersonId, PlayerId, StoryId};
use pw_world::agenda::Task;
use pw_world::event::{Cause, EventKind, Fact, Visibility};
use pw_world::incident::Exposure;
use pw_world::info::{Fidelity, InfoKind};
use pw_world::media::{Angle, ClaimType, Focus, JournalistProfile, OutletKind, OutletProfile, Scope, SourceTie, Stance, StoryLink, StoryThread, Style, ThreadState, ThreadSubject, Verification};
use pw_world::{FxHashMap, MindKind, NameId, Person, StoryKind, World};
use smallvec::SmallVec;

use crate::consider;
use crate::media::{Draft, big_enough, publish_draft};

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

fn style_of(kind: OutletKind) -> (Scope, Style) {
    match kind {
        OutletKind::National => (Scope::National, Style::Broadsheet),
        OutletKind::Tabloid => (Scope::National, Style::Tabloid),
        OutletKind::Broadcaster => (Scope::National, Style::Broadcast),
        OutletKind::DataSite => (Scope::International, Style::Statistical),
        OutletKind::Local => (Scope::Local, Style::Local),
        OutletKind::FanChannel => (Scope::Local, Style::Fan),
    }
}

/// Give every outlet and journalist an institutional/personal profile,
/// seeded from the world seed (journalist-generation stream).
pub fn ensure_profiles(w: &mut World) {
    let today = w.date;
    let outlets: Vec<OutletId> = w.media.outlets.ids().filter(|o| !w.media.outlet_profiles.contains_key(o)).collect();
    for o in outlets {
        let out = &w.media.outlets[o];
        let (scope, style) = style_of(out.kind);
        let n = |k: u64| pw_core::rng::noise(&[w.seed, stream::JOURNALISTS, u64::from(o.0), k]);
        let (appetite, depth, corrections, quota) = match style {
            Style::Broadsheet => (35.0, 60.0, 75.0, 12),
            Style::Tabloid => (85.0, 20.0, 15.0, 20),
            Style::Broadcast => (50.0, 45.0, 50.0, 10),
            Style::Statistical => (10.0, 90.0, 80.0, 4),
            Style::Local => (45.0, 40.0, 55.0, 6),
            Style::Fan => (80.0, 30.0, 10.0, 10),
        };
        let pct = |v: f32| v.clamp(0.0, 100.0) as u8;
        let p = OutletProfile {
            scope,
            style,
            language: out.nation,
            rumour_appetite: pct(appetite + n(1) * 15.0),
            tactical_depth: pct(depth + n(2) * 15.0),
            corrections: pct(corrections + n(3) * 15.0),
            quota,
            published_this_week: 0,
            audience: u32::from(out.reach).pow(2) * 10_000,
            affinity: out.leaning,
        };
        w.media.outlet_profiles.insert(o, p);
    }
    let journalists: Vec<PersonId> = w.media.journalists.keys().copied().filter(|p| !w.media.journalist_profiles.contains_key(p)).collect();
    let mut journalists = journalists;
    journalists.sort();
    for j in journalists {
        let (outlet, beat) = {
            let jj = &w.media.journalists[&j];
            (jj.outlet, jj.beat.clone())
        };
        let mut rng = w.rng(stream::JOURNALISTS, &[u64::from(j.0), 0x9f]);
        let style = w.media.outlet_profiles.get(&outlet).map_or(Style::Broadsheet, |p| p.style);
        let age = consider::age(w, j);
        let max_years = ((age - 22.0).max(0.0)).min(20.0) as i32;
        let focus = match style {
            Style::Statistical => Focus::Data,
            Style::Tabloid => [Focus::Scandal, Focus::Transfers, Focus::HumanInterest][rng.index(3)],
            Style::Local | Style::Fan => [Focus::Local, Focus::Youth, Focus::Transfers][rng.index(3)],
            _ => [Focus::Transfers, Focus::Tactics, Focus::Youth, Focus::HumanInterest][rng.index(4)],
        };
        let mut beat_since: SmallVec<[(ClubId, Date); 4]> = SmallVec::new();
        for &c in &beat {
            beat_since.push((c, today.add_days(-365 * rng.range_i32(0, max_years.max(0)))));
        }
        let bias = if !beat.is_empty() && rng.chance(0.35) { beat[rng.index(beat.len())] } else { ClubId::NONE };
        let reach = if outlet.is_some() { f32::from(w.media.outlets[outlet].reach) } else { 5.0 };
        let years_in = rng.range_i32(0, max_years.max(0));
        let languages: SmallVec<[NationId; 2]> = w.lives.get(j).map(|l| l.languages.iter().filter(|x| x.1 >= 60).map(|x| x.0).take(2).collect()).unwrap_or_default();
        let prof = JournalistProfile {
            knowledge: rng.range_i32(30, 92) as u8,
            tactical: rng.range_i32(20, 90) as u8,
            ambition: (consider::hid(w, j, Hidden::Ambition) * 5.0) as u8,
            risk: rng.range_i32(10, 90) as u8,
            bias,
            focus,
            reputation: (reach * 300.0 + f32::from(years_in as u16) * 120.0 + rng.normal() * 400.0).clamp(100.0, 10_000.0) as u16,
            hits: 0,
            misses: 0,
            beat_since,
            ties: SmallVec::new(),
            employers: [(outlet, today.add_days(-365 * years_in))].into_iter().collect(),
            languages,
        };
        w.media.journalist_profiles.insert(j, prof);
        // Years on a beat mean people there already talk to you.
        seed_ties(w, j);
    }
}

/// Initial sources for an experienced beat journalist.
fn seed_ties(w: &mut World, j: PersonId) {
    let today = w.date;
    let beat = w.media.journalists.get(&j).map(|x| x.beat.clone()).unwrap_or_default();
    let mut rng = w.rng(stream::JOURNALISTS, &[u64::from(j.0), 0x7e5]);
    for club in beat {
        let tenure = w.media.journalist_profiles.get(&j).map_or(0.0, |p| p.tenure(club, today));
        let n = (tenure * 0.6).min(6.0) as usize;
        let pool = people_at(w, club);
        for _ in 0..n {
            if pool.is_empty() {
                break;
            }
            let s = pool[rng.index(pool.len())];
            add_tie(w, j, s, (30.0 + tenure * 4.0).min(90.0) as u8);
        }
    }
}

/// People at a club a journalist could get to know: players, staff, and
/// the agents of the club's players.
fn people_at(w: &World, club: ClubId) -> Vec<PersonId> {
    let mut v: Vec<PersonId> = Vec::new();
    let t = w.clubs[club].first_team();
    if t.is_some() {
        v.extend(w.teams[t].squad.iter().map(|&p| w.players.cold[p].person));
        for &p in &w.teams[t].squad {
            if let Some(a) = w.agents.agent_of(p) {
                v.push(w.agents.list[a].person);
            }
        }
    }
    v.extend(w.clubs[club].staff.iter().map(|&s| w.staff[s].person));
    v.sort();
    v.dedup();
    v
}

fn add_tie(w: &mut World, j: PersonId, s: PersonId, strength: u8) {
    let today = w.date;
    if s == j || s.is_none() {
        return;
    }
    let Some(prof) = w.media.journalist_profiles.get_mut(&j) else { return };
    if prof.ties.iter().any(|t| t.person == s) {
        return;
    }
    if prof.ties.len() >= 12 {
        // Replace the weakest.
        if let Some(i) = prof.ties.iter().enumerate().min_by_key(|(_, t)| t.strength).map(|(i, _)| i) {
            let gone = prof.ties.remove(i).person;
            if let Some(jj) = w.media.journalists.get_mut(&j) {
                jj.sources.retain(|&x| x != gone);
            }
        }
    }
    let prof = w.media.journalist_profiles.get_mut(&j).expect("profile");
    prof.ties.push(SourceTie { person: s, since: today, strength, reliability: 50, hits: 0, misses: 0, last_used: today });
    if let Some(jj) = w.media.journalists.get_mut(&j) {
        if !jj.sources.contains(&s) {
            jj.sources.push(s);
        }
    }
}

// ---------------------------------------------------------------------------
// Candidates
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Candidate {
    kind: StoryKind,
    player: PlayerId,
    person: PersonId,
    club: ClubId,
    other_club: ClubId,
    fee: Money,
    info: u32,
    event: EventId,
    public: bool,
    importance: f32,
    sensitivity: u8,
    tone: i8,
    subject: Option<ThreadSubject>,
    earliest: u16,
    rivalry: f32,
    fixture: u64,
    /// A supporter post the story is about (`NO_POST` if none).
    post: u32,
}

impl Candidate {
    fn new(kind: StoryKind) -> Self {
        Candidate {
            kind,
            player: PlayerId::NONE,
            person: PersonId::NONE,
            club: ClubId::NONE,
            other_club: ClubId::NONE,
            fee: 0,
            info: u32::MAX,
            event: EventId::NONE,
            public: false,
            importance: 0.3,
            sensitivity: 20,
            tone: 0,
            subject: None,
            earliest: 480,
            rivalry: 0.0,
            fixture: 0,
            post: pw_world::socialnet::NO_POST,
        }
    }
}

fn person_of(w: &World, p: PlayerId) -> PersonId {
    if p.is_some() { w.players.cold[p].person } else { PersonId::NONE }
}

/// What a piece of information would be as a story.
fn from_info(w: &World, info: u32) -> Option<Candidate> {
    let it = w.grapevine.get(info);
    let mut c = Candidate::new(StoryKind::Leak);
    c.info = info;
    c.event = it.event;
    c.sensitivity = it.sensitivity;
    match it.kind {
        InfoKind::Interest { player, .. } | InfoKind::Exploring { player, .. } => {
            c.kind = StoryKind::TransferRumour;
            c.player = player;
            c.person = person_of(w, player);
            c.club = w.players.hot[player].club;
            c.other_club = if let InfoKind::Interest { club, .. } = it.kind { club } else { ClubId::NONE };
            c.importance = 0.35;
            c.subject = (c.other_club.is_some()).then_some(ThreadSubject::Transfer { player, club: c.other_club });
        }
        InfoKind::Bid { club, player, fee } => {
            c.kind = StoryKind::TransferRumour;
            c.player = player;
            c.person = person_of(w, player);
            c.club = w.players.hot[player].club;
            c.other_club = club;
            c.fee = fee;
            c.importance = 0.55;
            c.subject = Some(ThreadSubject::Transfer { player, club });
        }
        InfoKind::Unhappy { player, .. } => {
            c.kind = StoryKind::Unhappy;
            c.player = player;
            c.person = person_of(w, player);
            c.club = w.players.hot[player].club;
            c.importance = 0.45;
            c.tone = -30;
        }
        InfoKind::JobInDanger { club, manager } => {
            c.kind = StoryKind::ManagerPressure;
            c.person = manager;
            c.club = club;
            c.importance = 0.6;
            c.tone = -30;
            c.subject = Some(ThreadSubject::ManagerPressure { club });
        }
        InfoKind::Discipline { player, club } => {
            c.kind = StoryKind::Discipline;
            c.player = player;
            c.person = person_of(w, player);
            c.club = club;
            c.importance = 0.45;
            c.tone = -35;
        }
        InfoKind::InjuryWorse { player, .. } => {
            c.kind = StoryKind::Injury;
            c.player = player;
            c.person = person_of(w, player);
            c.club = w.players.hot[player].club;
            c.importance = 0.5;
            c.tone = -10;
            c.subject = Some(ThreadSubject::Injury { player });
        }
        InfoKind::ContractTalks { player, club, .. } => {
            c.kind = StoryKind::Contract;
            c.player = player;
            c.person = person_of(w, player);
            c.club = club;
            c.importance = 0.4;
            c.subject = Some(ThreadSubject::Contract { player, club });
        }
        InfoKind::Private { person, .. } => {
            c.kind = StoryKind::Personal;
            c.person = person;
            c.player = w.people[person].player;
            c.club = w.club_of_person(person);
            c.importance = 0.3;
            c.tone = -10;
        }
        InfoKind::DressingRoom { club, leader } => {
            c.kind = StoryKind::Leak;
            c.player = leader;
            c.person = person_of(w, leader);
            c.club = club;
            c.importance = 0.65;
            c.tone = -40;
            c.subject = Some(ThreadSubject::Unrest { club });
        }
        InfoKind::Incident { incident } => {
            let inc = w.incidents.get(incident)?;
            c.kind = StoryKind::IncidentNews;
            c.person = inc.parties.first().copied().unwrap_or(PersonId::NONE);
            c.player = inc.players.first().copied().unwrap_or(PlayerId::NONE);
            c.club = inc.club;
            c.importance = 0.25 + f32::from(inc.severity) / 200.0;
            c.tone = -35;
            c.subject = Some(ThreadSubject::Incident { incident });
        }
        _ => return None,
    }
    Some(c)
}

// ---------------------------------------------------------------------------
// The pipeline
// ---------------------------------------------------------------------------

fn freshness(date: Date, today: Date) -> f32 {
    (1.0 - date.days_until(today) as f32 / 10.0).clamp(0.1, 1.0)
}

/// How newsworthy a candidate is for one outlet and journalist, 0–1.
fn newsworthiness(w: &World, c: &Candidate, outlet: OutletId, j: PersonId, age_days: i32) -> (f32, [u8; 3]) {
    let prof = w.media.outlet_profiles.get(&outlet);
    let style = prof.map_or(Style::Broadsheet, |p| p.style);
    let appetite = prof.map_or(0.5, |p| f32::from(p.rumour_appetite) / 100.0);
    let rep = if c.player.is_some() {
        f32::from(w.players.cold[c.player].rep.current) / 10_000.0
    } else if c.club.is_some() {
        f32::from(w.clubs[c.club].reputation) / 10_000.0
    } else {
        0.3
    };
    let fame = if c.person.is_some() { f32::from(w.renown.of(c.person).fame) / 10_000.0 } else { 0.0 };
    let beat = w.media.journalists.get(&j).is_some_and(|x| x.beat.contains(&c.club));
    let out = &w.media.outlets[outlet];
    let local = if c.club.is_none() {
        0.5
    } else if out.leaning == c.club || beat {
        1.0
    } else if w.clubs[c.club].nation == out.nation {
        0.55
    } else if prof.is_some_and(|p| p.scope == Scope::International) {
        0.25 + rep * 0.5
    } else {
        0.1
    };
    let similar = w.media.stories.iter().rev().take(400).filter(|s| s.club == c.club && s.kind == c.kind && s.date.days_until(w.date) < 30).count() as f32;
    let rarity = 1.0 / (1.0 + similar * 0.6);
    let controversy = f32::from(c.sensitivity) / 100.0;
    let private = c.kind == StoryKind::Personal;
    let style_mult = match style {
        Style::Tabloid => 1.0 + controversy * 0.5 + if private { 0.5 } else { 0.0 },
        Style::Statistical => if private || c.kind == StoryKind::IncidentNews { 0.2 } else { 0.8 },
        Style::Broadsheet => if private { 0.3 } else { 1.0 },
        Style::Local => 1.0 + if local >= 1.0 { 0.3 } else { -0.3 },
        Style::Fan => if local >= 1.0 { 1.4 } else { 0.3 },
        Style::Broadcast => 1.0,
    };
    let timely = (1.0 - age_days as f32 / 14.0).clamp(0.15, 1.0);
    let base = c.importance * 0.5 + rep * 0.3 + fame * 0.2;
    let news = base * local * (0.7 + controversy * 0.5 * (0.5 + appetite)) * rarity * timely * (1.0 + c.rivalry * 0.5) * style_mult;
    let q = |x: f32| (x.clamp(0.0, 1.0) * 100.0) as u8;
    (news.clamp(0.0, 1.0), [q(c.importance), q(local), q(controversy)])
}

/// The journalist rings their sources.
fn verify(w: &mut World, j: PersonId, c: &Candidate) -> Verification {
    let today = w.date;
    if c.public {
        return Verification { asked: 0, confirmed: 0, denied: 0, confidence: 100 };
    }
    let own = if c.info != u32::MAX {
        w.grapevine.get(c.info).knower(j).map_or(40.0, |k| {
            let f = match k.fidelity {
                Fidelity::Accurate => 1.0,
                Fidelity::Partial | Fidelity::Exaggerated => 0.8,
                Fidelity::Outdated => 0.6,
                Fidelity::Garbled => 0.5,
                // A planted story sounds convincing to whoever received it.
                Fidelity::Planted => 0.9,
            };
            f32::from(k.confidence) * f
        })
    } else {
        40.0
    };
    let ties: Vec<SourceTie> = w.media.journalist_profiles.get(&j).map(|p| p.ties.iter().copied().filter(|t| w.club_of_person(t.person) == c.club || t.person == c.person).collect()).unwrap_or_default();
    let mut ties = ties;
    ties.sort_by(|a, b| b.strength.cmp(&a.strength).then(a.person.cmp(&b.person)));
    let (mut asked, mut confirmed, mut denied) = (0u8, 0u8, 0u8);
    let hushed = matches!(c.subject, Some(ThreadSubject::Incident { incident }) if w.incidents.get(incident).is_some_and(|i| i.hushed));
    for t in ties.iter().take(3) {
        asked += 1;
        let knows = c.info != u32::MAX && w.grapevine.get(c.info).knows(t.person);
        let discretion = consider::hid(w, t.person, Hidden::Professionalism) / 20.0 + if hushed { 0.25 } else { 0.0 };
        let roll = w.roll(stream::NEWSROOM, &[u64::from(j.0), u64::from(t.person.0), u64::from(c.info), period::day(today)]);
        if knows && roll < 0.35 + f32::from(t.strength) / 200.0 - discretion * 0.3 {
            confirmed += 1;
        } else if roll > 0.75 + discretion * 0.1 - if c.person.is_some() { consider::affinity(w, t.person, c.person).max(0.0) * 0.3 } else { 0.0 } {
            denied += 1;
        }
        if let Some(p) = w.media.journalist_profiles.get_mut(&j) {
            if let Some(x) = p.ties.iter_mut().find(|x| x.person == t.person) {
                x.last_used = today;
                if knows {
                    x.strength = x.strength.saturating_add(2).min(100);
                }
            }
        }
    }
    let confidence = (own * 0.6 + f32::from(confirmed) * 18.0 - f32::from(denied) * 12.0 + 10.0).clamp(0.0, 100.0) as u8;
    Verification { asked, confirmed, denied, confidence }
}

/// The desk decides: run it or not, and as what kind of claim.
fn editorial(w: &World, outlet: OutletId, news: f32, v: &Verification, public: bool) -> Option<ClaimType> {
    let out = &w.media.outlets[outlet];
    let prof = w.media.outlet_profiles.get(&outlet);
    let acc = f32::from(out.accuracy) / 20.0;
    let sens = f32::from(out.sensationalism) / 20.0;
    let behind = prof.map_or(0.0, |p| if p.published_this_week < p.quota / 2 { 1.0 } else { 0.0 });
    let bar = 0.18 + acc * 0.3 - sens * 0.12 - behind * 0.06;
    let score = news * if public { 1.0 } else { 0.45 + f32::from(v.confidence) / 180.0 };
    if score < bar {
        return None;
    }
    if public {
        return Some(ClaimType::Fact);
    }
    if acc >= 0.75 && v.confidence < 45 {
        return None;
    }
    Some(if v.confidence >= 80 && v.confirmed >= 2 {
        ClaimType::Report
    } else if v.confidence >= 55 {
        if acc >= 0.6 { ClaimType::Report } else { ClaimType::Rumour }
    } else if sens >= 0.6 {
        ClaimType::Speculation
    } else {
        ClaimType::Rumour
    })
}

fn pick_angle(w: &World, outlet: OutletId, c: &Candidate) -> Angle {
    let style = w.media.outlet_profiles.get(&outlet).map_or(Style::Broadsheet, |p| p.style);
    let affinity = w.media.outlet_profiles.get(&outlet).map_or(ClubId::NONE, |p| p.affinity);
    let saga = c.subject.and_then(|s| w.media.thread(s)).is_some_and(|t| t.stories.len() >= 2 && t.state == ThreadState::Open);
    if saga && style != Style::Tabloid {
        return Angle::Saga;
    }
    match (style, c.kind) {
        (Style::Statistical, _) => Angle::Numbers,
        (Style::Tabloid, StoryKind::Discipline | StoryKind::Leak | StoryKind::IncidentNews) => Angle::Villain,
        (Style::Tabloid, StoryKind::ManagerPressure) => Angle::Crisis,
        (Style::Tabloid, StoryKind::MatchReport) => Angle::Hero,
        (Style::Fan, _) if affinity == c.club && c.tone >= 0 => Angle::Hero,
        (Style::Fan, _) if affinity.is_some() && affinity != c.club => Angle::Villain,
        (Style::Local, StoryKind::Personal | StoryKind::Injury) => Angle::HumanInterest,
        (Style::Local, StoryKind::TransferRumour) => Angle::Loyalty,
        (_, StoryKind::IncidentNews | StoryKind::Leak) => Angle::Conflict,
        (_, StoryKind::ManagerPressure) => Angle::Crisis,
        _ => Angle::Straight,
    }
}

/// When in the day an outlet runs it.
fn minute_for(w: &World, outlet: OutletId, earliest: u16, key: u64) -> u16 {
    let style = w.media.outlet_profiles.get(&outlet).map_or(Style::Broadsheet, |p| p.style);
    let jitter = (pw_core::rng::hash_key(&[key, 0x3a]) % 90) as u16;
    let base = match style {
        Style::Broadsheet | Style::Local => 360,
        Style::Tabloid => 300,
        Style::Broadcast | Style::Fan | Style::Statistical => 600,
    };
    base.max(earliest).saturating_add(jitter).min(1439)
}

/// Run a candidate through one journalist's desk. Returns the story if it ran.
fn run(w: &mut World, c: Candidate, j: PersonId, age_days: i32) -> Option<StoryId> {
    let today = w.date;
    let outlet = w.media.journalists.get(&j)?.outlet;
    if outlet.is_none() {
        return None;
    }
    let (news, why) = newsworthiness(w, &c, outlet, j, age_days);
    if news < 0.12 {
        return None;
    }
    let v = verify(w, j, &c);
    let claim_type = editorial(w, outlet, news, &v, c.public)?;
    let angle = pick_angle(w, outlet, &c);
    let affinity = w.media.outlet_profiles.get(&outlet).map_or(ClubId::NONE, |p| p.affinity);
    let sens = f32::from(w.media.outlets[outlet].sensationalism) / 20.0;
    // Tone: the story's own, louder in some outlets, bent by affinity.
    let mut tone = f32::from(c.tone) * (0.8 + sens * 0.5);
    if affinity.is_some() {
        tone += if affinity == c.club { 20.0 } else if w.media.rivalry(affinity, c.club) >= 50 { -20.0 } else { 0.0 };
    }
    let leaker = if c.info != u32::MAX { w.grapevine.chain(c.info, j).last().map_or(PersonId::NONE, |t| t.from) } else { PersonId::NONE };
    let source = if let Some(p) = w.net.post(c.post) {
        Cause::Fact(Fact::Viral { post: c.post, reposts: p.reposts })
    } else if c.public || c.info == u32::MAX {
        if c.event.is_some() { Cause::Event(c.event) } else { Cause::Fact(Fact::Newsworthy { importance: why[0], relevance: why[1], controversy: why[2] }) }
    } else {
        Cause::Fact(Fact::Heard { info: c.info, from: leaker })
    };
    let claim = match claim_type {
        ClaimType::Fact => 100,
        ClaimType::Report => 75,
        ClaimType::Rumour => 45,
        ClaimType::Speculation => 30,
        _ => 50,
    };
    let grounded = c.public || (c.info != u32::MAX && w.grapevine.get(c.info).true_now && w.grapevine.get(c.info).knower(j).is_some_and(|k| !matches!(k.fidelity, Fidelity::Garbled | Fidelity::Planted)));
    let key = pw_core::rng::hash_key(&[u64::from(j.0), u64::from(c.info), c.fixture, today.0 as u64]);
    let id = publish_draft(
        w,
        Draft {
            journalist: j,
            kind: c.kind,
            player: c.player,
            person: c.person,
            club: c.club,
            other_club: c.other_club,
            fee: c.fee,
            claim,
            grounded,
            source,
            leaker,
            tone: tone.clamp(-100.0, 100.0) as i8,
            claim_type,
            angle,
            info: c.info,
            thread: c.subject,
            verification: v,
            minute: minute_for(w, outlet, c.earliest, key),
            news: (news * 100.0) as u8,
        },
    );
    // The source who told them becomes (or stays) a source.
    if leaker.is_some() {
        add_tie(w, j, leaker, 40);
    }
    // What comes next.
    let thread = w.media.stories[id].thread;
    if thread != u32::MAX {
        w.agenda.schedule(today.add_days(3 + (key % 4) as i32), 540, Task::FollowUp { thread });
    }
    if matches!(claim_type, ClaimType::Rumour | ClaimType::Speculation | ClaimType::Report) && matches!(c.kind, StoryKind::TransferRumour | StoryKind::Unhappy | StoryKind::Leak | StoryKind::Discipline) {
        w.agenda.schedule(today.add_days(1 + (key % 3) as i32), 600 + (key % 300) as u16, Task::Denial { story: id });
    }
    Some(id)
}

/// Daily: leaks known to journalists, public incidents, big match facts,
/// and everything on the agenda that is due.
pub fn daily(w: &mut World) {
    let today = w.date;
    leaks(w);
    public_incidents(w);
    match_stories(w);
    for p in w.agenda.take_due(today) {
        match p.task {
            Task::FollowUp { thread } => follow_up(w, thread),
            Task::Analysis { story } => analysis(w, story),
            Task::Denial { story } => denial(w, story),
        }
    }
}

fn leaks(w: &mut World) {
    let today = w.date;
    let journalists: FxHashMap<PersonId, ()> = w.media.journalists.keys().map(|&j| (j, ())).collect();
    let active = w.grapevine.active.clone();
    for info in active {
        let it = w.grapevine.get(info);
        if it.published.is_some() || it.closed {
            continue;
        }
        let holders: Vec<(PersonId, Date)> = it.holders.iter().filter(|k| journalists.contains_key(&k.person) && k.date.days_until(today) <= 7).map(|k| (k.person, k.date)).collect();
        if holders.is_empty() {
            continue;
        }
        let Some(c) = from_info(w, info) else { continue };
        for (j, learned) in holders {
            if w.grapevine.get(info).published.is_some() {
                break;
            }
            let age = learned.days_until(today);
            let _ = run(w, c, j, age);
        }
    }
}

fn public_incidents(w: &mut World) {
    let today = w.date;
    let todays: Vec<u32> = w.incidents.list.iter().rev().take_while(|i| i.date == today).filter(|i| pw_world::incident::def(i.kind).exposure == Exposure::Public).map(|i| i.id).collect();
    for id in todays {
        let inc = w.incidents.get(id).expect("incident").clone();
        let mut c = Candidate::new(StoryKind::IncidentNews);
        c.public = true;
        c.event = inc.event;
        c.club = inc.club;
        c.person = inc.parties.first().copied().unwrap_or(PersonId::NONE);
        c.player = inc.players.first().copied().unwrap_or(PlayerId::NONE);
        c.importance = 0.2 + f32::from(inc.severity) / 200.0;
        c.sensitivity = inc.severity / 2;
        c.tone = -20;
        c.subject = Some(ThreadSubject::Incident { incident: id });
        c.earliest = 600;
        let nation = if inc.club.is_some() { w.clubs[inc.club].nation } else { inc.nation };
        for j in covering(w, nation, inc.club, 2) {
            let _ = run(w, c, j, 0);
        }
    }
}

/// A supporter post spread far enough that the press writes about the
/// reaction itself. The story says what supporters said, sourced to the post.
pub fn fan_reaction(w: &mut World, club: ClubId, about: PersonId, post: u32) {
    let Some(p) = w.net.post(post) else { return };
    let reposts = p.reposts;
    let tone = match p.concept {
        pw_world::socialnet::Concept::Criticise | pw_world::socialnet::Concept::Mock | pw_world::socialnet::Concept::Sarcasm | pw_world::socialnet::Concept::CallOut => -25,
        pw_world::socialnet::Concept::Praise | pw_world::socialnet::Concept::Celebrate | pw_world::socialnet::Concept::Defend => 20,
        _ => 0,
    };
    // One reaction story per club and subject per week.
    let today = w.date;
    if w.media.stories.iter().rev().take(300).any(|s| s.kind == StoryKind::FanReaction && s.club == club && s.person == about && s.date.days_until(today) < 7) {
        return;
    }
    let mut c = Candidate::new(StoryKind::FanReaction);
    c.public = true;
    c.club = club;
    c.person = about;
    c.importance = (0.15 + (reposts as f32).log10() / 20.0).min(0.6);
    c.tone = tone;
    c.earliest = 660;
    c.post = post;
    let nation = w.clubs[club].nation;
    for j in covering(w, nation, club, 1) {
        let _ = run(w, c, j, 0);
    }
}

/// Journalists who would cover a club (beat first), up to `n`.
fn covering(w: &World, nation: NationId, club: ClubId, n: usize) -> Vec<PersonId> {
    let mut v: Vec<(u8, PersonId)> = w
        .media
        .journalists
        .values()
        .filter(|j| j.outlet.is_some() && w.media.outlets[j.outlet].nation == nation)
        .map(|j| (if club.is_some() && j.beat.contains(&club) { 0 } else if w.media.outlets[j.outlet].leaning == club { 1 } else { 2 }, j.person))
        .collect();
    v.sort();
    v.into_iter().take(n).map(|x| x.1).collect()
}

/// First reports on the day's notable matches (late winners, hat-tricks,
/// comebacks, derbies, big games), and analysis the next morning.
fn match_stories(w: &mut World) {
    let today = w.date;
    let facts: Vec<pw_world::matchfacts::MatchFacts> = w.recent_matches.on(today).cloned().collect();
    for m in facts {
        let notable = m.late_winner.is_some() || !m.hat_tricks.is_empty() || m.comeback || m.significance >= 40 || m.derby;
        if !notable || !(big_enough(w, m.home) || big_enough(w, m.away)) {
            continue;
        }
        let star = m.late_winner.map(|g| g.player).or_else(|| m.hat_tricks.first().copied()).unwrap_or(m.pom);
        let mut c = Candidate::new(StoryKind::MatchReport);
        c.public = true;
        c.club = m.home;
        c.other_club = m.away;
        c.player = star;
        c.person = person_of(w, star);
        c.importance = 0.35 + if m.late_winner.is_some() { 0.15 } else { 0.0 } + if !m.hat_tricks.is_empty() { 0.15 } else { 0.0 } + f32::from(m.significance) / 400.0;
        c.rivalry = f32::from(m.significance) / 100.0;
        c.tone = 30;
        c.earliest = 1140;
        c.fixture = m.uid;
        let nation = w.clubs[m.home].nation;
        let mut ran: Option<StoryId> = None;
        for j in covering(w, nation, m.home, 2).into_iter().chain(covering(w, nation, m.away, 1)) {
            if let Some(id) = run(w, c, j, 0) {
                w.media.links.insert(id, StoryLink::Fixture { uid: m.uid, home: m.home, away: m.away, hg: m.hg, ag: m.ag, star });
                ran.get_or_insert(id);
            }
        }
        if let Some(id) = ran {
            w.agenda.schedule(today.add_days(1), 480, Task::Analysis { story: id });
        }
    }
}

// ---------------------------------------------------------------------------
// Threads, follow-ups, denials, corrections
// ---------------------------------------------------------------------------

/// Attach a story to its thread (opening one if needed).
pub(crate) fn thread_for(w: &mut World, s: ThreadSubject, story: StoryId, ev: EventId) -> u32 {
    let today = w.date;
    if let Some(&i) = w.media.thread_index.get(&s) {
        let t = &mut w.media.threads[i as usize];
        if t.state == ThreadState::Open {
            t.stories.push(story);
            t.last = today;
            if t.events.len() < 4 {
                t.events.push(ev);
            }
            return i;
        }
    }
    let id = w.media.threads.len() as u32;
    let mut events: SmallVec<[EventId; 4]> = SmallVec::new();
    events.push(ev);
    w.media.threads.push(StoryThread { id, subject: s, opened: today, last: today, stories: vec![story], events, state: ThreadState::Open, closed: None });
    w.media.thread_index.insert(s, id);
    id
}

/// Earlier, resolved threads about the same player that a story can refer to
/// ("months after the collapsed move…").
pub(crate) fn back_references(w: &World, player: PlayerId, current: u32) -> SmallVec<[StoryId; 2]> {
    let mut v: SmallVec<[StoryId; 2]> = SmallVec::new();
    if player.is_none() {
        return v;
    }
    for t in w.media.threads.iter().rev().take(3000) {
        if v.len() >= 2 {
            break;
        }
        if t.id == current || t.state == ThreadState::Open {
            continue;
        }
        let about = match t.subject {
            ThreadSubject::Transfer { player: p, .. } | ThreadSubject::Contract { player: p, .. } | ThreadSubject::Injury { player: p } => p == player,
            _ => false,
        };
        if about && t.closed.is_some_and(|d| d.days_until(w.date) < 400) {
            if let Some(&last) = t.stories.last() {
                v.push(last);
            }
        }
    }
    v
}

fn close_thread(w: &mut World, thread: u32, state: ThreadState) {
    let today = w.date;
    let t = &mut w.media.threads[thread as usize];
    if t.state != ThreadState::Open {
        return;
    }
    t.state = state;
    t.closed = Some(today);
    let stories = t.stories.clone();
    // Journalists are judged on how their claims turned out.
    for sid in stories {
        let s = w.media.stories[sid].clone();
        if !matches!(s.claim_type, ClaimType::Report | ClaimType::Rumour | ClaimType::Speculation) {
            continue;
        }
        let right = state == ThreadState::Happened;
        if let Some(p) = w.media.journalist_profiles.get_mut(&s.journalist) {
            if right {
                p.hits = p.hits.saturating_add(1);
            } else {
                p.misses = p.misses.saturating_add(1);
            }
            // …and learn how reliable the person who told them was.
            if s.leaker.is_some() {
                if let Some(t) = p.ties.iter_mut().find(|t| t.person == s.leaker) {
                    if right {
                        t.hits = t.hits.saturating_add(1);
                        t.reliability = (t.reliability + 8).min(100);
                    } else {
                        t.misses = t.misses.saturating_add(1);
                        t.reliability = t.reliability.saturating_sub(12);
                    }
                }
            }
        }
        // Careful outlets correct confident claims that proved wrong.
        if !right && s.claim >= 70 && s.outlet.is_some() {
            let corrects = w.media.outlet_profiles.get(&s.outlet).is_some_and(|p| p.corrections >= 60);
            if corrects {
                correction(w, sid);
            }
        }
    }
}

/// Check a running story for developments.
fn follow_up(w: &mut World, thread: u32) {
    let today = w.date;
    let Some(t) = w.media.threads.get(thread as usize).cloned() else { return };
    if t.state != ThreadState::Open {
        return;
    }
    let age = t.opened.days_until(today);
    let again = |w: &mut World, days: i32| w.agenda.schedule(today.add_days(days), 540, Task::FollowUp { thread });
    match t.subject {
        ThreadSubject::Transfer { player, club } => {
            if w.players.hot[player].club == club {
                close_thread(w, thread, ThreadState::Happened);
            } else if w.events.since(t.opened).iter().any(|e| matches!(e.kind, EventKind::Transfer { player: p, .. } if p == player) || matches!(e.kind, EventKind::ContractSigned { player: p, renewal: true, .. } if p == player)) {
                close_thread(w, thread, ThreadState::Collapsed);
            } else if t.stories.iter().any(|&s| w.media.stories[s].kind == StoryKind::Denial) && age > 20 {
                close_thread(w, thread, ThreadState::Denied);
            } else if age > 45 {
                close_thread(w, thread, ThreadState::Faded);
            } else {
                again(w, 5);
            }
        }
        ThreadSubject::Injury { player } => {
            if w.players.hot[player].injury == 0 {
                close_thread(w, thread, ThreadState::Happened);
            } else {
                again(w, 7);
            }
        }
        ThreadSubject::ManagerPressure { club } => {
            let sacked = w.events.since(t.opened).iter().any(|e| matches!(e.kind, EventKind::ManagerSacked { club: c, .. } if c == club));
            let wins = w.recent_matches.of_club(club).rev().take(3).filter(|m| m.winner() == Some(club)).count();
            if sacked {
                close_thread(w, thread, ThreadState::Happened);
            } else if wins >= 2 || age > 60 {
                close_thread(w, thread, ThreadState::Faded);
            } else {
                again(w, 7);
            }
        }
        ThreadSubject::Incident { incident } => {
            let responded = w.incidents.get(incident).map_or(false, |i| i.responses.iter().any(|r| r.date > t.last));
            if responded {
                // "The club responds": public only if the response was.
                let inc = w.incidents.get(incident).cloned();
                if let Some(inc) = inc {
                    let r = inc.responses.last().copied();
                    if let Some(r) = r {
                        let visible = w.events.get(r.event).is_some_and(|e| matches!(e.vis, Visibility::Public));
                        let leaked = inc.info != u32::MAX && w.grapevine.get(inc.info).published.is_some();
                        if visible || leaked {
                            let mut c = Candidate::new(StoryKind::IncidentNews);
                            c.public = visible;
                            c.info = if visible { u32::MAX } else { inc.info };
                            c.event = r.event;
                            c.club = inc.club;
                            c.person = r.by;
                            c.importance = 0.35;
                            c.subject = Some(ThreadSubject::Incident { incident });
                            let nation = if inc.club.is_some() { w.clubs[inc.club].nation } else { inc.nation };
                            if let Some(j) = covering(w, nation, inc.club, 1).first().copied() {
                                let _ = run(w, c, j, 0);
                            }
                        }
                    }
                }
            }
            if w.incidents.get(incident).is_some_and(|i| i.resolved) || age > 30 {
                close_thread(w, thread, ThreadState::Faded);
            } else {
                again(w, 4);
            }
        }
        ThreadSubject::Unrest { club } => {
            let sacked = w.events.since(t.opened).iter().any(|e| matches!(e.kind, EventKind::ManagerSacked { club: c, .. } if c == club));
            if sacked {
                close_thread(w, thread, ThreadState::Happened);
            } else if age > 30 {
                close_thread(w, thread, ThreadState::Faded);
            } else {
                again(w, 7);
            }
        }
        ThreadSubject::Contract { player, club } => {
            let renewed = w.events.since(t.opened).iter().any(|e| matches!(e.kind, EventKind::ContractSigned { player: p, club: c, renewal: true, .. } if p == player && c == club));
            if renewed {
                close_thread(w, thread, ThreadState::Happened);
            } else if w.players.hot[player].club != club {
                close_thread(w, thread, ThreadState::Collapsed);
            } else if age > 60 {
                close_thread(w, thread, ThreadState::Faded);
            } else {
                again(w, 10);
            }
        }
        ThreadSubject::Leak { .. } => {
            if age > 30 {
                close_thread(w, thread, ThreadState::Faded);
            }
        }
    }
}

/// Someone with a reason denies a story: the club (it isn't true, or they
/// won't sell), or the player's agent (unless they planted it).
fn denial(w: &mut World, story: StoryId) {
    let s = w.media.stories[story].clone();
    if s.player.is_none() {
        return;
    }
    let club = w.players.hot[s.player].club;
    let agent = w.agents.agent_of(s.player).map_or(PersonId::NONE, |a| w.agents.list[a].person);
    let planted_by_agent = s.leaker.is_some() && s.leaker == agent;
    let club_voice = if club.is_some() { w.clubs[club].manager.get().map_or(PersonId::NONE, |m| w.staff[m].person) } else { PersonId::NONE };
    // Who would deny, and would they?
    let (speaker, why) = if !s.grounded && club_voice.is_some() {
        (club_voice, 0.8)
    } else if s.kind == StoryKind::TransferRumour && club_voice.is_some() && w.players.cold[s.player].status != pw_world::SquadStatus::NotNeeded {
        (club_voice, 0.4)
    } else if agent.is_some() && !planted_by_agent && !s.grounded {
        (agent, 0.6)
    } else {
        return;
    };
    let roll = w.roll(stream::NEWSROOM, &[u64::from(story.0), 0xde4]);
    if roll > why || w.people[speaker].mind == MindKind::External {
        // Humans deny through the press themselves if they wish.
        return;
    }
    let nation = if club.is_some() { w.clubs[club].nation } else { w.people[speaker].nation };
    let Some(j) = covering(w, nation, club, 1).first().copied() else { return };
    let id = publish_draft(
        w,
        Draft {
            journalist: j,
            kind: StoryKind::Denial,
            player: s.player,
            person: s.person,
            club,
            other_club: s.other_club,
            fee: 0,
            claim: 90,
            grounded: true,
            source: Cause::Event(s.event),
            leaker: PersonId::NONE,
            tone: 0,
            claim_type: ClaimType::Denial,
            angle: Angle::Straight,
            info: u32::MAX,
            thread: if s.thread != u32::MAX { Some(w.media.threads[s.thread as usize].subject) } else { None },
            verification: Verification { asked: 1, confirmed: 1, denied: 0, confidence: 90 },
            minute: 660,
            news: 30,
        },
    );
    w.media.links.insert(id, StoryLink::Quote(pw_world::media::Quote { speaker, about: s.person, stance: Stance::Deny }));
}

fn correction(w: &mut World, story: StoryId) {
    let s = w.media.stories[story].clone();
    let id = publish_draft(
        w,
        Draft {
            journalist: s.journalist,
            kind: StoryKind::Correction,
            player: s.player,
            person: s.person,
            club: s.club,
            other_club: s.other_club,
            fee: 0,
            claim: 100,
            grounded: true,
            source: Cause::Event(s.event),
            leaker: PersonId::NONE,
            tone: 0,
            claim_type: ClaimType::Correction,
            angle: Angle::Straight,
            info: u32::MAX,
            thread: None,
            verification: Verification { asked: 0, confirmed: 0, denied: 0, confidence: 100 },
            minute: 420,
            news: 10,
        },
    );
    w.media.stories[id].refs.push(story);
}

/// Considered analysis the morning after a big match, by the outlet with
/// the deepest tactical coverage.
fn analysis(w: &mut World, story: StoryId) {
    let s = w.media.stories[story].clone();
    let Some(StoryLink::Fixture { uid, home, away, hg, ag, star }) = w.media.links.get(&story).cloned() else { return };
    let nation = w.clubs[home].nation;
    let best = w
        .media
        .journalists
        .values()
        .filter(|j| j.outlet.is_some() && w.media.outlets[j.outlet].nation == nation)
        .filter_map(|j| w.media.outlet_profiles.get(&j.outlet).map(|p| (p.tactical_depth, j.person)))
        .filter(|x| x.0 >= 60)
        .max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)))
        .map(|x| x.1);
    let Some(j) = best else { return };
    let id = publish_draft(
        w,
        Draft {
            journalist: j,
            kind: StoryKind::Analysis,
            player: star,
            person: person_of(w, star),
            club: home,
            other_club: away,
            fee: 0,
            claim: 60,
            grounded: true,
            source: Cause::Event(s.event),
            leaker: PersonId::NONE,
            tone: 10,
            claim_type: ClaimType::Opinion,
            angle: Angle::Numbers,
            info: u32::MAX,
            thread: None,
            verification: Verification { asked: 0, confirmed: 0, denied: 0, confidence: 80 },
            minute: 480,
            news: 40,
        },
    );
    w.media.links.insert(id, StoryLink::Fixture { uid, home, away, hg, ag, star });
    w.media.stories[id].refs.push(story);
}

// ---------------------------------------------------------------------------
// Sources and careers
// ---------------------------------------------------------------------------

/// Weekly: beat journalists get to know people at the clubs they cover —
/// the longer they have been there, the more doors are open. Quotas reset.
pub fn weekly(w: &mut World) {
    let today = w.date;
    ensure_profiles(w);
    for p in w.media.outlet_profiles.values_mut() {
        p.published_this_week = 0;
    }
    let mut js: Vec<PersonId> = w.media.journalists.keys().copied().collect();
    js.sort();
    for j in js {
        let beat = w.media.journalists[&j].beat.clone();
        // Existing sources without a recorded tie get one.
        let sources = w.media.journalists[&j].sources.clone();
        for s in sources {
            add_tie(w, j, s, 30);
        }
        for club in beat {
            let tenure = w.media.journalist_profiles.get(&j).map_or(0.0, |p| p.tenure(club, today));
            let roll = w.roll(stream::JOURNALISTS, &[u64::from(j.0), u64::from(club.0), period::week(today)]);
            if roll > 0.03 + tenure * 0.01 {
                continue;
            }
            let pool = people_at(w, club);
            if pool.is_empty() {
                continue;
            }
            // The loose-lipped and the media-friendly are easier to know.
            let pick = pool
                .iter()
                .copied()
                .max_by(|&a, &b| {
                    let s = |x: PersonId| consider::hid(w, x, Hidden::Controversy) + w.lives.get(x).map_or(0.0, |l| f32::from(l.routine.media)) * 2.0 + w.roll(stream::JOURNALISTS, &[u64::from(j.0), u64::from(x.0), period::week(today)]) * 8.0;
                    s(a).total_cmp(&s(b)).then(b.cmp(&a))
                })
                .expect("pool");
            add_tie(w, j, pick, 25);
            let compat = consider::compat(w, pick, j);
            w.social.adjust(pick, j, today, compat, 2, 2, 0);
        }
        // Ties unused for a year wither.
        if let Some(p) = w.media.journalist_profiles.get_mut(&j) {
            p.ties.retain(|t| t.last_used.days_until(today) < 365 || t.strength >= 60);
        }
    }
}

/// July: journalists' careers move — the accurate and ambitious to bigger
/// outlets, the careless out, the old into retirement; outlets hire.
pub fn yearly(w: &mut World) {
    let today = w.date;
    let mut js: Vec<PersonId> = w.media.journalists.keys().copied().collect();
    js.sort();
    for j in js {
        let Some(prof) = w.media.journalist_profiles.get(&j).cloned() else { continue };
        let outlet = w.media.journalists[&j].outlet;
        let reach = if outlet.is_some() { w.media.outlets[outlet].reach } else { 1 };
        let net = i32::from(prof.hits) - i32::from(prof.misses);
        if let Some(p) = w.media.journalist_profiles.get_mut(&j) {
            p.reputation = (i32::from(p.reputation) + net * 60 + i32::from(reach) * 10).clamp(100, 10_000) as u16;
        }
        let age = consider::age(w, j);
        if age >= 67.0 {
            leave(w, j, outlet);
            continue;
        }
        let serious = w.media.outlet_profiles.get(&outlet).is_some_and(|p| p.corrections >= 60);
        if serious && net <= -6 {
            leave(w, j, outlet);
            continue;
        }
        if prof.ambition >= 65 && net >= 3 && outlet.is_some() {
            let nation = w.media.outlets[outlet].nation;
            let target = w
                .media
                .outlets
                .iter_enumerated()
                .filter(|(o, x)| *o != outlet && x.nation == nation && x.reach > reach && x.kind != OutletKind::FanChannel)
                .map(|(o, _)| o)
                .min_by_key(|o| w.media.outlets[*o].reach);
            if let Some(to) = target {
                w.media.journalists.get_mut(&j).expect("journalist").outlet = to;
                if let Some(p) = w.media.journalist_profiles.get_mut(&j) {
                    p.employers.push((to, today));
                }
                w.events.push(today, Visibility::Public, EventKind::JournalistMoved { person: j, from: outlet, to });
            }
        }
    }
    hire(w);
}

fn leave(w: &mut World, j: PersonId, outlet: OutletId) {
    w.media.journalists.remove(&j);
    w.events.push(w.date, Visibility::Public, EventKind::JournalistLeft { person: j, outlet });
}

/// Outlets with too few journalists hire new ones.
fn hire(w: &mut World) {
    let today = w.date;
    let outlets: Vec<OutletId> = w.media.outlets.ids().collect();
    for o in outlets {
        let staff = w.media.journalists.values().filter(|j| j.outlet == o).count();
        let want = if w.media.outlets[o].kind == OutletKind::FanChannel { 1 } else { 2 };
        if staff >= want {
            continue;
        }
        let nation = w.media.outlets[o].nation;
        let mut rng = w.rng(stream::JOURNALISTS, &[u64::from(o.0), period::year(today), 0x41e]);
        let (first, last) = crate::people::random_name(w, nation, &mut rng);
        let dob = today.add_days(-(365 * rng.range_i32(23, 45)));
        let person = w.people.push(Person {
            first,
            last,
            common: NameId::NONE,
            dob,
            nation,
            nation2: Default::default(),
            hidden: crate::generate::hidden_random(&mut rng),
            player: Default::default(),
            staff: Default::default(),
            mind: MindKind::Ai,
        });
        let leaning = w.media.outlets[o].leaning;
        let mut beat: SmallVec<[ClubId; 4]> = SmallVec::new();
        if leaning.is_some() {
            beat.push(leaning);
        } else {
            let mut clubs: Vec<ClubId> = w.clubs.ids().filter(|&c| w.clubs[c].nation == nation && big_enough(w, c)).collect();
            clubs.sort_by(|&a, &b| w.clubs[b].reputation.cmp(&w.clubs[a].reputation).then(a.cmp(&b)));
            if !clubs.is_empty() {
                beat.push(clubs[rng.index(clubs.len().min(8))]);
            }
        }
        w.media.journalists.insert(person, pw_world::media::Journalist { person, outlet: o, beat, sources: SmallVec::new(), credibility: 45 });
        w.events.push(today, Visibility::Public, EventKind::JournalistHired { person, outlet: o });
    }
    ensure_profiles(w);
}

/// How reliable a journalist believes a source to be (for audits and views).
pub fn source_reliability(w: &World, j: PersonId, s: PersonId) -> Option<u8> {
    w.media.journalist_profiles.get(&j).and_then(|p| p.tie(s)).map(|t| t.reliability)
}

