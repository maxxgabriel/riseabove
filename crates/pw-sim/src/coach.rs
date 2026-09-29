//! The people around a live match: observe -> diagnose -> adapt (locked design 7.1-7.22, 7.33-7.34).
//!
//! `MatchCoach` plugs into the match engine (`pw_match::Coach`). At its windows each side is handed football evidence (a `Look`), never
//! engine parameters. Staff see it through their own eyes and may disagree with each other and with the manager; the manager notices
//! some of it, explains it (several explanations, one chosen, one rejected), waits for as much evidence as his temperament asks for
//! (less at half-time, less if he is reacting to a change the other side just made, less when the scoreline presses), and answers.
//! What he decided, and why, is kept with the match for afterwards (`tactics::settle`).

use pw_core::rng::Rng;
use pw_core::{ClubId, PlayerId, Pos, PosGroup, Role, StaffId, Tactics};
use pw_match::{Call, Coach, Look, Mind, SideLook, SubCall};
use pw_world::tactics::*;
use pw_world::{Fixture, FxHashMap, StaffRole, World};
use smallvec::SmallVec;

use crate::selection::Selection;
use crate::tactics::{MatchCtx, Profile, blend, choose_response, execution, shifted};

/// One piece of football evidence and how strong it is, 0..1.
#[derive(Clone, Copy, Debug)]
pub struct Sign {
    pub kind: SignKind,
    pub who: PlayerId,
    pub strength: f32,
}

/// What the football evidence says for a side. This is the truth of what happened, before anyone's eyes are put to it.
pub fn signs(look: &Look, s: usize) -> SmallVec<[Sign; 7]> {
    let (me, op) = (&look.sides[s], &look.sides[1 - s]);
    let minute = f32::from(look.minute.max(1));
    let mut out: SmallVec<[Sign; 7]> = SmallVec::new();
    let (mine, theirs) = (f32::from(me.shots) + 2.0 * f32::from(me.big_chances), f32::from(op.shots) + 2.0 * f32::from(op.big_chances));
    if theirs - mine >= 3.0 {
        out.push(Sign { kind: SignKind::ChancesAgainst, who: PlayerId::NONE, strength: ((theirs - mine) / 8.0).clamp(0.2, 1.0) });
    }
    if look.minute >= 25 && mine <= 1.0 + minute / 60.0 {
        out.push(Sign { kind: SignKind::NoChances, who: PlayerId::NONE, strength: (1.0 - mine / (2.0 + minute / 30.0)).clamp(0.2, 1.0) });
    }
    if me.possession <= 43 && look.minute >= 20 {
        out.push(Sign { kind: SignKind::ControlLost, who: PlayerId::NONE, strength: ((46.0 - f32::from(me.possession)) / 12.0).clamp(0.2, 1.0) });
    }
    if me.lost_own_third >= 3 {
        out.push(Sign { kind: SignKind::BuildUpBroken, who: PlayerId::NONE, strength: (f32::from(me.lost_own_third) / 6.0).clamp(0.25, 1.0) });
    }
    let beaten = me.players.iter().filter(|p| p.pos.group() != PosGroup::Gk).map(|p| (p, f32::from(p.tally.duels_lost) + 0.5 * f32::from(p.tally.fouls) - 0.5 * f32::from(p.tally.duels_won))).max_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((p, score)) = beaten
        && score >= 2.0
    {
        out.push(Sign { kind: SignKind::BeatenMan, who: p.player, strength: (score / 4.0).clamp(0.25, 1.0) });
    }
    let field: SmallVec<[f32; 11]> = me.players.iter().filter(|p| p.pos.group() != PosGroup::Gk).map(|p| f32::from(p.condition)).collect();
    if !field.is_empty() && look.minute >= 40 {
        let mean = field.iter().sum::<f32>() / field.len() as f32;
        if mean < 72.0 {
            out.push(Sign { kind: SignKind::Fading, who: PlayerId::NONE, strength: ((72.0 - mean) / 25.0).clamp(0.2, 1.0) });
        }
    }
    if look.minute >= 55
        && let Some(p) = me.players.iter().filter(|p| p.booked && p.pos.group() != PosGroup::Gk && p.pos.group() != PosGroup::Att && (p.tally.fouls >= 2 || (p.tally.fouls >= 1 && p.tally.duels_lost >= 2))).max_by_key(|p| (p.tally.fouls, p.tally.duels_lost))
    {
        out.push(Sign { kind: SignKind::Booked, who: p.player, strength: (0.5 + 0.15 * f32::from(p.tally.fouls)).clamp(0.4, 1.0) });
    }
    out
}

