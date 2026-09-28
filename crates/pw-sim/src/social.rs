//! Weekly social dynamics for every squad in the world (07 §11, 09 §4).
//!
//! Coaches notice who trains well and who coasts; teammates who share the
//! pitch grow close, players fighting for one shirt become rivals; promises
//! come due and are kept or broken; an unhappy, influential player can turn a
//! dressing room. All of it is recorded as memories with causes, and all of it
//! runs whether or not anyone in the squad is controlled by a human.

use pw_core::rng::{Rng, stream};
use pw_core::{Attr, ClubId, EventId, PersonId, PlayerId, StaffAttr, TeamId};
use pw_world::event::{Cause, Causes, CoachNote, EventKind, Fact, Visibility};
use pw_world::social::PromiseState;
use pw_world::{MemoryKind, PlayerStatus, PromiseKind, StaffRole, TeamKind, World};

use crate::consider;

/// The person who manages a team day to day: the manager for senior sides,
/// the head of youth (or the manager) for academy sides.
pub fn team_manager(w: &World, team: TeamId) -> Option<PersonId> {
    let t = &w.teams[team];
    let club = &w.clubs[t.club];
    if t.kind.is_youth() {
        if let Some(&s) = club.staff.iter().find(|&&s| w.staff[s].role == StaffRole::HeadOfYouth) {
            return Some(w.staff[s].person);
        }
    }
    club.manager.get().map(|m| w.staff[m].person)
}

fn team_minutes_7d(w: &World, team: TeamId) -> u32 {
    let from = w.date.add_days(-7);
    w.fixtures
        .between(from, w.date)
        .filter(|&f| {
            let fx = w.fixtures.get(f);
            fx.score.is_some() && fx.involves(team)
        })
        .count() as u32
        * 90
}

pub fn weekly(w: &mut World) {
    let today = w.date;
    let week = (today.0 / 7) as u64;
    let teams: Vec<TeamId> = w.teams.ids().collect();
    let team_mins: Vec<u32> = teams.iter().map(|&t| team_minutes_7d(w, t)).collect();

    for &team in &teams {
        let Some(mgr) = team_manager(w, team) else { continue };
        let squad = w.teams[team].squad.clone();
        let club = w.teams[team].club;
        let discipline = consider::staff_attr(w, mgr, StaffAttr::Discipline);
        let man_mgmt = consider::staff_attr(w, mgr, StaffAttr::ManManagement);
        for &p in &squad {
            if w.players.hot[p].status != PlayerStatus::Active {
                continue;
            }
            let who = consider::person(w, p);
            let mut rng = Rng::keyed(&[w.seed, stream::SOCIAL, u64::from(p.0), week]);
            coach_observes(w, p, who, mgr, club, discipline, &mut rng);
            player_view_of_manager(w, p, who, mgr, man_mgmt);
        }
        if w.teams[team].kind != TeamKind::U18 {
            teammates(w, team, &squad, week);
        }
        unrest(w, team, mgr, week);
    }
    promises(w, &teams, &team_mins);
    for h in w.players.hot.iter_mut() {
        h.minutes_week = 0;
    }
}

