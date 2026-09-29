//! Belief dossiers: what a club's people think of a player and why (locked design 1.3-1.10, 1.14).
//!
//! This module is where truth is turned into perception, so it reads true attributes on purpose (`// truth-ok`): each staff member
//! forms their own reading from their own competence in the relevant domain, the exposure they had and a stable personal bias, and the
//! club's judgement is what its manager makes of those readings, weighing them by trust rather than averaging them.
//! Everything downstream (`scouting::view`, and through it selection, planning and the market) reads the result, never the truth.

use pw_core::math::interp;
use pw_core::{Attr, ClubId, Hidden, PlayerId, Pos, StaffAttr, StaffId};
use pw_world::dossier::{Confidence, Domain, Dossier, Evidence, Field, Opinion, Pending, Reason, Revision, Risk, RiskKind, Span};
use pw_world::knowledge::{Observer, perceive, perceived_ca, perceived_pa, sigma};
use pw_world::{EventKind, PlayerStatus, StaffRole, TeamKind, Visibility, World};
use smallvec::SmallVec;

/// A dossier older than this is rebuilt on demand by the monthly pass; readers fall back to the club's general view beyond twice this.
pub const FRESH_DAYS: i32 = 45;

/// How good a staff member is at one kind of judgement, 1..20. A licence is a foundation for tactical and technical work, never
/// a window on ability (section 1.3): it adds nothing to judging ability or potential.
pub fn competence(w: &World, s: StaffId, d: Domain) -> f32 {
    let st = &w.staff[s];
    let a = |x: StaffAttr| st.attrs.f(x);
    let licence = f32::from(crate::affairs::coaching_level(w, st.person));
    let c = match d {
        Domain::CurrentAbility => a(StaffAttr::JudgingAbility),
        Domain::Potential | Domain::Trajectory => a(StaffAttr::JudgingPotential),
        Domain::Youth => (a(StaffAttr::JudgingPotential) + a(StaffAttr::Youngsters)) / 2.0,
        Domain::TacticalFit | Domain::Role => (a(StaffAttr::TacticalKnowledge) + a(StaffAttr::Tactical)) / 2.0 + licence * 0.4,
        Domain::Technical => (a(StaffAttr::Technical) + a(StaffAttr::Attacking) + a(StaffAttr::Defending)) / 3.0 + licence * 0.3,
        Domain::Physical => (a(StaffAttr::Fitness) + a(StaffAttr::SportsScience)) / 2.0,
        Domain::Durability => (a(StaffAttr::Physiotherapy) + a(StaffAttr::SportsScience)) / 2.0,
        Domain::Mental | Domain::Personality | Domain::Adaptability => (a(StaffAttr::Mental) + a(StaffAttr::ManManagement)) / 2.0,
    };
    c.clamp(1.0, 20.0)
}

/// How much a role's opinion counts on a question about a player of this age (section 1.6, 1.8): the academy director on a
/// sixteen-year-old, the analyst on a player with a season of numbers, the assistant on the first team.
fn relevance(role: StaffRole, age: f32, minutes: u16, potential: bool) -> f32 {
    let youth = age < 21.0;
    match role {
        StaffRole::Manager => 1.0,
        StaffRole::Assistant => 0.9,
        StaffRole::Coach => 0.75,
        StaffRole::HeadOfYouth => {
            if youth {
                1.05
            } else {
                0.35
            }
        }
        StaffRole::DirectorOfFootball => 0.7,
        StaffRole::Analyst => {
            if potential {
                0.35
            } else if minutes >= 600 {
                0.9
            } else {
                0.4
            }
        }
        _ => 0.0,
    }
}

/// How much the manager trusts this person (0.1..1): years worked together, whether they were inherited or hired, their standing,
/// how their past readings held up, and how the two get on (section 1.7). A discounted opinion that later proves right raises this.
pub fn trust(w: &World, manager: StaffId, s: StaffId) -> f32 {
    if manager == s {
        return 1.0;
    }
    let (m, st) = (&w.staff[manager], &w.staff[s]);
    let together = (m.joined.max(st.joined).days_until(w.date).max(0) as f32 / 365.0).min(6.0);
    let inherited = st.joined < m.joined;
    let standing = f32::from(st.reputation) / 10_000.0;
    let record = w.dossiers.track.get(&s).map_or(0.0, |t| t.record());
    let regard = w.social.get(m.person, st.person).map_or(0.0, |r| (f32::from(r.respect) - 50.0) / 50.0);
    let ego = ego(w, manager);
    let t = 0.45 + 0.05 * together - if inherited { 0.12 } else { -0.03 } + 0.25 * standing + 0.30 * record + 0.15 * regard - 0.10 * ego;
    t.clamp(0.1, 1.0)
}