/// A cumulative count (higher is worse for the side) of the thing a decision was meant to fix, at a look: for judging afterwards
/// whether the problem was easing. A man taken off for "form" is judged on the side's chances against, since the leak may be structural.
pub fn measure(tr: &Trace, look: &Look, side: usize) -> f32 {
    let (s, o) = (&look.sides[side], &look.sides[1 - side]);
    let kind = match (tr.response, tr.sign, tr.believed) {
        (Response::ReplacePlayer, _, _) => SignKind::ChancesAgainst,
        (_, SignKind::Scoreline, Diagnosis::NeedAGoal) => SignKind::NoChances,
        (_, SignKind::Scoreline, _) => SignKind::ChancesAgainst,
        (_, k, _) => k,
    };
    match kind {
        SignKind::ChancesAgainst => f32::from(o.shots) + 2.0 * f32::from(o.big_chances),
        SignKind::NoChances => -(f32::from(s.shots) + 2.0 * f32::from(s.big_chances)),
        SignKind::ControlLost => f32::from(o.territory) - f32::from(s.territory),
        SignKind::BuildUpBroken => f32::from(s.lost_own_third),
        SignKind::BeatenMan => s.players.iter().find(|p| p.player == tr.about).map_or(0.0, |p| f32::from(p.tally.duels_lost) + 0.5 * f32::from(p.tally.fouls)),
        SignKind::Fading => {
            let f: SmallVec<[f32; 11]> = s.players.iter().filter(|p| p.pos.group() != PosGroup::Gk).map(|p| f32::from(p.condition)).collect();
            if f.is_empty() { 0.0 } else { 100.0 - f.iter().sum::<f32>() / f.len() as f32 }
        }
        SignKind::Booked | SignKind::Scoreline => f32::from(s.reds) * 3.0 + f32::from(s.fouls) * 0.2,
    }
}

// ------------------------------------------------------------------ staff

/// A member of the staff who watches the game and speaks up.
#[derive(Clone, Copy, Debug)]
struct Observer {
    staff: StaffId,
    role: StaffRole,
    acuity: f32,
    trust: f32,
    credit: f32,
}

#[derive(Clone, Copy, Debug)]
struct Heard {
    staff: StaffId,
    sign: SignKind,
    who: PlayerId,
    diag: Diagnosis,
    weight: f32,
}

/// The tip recorded when the manager did not act on it.
#[derive(Clone, Copy, Debug)]
pub struct Tip {
    pub staff: StaffId,
    pub sign: SignKind,
    pub who: PlayerId,
    pub window: usize,
}

pub struct LiveTrace {
    pub trace: Trace,
    /// Index of the look (in `Logs::hist`) at which he decided.
    pub window: usize,
}

/// One side's thinking during a match.
pub struct Brain {
    pub manager: StaffId,
    pub profile: Profile,
    club: ClubId,
    opp: ClubId,
    observers: SmallVec<[Observer; 3]>,
    first_seen: [Option<u8>; 8],
    pub traces: Vec<LiveTrace>,
    pub unheeded: Vec<Tip>,
    pub trailing_at_ht: bool,
    pub leading_at_ht: bool,
    pub stood_pat_behind: bool,
    pub worst_sign: Option<SignKind>,
    worst_strength: f32,
    last_style: Option<Tactics>,
}

/// What is left of a match's thinking once it is over.
pub struct Logs {
    pub brains: [Option<Brain>; 2],
    pub hist: Vec<Look>,
    pub full_time: Option<Look>,
}