/// What the coaching staff sees in training this week becomes streaks, and
/// streaks become memories in the manager's head.
fn coach_observes(w: &mut World, p: PlayerId, who: PersonId, mgr: PersonId, club: ClubId, discipline: f32, rng: &mut Rng) {
    let today = w.date;
    let delta = consider::training_delta(w, p);
    let form = consider::form_delta(w, p);
    {
        let l = &mut w.lives[who];
        if delta < -0.6 {
            l.train_low_weeks = l.train_low_weeks.saturating_add(1);
            l.train_high_weeks = 0;
        } else if delta > 0.6 {
            l.train_high_weeks = l.train_high_weeks.saturating_add(1);
            l.train_low_weeks = 0;
        } else {
            l.train_low_weeks = l.train_low_weeks.saturating_sub(1);
            l.train_high_weeks = l.train_high_weeks.saturating_sub(1);
        }
        match form {
            Some(f) if f < -0.6 => l.form_low_weeks = l.form_low_weeks.saturating_add(1),
            _ => l.form_low_weeks = 0,
        }
    }
    let compat = consider::compat(w, mgr, who);
    // Ambient drift: every week's work nudges how the manager sees the player.
    let respect = (delta * 1.5 + form.unwrap_or(0.0) * 1.0).round() as i32;
    let trust = (delta * 1.0).round() as i32;
    w.social.adjust(mgr, who, today, compat, 0, trust, respect);

    let low = w.lives[who].train_low_weeks;
    let high = w.lives[who].train_high_weeks;
    // Strict managers notice sooner; a streak is remembered once, then again if it drags on.
    let notice = 0.25 + discipline / 40.0;
    if low >= 3 && low % 3 == 0 && rng.chance(notice) {
        let causes: Causes = pw_world::causes![Cause::Fact(Fact::TrainingSlump { player: p, weeks: low })];
        let ev = w.events.push_caused(today, Visibility::Club(club), EventKind::CoachNote { player: p, by: mgr, note: CoachNote::PoorTraining }, causes);
        w.social.remember(mgr, who, MemoryKind::PoorAttitude, today, ev, false, 0.8 + discipline / 40.0, compat);
    }
    if high >= 3 && high % 3 == 0 && rng.chance(0.5 + discipline / 60.0) {
        let causes: Causes = pw_world::causes![Cause::Fact(Fact::TrainingSurge { player: p, weeks: high })];
        let ev = w.events.push_caused(today, Visibility::Club(club), EventKind::CoachNote { player: p, by: mgr, note: CoachNote::ExcellentTraining }, causes);
        w.social.remember(mgr, who, MemoryKind::ExtraWork, today, ev, false, 1.0, compat);
    }
}

/// How the player feels about the person picking the team.
fn player_view_of_manager(w: &mut World, p: PlayerId, who: PersonId, mgr: PersonId, man_mgmt: f32) {
    let today = w.date;
    let grievance = consider::minutes_grievance(w, p);
    let (share, expected) = consider::minutes_share(w, p);
    let compat = consider::compat(w, who, mgr);
    // Good man-managers soften the blow of being left out.
    let soften = 0.5 + (20.0 - man_mgmt) / 40.0;
    let aff = if grievance > 0.3 {
        -(grievance * 4.0 * soften)
    } else if share >= expected {
        1.0
    } else {
        0.0
    };
    w.social.adjust(who, mgr, today, compat, aff.round() as i32, 0, 0);
}

/// Teammates: shared minutes build bonds, similar players competing for one
/// place become rivals. Only pairs that actually interact are stored.
fn teammates(w: &mut World, team: TeamId, squad: &[PlayerId], week: u64) {
    let today = w.date;
    let active: Vec<PlayerId> = squad.iter().copied().filter(|&p| w.players.hot[p].status == PlayerStatus::Active).collect();
    for (i, &a) in active.iter().enumerate() {
        for &b in &active[i + 1..] {
            let (pa, pb) = (consider::person(w, a), consider::person(w, b));
            let ha = &w.players.hot[a];
            let hb = &w.players.hot[b];
            let played_together = ha.minutes_week > 0 && hb.minutes_week > 0;
            let compat = consider::compat(w, pa, pb);
            let same_nation = w.people[pa].nation == w.people[pb].nation;
            if !played_together && !same_nation && w.social.get(pa, pb).is_none() {
                continue;
            }
            let bump = i32::from(played_together) + i32::from(compat > 10) - i32::from(compat < -10);
            if bump != 0 {
                w.social.adjust(pa, pb, today, compat, bump, 0, 0);
                w.social.adjust(pb, pa, today, compat, bump, 0, 0);
            }
            // Rivalry: same position, close in ability, both short of minutes.
            let ca = &w.players.cold[a];
            let cb = &w.players.cold[b];
            if ca.best_pos == cb.best_pos && (i32::from(ca.ca) - i32::from(cb.ca)).abs() <= 8 {
                let short = consider::minutes_grievance(w, a).max(consider::minutes_grievance(w, b));
                let mut rng = Rng::keyed(&[w.seed, stream::SOCIAL, u64::from(a.0), u64::from(b.0), week]);
                if short > 0.3 && rng.chance(0.08 * short) {
                    w.social.remember(pa, pb, MemoryKind::Rivalry, today, EventId::NONE, false, 0.8, compat);
                    w.social.remember(pb, pa, MemoryKind::Rivalry, today, EventId::NONE, false, 0.8, compat);
                }
            }
            if played_together && rng_pair(w, a, b, week) < 0.02 {
                w.social.remember(pa, pb, MemoryKind::SharedPitch, today, EventId::NONE, false, 0.6, compat);
                w.social.remember(pb, pa, MemoryKind::SharedPitch, today, EventId::NONE, false, 0.6, compat);
            }
        }
    }
    let _ = team;
}

