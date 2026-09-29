//! Tactical intelligence (locked design 7.1-7.34): who a manager is as a tactician, what he believes of the opponent before kick-off,
//! how he chooses among answers, how well the squad can carry them out, what the club remembers, and what he learns afterwards.
//!
//! The pipeline in a match is observe -> diagnose -> adapt (`coach.rs` runs it live; `pw_match::Coach` is the seam):
//!
//! * before the game the manager holds a *belief* about the opponent (`dossier`), built from how they were seen to play and what the
//!   club remembers of the last meeting, with uncertainty that follows how good his preparation is; he can be surprised on purpose;
//! * during it the staff see football evidence, never engine numbers; each has their own eye and their own record of being right;
//! * he explains what he sees, keeps the explanations he is unsure of, and needs evidence of his own temperament before changing;
//! * a change is only carried out as far as the squad's drilling and the manager's communication allow;
//! * afterwards the change is judged twice: by what it did to the problem, and by what the scoreboard did. Managers learn from both,
//!   the worse readers mostly from the scoreboard, so wrong lessons are learned.
//!
//! There is no table saying which answer beats which problem: applicability is one input among philosophy, lessons, memory, urgency and
//! temperament, and the match engine decides what any answer actually does.

use pw_core::rng::{Rng, hash_key, stream};
use pw_core::{ClubId, Hidden, Mentality, PlayerId, StaffAttr, StaffId, Tactics};
use pw_match::MatchResult;
use pw_world::dossier::Confidence;
use pw_world::event::{EventKind, Visibility};
use pw_world::tactics::*;
use pw_world::{Archetype, Fixture, Philosophy, StaffRole, TeamKind, World};
use smallvec::SmallVec;

use crate::coach::Logs;
use crate::selection::Selection;

// ------------------------------------------------------------------ the tactician

/// A manager as a tactician: many abilities, not one (7.5), and a temperament that sets when he changes things (7.15).
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    /// Quality of pre-match work.
    pub prep: f32,
    /// Match reading: noticing the pattern that matters.
    pub reading: f32,
    /// Willingness and ability to change what he does.
    pub adapt: f32,
    /// Minutes of evidence he wants before changing the plan.
    pub patience: f32,
    pub stubborn: f32,
    /// Getting the message across, including in a language the players speak.
    pub comms: f32,
    pub daring: f32,
    /// Resistance to what his staff tell him.
    pub ego: f32,
}

/// The manager of a club and how he works. `None` for a club without one.
pub fn profile(w: &World, club: ClubId) -> Option<(StaffId, Profile)> {
    let m = w.clubs[club].manager.get()?;
    let s = &w.staff[m];
    let person = &w.people[s.person];
    let a = |x: StaffAttr| s.attrs.f(x) / 20.0;
    let h = |x: Hidden| person.hidden.f(x) / 20.0;
    let exp = (s.record.games as f32 / 400.0).min(1.0);
    let best = |role: StaffRole| w.clubs[club].staff.iter().filter(|&&x| w.staff[x].role == role && !w.staff[x].retired).map(|&x| w.staff[x].role_rating(role) / 20.0).fold(0.0f32, f32::max);
    let support = best(StaffRole::Analyst).max(best(StaffRole::Assistant) * 0.9);
    let prep = (0.4 * a(StaffAttr::TacticalKnowledge) + 0.25 * support + 0.2 * a(StaffAttr::JudgingAbility) + 0.15 * exp).clamp(0.0, 1.0);
    let reading = (0.4 * a(StaffAttr::TacticalKnowledge) + 0.3 * a(StaffAttr::JudgingAbility) + 0.15 * exp + 0.15 * h(Hidden::Consistency)).clamp(0.0, 1.0);
    let adapt = (0.55 * h(Hidden::Adaptability) + 0.3 * a(StaffAttr::TacticalKnowledge) + 0.15 * exp).clamp(0.0, 1.0);
    let phil = s.philosophy;
    let (base_patience, base_stubborn) = match phil.archetype {
        Archetype::Rotator => (12.0, 0.2),
        Archetype::Developer => (25.0, 0.35),
        Archetype::Pragmatist => (20.0, 0.4),
        Archetype::Loyalist => (35.0, 0.7),
    };
    let nation = w.clubs[club].nation;
    let language = f32::from(w.lives[s.person].fluency(nation)) / 100.0;
    let language = if w.people[s.person].nation == nation { 1.0 } else { language };
    let p = Profile {
        prep,
        reading,
        adapt,
        patience: (base_patience + (h(Hidden::Temperament) - 0.5) * 16.0 - (h(Hidden::Ambition) - 0.5) * 10.0 - adapt * 6.0).clamp(8.0, 50.0),
        stubborn: (0.5 * base_stubborn + 0.5 * (1.0 - adapt)).clamp(0.0, 1.0),
        comms: (0.5 * a(StaffAttr::ManManagement) + 0.3 * a(StaffAttr::Motivating) + 0.2 * language).clamp(0.0, 1.0),
        daring: (0.5 * h(Hidden::Ambition) + 0.5 * (f32::from(phil.mentality) + 2.0) / 4.0).clamp(0.0, 1.0),
        ego: (0.7 * (h(Hidden::Ambition) + h(Hidden::Controversy)) / 2.0 + 0.3 * f32::from(s.reputation.min(10_000)) / 10_000.0).clamp(0.0, 1.0),
    };
    Some((m, p))
}