/// Whoever compiles the club's notes on an opponent: the best analyst, else the best assistant, else the manager.
pub fn note_taker(w: &World, club: ClubId) -> StaffId {
    let best = |role: StaffRole| w.clubs[club].staff.iter().copied().filter(|&s| w.staff[s].role == role && !w.staff[s].retired).max_by(|&a, &b| w.staff[a].role_rating(role).total_cmp(&w.staff[b].role_rating(role)).then(b.cmp(&a)));
    best(StaffRole::Analyst).or_else(|| best(StaffRole::Assistant)).unwrap_or(w.clubs[club].manager)
}

fn observers_of(w: &World, club: ClubId, manager: StaffId) -> SmallVec<[Observer; 3]> {
    let mgr_person = w.staff[manager].person;
    let mut out = SmallVec::new();
    for role in [StaffRole::Assistant, StaffRole::Analyst, StaffRole::FitnessCoach] {
        let pick = w.clubs[club].staff.iter().copied().filter(|&s| s != manager && w.staff[s].role == role && !w.staff[s].retired).max_by(|&a, &b| w.staff[a].role_rating(role).total_cmp(&w.staff[b].role_rating(role)).then(b.cmp(&a)));
        if let Some(s) = pick {
            let st = &w.staff[s];
            out.push(Observer {
                staff: s,
                role,
                acuity: (st.role_rating(role) / 20.0).clamp(0.05, 1.0),
                trust: crate::consider::trust(w, mgr_person, st.person),
                credit: w.tactics.credit.get(&(manager, s)).map_or(0.5, |c| c.standing()),
            });
        }
    }
    out
}

/// How well an observer reads one kind of sign: analysts are better with numbers than with a man's bad day; the fitness coach sees legs.
fn acuity_for(o: &Observer, kind: SignKind) -> Option<f32> {
    let a = o.acuity;
    match (o.role, kind) {
        (StaffRole::FitnessCoach, SignKind::Fading | SignKind::Booked) => Some(a),
        (StaffRole::FitnessCoach, _) => None,
        (StaffRole::Analyst, SignKind::ChancesAgainst | SignKind::ControlLost | SignKind::BuildUpBroken | SignKind::NoChances) => Some((a + 0.1).min(1.0)),
        (StaffRole::Analyst, SignKind::BeatenMan | SignKind::Fading) => Some((a - 0.15).max(0.05)),
        (_, SignKind::Scoreline) => None,
        _ => Some(a),
    }
}

/// How much of each explanation the evidence supports before anyone's judgement: the priors on why a sign is there.
fn diag_weights(kind: SignKind, s: &SideLook, o: &SideLook, gap: f32, beaten_man: bool) -> SmallVec<[(Diagnosis, f32); 6]> {
    use Diagnosis as D;
    let attacking = if s.tactics.mentality >= pw_core::Mentality::Positive { 0.3 } else { 0.0 };
    let pressing = if s.tactics.press >= 60 { 0.3 } else { 0.0 };
    let deep = if s.tactics.mentality <= pw_core::Mentality::Defensive { 0.3 } else { 0.0 };
    let mut v: SmallVec<[(Diagnosis, f32); 6]> = SmallVec::new();
    match kind {
        SignKind::ChancesAgainst => {
            let they_miss = if o.shots >= 6 && o.on_target * 4 <= o.shots { 0.3 } else { 0.0 };
            v.extend([
                (D::TooOpen, 0.35 + attacking + 0.15 * f32::from(s.tactics.line > 60)),
                (D::PressNotWorking, 0.25 + pressing),
                (D::TheyAreBetter, 0.2 + 0.4 * gap.max(0.0)),
                (D::IndividualForm, 0.2 + if beaten_man { 0.4 } else { 0.0 }),
                (D::Unlucky, 0.12 + they_miss),
            ]);
        }
        SignKind::NoChances => v.extend([(D::TooCautious, 0.35 + deep), (D::TheyAreBetter, 0.25 + 0.4 * gap.max(0.0)), (D::PressNotWorking, 0.15), (D::Unlucky, 0.15 + if s.shots >= 5 { 0.25 } else { 0.0 })]),
        SignKind::ControlLost => v.extend([(D::PressNotWorking, 0.3 + pressing), (D::TheyAreBetter, 0.3 + 0.3 * gap.max(0.0)), (D::TooOpen, 0.2 + attacking), (D::Fatigue, 0.1)]),
        SignKind::BuildUpBroken => v.extend([(D::TheyAreBetter, 0.3 + 0.3 * gap.max(0.0)), (D::IndividualForm, 0.25 + if beaten_man { 0.3 } else { 0.0 }), (D::TooOpen, 0.15), (D::Fatigue, 0.1)]),
        SignKind::BeatenMan => v.extend([(D::IndividualForm, 0.45), (D::TooOpen, 0.3 + attacking), (D::Fatigue, 0.2)]),
        SignKind::Fading => v.push((D::Fatigue, 1.0)),
        SignKind::Booked => v.push((D::CardRisk, 1.0)),
        SignKind::Scoreline => {}
    }
    v
}