/// How sure of themselves a manager is, 0..1: ambition and a taste for controversy, from the person's own make-up.
pub fn ego(w: &World, manager: StaffId) -> f32 {
    let h = &w.people[w.staff[manager].person].hidden;
    // truth-ok: a manager's temperament is part of who they are, not a belief about someone else.
    ((h.f(Hidden::Ambition) + h.f(Hidden::Controversy)) / 40.0).clamp(0.0, 1.0)
}

fn confidence(band: f32, tight: f32, loose: f32, looser: f32) -> Confidence {
    if band <= tight {
        Confidence::VeryHigh
    } else if band <= loose {
        Confidence::High
    } else if band <= looser {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

struct Reading {
    by: StaffId,
    role: StaffRole,
    ca: Span,
    pa: Span,
    /// Multiplies trust to give the manager's weight on this reading.
    relevance: f32,
}

/// Every evaluator's own reading of the player.
fn readings(w: &World, club: ClubId, p: PlayerId) -> Vec<Reading> {
    let c = &w.players.cold[p];
    let h = &w.players.hot[p];
    let t = &w.data.tuning.perception;
    let age = w.age_years(p);
    let seen = w.knowledge.seen(club, p);
    let minutes = seen.map_or(0, |s| s.minutes);
    let famous = c.rep.world >= t.famous_reputation;
    let in_academy = h.team.is_some() && w.teams[h.team].club == club && !matches!(w.teams[h.team].kind, TeamKind::First | TeamKind::Reserve);
    let mut out = Vec::new();
    for &s in &w.clubs[club].staff {
        let st = &w.staff[s];
        if st.retired {
            continue;
        }
        let rel_ca = relevance(st.role, age, minutes, false);
        if rel_ca <= 0.0 {
            continue;
        }
        let potential_domain = if age < 19.0 && matches!(st.role, StaffRole::HeadOfYouth | StaffRole::Coach) { Domain::Youth } else { Domain::Potential };
        let mut sg = sigma(t, seen, competence(w, s, Domain::CurrentAbility), w.date, famous);
        if st.role == StaffRole::HeadOfYouth && in_academy {
            sg *= 0.75;
        }
        if st.role == StaffRole::Analyst && minutes >= 600 {
            sg *= 0.8;
        }
        let who = Observer::Person(st.person.0);
        // truth-ok: the reading is made from truth on purpose; noise, exposure and the evaluator's own bias stand between it and the club.
        let (ca, ca_band) = perceived_ca(f32::from(c.ca), sg, who, p);
        let (pa, mut pa_band) = perceived_pa(f32::from(c.pa), ca, sg, competence(w, s, potential_domain), who, p);
        if st.role == StaffRole::Analyst {
            // Progression indicators, not a direct view of potential.
            pa_band += 6.0;
        }
        out.push(Reading { by: s, role: st.role, ca: Span { mid: ca, band: ca_band }, pa: Span { mid: pa, band: pa_band }, relevance: rel_ca });
    }
    // Scouts speak through their reports, weighted by how recent and well founded they are.
    for r in w.scouting.of(club, p) {
        let age_days = r.date.days_until(w.date).max(0) as f32;
        let fresh = pw_core::math::exp(-age_days / 180.0);
        let founded = 0.5 + f32::from(r.minutes) / 900.0;
        out.push(Reading {
            by: r.scout,
            role: w.staff.get(r.scout).map_or(StaffRole::Scout, |s| s.role),
            ca: Span { mid: f32::from(r.ca), band: f32::from(r.band) + age_days / 30.0 },
            pa: Span { mid: f32::from(r.pa), band: f32::from(r.band) * 1.5 + age_days / 30.0 },
            relevance: fresh * founded,
        });
    }
    out
}

/// What the manager makes of the readings: a weighted combination, its uncertainty (widened by disagreement) and the weights used.
fn combine(w: &World, manager: StaffId, rs: &[Reading]) -> (Span, Span, f32, SmallVec<[Opinion; 6]>) {
    let ego = if manager.is_some() { ego(w, manager) } else { 0.5 };
    let others = 1.1 - 0.5 * ego;
    let weights: Vec<(f32, f32)> = rs
        .iter()
        .map(|r| {
            let tr = if manager.is_some() { trust(w, manager, r.by) } else { 0.6 };
            let own = if r.by == manager { 1.0 + 0.8 * ego } else { others };
            let precision = 1.0 / (r.ca.band * r.ca.band + 1.0);
            (tr, tr * own * r.relevance * precision)
        })
        .collect();
    let total: f32 = weights.iter().map(|x| x.1).sum::<f32>().max(1e-6);
    let mean = |f: &dyn Fn(&Reading) -> f32| rs.iter().zip(&weights).map(|(r, x)| f(r) * x.1).sum::<f32>() / total;
    let ca = mean(&|r| r.ca.mid);
    let pa = mean(&|r| r.pa.mid);
    let spread = (rs.iter().zip(&weights).map(|(r, x)| (r.ca.mid - ca).powi(2) * x.1).sum::<f32>() / total).sqrt();
    let neff = (total * total / weights.iter().map(|x| x.1 * x.1).sum::<f32>().max(1e-9)).max(1.0);
    let band = |f: &dyn Fn(&Reading) -> f32| ((mean(&|r| f(r).powi(2)) / neff) + spread * spread * 0.5).sqrt();
    let ca_band = band(&|r| r.ca.band);
    let pa_band = band(&|r| r.pa.band) + (pa - ca).max(0.0) * 0.05;
    let opinions = rs.iter().zip(&weights).map(|(r, x)| Opinion { by: r.by, role: r.role, ca: r.ca, pa: r.pa, weight: x.1 / total, trust: x.0 }).collect();
    (Span { mid: ca, band: ca_band }, Span { mid: pa.max(ca), band: pa_band }, spread, opinions)
}

/// Share of the growth still to come that arrives within a year, by age.
fn yearly_share(age: f32) -> f32 {
    interp(&[(16.0, 0.28), (19.0, 0.22), (22.0, 0.15), (25.0, 0.07), (28.0, 0.0), (33.0, -0.05)], age)
}

/// The most expert available person for a domain among the club's staff, and what they make of a truth in `[1, 20]`.
fn expert_view(w: &World, club: ClubId, p: PlayerId, d: Domain, truth: f32, fld: u64, exposure: bool) -> Option<(f32, f32)> {
    let s = w.clubs[club].staff.iter().copied().filter(|&s| !w.staff[s].retired).max_by(|&a, &b| competence(w, a, d).total_cmp(&competence(w, b, d)).then(b.0.cmp(&a.0)))?;
    let c = competence(w, s, d);
    // Without exposure the same expert can say very little.
    let sg = (5.5 - 0.22 * c) * if exposure { 1.0 } else { 2.5 };
    Some((perceive(truth, sg, Observer::Person(w.staff[s].person.0), p, fld).clamp(1.0, 20.0), sg))
}

fn mean_attr(w: &World, p: PlayerId, attrs: &[Attr]) -> f32 {
    attrs.iter().map(|&a| w.players.cold[p].attr(a)).sum::<f32>() / attrs.len() as f32
}

fn risks(w: &World, club: ClubId, p: PlayerId, ev: &Evidence, age: f32) -> SmallVec<[Risk; 4]> {
    let c = &w.players.cold[p];
    let exposed = ev.training || ev.minutes_seen >= 300;
    let mut out: SmallVec<[Risk; 4]> = SmallVec::new();
    // truth-ok: the risks are the club's readings of true make-up through its experts, degraded when exposure is thin.
    let physical = mean_attr(w, p, &[Attr::Pace, Attr::Acceleration, Attr::Stamina, Attr::Strength, Attr::NaturalFitness, Attr::Agility]);
    if let Some((b, _)) = expert_view(w, club, p, Domain::Physical, physical, 2000, exposed) {
        let bar = if age < 22.0 { 12.0 } else { 10.5 };
        if b < bar || !exposed {
            out.push(Risk { kind: RiskKind::Physical, level: ((bar - b) / 6.0).clamp(0.0, 1.0), known: exposed });
        }
    }
    let decisions = mean_attr(w, p, &[Attr::Decisions, Attr::Composure, Attr::Anticipation, Attr::Concentration]);
    if let Some((b, _)) = expert_view(w, club, p, Domain::Mental, decisions, 2001, exposed)
        && (b < 11.0 || !exposed)
    {
        out.push(Risk { kind: RiskKind::Decisions, level: ((11.0 - b) / 6.0).clamp(0.0, 1.0), known: exposed });
    }
    // The injury record is public knowledge.
    if c.injuries_career >= 3 {
        out.push(Risk { kind: RiskKind::Injuries, level: (f32::from(c.injuries_career) / 10.0).min(1.0), known: true });
    }
    let hidden = &w.people[c.person].hidden;
    let prof = hidden.f(Hidden::Professionalism);
    if let Some((b, _)) = expert_view(w, club, p, Domain::Personality, prof, 2002, ev.training || ev.minutes_seen >= 600)
        && (b < 9.5 || !(ev.training || ev.minutes_seen >= 600))
    {
        out.push(Risk { kind: RiskKind::Attitude, level: ((11.0 - b) / 8.0).clamp(0.0, 1.0), known: ev.training || ev.minutes_seen >= 600 });
    }
    let consistency = hidden.f(Hidden::Consistency);
    if let Some((b, _)) = expert_view(w, club, p, Domain::Mental, consistency, 2003, exposed)
        && exposed
        && b < 9.0
    {
        out.push(Risk { kind: RiskKind::Consistency, level: ((10.5 - b) / 7.0).clamp(0.0, 1.0), known: true });
    }
    out.truncate(4);
    out
}

/// The positions he has actually played and trained at, best first (what a club can see, not a hidden best position).
fn projected_roles(w: &World, p: PlayerId) -> SmallVec<[Pos; 2]> {
    let c = &w.players.cold[p];
    let mut ix: Vec<usize> = (0..pw_core::pos::N_POS).collect();
    ix.sort_by(|&a, &b| c.familiarity[b].cmp(&c.familiarity[a]).then(a.cmp(&b)));
    ix.into_iter().take(2).filter(|&i| c.familiarity[i] >= 12).map(|i| Pos::ALL[i]).collect()
}

fn evidence(w: &World, club: ClubId, p: PlayerId) -> Evidence {
    let h = &w.players.hot[p];
    let mine = h.team.is_some() && w.teams[h.team].club == club;
    let seen = w.knowledge.seen(club, p);
    Evidence {
        training: mine,
        minutes_seen: seen.map_or(0, |s| s.minutes),
        reports: w.scouting.of(club, p).len() as u8,
        analytics: w.clubs[club].staff.iter().any(|&s| w.staff[s].role == StaffRole::Analyst && !w.staff[s].retired),
        medical: mine,
        days_since_seen: seen.map_or(999, |s| s.last.days_until(w.date).clamp(0, 999) as u16),
    }
}

/// The deciding person at a club: the manager, else the director of football, else the best-informed staff member.
pub fn viewer(w: &World, club: ClubId) -> StaffId {
    let c = &w.clubs[club];
    c.manager
        .get()
        .or_else(|| c.staff.iter().copied().find(|&s| w.staff[s].role == StaffRole::DirectorOfFootball && !w.staff[s].retired))
        .or_else(|| c.staff.iter().copied().find(|&s| !w.staff[s].retired))
        .unwrap_or(StaffId::NONE)
}

/// Build the club's dossier on a player from scratch, with no history. `None` when the club has nobody to form an opinion.
pub fn build(w: &World, club: ClubId, p: PlayerId) -> Option<Dossier> {
    let rs = readings(w, club, p);
    if rs.is_empty() {
        return None;
    }
    let manager = viewer(w, club);
    let (ca, pa, spread, opinions) = combine(w, manager, &rs);
    let age = w.age_years(p);
    let ev = evidence(w, club, p);
    let grow = yearly_share(age);
    let short = Span { mid: ca.mid + (pa.mid - ca.mid) * grow.max(0.0) + (ca.mid * grow).min(0.0), band: ca.band + (pa.band - ca.band) * 0.4 };
    let ceiling_conf = confidence(pa.band, 8.0, 14.0, 22.0);
    let direction = if grow > 0.06 && pa.mid - ca.mid > 6.0 {
        1
    } else if grow < 0.0 {
        -1
    } else {
        0
    };
    Some(Dossier {
        player: p,
        club,
        date: w.date,
        viewer: manager,
        current: ca,
        current_confidence: confidence(ca.band, 5.0, 9.0, 15.0),
        short_term: short,
        ceiling: pa,
        ceiling_confidence: ceiling_conf,
        direction,
        direction_confidence: if age < 21.0 { ceiling_conf.min(Confidence::Medium) } else { ceiling_conf },
        risks: risks(w, club, p, &ev, age),
        roles: projected_roles(w, p),
        evidence: ev,
        opinions,
        spread,
        history: SmallVec::new(),
    })
}

fn reason_for(old: &Dossier, new: &Dossier, w: &World) -> Reason {
    if old.viewer != new.viewer {
        return Reason::NewManager;
    }
    let staff = |d: &Dossier| d.opinions.iter().map(|o| o.by).collect::<Vec<_>>();
    let (mut a, mut b) = (staff(old), staff(new));
    a.sort();
    b.sort();
    if a != b {
        return if new.evidence.reports > old.evidence.reports { Reason::NewReport } else { Reason::NewStaff };
    }
    if new.evidence.reports > old.evidence.reports {
        return Reason::NewReport;
    }
    if w.players.cold[new.player].injuries_career > 0 && new.risks.iter().any(|r| r.kind == RiskKind::Injuries) && !old.risks.iter().any(|r| r.kind == RiskKind::Injuries) {
        return Reason::Injury;
    }
    if new.evidence.minutes_seen > old.evidence.minutes_seen + 90 {
        return Reason::MoreEvidence;
    }
    if new.evidence.days_since_seen > old.evidence.days_since_seen + 45 {
        return Reason::Stale;
    }
    if w.age_years(new.player) < 23.0 {
        return Reason::Development;
    }
    Reason::Reweighed
}

/// Carry the old dossier's history over and record what moved and why. Small drifts are not revisions.
fn revised(w: &World, old: Option<&Dossier>, mut new: Dossier) -> Dossier {
    let Some(old) = old else { return new };
    new.history = old.history.clone();
    let reason = reason_for(old, &new, w);
    let mut note = |field: Field, from: f32, to: f32, min: f32| {
        if (to - from).abs() >= min {
            if new.history.len() >= 4 {
                new.history.remove(0);
            }
            new.history.push(Revision { date: w.date, field, from, to, reason });
        }
    };
    note(Field::Current, old.current.mid, new.current.mid, 4.0);
    note(Field::Ceiling, old.ceiling.mid, new.ceiling.mid, 6.0);
    note(Field::Direction, f32::from(old.direction), f32::from(new.direction), 1.0);
    new
}

/// Monthly: rebuild the dossiers of everyone a club has a reason to have an opinion on (its own people, shortlisted targets, players
/// its scouts have reported on), keep their history, prune the stale, and log old readings to be checked later.
pub fn monthly(w: &mut World) {
    let today = w.date;
    let mut wanted: Vec<(ClubId, PlayerId)> = Vec::new();
    for c in w.clubs.ids() {
        for &t in &w.clubs[c].teams {
            wanted.extend(w.teams[t].squad.iter().map(|&p| (c, p)));
        }
    }
    for (&(club, _), sl) in &w.deals.shortlists {
        wanted.extend(sl.targets.iter().map(|&(p, _)| (club, p)));
    }
    wanted.extend(w.scouting.reports.keys().copied());
    wanted.sort();
    wanted.dedup();
    wanted.retain(|&(_, p)| w.players.hot[p].status != PlayerStatus::Retired);

    let mut fresh: Vec<Dossier> = Vec::with_capacity(wanted.len());
    for &(club, p) in &wanted {
        if let Some(d) = build(w, club, p) {
            fresh.push(revised(w, w.dossiers.get(club, p), d));
        }
    }
    let keep: rustc_hash::FxHashSet<(ClubId, PlayerId)> = wanted.iter().copied().collect();
    w.dossiers.map.retain(|k, d| keep.contains(k) || d.date.days_until(today) < 365);
    // A year-old reading of a young player is worth checking later; log one per evaluator per player.
    if today.month() == 1 {
        for d in &fresh {
            if w.age_years(d.player) < 24.0 && w.players.hot[d.player].status == PlayerStatus::Active {
                for o in d.opinions.iter().filter(|o| o.weight >= 0.15) {
                    w.dossiers.pending.push(Pending { by: o.by, club: d.club, player: d.player, date: today, ca: o.ca.mid, pa: o.pa.mid });
                }
            }
        }
    }
    for d in fresh {
        w.dossiers.map.insert((d.club, d.player), d);
    }
    settle(w);
}

/// Check readings that are a year old against what became visible: the market's own reading of the player now (not hidden truth).
/// An evaluator whose reading was close gains standing with their managers; one who missed badly loses it. A conspicuous vindication
/// becomes an event the press can pick up years later (section 1.16).
pub fn settle(w: &mut World) {
    let today = w.date;
    let due: Vec<Pending> = w.dossiers.pending.iter().copied().filter(|q| q.date.days_until(today) >= 360).collect();
    if due.is_empty() {
        return;
    }
    w.dossiers.pending.retain(|q| q.date.days_until(today) < 360);
    for q in due {
        if w.players.hot[q.player].status == PlayerStatus::Retired {
            continue;
        }
        let seen_now = crate::market::public_view(w, q.player).0;
        // What the evaluator expected the player to be a year on: the level plus the growth they thought was coming.
        let expected = q.ca + (q.pa - q.ca) * yearly_share(w.age_years(q.player) - 1.0).max(0.0);
        let error = seen_now - expected;
        let track = w.dossiers.track.entry(q.by).or_default();
        if error.abs() <= 6.0 {
            track.right = track.right.saturating_add(1);
        } else if error.abs() >= 14.0 {
            track.wrong = track.wrong.saturating_add(1);
        }
        // A player who turned out far better than the club's own people said, and is now well known, is a story.
        let famous = w.players.cold[q.player].rep.world >= 5000;
        if error >= 25.0 && famous && w.staff.get(q.by).is_some() {
            w.events.push(today, Visibility::Public, EventKind::AssessmentVindicated { staff: q.by, player: q.player, club: q.club, was_right: false });
        } else if error.abs() <= 4.0 && q.pa >= q.ca + 25.0 && famous {
            w.events.push(today, Visibility::Public, EventKind::AssessmentVindicated { staff: q.by, player: q.player, club: q.club, was_right: true });
        }
    }
}

/// The club's reading as (ca, ca band, pa, pa band), if there is a dossier fresh enough to use.
pub fn reading(w: &World, club: ClubId, p: PlayerId) -> Option<(f32, f32, f32, f32)> {
    let d = w.dossiers.get(club, p)?;
    (d.date.days_until(w.date) <= 2 * FRESH_DAYS).then_some((d.current.mid, d.current.band, d.ceiling.mid, d.ceiling.band))
}

/// What a decision-maker with this philosophy makes of the same evidence (section 1.11): the level he can use now, plus the ceiling
/// discounted by how much he trusts youth and how much time he can afford, less what he does not value. The dossier stays the same;
/// the conclusion differs.
pub fn worth(w: &World, club: ClubId, p: PlayerId, patience: f32) -> Option<f32> {
    let d = w.dossiers.get(club, p)?;
    let manager = d.viewer;
    let youth_trust = if manager.is_some() { f32::from(w.staff[manager].philosophy.youth_trust) / 100.0 } else { 0.5 };
    let growth = (d.short_term.mid - d.current.mid).max(0.0) + (d.ceiling.mid - d.short_term.mid).max(0.0) * 0.4;
    let risk: f32 = d.risks.iter().filter(|r| r.known).map(|r| r.level).sum::<f32>() * 2.0;
    Some(d.current.mid + growth * youth_trust * patience - risk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_falls_as_the_band_widens() {
        assert_eq!(confidence(3.0, 5.0, 9.0, 15.0), Confidence::VeryHigh);
        assert_eq!(confidence(7.0, 5.0, 9.0, 15.0), Confidence::High);
        assert_eq!(confidence(12.0, 5.0, 9.0, 15.0), Confidence::Medium);
        assert_eq!(confidence(30.0, 5.0, 9.0, 15.0), Confidence::Low);
    }

    #[test]
    fn young_players_are_expected_to_grow_and_old_ones_to_decline() {
        assert!(yearly_share(17.0) > yearly_share(22.0));
        assert!(yearly_share(22.0) > yearly_share(27.0));
        assert!(yearly_share(35.0) < 0.0);
    }

    #[test]
    fn the_role_that_knows_most_counts_most() {
        // The academy director on a teenager beats the analyst; the analyst with a season of numbers beats the same analyst without.
        assert!(relevance(StaffRole::HeadOfYouth, 16.0, 0, true) > relevance(StaffRole::Analyst, 16.0, 0, true));
        assert!(relevance(StaffRole::Analyst, 26.0, 1500, false) > relevance(StaffRole::Analyst, 26.0, 100, false));
        assert!(relevance(StaffRole::HeadOfYouth, 16.0, 0, false) > relevance(StaffRole::HeadOfYouth, 28.0, 0, false));
        assert_eq!(relevance(StaffRole::Physio, 20.0, 500, false), 0.0);
    }
}