/// What a match is worth and what the manager knows of its context (7.16-7.17): the aggregate, elimination, the stakes.
#[derive(Clone, Copy, Debug)]
pub struct MatchCtx {
    pub importance: f32,
    /// A winner is needed today.
    pub decisive: bool,
    /// First-leg goals as (this home team, this away team).
    pub first_leg: Option<(u8, u8)>,
    /// Each side's view of the other's standing: -1 (far weaker) .. +1 (far stronger).
    pub opposition: [f32; 2],
    pub referee: f32,
}

impl MatchCtx {
    pub fn for_fixture(w: &World, fx: &Fixture, importance: f32, decisive: bool, first_leg: Option<(u8, u8)>, referee: f32) -> Self {
        let rep = |t: pw_core::TeamId| f32::from(w.clubs[w.teams[t].club].reputation);
        let gap = ((rep(fx.away) - rep(fx.home)) / 4000.0).clamp(-1.0, 1.0);
        MatchCtx { importance, decisive, first_leg, opposition: [gap, -gap], referee }
    }

    /// The side's lead on the day.
    pub fn on_day(&self, side: usize, goals: [u8; 2]) -> i32 {
        i32::from(goals[side]) - i32::from(goals[1 - side])
    }

    /// The side's lead counting the first leg, when it is a tie.
    pub fn margin(&self, side: usize, goals: [u8; 2]) -> i32 {
        let (mine, theirs) = (i32::from(goals[side]), i32::from(goals[1 - side]));
        let carried = self.first_leg.map_or(0, |(h, a)| if side == 0 { i32::from(h) - i32::from(a) } else { i32::from(a) - i32::from(h) });
        mine - theirs + carried
    }

    /// How badly the scoreline needs changing for this side: -1 (must protect) .. +1 (must chase), from the margin, the clock and what is
    /// at stake (the same 0-0 is not the same when two goals down on aggregate).
    pub fn urgency(&self, side: usize, goals: [u8; 2], minute: u8) -> f32 {
        let m = self.margin(side, goals);
        let clock = (f32::from(minute) / 90.0).clamp(0.0, 1.0);
        let need = if m < 0 {
            (-m as f32 / 2.0).min(1.0)
        } else if m == 0 && (self.decisive || self.first_leg.is_some()) {
            0.35
        } else {
            0.0
        };
        let protect = if m > 0 { (m as f32 / 2.0).min(1.0) * 0.8 } else { 0.0 };
        ((need - protect) * (0.4 + 0.6 * clock) * (0.6 + 0.6 * self.importance)).clamp(-1.0, 1.0)
    }
}

// ------------------------------------------------------------------ what the squad can carry out

/// How well a squad knows one way of playing, 0..1 (7.10-7.11). Improvisation is what the club has never rehearsed.
pub fn drilled(w: &World, club: ClubId, style: usize) -> f32 {
    w.tactics.drill.get(&club).map_or(0.5, |d| f32::from(d.fam[style.min(N_STYLES - 1)]) / 100.0)
}

/// The share of an intended change that is actually carried out (7.9, 7.22, 7.32): how well drilled the squad is in the target
/// style, how well the manager gets it across, how much the eleven understand, how tired and how stressed they are, and the setting
/// (half-time allows explanation; the touchline in the last quarter does not).
pub fn execution(w: &World, club: ClubId, xi: &[PlayerId], target: &Tactics, prof: &Profile, minute: u8, half_time: bool) -> f32 {
    let drill = drilled(w, club, style_of(target));
    let n = xi.len().max(1) as f32;
    let (mut intel, mut fresh, mut calm, mut settled) = (0.0, 0.0, 0.0, 0.0);
    for &p in xi {
        let c = &w.players.cold[p];
        intel += (c.attrs.get(pw_core::Attr::Teamwork) + c.attrs.get(pw_core::Attr::Decisions) + c.attrs.get(pw_core::Attr::Anticipation)) / 60.0;
        fresh += f32::from(w.players.hot[p].condition) / 100.0;
        calm += 1.0 - f32::from(w.lives[c.person].stress) / 100.0;
        settled += crate::adaptation::effect(w, p).1;
    }
    let setting = if half_time {
        1.1
    } else if minute >= 70 {
        0.9
    } else {
        1.0
    };
    ((0.35 + 0.65 * drill) * (0.7 + 0.3 * prof.comms) * (0.8 + 0.2 * intel / n) * (0.85 + 0.15 * fresh / n) * (0.9 + 0.1 * calm / n) * (0.85 + 0.15 * settled / n) * setting).clamp(0.15, 1.0)
}