// ------------------------------------------------------------------ the coach

pub struct MatchCoach<'w> {
    w: &'w World,
    uid: u64,
    ctx: MatchCtx,
    brains: [Option<Brain>; 2],
    minds: FxHashMap<PlayerId, Mind>,
    hist: Vec<Look>,
    full: Option<Look>,
    /// Which side each player is on, for the minds.
    sides: FxHashMap<PlayerId, u8>,
}

impl<'w> MatchCoach<'w> {
    pub fn new(w: &'w World, fx: &Fixture, ctx: MatchCtx, sels: [&Selection; 2], minds: FxHashMap<PlayerId, Mind>) -> Self {
        let clubs = [w.teams[sels[0].team].club, w.teams[sels[1].team].club];
        let brain = |i: usize| {
            let (m, profile) = crate::tactics::profile(w, clubs[i])?;
            Some(Brain {
                manager: m,
                profile,
                club: clubs[i],
                opp: clubs[1 - i],
                observers: observers_of(w, clubs[i], m),
                first_seen: [None; 8],
                traces: Vec::new(),
                unheeded: Vec::new(),
                trailing_at_ht: false,
                leading_at_ht: false,
                stood_pat_behind: false,
                worst_sign: None,
                worst_strength: 0.0,
                last_style: None,
            })
        };
        let mut sides = FxHashMap::default();
        for (i, sel) in sels.iter().enumerate() {
            for &p in sel.xi.iter().chain(sel.bench.iter()) {
                sides.insert(p, i as u8);
            }
        }
        Self { w, uid: fx.uid, ctx, brains: [brain(0), brain(1)], minds, hist: Vec::new(), full: None, sides }
    }

    pub fn into_logs(self) -> Logs {
        Logs { brains: self.brains, hist: self.hist, full_time: self.full }
    }

    fn think(&mut self, s: usize, look: &Look, window: usize, prev: Option<&Look>) -> Call {
        let w = self.w;
        let Some(mut brain) = self.brains[s].take() else { return Call::default() };
        let call = brain.think(w, self.uid, &self.ctx, s, look, window, prev);
        self.brains[s] = Some(brain);
        call
    }
}

impl Coach for MatchCoach<'_> {
    fn mind(&self, side: u8, player: PlayerId) -> Mind {
        if self.sides.get(&player).is_some_and(|&s| s != side) {
            return Mind::NEUTRAL;
        }
        self.minds.get(&player).copied().unwrap_or(Mind::NEUTRAL)
    }

    fn call(&mut self, look: &Look) -> [Call; 2] {
        let window = self.hist.len();
        let prev = self.hist.last().cloned();
        self.hist.push(look.clone());
        let a = self.think(0, look, window, prev.as_ref());
        let b = self.think(1, look, window, prev.as_ref());
        [a, b]
    }

    fn applied(&mut self, side: u8, _call: &Call, made: &[bool]) {
        // A substitution the engine refused was never made: the decision that wanted it did not happen.
        if made.contains(&false)
            && let Some(b) = self.brains[usize::from(side)].as_mut()
            && let Some(t) = b.traces.last_mut()
            && matches!(t.trace.response, Response::ReplacePlayer | Response::Refresh)
        {
            t.trace.executed = 0;
        }
    }

    fn full_time(&mut self, look: &Look) {
        self.full = Some(look.clone());
    }
}