fn rng_pair(w: &World, a: PlayerId, b: PlayerId, week: u64) -> f32 {
    Rng::keyed(&[w.seed, stream::SOCIAL, 0x5a, u64::from(a.0), u64::from(b.0), week]).f32()
}

/// Influence in the dressing room: leadership, reputation, tenure, age.
pub fn influence(w: &World, p: PlayerId) -> f32 {
    let c = &w.players.cold[p];
    let lead = c.attrs.get(Attr::Leadership) / 20.0;
    let rep = f32::from(c.rep.current) / 10_000.0;
    let tenure = (c.joined.days_until(w.date) as f32 / 365.0 / 5.0).min(1.0);
    let age = ((w.age_years(p) - 20.0) / 12.0).clamp(0.0, 1.0);
    0.4 * lead + 0.25 * rep + 0.2 * tenure + 0.15 * age
}

/// An influential player who is badly unhappy with the manager spreads it.
fn unrest(w: &mut World, team: TeamId, mgr: PersonId, week: u64) {
    let today = w.date;
    let club = w.teams[team].club;
    let squad = w.teams[team].squad.clone();
    let mut worst: Option<(PlayerId, f32)> = None;
    for &p in &squad {
        let who = consider::person(w, p);
        let infl = influence(w, p);
        if infl < 0.45 {
            continue;
        }
        let unhappy = (50.0 - f32::from(w.players.hot[p].morale)).max(0.0) / 50.0;
        let grudge = consider::grievance(w, who, mgr);
        let s = infl * (unhappy + grudge * 0.5);
        if s > 0.35 && worst.is_none_or(|(_, v)| s > v) {
            worst = Some((p, s));
        }
    }
    let Some((leader, strength)) = worst else { return };
    let mut rng = Rng::keyed(&[w.seed, stream::SOCIAL, u64::from(leader.0), week, 0x77]);
    if !rng.chance(strength * 0.25) {
        return;
    }
    let lp = consider::person(w, leader);
    let causes: Causes = pw_world::causes![
        Cause::Fact(Fact::LowTrust { from: lp, about: mgr, trust: (consider::trust(w, lp, mgr) * 100.0) as u8 }),
        Cause::Fact(Fact::MinutesShortfall { player: leader, share_pct: (consider::minutes_share(w, leader).0 * 100.0) as u8, expected_pct: (consider::minutes_share(w, leader).1 * 100.0) as u8 }),
    ];
    let ev = w.events.push_caused(today, Visibility::Club(club), EventKind::Unrest { club, player: leader }, causes);
    // Friends of the leader take his side.
    for &p in &squad {
        if p == leader {
            continue;
        }
        let who = consider::person(w, p);
        let aff = consider::affinity(w, who, lp);
        if aff > 0.2 {
            let compat = consider::compat(w, who, mgr);
            w.social.adjust(who, mgr, today, compat, -(aff * 6.0) as i32, -(aff * 4.0) as i32, 0);
            let h = &mut w.players.hot[p];
            h.morale = h.morale.saturating_sub((aff * 5.0) as u8);
        }
    }
    let compat = consider::compat(w, mgr, lp);
    w.social.remember(mgr, lp, MemoryKind::LetDown, today, ev, false, 0.8, compat);
}