/// What a response does to a set of instructions when it is fully carried out.
pub fn shifted(t: Tactics, r: Response) -> Tactics {
    let add = |v: u8, d: i32| (i32::from(v) + d).clamp(0, 100) as u8;
    let mut n = t;
    match r {
        Response::Compact => {
            n.mentality = t.mentality.shift(-1);
            n.line = add(t.line, -10);
            n.press = add(t.press, -10);
            n.tempo = add(t.tempo, -5);
        }
        Response::GoForIt => {
            n.mentality = t.mentality.shift(1);
            n.line = add(t.line, 8);
            n.press = add(t.press, 8);
            n.tempo = add(t.tempo, 8);
        }
        Response::PressHigher => {
            n.press = add(t.press, 22);
            n.line = add(t.line, 10);
        }
        Response::SitDeep => {
            n.line = add(t.line, -20);
            n.press = add(t.press, -22);
            n.directness = add(t.directness, 10);
        }
        Response::PlayThrough => {
            n.directness = add(t.directness, -22);
            n.tempo = add(t.tempo, -8);
            n.width = add(t.width, 5);
        }
        Response::GoDirect => {
            n.directness = add(t.directness, 25);
            n.tempo = add(t.tempo, 8);
        }
        Response::Hold | Response::ReplacePlayer | Response::Refresh | Response::CoverBooked => {}
    }
    n
}

/// Part of the way to `target`: the squad carries out `share` of the change. The mentality moves only if most of it lands.
pub fn blend(from: Tactics, to: Tactics, share: f32) -> Tactics {
    let mix = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * share).round().clamp(0.0, 100.0) as u8;
    Tactics {
        formation: from.formation,
        mentality: if share >= 0.5 { to.mentality } else { from.mentality },
        tempo: mix(from.tempo, to.tempo),
        width: mix(from.width, to.width),
        directness: mix(from.directness, to.directness),
        line: mix(from.line, to.line),
        press: mix(from.press, to.press),
    }
}

// ------------------------------------------------------------------ choosing an answer

/// How well an answer fits a diagnosis, 0..1 (7.7). This is the range of sensible options, not a counter table: it is one input.
pub fn applicability(d: Diagnosis, r: Response) -> f32 {
    use Diagnosis as D;
    use Response as R;
    match (d, r) {
        (D::TooOpen, R::Compact) => 0.9,
        (D::TooOpen, R::SitDeep) => 0.6,
        (D::TooOpen, R::Hold | R::PlayThrough) => 0.3,
        (D::TooCautious, R::GoForIt) => 0.8,
        (D::TooCautious, R::PressHigher) => 0.6,
        (D::TooCautious, R::PlayThrough) => 0.5,
        (D::TooCautious, R::GoDirect) => 0.4,
        (D::PressNotWorking, R::PlayThrough) => 0.8,
        (D::PressNotWorking, R::GoDirect) => 0.7,
        (D::PressNotWorking, R::SitDeep) => 0.5,
        (D::PressNotWorking, R::Hold) => 0.4,
        (D::PressNotWorking, R::Compact) => 0.3,
        (D::TheyAreBetter, R::SitDeep | R::Compact) => 0.6,
        (D::TheyAreBetter, R::GoDirect) => 0.5,
        (D::TheyAreBetter, R::Hold) => 0.4,
        (D::IndividualForm, R::ReplacePlayer) => 0.9,
        (D::IndividualForm, R::Hold) => 0.4,
        (D::IndividualForm, R::Compact) => 0.3,
        (D::Fatigue, R::Refresh) => 0.9,
        (D::Fatigue, R::SitDeep) => 0.5,
        (D::Fatigue, R::Hold) => 0.3,
        (D::Unlucky, R::Hold) => 0.8,
        (D::Unlucky, R::GoForIt) => 0.3,
        (D::CardRisk, R::CoverBooked) => 0.9,
        (D::CardRisk, R::Hold) => 0.3,
        (D::NeedAGoal, R::GoForIt) => 0.9,
        (D::NeedAGoal, R::GoDirect) => 0.6,
        (D::NeedAGoal, R::PressHigher) => 0.5,
        (D::NeedAGoal, R::PlayThrough) => 0.4,
        (D::ProtectingALead, R::Compact | R::SitDeep) => 0.8,
        (D::ProtectingALead, R::Hold) => 0.5,
        (D::ProtectingALead, R::Refresh) => 0.4,
        _ => 0.0,
    }
}

/// How much an answer is in keeping with the way this manager believes football should be played, 0..1.
pub fn philosophy_fit(phil: &Philosophy, base: Tactics, r: Response) -> f32 {
    if r == Response::Hold {
        return 0.7;
    }
    let t = shifted(base, r);
    let ment = f32::from(phil.mentality) / 2.0;
    let d = (f32::from(t.press.abs_diff(phil.press)) + f32::from(t.tempo.abs_diff(phil.tempo)) + f32::from(t.directness.abs_diff(phil.directness))) / 300.0 + (f32::from(t.mentality.level() as i8) / 2.0 - ment).abs() / 4.0;
    (1.0 - d).clamp(0.0, 1.0)
}

/// What a club's staff remember of an opponent, discounted when whoever wrote it down has left (7.24).
pub fn memory_of(w: &World, club: ClubId, opp: ClubId) -> Option<(OppMemory, f32)> {
    let m = *w.tactics.memory.get(&(club, opp))?;
    let author_here = m.author.is_some() && w.staff.get(m.author).is_some_and(|s| s.club == club && !s.retired);
    let age = m.last.days_until(w.date).max(0) as f32;
    let fade = (1.0 - age / 1100.0).clamp(0.2, 1.0);
    Some((m, if author_here { fade } else { fade * 0.5 }))
}