impl Brain {
    #[allow(clippy::too_many_arguments)]
    fn think(&mut self, w: &World, uid: u64, ctx: &MatchCtx, s: usize, look: &Look, window: usize, prev: Option<&Look>) -> Call {
        let prof = self.profile;
        let me = &look.sides[s];
        let op = &look.sides[1 - s];
        let minute = look.minute;
        let half = look.half_time;
        let opp_club = Some(self.opp);
        let mut rng = Rng::keyed(&[crate::tactics::key(w, uid, s, minute, 0xc0ac)]);
        if half {
            let m = ctx.on_day(s, look.goals);
            self.trailing_at_ht = m < 0;
            self.leading_at_ht = m > 0;
        }
        // What the other side just did.
        let reacting = prev.is_some_and(|p| {
            let (a, b) = (p.sides[1 - s].tactics, op.tactics);
            style_of(&a) != style_of(&b) || a.mentality != b.mentality || a.press.abs_diff(b.press) >= 15 || a.directness.abs_diff(b.directness) >= 15
        }) && rng.chance(prof.reading);

        // Evidence, as the manager sees it: he misses some of it, the more so the worse he reads a game.
        let truth = signs(look, s);
        let mut seen: SmallVec<[Sign; 7]> = SmallVec::new();
        for t in &truth {
            let perceived = t.strength + rng.normal() * (1.0 - prof.reading) * 0.45;
            if perceived >= 0.28 {
                seen.push(Sign { strength: perceived.clamp(0.0, 1.0), ..*t });
            }
            if t.strength > self.worst_strength {
                self.worst_strength = t.strength;
                self.worst_sign = Some(t.kind);
            }
        }
        for k in SignKind::ALL {
            let i = k as usize;
            if seen.iter().any(|x| x.kind == k) {
                self.first_seen[i].get_or_insert(minute);
            } else {
                self.first_seen[i] = None;
            }
        }
        seen.sort_by(|a, b| b.strength.total_cmp(&a.strength).then((a.kind as usize).cmp(&(b.kind as usize))));

        // What the staff say, from where they sit.
        let gap = ctx.opposition[s];
        let beaten = truth.iter().any(|t| t.kind == SignKind::BeatenMan);
        let mut heard: SmallVec<[Heard; 4]> = SmallVec::new();
        for o in &self.observers {
            let mut best: Option<(f32, Sign)> = None;
            for t in &truth {
                let Some(a) = acuity_for(o, t.kind) else { continue };
                let mut r = Rng::keyed(&[crate::tactics::key(w, uid, s, minute, u64::from(o.staff.0) * 16 + t.kind as u64)]);
                let perceived = t.strength + r.normal() * (1.0 - a) * 0.6;
                if perceived >= 0.3 && best.is_none_or(|b| perceived > b.0) {
                    best = Some((perceived, *t));
                }
            }
            let Some((_, t)) = best else { continue };
            let mut r = Rng::keyed(&[crate::tactics::key(w, uid, s, minute, 0x71b + u64::from(o.staff.0))]);
            let ph = w.staff[o.staff].philosophy;
            let weights = diag_weights(t.kind, me, op, gap, beaten);
            let pick = weights.iter().map(|&(d, x)| {
                // Their own way of seeing football: a pressing coach blames the press last and the caution first.
                let lean = match d {
                    Diagnosis::PressNotWorking => (f32::from(ph.press) - 50.0) / 250.0,
                    Diagnosis::TooCautious => (f32::from(ph.press) - 50.0) / 300.0,
                    Diagnosis::TooOpen => (50.0 - f32::from(ph.press)) / 300.0,
                    _ => 0.0,
                };
                (d, x + lean + r.normal() * (1.0 - o.acuity) * 0.3)
            });
            if let Some((d, _)) = pick.max_by(|a, b| a.1.total_cmp(&b.1)) {
                let weight = o.acuity * (0.4 + o.trust) * (o.credit * 2.0) * (1.0 - 0.6 * prof.ego);
                heard.push(Heard { staff: o.staff, sign: t.kind, who: t.who, diag: d, weight });
            }
        }

        let mut call = Call::default();
        let mut acted: SmallVec<[SignKind; 2]> = SmallVec::new();
        let mut structural_done = false;
        let mut bench_used: SmallVec<[PlayerId; 3]> = SmallVec::new();
        let mut out_used: SmallVec<[PlayerId; 3]> = SmallVec::new();

        // The scoreline may call for something on its own.
        let urgency = ctx.urgency(s, look.goals, minute);
        let gate = 72.0 - 14.0 * prof.daring - 10.0 * urgency.abs() + prof.stubborn * 6.0;
        let scoreline = (urgency.abs() >= 0.2 && (half || f32::from(minute) >= gate)).then_some(Sign { kind: SignKind::Scoreline, who: PlayerId::NONE, strength: urgency.abs() });
        let mut candidates: SmallVec<[Sign; 8]> = SmallVec::new();
        if let Some(sc) = scoreline {
            candidates.push(sc);
        }
        candidates.extend(seen.iter().copied());

        for sign in candidates {
            if acted.len() >= 2 {
                break;
            }
            let kind = sign.kind;
            let first = self.first_seen[kind as usize].unwrap_or(minute);
            let mut had = minute.saturating_sub(first);
            if kind == SignKind::Scoreline {
                // The scoreline is not evidence that builds up: the clock gate above is its threshold.
                had = 255;
            }
            if half && kind != SignKind::Scoreline {
                // More evidence: the whole half is in front of him.
                had = had.max(minute.min(30));
            }
            let quick = matches!(kind, SignKind::Fading | SignKind::Booked | SignKind::Scoreline);
            let mut wanted = prof.patience * (1.15 - 0.5 * sign.strength) * (1.0 - 0.5 * urgency.abs());
            if half {
                wanted *= 0.6;
            }
            if reacting {
                wanted *= 0.4;
            }
            if quick {
                wanted *= 0.3;
            }
            if sign.strength >= 0.9 {
                wanted *= 0.5;
            }
            if f32::from(had) < wanted {
                continue;
            }

            // Explanations: what the evidence supports, his own noise, and what the staff said.
            let mut cands: SmallVec<[(Diagnosis, f32); 6]> = if kind == SignKind::Scoreline {
                if urgency > 0.0 { smallvec::smallvec![(Diagnosis::NeedAGoal, 1.0)] } else { smallvec::smallvec![(Diagnosis::ProtectingALead, 1.0)] }
            } else {
                diag_weights(kind, me, op, gap, beaten)
            };
            for c in cands.iter_mut() {
                c.1 += rng.normal() * (1.0 - prof.reading) * 0.35;
                for h in heard.iter().filter(|h| h.sign == kind && h.diag == c.0) {
                    c.1 += h.weight * 1.6;
                }
            }
            cands.sort_by(|a, b| b.1.total_cmp(&a.1));
            let (believed, top) = cands[0];
            let rejected = cands.get(1).map(|c| c.0);
            let total: f32 = cands.iter().map(|c| c.1.max(0.05)).sum();
            let confidence = (100.0 * top.max(0.05) / total).clamp(25.0, 95.0) as u8;

            let backed: SmallVec<[StaffId; 2]> = heard.iter().filter(|h| h.sign == kind && h.diag == believed && h.weight >= 0.15).map(|h| h.staff).collect();
            let against: SmallVec<[StaffId; 2]> = heard.iter().filter(|h| h.sign == kind && h.diag != believed && h.weight >= 0.3).map(|h| h.staff).collect();

            let base = me.tactics;
            let (mgr, club) = (self.manager, self.club);
            let (response, _) = choose_response(w, club, mgr, &prof, base, believed, urgency, opp_club, &mut rng);
            if response == Response::Hold {
                if !against.is_empty() || !backed.is_empty() {
                    self.traces.push(LiveTrace { trace: make_trace(w, uid, club, mgr, look, s, sign, believed, confidence, rejected, response, wanted, had, false, false, reacting, 0, backed, against), window });
                    acted.push(kind);
                }
                continue;
            }
            let mut executed = 100u8;
            let mut saw_risk = false;
            let mut took_risk = false;
            // Blaming a man for a leak in the team means pointing at a man: the one he saw being beaten.
            let about = if sign.who.is_some() { sign.who } else { seen.iter().find(|x| x.kind == SignKind::BeatenMan).map_or(PlayerId::NONE, |x| x.who) };
            match response {
                Response::Compact | Response::GoForIt | Response::PressHigher | Response::SitDeep | Response::PlayThrough | Response::GoDirect => {
                    if structural_done {
                        continue;
                    }
                    let cur = call.tactics.unwrap_or(base);
                    let target = shifted(cur, response);
                    let xi: SmallVec<[PlayerId; 11]> = me.players.iter().map(|p| p.player).collect();
                    let share = execution(w, club, &xi, &target, &prof, minute, half);
                    let mut next = blend(cur, target, share);
                    // Half-understood instructions go wrong in their own way (7.32): no defiance, just confusion.
                    if share < 0.6 && rng.chance((0.6 - share) * 0.6) {
                        let d = if rng.chance(0.5) { 15 } else { -15 };
                        if rng.chance(0.5) {
                            next.press = (i32::from(next.press) + d).clamp(0, 100) as u8;
                        } else {
                            next.line = (i32::from(next.line) + d).clamp(0, 100) as u8;
                        }
                    }
                    executed = (share * 100.0) as u8;
                    // He may see that pushing up leaves the back open, and do it anyway because the game needs it.
                    if matches!(response, Response::GoForIt | Response::PressHigher) {
                        saw_risk = rng.f32() < prof.reading;
                        took_risk = saw_risk;
                    }
                    call.tactics = Some(next);
                    structural_done = true;
                    // The scoreline's answers come with a change of personnel late on.
                    if kind == SignKind::Scoreline && minute >= 55 && me.subs_made < me.subs_max {
                        let want_att = response == Response::GoForIt || response == Response::GoDirect;
                        if let Some(sub) = intent_sub(w, club, me, want_att, &bench_used, &out_used) {
                            bench_used.push(sub.inn);
                            out_used.push(sub.out);
                            call.subs.push(sub);
                        }
                    }
                }
                Response::ReplacePlayer => {
                    let Some(p) = me.players.iter().find(|p| p.player == about) else { continue };
                    if me.subs_made >= me.subs_max || out_used.contains(&about) {
                        continue;
                    }
                    let Some(inn) = pick_bench(w, club, me, Some(p.pos.group()), &bench_used) else { continue };
                    call.subs.push(SubCall { out: about, inn });
                    bench_used.push(inn);
                    out_used.push(about);
                }
                Response::Refresh => {
                    let tired = me.players.iter().filter(|p| p.pos.group() != PosGroup::Gk && !out_used.contains(&p.player)).min_by_key(|p| p.condition);
                    let Some(p) = tired else { continue };
                    if me.subs_made >= me.subs_max {
                        continue;
                    }
                    let Some(inn) = pick_bench(w, club, me, Some(p.pos.group()), &bench_used) else { continue };
                    call.subs.push(SubCall { out: p.player, inn });
                    bench_used.push(inn);
                    out_used.push(p.player);
                }
                Response::CoverBooked => {
                    let Some(p) = me.players.iter().find(|p| p.player == about) else { continue };
                    let safer = match p.pos.group() {
                        PosGroup::Def => Some(if matches!(p.pos, Pos::DC) { Role::CentreBack } else { Role::FullBack }),
                        PosGroup::Mid => Some(Role::Anchor),
                        _ => None,
                    };
                    match safer {
                        Some(role) if role != p.role => call.roles.push((about, role)),
                        _ => {
                            if me.subs_made >= me.subs_max {
                                continue;
                            }
                            let Some(inn) = pick_bench(w, club, me, Some(p.pos.group()), &bench_used) else { continue };
                            call.subs.push(SubCall { out: about, inn });
                            bench_used.push(inn);
                            out_used.push(about);
                        }
                    }
                }
                Response::Hold => {}
            }
            self.traces.push(LiveTrace { trace: make_trace(w, uid, club, mgr, look, s, sign, believed, confidence, rejected, response, wanted, had, saw_risk, took_risk, reacting, executed, backed, against), window });
            acted.push(kind);
        }
        // Tips that were not acted on stay on the record: whoever was right will be known afterwards.
        for h in &heard {
            if h.weight >= 0.2 && !acted.contains(&h.sign) {
                self.unheeded.push(Tip { staff: h.staff, sign: h.sign, who: h.who, window });
            }
        }
        // Behind at the hour and did nothing: the raw material of "no Plan B".
        if !half && (55..=65).contains(&minute) && ctx.on_day(s, look.goals) < 0 && !structural_done && call.subs.is_empty() {
            self.stood_pat_behind = true;
        }
        if let Some(t) = call.tactics {
            self.last_style = Some(t);
        }
        call
    }
}