/// Promises come due: minutes counted, statuses checked, words kept or broken.
fn promises(w: &mut World, teams: &[TeamId], team_mins: &[u32]) {
    let today = w.date;
    let _ = teams;
    let n = w.social.promises.len();
    for i in 0..n {
        let pr = w.social.promises[i].clone();
        if pr.state != PromiseState::Open {
            continue;
        }
        let promisee_player = w.people[pr.to].player;
        let promiser_club = w.club_of_person(pr.from);
        // A promise lapses if either side has left the club it was made at.
        let player_club = if promisee_player.is_some() { w.players.hot[promisee_player].club } else { w.club_of_person(pr.to) };
        let manager_promise = pr.kind != PromiseKind::ImproveTraining;
        if manager_promise && (promiser_club != pr.club || player_club != pr.club) {
            w.social.promises[i].state = PromiseState::Void;
            continue;
        }
        if let PromiseKind::Minutes { .. } = pr.kind {
            if promisee_player.is_some() {
                let team = w.players.hot[promisee_player].team;
                let tm = if team.is_some() { team_mins[team.0 as usize] } else { 0 };
                let pm = u32::from(w.players.hot[promisee_player].minutes_week);
                let e = &mut w.social.promises[i];
                e.team_minutes += tm;
                e.player_minutes += pm;
            }
        }
        if today < pr.due {
            continue;
        }
        let pr = w.social.promises[i].clone();
        let kept = match pr.kind {
            PromiseKind::Minutes { share } => pr.team_minutes > 0 && pr.player_minutes as f32 >= share * pr.team_minutes as f32 * 0.9,
            PromiseKind::Status(s) => promisee_player.is_some() && w.players.cold[promisee_player].status <= s,
            PromiseKind::NewContract => {
                w.events.since(pr.made).iter().any(|e| matches!(e.kind, EventKind::ContractSigned { player, renewal: true, .. } | EventKind::TalksOpened { player, .. } if player == promisee_player))
            }
            PromiseKind::LetLeave => {
                !w.events.since(pr.made).iter().any(|e| matches!(e.kind, EventKind::BidRejected { player, fee, .. } if player == promisee_player && fee >= w.players.cold[promisee_player].value))
            }
            PromiseKind::Position(pos) => promisee_player.is_some() && w.players.cold[promisee_player].familiarity[pos.idx()] >= 15,
            PromiseKind::Loan => w.events.since(pr.made).iter().any(|e| matches!(e.kind, EventKind::LoanMove { player, .. } if player == promisee_player)),
            PromiseKind::Captaincy => {
                promisee_player.is_some() && {
                    let t = w.players.hot[promisee_player].team;
                    t.is_some() && w.teams[t].captain == promisee_player
                }
            }
            PromiseKind::ImproveTraining => {
                let p = w.people[pr.from].player;
                p.is_some() && (consider::training_delta(w, p) > 0.2 || w.lives[pr.from].train_high_weeks >= 2)
            }
        };
        settle(w, i, kept);
    }
}

/// Close a promise and let both people remember how it ended.
pub fn settle(w: &mut World, idx: usize, kept: bool) {
    let today = w.date;
    let pr = w.social.promises[idx].clone();
    w.social.promises[idx].state = if kept { PromiseState::Kept } else { PromiseState::Broken };
    let causes: Causes = pw_world::causes![Cause::Event(pr.cause), Cause::Fact(Fact::PromiseDue { promise: pr.id })];
    let kind = if kept { EventKind::PromiseKept { promise: pr.id, from: pr.from, to: pr.to } } else { EventKind::PromiseBroken { promise: pr.id, from: pr.from, to: pr.to } };
    let ev = w.events.push_caused(today, Visibility::Between(pr.from, pr.to), kind, causes);
    let compat = consider::compat(w, pr.to, pr.from);
    let memory = if kept { MemoryKind::PromiseKept } else { MemoryKind::PromiseBroken };
    w.social.remember(pr.to, pr.from, memory, today, ev, false, 1.2, compat);
    let p = w.people[pr.to].player;
    if p.is_some() {
        let h = &mut w.players.hot[p];
        h.morale = if kept { h.morale.saturating_add(6).min(100) } else { h.morale.saturating_sub(10) };
    }
}

/// Monthly: captains are chosen by managers from the people the squad respects.
pub fn monthly(w: &mut World) {
    let teams: Vec<TeamId> = w.teams.ids().collect();
    for team in teams {
        let Some(mgr) = team_manager(w, team) else { continue };
        let squad = w.teams[team].squad.clone();
        let best = squad
            .iter()
            .copied()
            .filter(|&p| w.players.hot[p].status == PlayerStatus::Active)
            .map(|p| {
                let who = consider::person(w, p);
                let score = influence(w, p) + consider::trust(w, mgr, who) * 0.5 + consider::affinity(w, mgr, who) * 0.2;
                (p, score)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)));
        let current = w.teams[team].captain;
        if let Some((p, s)) = best {
            // Captains are not changed lightly: a clear margin is needed.
            let keep = current.is_some()
                && squad.contains(&current)
                && w.players.hot[current].status == PlayerStatus::Active
                && influence(w, current) + consider::trust(w, mgr, consider::person(w, current)) * 0.5 + 0.15 >= s;
            if !keep && current != p {
                w.teams[team].captain = p;
                let first = w.teams[team].kind == TeamKind::First;
                let vis = if first { Visibility::Public } else { Visibility::Club(w.teams[team].club) };
                w.events.push(w.date, vis, EventKind::Captaincy { player: p, team });
            }
        }
    }
}