/// Score of each answer to a diagnosis for this manager; the highest is chosen. Everything that differs between managers is here.
#[allow(clippy::too_many_arguments)]
pub fn choose_response(w: &World, club: ClubId, manager: StaffId, prof: &Profile, base: Tactics, d: Diagnosis, urgency: f32, opp: Option<ClubId>, rng: &mut Rng) -> (Response, f32) {
    let phil = w.staff[manager].philosophy;
    let rec = w.tactics.managers.get(&manager);
    let memory = opp.and_then(|o| memory_of(w, club, o));
    let mut best = (Response::Hold, f32::MIN);
    for r in Response::ALL {
        let ap = applicability(d, r);
        if ap <= 0.0 && r != Response::Hold {
            continue;
        }
        let lesson = rec.map_or(0.0, |m| f32::from(m.lessons[r.idx()]) / 100.0);
        let mem = memory.map_or(0.0, |(m, k)| k * (if m.worked == Some(r) { 0.25 } else { 0.0 } - if m.failed == Some(r) { 0.25 } else { 0.0 }));
        let mut score = 0.9 * ap + 0.6 * philosophy_fit(&phil, base, r) + 0.5 * lesson + mem;
        if r == Response::Hold {
            score += prof.stubborn * 0.6 - urgency.abs() * 0.5;
        } else if r.is_structural() {
            // Risk appetite: daring managers reach for the attacking answers, cautious ones for the safe ones.
            let attacking = matches!(r, Response::GoForIt | Response::PressHigher | Response::GoDirect);
            score += if attacking { (prof.daring - 0.5) * 0.5 + urgency.max(0.0) * 0.5 } else { (0.5 - prof.daring) * 0.3 + (-urgency).max(0.0) * 0.5 };
        }
        score += rng.normal() * 0.12 * (1.3 - prof.adapt);
        if score > best.1 {
            best = (r, score);
        }
    }
    best
}

// ------------------------------------------------------------------ before kick-off

/// What a manager believes of the opponent before the match, with how sure he is (7.2). Never the opponent's actual instructions.
#[derive(Clone, Copy, Debug)]
pub struct Belief {
    pub formation: u8,
    pub press: u8,
    pub tempo: u8,
    pub direct: u8,
    pub shape_confidence: Confidence,
    pub style_confidence: Confidence,
    /// Who starts up front, and whether he is even fit, is the least certain thing.
    pub lineup_confidence: Confidence,
    /// Built from little or old evidence.
    pub thin: bool,
}

/// The belief dossier of `club` on `opp`: from how the opponent was seen to play, the club's memory of the last meeting, and the
/// quality of the preparation work; older and thinner evidence and a poorer analyst give a fuzzier picture.
pub fn dossier(w: &World, club: ClubId, opp: ClubId) -> Belief {
    let prof = profile(w, club).map(|x| x.1);
    let prep = prof.map_or(0.4, |p| p.prep);
    let recs: SmallVec<[StyleRec; 6]> = w.tactics.styles.get(&opp).map(|v| v.iter().copied().collect()).unwrap_or_default();
    let memory = memory_of(w, club, opp);
    let mut rng = Rng::keyed(&[w.seed, stream::TACTICS, u64::from(club.0), u64::from(opp.0), w.date.0 as u64, 0xd055]);
    let trend = w.culture.nations.get(&w.clubs[opp].nation).map_or((50, 50, 50), |c| (c.trend_press, c.trend_tempo, c.trend_direct));
    if recs.is_empty() && memory.is_none() {
        let sd = 16.0;
        let g = |x: u8, rng: &mut Rng| (f32::from(x) + rng.normal() * sd).round().clamp(0.0, 100.0) as u8;
        return Belief { formation: 0, press: g(trend.0, &mut rng), tempo: g(trend.1, &mut rng), direct: g(trend.2, &mut rng), shape_confidence: Confidence::Low, style_confidence: Confidence::Low, lineup_confidence: Confidence::Low, thin: true };
    }
    let n = recs.len() as f32;
    // Recent matches weigh more.
    let (mut wsum, mut press, mut tempo, mut direct) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let mut counts = [0u8; 8];
    for (i, r) in recs.iter().enumerate() {
        let wt = 1.0 + i as f32 * 0.5;
        wsum += wt;
        press += f32::from(r.press) * wt;
        tempo += f32::from(r.tempo) * wt;
        direct += f32::from(r.direct) * wt;
        counts[usize::from(r.formation).min(7)] += 1;
    }
    let (mut press, mut tempo, mut direct) = if wsum > 0.0 { (press / wsum, tempo / wsum, direct / wsum) } else { (f32::from(trend.0), f32::from(trend.1), f32::from(trend.2)) };
    if let Some((m, k)) = memory {
        // What the last meeting showed counts for more with a good note-taker.
        let wt = 0.35 * k * (0.5 + prep);
        press = press * (1.0 - wt) + f32::from(m.press) * wt;
        tempo = tempo * (1.0 - wt) + f32::from(m.tempo) * wt;
        direct = direct * (1.0 - wt) + f32::from(m.direct) * wt;
    }
    let stale = recs.last().map_or(60, |r| r.date.days_until(w.date).max(0));
    let sigma = (1.0 - prep) * 14.0 + (stale as f32 / 30.0).min(2.0) * 4.0 + 12.0 / (1.0 + n);
    let fuzz = |x: f32, rng: &mut Rng| (x + rng.normal() * sigma).round().clamp(0.0, 100.0) as u8;
    let mode = counts.iter().enumerate().max_by_key(|&(i, &c)| (c, std::cmp::Reverse(i))).map_or(0, |(i, _)| i as u8);
    let spread = if recs.len() >= 2 { recs.iter().map(|r| (f32::from(r.press) - press).abs()).sum::<f32>() / n } else { 30.0 };
    let style_confidence = match (recs.len(), spread < 10.0, memory.is_some() && prep > 0.65) {
        (4.., true, true) => Confidence::VeryHigh,
        (4.., true, _) => Confidence::High,
        (2.., _, _) => Confidence::Medium,
        _ => Confidence::Low,
    };
    let shape_confidence = if counts[usize::from(mode)] as f32 >= n * 0.7 && recs.len() >= 3 { Confidence::High } else if recs.len() >= 2 { Confidence::Medium } else { Confidence::Low };
    let lineup_confidence = if prep > 0.7 && recs.len() >= 4 { Confidence::Medium } else { Confidence::Low };
    Belief { formation: mode, press: fuzz(press, &mut rng), tempo: fuzz(tempo, &mut rng), direct: fuzz(direct, &mut rng), shape_confidence, style_confidence, lineup_confidence, thin: recs.len() < 3 || stale > 30 }
}