#[allow(clippy::too_many_arguments)]
fn make_trace(
    w: &World,
    uid: u64,
    club: ClubId,
    manager: StaffId,
    look: &Look,
    s: usize,
    sign: Sign,
    believed: Diagnosis,
    confidence: u8,
    rejected: Option<Diagnosis>,
    response: Response,
    wanted: f32,
    had: u8,
    saw_risk: bool,
    took_risk: bool,
    reacting: bool,
    executed: u8,
    backed_by: SmallVec<[StaffId; 2]>,
    against: SmallVec<[StaffId; 2]>,
) -> Trace {
    Trace {
        uid,
        date: w.date,
        club,
        manager,
        minute: look.minute,
        half_time: look.half_time,
        score: (look.goals[s], look.goals[1 - s]),
        sign: sign.kind,
        believed,
        confidence,
        rejected,
        response,
        about: sign.who,
        wanted_minutes: wanted.round().clamp(0.0, 255.0) as u8,
        had_minutes: had,
        saw_risk,
        took_risk,
        reacting,
        executed,
        backed_by,
        against,
        by_process: None,
        by_result: None,
    }
}

/// The best bench player the club believes it has for a job, by how it rates him (never his hidden ability).
fn pick_bench(w: &World, club: ClubId, me: &SideLook, group: Option<PosGroup>, used: &[PlayerId]) -> Option<PlayerId> {
    let rate = |p: PlayerId| crate::scouting::view(w, club, p).0;
    let pick = |g: Option<PosGroup>| me.bench.iter().filter(|(p, pos)| !used.contains(p) && pos.group() != PosGroup::Gk && g.is_none_or(|g| pos.group() == g)).max_by(|a, b| rate(a.0).total_cmp(&rate(b.0)).then(b.0.cmp(&a.0))).map(|x| x.0);
    pick(group).or_else(|| pick(None))
}

/// A substitution the scoreline asks for: an attacker for a defender or midfielder when chasing, a defender for a forward when guarding.
fn intent_sub(w: &World, club: ClubId, me: &SideLook, chase: bool, bench_used: &[PlayerId], out_used: &[PlayerId]) -> Option<SubCall> {
    let defenders = me.players.iter().filter(|p| p.pos.group() == PosGroup::Def).count();
    let (out_groups, in_group): (&[PosGroup], PosGroup) = if chase {
        if defenders >= 4 { (&[PosGroup::Def, PosGroup::Mid], PosGroup::Att) } else { (&[PosGroup::Mid], PosGroup::Att) }
    } else {
        (&[PosGroup::Att], PosGroup::Def)
    };
    let out = me.players.iter().filter(|p| out_groups.contains(&p.pos.group()) && !out_used.contains(&p.player)).min_by_key(|p| p.condition)?;
    let inn = pick_bench(w, club, me, Some(in_group), bench_used)?;
    Some(SubCall { out: out.player, inn })
}