/// What a manager settled on before the game.
#[derive(Clone, Copy, Debug)]
pub struct Prep {
    pub manager: StaffId,
    pub belief: Belief,
    /// The problem he expects, and what he chose to do about it.
    pub expects: Option<SignKind>,
    pub response: Response,
    /// He deliberately played differently from how he is known to.
    pub surprise: bool,
    /// Share of the intended change the squad could carry out.
    pub executed: f32,
}

/// Before kick-off each manager reads his dossier on the other side, may change his approach for it (at the squad's drilled level), and
/// may deliberately play against type. Changes `Selection::tactics`; the belief is never handed to the engine.
pub fn prepare(w: &World, sels: [&mut Selection; 2], ctx: &MatchCtx) -> [Option<Prep>; 2] {
    let clubs = [w.teams[sels[0].team].club, w.teams[sels[1].team].club];
    let mut out = [None, None];
    for (i, sel) in sels.into_iter().enumerate() {
        let (club, opp) = (clubs[i], clubs[1 - i]);
        let Some((m, prof)) = profile(w, club) else { continue };
        let belief = dossier(w, club, opp);
        let base = sel.tactics;
        let mut rng = Rng::keyed(&[w.seed, stream::TACTICS, u64::from(club.0), u64::from(opp.0), w.date.0 as u64, 0x94e]);
        let mem = memory_of(w, club, opp);
        // The problem he expects them to pose: only when he is reasonably sure of what they are.
        let sure = belief.style_confidence >= Confidence::Medium;
        let expects = if sure && belief.press >= 65 && base.directness <= 55 {
            Some(SignKind::BuildUpBroken)
        } else if sure && belief.press <= 35 && belief.tempo <= 45 && base.mentality >= Mentality::Balanced {
            Some(SignKind::NoChances)
        } else if ctx.opposition[i] >= 0.3 || mem.is_some_and(|(x, k)| k > 0.4 && x.hurt_by == Some(SignKind::ChancesAgainst)) {
            Some(SignKind::ChancesAgainst)
        } else {
            None
        };
        let mut response = Response::Hold;
        let mut tactics = base;
        let mut executed = 1.0;
        if let Some(sign) = expects {
            let d = match sign {
                SignKind::BuildUpBroken => Diagnosis::PressNotWorking,
                SignKind::NoChances => Diagnosis::TooCautious,
                _ => Diagnosis::TheyAreBetter,
            };
            // Coming up against a better side, or a press he has met, is planned for calmly and at a low temperature of urgency.
            let (r, _) = choose_response(w, club, m, &prof, base, d, 0.0, Some(opp), &mut rng);
            if r.is_structural() && rng.chance(0.5 + 0.5 * prof.prep) {
                let target = shifted(base, r);
                executed = execution(w, club, &sel.xi, &target, &prof, 0, true).max(0.3) * (0.7 + 0.3 * prof.prep);
                tactics = blend(base, target, executed.min(1.0));
                response = r;
            }
        }
        // Surprise: a manager who learned something the hard way, and can change, sometimes plays against type.
        let surprise_p = 0.22 * prof.adapt * prof.prep + if mem.is_some_and(|(x, k)| k > 0.4 && x.hurt_by.is_some()) { 0.12 } else { 0.0 };
        let surprise = rng.chance(surprise_p);
        if surprise {
            let d = if rng.chance(0.5) { 1 } else { -1 };
            let amount = (12.0 + rng.f32() * 13.0) as i32 * d;
            if rng.chance(0.5) {
                tactics.press = (i32::from(tactics.press) + amount).clamp(0, 100) as u8;
            } else {
                tactics.directness = (i32::from(tactics.directness) + amount).clamp(0, 100) as u8;
            }
        }
        sel.tactics = tactics;
        out[i] = Some(Prep { manager: m, belief, expects, response, surprise, executed });
    }
    out
}

// ------------------------------------------------------------------ after the match

fn verdict(before: f32, after: f32) -> Verdict {
    let ratio = (after + 0.02) / (before + 0.02);
    if ratio < 0.8 {
        Verdict::Helped
    } else if ratio > 1.25 {
        Verdict::Worsened
    } else {
        Verdict::NoEffect
    }
}

fn score(v: Option<Verdict>) -> f32 {
    match v {
        Some(Verdict::Helped) => 1.0,
        Some(Verdict::Worsened) => -1.0,
        _ => 0.0,
    }
}

/// After the match: judge every change twice, learn (the wrong lesson too), update who was right among the staff, the club's memory of
/// the opponent, how each was seen to play, and the manager's record (7.23, 7.28, 7.29, 7.34).
pub fn settle(w: &mut World, fx: &Fixture, logs: Logs, preps: &[Option<Prep>; 2], tactics: [Tactics; 2], result: &MatchResult) {
    if w.teams[fx.home].kind != TeamKind::First || w.teams[fx.away].kind != TeamKind::First {
        return;
    }
    let today = w.date;
    let clubs = [w.teams[fx.home].club, w.teams[fx.away].club];
    // How each side was seen to play.
    for i in 0..2 {
        let t = tactics[i];
        let list = w.tactics.styles.entry(clubs[i]).or_default();
        if list.len() >= 6 {
            list.pop_front();
        }
        list.push_back(StyleRec { date: today, formation: t.formation, press: t.press, tempo: t.tempo, direct: t.directness, mentality: t.mentality.level() as i8 });
    }
    let goals = [result.home_goals, result.away_goals];
    let final_look = logs.full_time.as_ref();
    for i in 0..2 {
        let (club, opp) = (clubs[i], clubs[1 - i]);
        let Some(brain) = logs.brains[i].as_ref() else { continue };
        let m = brain.manager;
        let prof = brain.profile;
        let mut worked = None;
        let mut failed = None;
        let mut hurt = None;
        let mut recs: Vec<Trace> = Vec::new();
        for lt in &brain.traces {
            let mut tr = lt.trace.clone();
            // By process: the rate of the thing that was wrong, before and after the change. By result: what the scoreboard did.
            if let Some(at) = logs.hist.get(lt.window) {
                let prev = lt.window.checked_sub(1).and_then(|k| logs.hist.get(k));
                if let Some(next) = logs.hist.get(lt.window + 1).or(final_look) {
                    let (m1, t1) = (crate::coach::measure(&tr, at, i), f32::from(at.minute));
                    let (m0, t0) = prev.map_or((0.0, 0.0), |p| (crate::coach::measure(&tr, p, i), f32::from(p.minute)));
                    let (m2, t2) = (crate::coach::measure(&tr, next, i), f32::from(next.minute));
                    tr.by_process = Some(verdict((m1 - m0) / (t1 - t0).max(1.0), (m2 - m1) / (t2 - t1).max(1.0)));
                }
            }
            if let (Some(b), Some(last)) = (logs.hist.get(lt.window), final_look) {
                let then = b.goals;
                let (gain, loss) = (i32::from(last.goals[i]) - i32::from(then[i]), i32::from(last.goals[1 - i]) - i32::from(then[1 - i]));
                tr.by_result = Some(match (gain - loss).cmp(&0) {
                    std::cmp::Ordering::Greater => Verdict::Helped,
                    std::cmp::Ordering::Less => Verdict::Worsened,
                    std::cmp::Ordering::Equal => Verdict::NoEffect,
                });
            }
            recs.push(tr);
        }
        // What he settled on before kick-off is a decision too, and is kept the same way (there is no process to judge it by yet).
        if let Some(prep) = preps[i]
            && prep.response != Response::Hold
        {
            let believed = match prep.expects {
                Some(SignKind::BuildUpBroken) => Diagnosis::PressNotWorking,
                Some(SignKind::NoChances) => Diagnosis::TooCautious,
                _ => Diagnosis::TheyAreBetter,
            };
            recs.insert(
                0,
                Trace {
                    uid: fx.uid,
                    date: today,
                    club,
                    manager: m,
                    sign: prep.expects.unwrap_or(SignKind::ChancesAgainst),
                    believed,
                    confidence: match prep.belief.style_confidence {
                        Confidence::Low => 35,
                        Confidence::Medium => 55,
                        Confidence::High => 75,
                        Confidence::VeryHigh => 90,
                    },
                    response: prep.response,
                    executed: (prep.executed.min(1.0) * 100.0) as u8,
                    ..dummy_trace()
                },
            );
        }
        // Learn. The better reader weighs what happened to the problem; the rest mostly weigh the scoreboard.
        for tr in &recs {
            if tr.response == Response::Hold {
                continue;
            }
            let mix = prof.reading * score(tr.by_process) + (1.0 - prof.reading) * score(tr.by_result);
            let rec = w.tactics.managers.entry(m).or_default();
            let l = &mut rec.lessons[tr.response.idx()];
            *l = (i32::from(*l) + (14.0 * mix).round() as i32).clamp(-100, 100) as i8;
            rec.changes += 1;
            if tr.half_time {
                rec.half_time_changes += 1;
            }
            if tr.by_process == Some(Verdict::Helped) {
                rec.good_reads += 1;
                worked = Some(tr.response);
            }
            if tr.by_process == Some(Verdict::Worsened) {
                failed = Some(tr.response);
            }
            if tr.misread() {
                rec.misreads += 1;
            }
            if tr.by_result == Some(Verdict::Helped) && tr.by_process == Some(Verdict::Worsened) {
                rec.credited_luck += 1;
            }
            hurt = hurt.or(Some(tr.sign));
        }
        // History: comebacks, collapses, standing pat (what "brilliant in-game manager" and "no Plan B" are made of).
        let (mine, theirs) = (i32::from(goals[i]), i32::from(goals[1 - i]));
        let trailing_at_break = brain.trailing_at_ht;
        let led_at_break = brain.leading_at_ht;
        {
            let rec = w.tactics.managers.entry(m).or_default();
            if trailing_at_break && mine >= theirs && recs.iter().any(|t| t.half_time) {
                rec.comebacks += 1;
            }
            if led_at_break && mine < theirs {
                rec.collapses += 1;
            }
            if brain.stood_pat_behind {
                rec.stood_pat_behind += 1;
            }
        }
        // Who was right when staff and manager disagreed.
        for tr in &recs {
            for &s in &tr.backed_by {
                let c = w.tactics.credit.entry((m, s)).or_default();
                match tr.by_process {
                    Some(Verdict::Helped) => c.right += 1,
                    Some(Verdict::Worsened) => {
                        c.wrong += 1;
                        c.heeded_wrong += 1;
                    }
                    _ => {}
                }
            }
            for &s in &tr.against {
                let c = w.tactics.credit.entry((m, s)).or_default();
                match tr.by_process {
                    Some(Verdict::Helped) => c.wrong += 1,
                    Some(Verdict::Worsened) => {
                        c.right += 1;
                        c.ignored_right += 1;
                    }
                    _ => {}
                }
            }
        }
        // Tips he did not act on at all: right if the problem got worse, wrong if it went away on its own.
        for tip in &brain.unheeded {
            let (Some(b), Some(last)) = (logs.hist.get(tip.window), final_look) else { continue };
            let probe = Trace { sign: tip.sign, about: tip.who, response: Response::Hold, ..dummy_trace() };
            let (before, after) = (crate::coach::measure(&probe, b, i), crate::coach::measure(&probe, last, i));
            let (t1, t2) = (f32::from(b.minute).max(1.0), f32::from(last.minute).max(f32::from(b.minute) + 1.0));
            let v = verdict(before / t1, (after - before) / (t2 - t1));
            let c = w.tactics.credit.entry((m, tip.staff)).or_default();
            match v {
                Verdict::Worsened => {
                    c.right += 1;
                    c.ignored_right += 1;
                }
                Verdict::Helped => c.wrong += 1,
                Verdict::NoEffect => {}
            }
        }
        // The club's memory of this opponent.
        let mem = w.tactics.memory.get(&(club, opp)).copied();
        let seen = tactics[1 - i];
        let author = crate::coach::note_taker(w, club);
        let next = OppMemory {
            last: today,
            meetings: mem.map_or(1, |x| x.meetings.saturating_add(1)),
            press: mem.map_or(seen.press, |x| ((u16::from(x.press) + u16::from(seen.press)) / 2) as u8),
            tempo: mem.map_or(seen.tempo, |x| ((u16::from(x.tempo) + u16::from(seen.tempo)) / 2) as u8),
            direct: mem.map_or(seen.directness, |x| ((u16::from(x.direct) + u16::from(seen.directness)) / 2) as u8),
            formation: seen.formation,
            hurt_by: hurt.or(brain.worst_sign).or(mem.and_then(|x| x.hurt_by)),
            worked: worked.or(mem.and_then(|x| x.worked)),
            failed: failed.or(mem.and_then(|x| x.failed)),
            author,
        };
        w.tactics.memory.insert((club, opp), next);
        // Traces, and the public trace of an important change.
        for tr in recs {
            if tr.response.is_structural() || tr.half_time {
                let causes = pw_world::causes![pw_world::Cause::Fact(pw_world::Fact::Played { fixture: fx.uid })];
                w.events.push_caused(
                    today,
                    Visibility::Public,
                    EventKind::MatchTacticsChanged { club, manager: tr.manager, minute: tr.minute, half_time: tr.half_time, response: tr.response, believed: tr.believed },
                    causes,
                );
            }
            w.tactics.keep(tr);
        }
    }
}

fn dummy_trace() -> Trace {
    Trace {
        uid: 0,
        date: pw_core::Date::default(),
        club: ClubId::NONE,
        manager: StaffId::NONE,
        minute: 0,
        half_time: false,
        score: (0, 0),
        sign: SignKind::ChancesAgainst,
        believed: Diagnosis::Unlucky,
        confidence: 0,
        rejected: None,
        response: Response::Hold,
        about: PlayerId::NONE,
        wanted_minutes: 0,
        had_minutes: 0,
        saw_risk: false,
        took_risk: false,
        reacting: false,
        executed: 0,
        backed_by: SmallVec::new(),
        against: SmallVec::new(),
        by_process: None,
        by_result: None,
    }
}

// ------------------------------------------------------------------ over the weeks

/// Weekly: squads rehearse. Their manager's own way of playing gets sharper, the variations he practises grow, what is never used
/// fades; a new manager finds a squad that knows someone else's system (7.10-7.11).
pub fn weekly(w: &mut World) {
    let today = w.date;
    let clubs: Vec<ClubId> = w.clubs.iter_enumerated().filter(|(_, c)| c.manager.is_some()).map(|(id, _)| id).collect();
    for club in clubs {
        let Some((m, prof)) = profile(w, club) else { continue };
        let phil = w.staff[m].philosophy;
        let home = Tactics { mentality: Mentality::from_level(i32::from(phil.mentality)), tempo: phil.tempo, directness: phil.directness, press: phil.press, ..Tactics::default() };
        let main = style_of(&home);
        // The manager's second string: what he would turn to when the plan is not working.
        let second = match phil.archetype {
            Archetype::Loyalist => main,
            _ if prof.adapt > 0.55 => (main + 3) % N_STYLES,
            _ => (main + 5) % N_STYLES,
        };
        let variety = 1.0 + 3.0 * prof.adapt * prof.prep;
        let d = w.tactics.drill.entry(club).or_insert(Drill { manager: m, fam: [25; N_STYLES], since: today });
        if d.manager != m {
            // A new man, a new system: the squad starts nearly from scratch in his way, and keeps some of the old.
            for (i, f) in d.fam.iter_mut().enumerate() {
                *f = if i == main { (*f).min(35) } else { (*f as f32 * 0.6) as u8 };
            }
            d.manager = m;
            d.since = today;
        }
        for (i, f) in d.fam.iter_mut().enumerate() {
            let v = f32::from(*f);
            let v = if i == main {
                v + 5.0
            } else if i == second {
                v + variety
            } else {
                v - 1.2
            };
            *f = v.clamp(5.0, 100.0) as u8;
        }
    }
}

/// Yearly: memories of opponents not met for a long time are dropped, and the book of decisions is trimmed by its cap.
pub fn yearly(w: &mut World) {
    let today = w.date;
    w.tactics.memory.retain(|_, m| m.last.days_until(today) < 800);
    w.tactics.styles.retain(|_, v| v.back().is_some_and(|r| r.date.days_until(today) < 400));
    w.tactics.credit.retain(|&(m, s), _| w.staff.get(m).is_some_and(|x| !x.retired) && w.staff.get(s).is_some_and(|x| !x.retired));
}

/// What history supports saying of a manager as a tactician (7.28). Nothing is assigned: each claim needs the record for it.
pub fn reputation(w: &World, m: StaffId) -> Vec<&'static str> {
    let Some(r) = w.tactics.record_of(m) else { return Vec::new() };
    let mut out = Vec::new();
    if r.changes >= 10 && r.good_reads * 2 >= r.changes && r.comebacks >= 3 {
        out.push("a brilliant in-game manager");
    }
    if r.stood_pat_behind >= 8 && u32::from(r.changes) * 3 <= u32::from(r.stood_pat_behind) {
        out.push("has no Plan B");
    }
    if r.changes >= 10 && r.misreads > r.good_reads {
        out.push("often reads it wrong");
    }
    if r.collapses >= 5 && r.collapses * 2 >= r.comebacks.max(1) * 3 {
        out.push("lets leads slip");
    }
    out
}

/// The clock and the seed a match's tactical randomness hangs on.
pub fn key(w: &World, uid: u64, side: usize, minute: u8, salt: u64) -> u64 {
    hash_key(&[w.seed, stream::TACTICS, uid, side as u64, u64::from(minute), salt])
}

/// A short account of a decision: why the shape changed, what he thought was wrong, what he hoped for (7.34).
pub fn explain(t: &Trace) -> String {
    let when = if t.half_time {
        "At half-time".to_string()
    } else if t.minute == 0 {
        "Before kick-off".to_string()
    } else {
        format!("In minute {}", t.minute)
    };
    let mut s = format!("{when} the manager {}: the staff had seen that {}, and took it to mean that {}", t.response.label(), t.sign.label(), t.believed.label());
    if let Some(r) = t.rejected {
        s.push_str(&format!(" (the alternative was that {})", r.label()));
    }
    if t.response.is_structural() {
        s.push_str(&format!(", hoping to ease it ({}% sure)", t.confidence));
    }
    if matches!(t.response, Response::GoForIt | Response::PressHigher) {
        if t.saw_risk && t.took_risk {
            s.push_str("; the risk it carried was seen and accepted");
        } else if !t.saw_risk {
            s.push_str("; the danger in it was not seen");
        }
    }
    if t.executed < 60 && t.response.is_structural() {
        s.push_str("; the players carried out only part of it");
    }
    s.push('.');
    s
}
