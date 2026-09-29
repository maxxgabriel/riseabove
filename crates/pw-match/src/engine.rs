use std::sync::OnceLock;

use pw_core::attr::AttrGroup;
use pw_core::math::{exp, sigmoid};
use pw_core::rng::Rng;
use pw_core::{Attr, Hidden, Mentality, PlayerId, PlayerTraits, Pos, PosGroup, Role, Slot, Tactics};
use smallvec::SmallVec;

use crate::pitch::*;
use crate::types::*;

const NONE: u8 = u8::MAX;

#[cfg(test)]
pub(crate) mod diag {
    use std::sync::atomic::{AtomicU64, Ordering};
    /// 0 pass-short 1 medium 2 long 3 through 4 cross 5 carry-free 6 carry-won 7 restart
    pub static ROW5: [AtomicU64; 8] = [const { AtomicU64::new(0) }; 8];
    /// pass attempts by kind, and completions by kind
    pub static TRIED: [AtomicU64; 5] = [const { AtomicU64::new(0) }; 5];
    pub static DONE: [AtomicU64; 5] = [const { AtomicU64::new(0) }; 5];
    pub fn hit(a: &[AtomicU64], i: usize) {
        a[i].fetch_add(1, Ordering::Relaxed);
    }
}
const HALF: f32 = 2700.0;
const EXTRA_HALF: f32 = 900.0;
const CHECKPOINT: f32 = 300.0;

// ------------------------------------------------------------------ skills

type W = &'static [(Attr, f32)];

const PASS_SHORT: W = &[(Attr::Passing, 0.45), (Attr::Technique, 0.2), (Attr::Decisions, 0.15), (Attr::Composure, 0.1), (Attr::FirstTouch, 0.1)];
const PASS_MEDIUM: W = &[(Attr::Passing, 0.45), (Attr::Technique, 0.2), (Attr::Vision, 0.15), (Attr::Decisions, 0.1), (Attr::Composure, 0.1)];
const PASS_LONG: W = &[(Attr::Passing, 0.4), (Attr::Technique, 0.2), (Attr::Vision, 0.25), (Attr::Decisions, 0.15)];
const PASS_GK: W = &[(Attr::Kicking, 0.5), (Attr::Passing, 0.3), (Attr::Vision, 0.2)];
const PASS_THROUGH: W = &[(Attr::Vision, 0.4), (Attr::Passing, 0.35), (Attr::Technique, 0.15), (Attr::Decisions, 0.1)];
const CROSS: W = &[(Attr::Crossing, 0.6), (Attr::Technique, 0.2), (Attr::Vision, 0.1), (Attr::Decisions, 0.1)];
const RECEIVE: W = &[(Attr::FirstTouch, 0.45), (Attr::Technique, 0.2), (Attr::Composure, 0.15), (Attr::Anticipation, 0.2)];
const RECEIVE_RUN: W = &[(Attr::OffTheBall, 0.35), (Attr::Acceleration, 0.25), (Attr::FirstTouch, 0.25), (Attr::Anticipation, 0.15)];
const INTERCEPT: W = &[(Attr::Anticipation, 0.35), (Attr::Positioning, 0.3), (Attr::Marking, 0.15), (Attr::Concentration, 0.1), (Attr::Acceleration, 0.1)];
const DRIBBLE: W = &[(Attr::Dribbling, 0.35), (Attr::Acceleration, 0.2), (Attr::Agility, 0.15), (Attr::Balance, 0.1), (Attr::Technique, 0.1), (Attr::Flair, 0.1)];
const TACKLE: W = &[(Attr::Tackling, 0.35), (Attr::Positioning, 0.2), (Attr::Anticipation, 0.2), (Attr::Strength, 0.1), (Attr::Acceleration, 0.15)];
const AERIAL_ATT: W = &[(Attr::Heading, 0.3), (Attr::JumpingReach, 0.35), (Attr::Strength, 0.15), (Attr::Bravery, 0.1), (Attr::Anticipation, 0.1)];
const AERIAL_DEF: W = &[(Attr::JumpingReach, 0.35), (Attr::Heading, 0.25), (Attr::Marking, 0.2), (Attr::Strength, 0.1), (Attr::Positioning, 0.1)];
const KEEPER: W = &[(Attr::Reflexes, 0.35), (Attr::Handling, 0.2), (Attr::OneOnOnes, 0.2), (Attr::Positioning, 0.15), (Attr::Agility, 0.1)];
const PRESS: W = &[(Attr::WorkRate, 0.4), (Attr::Anticipation, 0.3), (Attr::Acceleration, 0.3)];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PassKind {
    Short,
    Medium,
    Long,
    Through,
    Cross,
}

impl PassKind {
    /// Logit offset for a technical error (misplaced ball); higher = cleaner.
    const fn accuracy(self) -> f32 {
        match self {
            PassKind::Short => 3.3,
            PassKind::Medium => 2.6,
            PassKind::Long => 1.7,
            PassKind::Through => 1.4,
            PassKind::Cross => 1.2,
        }
    }

    /// Logit offset for the passer/receiver pair beating an engaged defender.
    const fn duel(self) -> f32 {
        match self {
            PassKind::Short => 0.8,
            PassKind::Medium => 0.5,
            PassKind::Long => 0.0,
            PassKind::Through => -0.8,
            PassKind::Cross => -0.5,
        }
    }

    const fn duration(self) -> (f32, f32) {
        match self {
            PassKind::Short => (2.0, 3.4),
            PassKind::Medium => (2.8, 4.2),
            PassKind::Long => (3.4, 5.4),
            PassKind::Through => (3.0, 4.8),
            PassKind::Cross => (3.0, 4.2),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Act {
    Pass { slot: u8, zone: u8, kind: PassKind },
    Carry { zone: u8 },
    Shoot,
    Clear,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShotKind {
    Open,
    Header,
    FreeKick,
    Rebound,
}

/// Static pass reach between zones: short passes are easy to find, forward
/// options slightly favoured.
fn reach() -> &'static [[f32; N_ZONES]; N_ZONES] {
    static R: OnceLock<[[f32; N_ZONES]; N_ZONES]> = OnceLock::new();
    R.get_or_init(|| {
        let mut r = [[0.0; N_ZONES]; N_ZONES];
        for (b, row) in r.iter_mut().enumerate() {
            let bx = zone_xy(b).0 as f32;
            for (t, v) in row.iter_mut().enumerate() {
                let d = zone_dist(b, t);
                let fwd = (zone_xy(t).0 as f32 - bx).max(0.0);
                *v = exp(-d * d / (2.0 * 2.3 * 2.3)) * (1.0 + 0.2 * fwd);
            }
        }
        r
    })
}

// ----------------------------------------------------------------- players

struct Mp {
    sheet: PlayerSheet,
    side: u8,
    slot: u8,
    gone: bool,
    cond: f32,
    base: f32,
    phys: f32,
    other: f32,
    yellow: u8,
    va: f32,
    conceded: u8,
    on_t: f32,
    off_t: Option<f32>,
    line: PlayerLine,
    zones: Option<Box<[u16; 30]>>,
}

impl Mp {
    #[inline]
    fn eff(&self, a: Attr) -> f32 {
        let m = if a.group() == AttrGroup::Physical { self.phys } else { self.other };
        self.sheet.attrs.get(a) * m
    }

    #[inline]
    fn sk(&self, w: W) -> f32 {
        let (mut s, mut t) = (0.0, 0.0);
        for &(a, wt) in w {
            s += self.eff(a) * wt;
            t += wt;
        }
        s / t
    }

    #[inline]
    fn on_pitch(&self) -> bool {
        self.slot != NONE && !self.gone
    }

    fn refresh(&mut self) {
        let c = self.cond / 100.0;
        self.phys = self.base * (0.72 + 0.28 * c);
        self.other = self.base * (0.88 + 0.12 * c);
    }

    #[inline]
    fn hidden(&self, h: Hidden) -> f32 {
        self.sheet.hidden.f(h)
    }
}

// ------------------------------------------------------------------ shapes

/// A team's positional distribution for every ball location, stored as
/// separable x/y marginals: occ(ball, zone, slot) = x[ball][slot][zx] · y[ball][slot][zy].
struct Shape {
    x: [[[f32; ZONES_X]; 11]; N_ZONES],
    y: [[[f32; ZONES_Y]; 11]; N_ZONES],
}

impl Shape {
    fn empty() -> Box<Self> {
        Box::new(Self { x: [[[0.0; ZONES_X]; 11]; N_ZONES], y: [[[0.0; ZONES_Y]; 11]; N_ZONES] })
    }

    #[inline]
    fn occ(&self, ball: usize, z: usize, i: usize) -> f32 {
        let (zx, zy) = zone_xy(z);
        self.x[ball][i][zx] * self.y[ball][i][zy]
    }

    fn set(&mut self, ball: usize, i: usize, px: f32, py: f32, sx: f32, sy: f32) {
        fn fill<const N: usize>(out: &mut [f32; N], p: f32, s: f32) {
            let mut sum = 0.0;
            for (k, v) in out.iter_mut().enumerate() {
                let c = (k as f32 + 0.5) / N as f32;
                *v = exp(-(c - p) * (c - p) / (2.0 * s * s));
                sum += *v;
            }
            for v in out.iter_mut() {
                *v /= sum.max(1e-9);
            }
        }
        fill(&mut self.x[ball][i], px, sx);
        fill(&mut self.y[ball][i], py, sy);
    }

    fn clear(&mut self, i: usize) {
        for b in 0..N_ZONES {
            self.x[b][i] = [0.0; ZONES_X];
            self.y[b][i] = [0.0; ZONES_Y];
        }
    }
}

struct Side {
    team: pw_core::TeamId,
    tactics: Tactics,
    mentality: Mentality,
    slots: [Slot; 11],
    on: [u8; 11],
    bench: SmallVec<[u8; 12]>,
    subs_left: u8,
    windows_left: u8,
    reactivity: f32,
    goals: u8,
    stats: TeamStats,
    poss_time: f32,
    last_change: f32,
    att: Box<Shape>,
    def: Box<Shape>,
    /// Best receiving zone and availability per (ball zone, slot).
    target: Box<[[(u8, f32); 11]; N_ZONES]>,
    press_sk: [f32; 11],
    int_sk: [f32; 11],
    outfield: [bool; 11],
}

impl Side {
    fn keeper_slot(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.pos == Pos::GK)
    }
}

// ------------------------------------------------------------------ engine

struct Engine<'a> {
    inp: &'a MatchInput<'a>,
    rng: Rng,
    mp: Vec<Mp>,
    sides: [Side; 2],
    clock: f32,
    poss: usize,
    ball: usize,
    carrier: usize,
    passer: u8,
    carries_since_pass: u8,
    through: bool,
    events: Vec<MatchEvent>,
    full: bool,
    k: f32,
    adv: f32,
    half_events: u32,
    ht: (u8, u8),
    extra_time: bool,
}

pub fn simulate(inp: &MatchInput) -> MatchResult {
    let mut e = Engine::new(inp);
    e.play();
    e.finish()
}

impl<'a> Engine<'a> {
    fn new(inp: &'a MatchInput<'a>) -> Self {
        let mut rng = Rng::new(inp.seed);
        let mut mp = Vec::with_capacity(46);
        let mut sides: [Side; 2] = [Self::side(&inp.home), Self::side(&inp.away)];
        for (s, sheet) in [&inp.home, &inp.away].into_iter().enumerate() {
            for (i, p) in sheet.xi.iter().enumerate() {
                sides[s].on[i] = mp.len() as u8;
                mp.push(Self::mp(p, s as u8, i as u8, inp, &mut rng, sheet.slots[i].pos));
            }
            for p in &sheet.bench {
                sides[s].bench.push(mp.len() as u8);
                mp.push(Self::mp(p, s as u8, NONE, inp, &mut rng, Pos::GK));
            }
            sides[s].subs_left = inp.max_subs;
            sides[s].windows_left = 3;
        }
        let k = inp.tuning.contest_k;
        let full = inp.lod == Lod::Full;
        let mut e = Self {
            inp,
            rng,
            mp,
            sides,
            clock: 0.0,
            poss: 0,
            ball: zone_id(2, 2),
            carrier: 0,
            passer: NONE,
            carries_since_pass: 0,
            through: false,
            events: Vec::with_capacity(if full { 700 } else { 48 }),
            full,
            k,
            adv: if inp.neutral { 0.0 } else { inp.tuning.home_advantage * k },
            half_events: 0,
            ht: (0, 0),
            extra_time: false,
        };
        if full {
            for m in &mut e.mp {
                m.zones = Some(Box::new([0; 30]));
            }
        }
        e.reshape(0);
        e.reshape(1);
        e
    }

    fn side(t: &TeamSheet) -> Side {
        Side {
            team: t.team,
            tactics: t.tactics,
            mentality: t.tactics.mentality,
            slots: t.slots,
            on: [NONE; 11],
            bench: SmallVec::new(),
            subs_left: 0,
            windows_left: 0,
            reactivity: t.manager_reactivity,
            goals: 0,
            stats: TeamStats::default(),
            poss_time: 0.0,
            last_change: 0.0,
            att: Shape::empty(),
            def: Shape::empty(),
            target: Box::new([[(0, 0.0); 11]; N_ZONES]),
            press_sk: [0.0; 11],
            int_sk: [0.0; 11],
            outfield: [false; 11],
        }
    }

    fn mp(p: &PlayerSheet, side: u8, slot: u8, inp: &MatchInput, rng: &mut Rng, pos: Pos) -> Mp {
        let h = |x: Hidden| p.hidden.f(x);
        let form = 1.0 + rng.normal() * 0.045 * (1.25 - h(Hidden::Consistency) / 20.0);
        let big = 1.0 + inp.importance * (h(Hidden::ImportantMatches) - 10.0) / 10.0 * 0.03;
        let sharp = 0.9 + 0.1 * (p.sharpness / 100.0).clamp(0.0, 1.0);
        let morale = 0.97 + 0.06 * (p.morale / 100.0).clamp(0.0, 1.0);
        let crowd = if side == 1 && !inp.neutral { 1.0 - 0.02 * (1.0 - h(Hidden::Pressure) / 20.0) * inp.importance } else { 1.0 };
        let mut m = Mp {
            sheet: p.clone(),
            side,
            slot,
            gone: false,
            cond: p.condition.clamp(20.0, 100.0),
            base: form * big * sharp * morale * crowd,
            phys: 1.0,
            other: 1.0,
            yellow: 0,
            va: 0.0,
            conceded: 0,
            on_t: 0.0,
            off_t: None,
            line: PlayerLine { player: p.id, side, started: slot != NONE, pos: (slot != NONE).then_some(pos), ..Default::default() },
            zones: None,
        };
        m.refresh();
        m
    }

    // ---------------------------------------------------------- geometry

    fn slot_xy(side: &Side) -> [(f32, f32); 11] {
        let mut xy = [(0.0, 0.0); 11];
        for (i, s) in side.slots.iter().enumerate() {
            xy[i] = s.pos.base_xy();
        }
        for pos in [Pos::DC, Pos::DM, Pos::MC, Pos::AMC, Pos::ST] {
            let idx: SmallVec<[usize; 4]> = (0..11).filter(|&i| side.slots[i].pos == pos).collect();
            let n = idx.len();
            if n > 1 {
                let span = if n == 2 { 0.24 } else { 0.44 };
                for (k, &i) in idx.iter().enumerate() {
                    xy[i].1 = 0.5 - span / 2.0 + span * k as f32 / (n - 1) as f32;
                }
            }
        }
        xy
    }

    /// Rebuild a side's shapes and pass targets after a structural change
    /// (kick-off, substitution, dismissal, tactical switch).
    fn reshape(&mut self, s: usize) {
        let xy = Self::slot_xy(&self.sides[s]);
        let side = &mut self.sides[s];
        let t = side.tactics;
        let ment = side.mentality.level() as f32;
        let width_u = Tactics::unit(t.width);
        let line_u = Tactics::unit(t.line);
        // Several per-slot arrays are indexed together.
        #[allow(clippy::needless_range_loop)]
        for i in 0..11 {
            side.outfield[i] = side.on[i] != NONE && side.slots[i].pos != Pos::GK;
            if side.on[i] == NONE {
                side.att.clear(i);
                side.def.clear(i);
                continue;
            }
            let role = side.slots[i].role;
            let prof = role.profile();
            let keeper = side.slots[i].pos == Pos::GK;
            let (bx, by) = xy[i];
            let toward_line = if by < 0.5 {
                -1.0
            } else if by > 0.5 {
                1.0
            } else {
                0.0
            };
            let mut ay = by;
            if prof.width > 0.0 {
                ay += toward_line * (prof.width * 0.12 + width_u.max(0.0) * 0.05);
            } else {
                ay -= (by - 0.5) * (-prof.width) * 0.5;
            }
            let (ax, ay) = ((bx + prof.push + 0.05 * ment + 0.08).min(0.93), ay.clamp(0.05, 0.95));
            let (dx, dy) = (bx * 0.8 + 0.03 + line_u * 0.05 + ment * 0.02, 0.5 + (by - 0.5) * 0.85);
            let sweep = if role == Role::SweeperKeeper { 0.03 } else { 0.0 };
            for b in 0..N_ZONES {
                let (bcx, bcy) = zone_center(b);
                if keeper {
                    side.att.set(b, i, 0.04 + sweep, 0.5, 0.04, 0.04);
                    side.def.set(b, i, 0.03 + sweep + if bcx > 0.5 { 0.03 } else { 0.0 }, 0.5, 0.04, 0.05);
                    continue;
                }
                // In possession the block moves with the ball.
                let line = 0.8 + 0.2 * (bcx - 0.5).max(0.0);
                let px = (ax + (bcx - 0.5) * 0.25).clamp(0.05, line);
                let py = ay + (bcy - ay) * 0.18;
                side.att.set(b, i, px, py, 0.06 + prof.roam * 0.6, 0.08 + prof.roam * 0.7);
                // Out of possession it slides toward the ball and drops into
                // its own box when threatened.
                let qx = (dx + (bcx - 0.4) * 0.45).clamp(0.03, 0.88);
                let qy = dy + (bcy - dy) * 0.3;
                side.def.set(b, i, qx, qy, 0.075 + prof.roam * 0.7, 0.09 + prof.roam * 0.6);
            }
        }

        let r = reach();
        // Zones and slots index several arrays together.
        #[allow(clippy::needless_range_loop)]
        for b in 0..N_ZONES {
            for i in 0..11 {
                if side.on[i] == NONE {
                    continue;
                }
                let (mut best, mut bz) = (0.0f32, b);
                for (tz, &rv) in r[b].iter().enumerate() {
                    let v = side.att.occ(b, tz, i) * rv;
                    if v > best {
                        best = v;
                        bz = tz;
                    }
                }
                side.target[b][i] = (bz as u8, (side.att.occ(b, bz, i) * N_ZONES as f32 / 4.0).min(1.0));
            }
        }
        self.reskill(s);
    }

    /// Refresh per-slot pressing/interception skill (condition changes).
    fn reskill(&mut self, s: usize) {
        let press_u = Tactics::unit(self.sides[s].tactics.press);
        for i in 0..11 {
            let p = self.sides[s].on[i];
            if p == NONE {
                self.sides[s].press_sk[i] = 0.0;
                self.sides[s].int_sk[i] = 0.0;
                continue;
            }
            let m = &self.mp[p as usize];
            let prof = self.sides[s].slots[i].role.profile();
            let keeper = self.sides[s].slots[i].pos == Pos::GK;
            self.sides[s].press_sk[i] = if keeper { 0.0 } else { m.sk(PRESS) / 20.0 * (0.6 + 0.4 * (press_u + 1.0)) * prof.press };
            self.sides[s].int_sk[i] = m.sk(INTERCEPT);
        }
    }

    /// Pressure the defending side applies at `bd` (its own frame).
    #[inline]
    fn pressure(&self, o: usize, bd: usize) -> f32 {
        let side = &self.sides[o];
        (0..11).map(|i| side.def.occ(bd, bd, i) * side.press_sk[i]).sum::<f32>() * 2.4
    }

    /// Expected outfield defenders in `zd` with the ball at `bd`, and their mean skill.
    #[inline]
    fn cover(&self, o: usize, bd: usize, zd: usize) -> (f32, f32) {
        let side = &self.sides[o];
        let (mut n, mut sk) = (0.0, 0.0);
        for i in 0..11 {
            if side.outfield[i] {
                let w = side.def.occ(bd, zd, i);
                n += w;
                sk += w * side.int_sk[i];
            }
        }
        (n, if n > 1e-5 { sk / n } else { 8.0 })
    }

    // ------------------------------------------------------------ helpers

    #[inline]
    fn contest(&self, side: usize, att: f32, def: f32, base: f32) -> f32 {
        let adv = if side == 0 { self.adv } else { -self.adv };
        sigmoid(self.k * (att - def) + base + adv)
    }

    fn pick(&mut self, s: usize, w: &[f32; 11], exclude: usize) -> Option<usize> {
        let mut ww = [0.0f32; 11];
        for i in 0..11 {
            let p = self.sides[s].on[i];
            if p != NONE && p as usize != exclude {
                ww[i] = w[i];
            }
        }
        if ww.iter().sum::<f32>() <= 0.0 {
            return (0..11).find(|&i| self.sides[s].on[i] != NONE && self.sides[s].on[i] as usize != exclude);
        }
        Some(self.rng.weighted(&ww))
    }

    fn defender(&mut self, o: usize, bd: usize, zd: usize) -> usize {
        let mut w = [0.0f32; 11];
        for (i, v) in w.iter_mut().enumerate() {
            if self.sides[o].outfield[i] {
                *v = self.sides[o].def.occ(bd, zd, i) + 0.002;
            }
        }
        let slot = self.pick(o, &w, usize::MAX).unwrap_or(0);
        self.sides[o].on[slot] as usize
    }

    fn attacker_near(&mut self, s: usize, b: usize, z: usize, exclude: usize) -> usize {
        let mut w = [0.0f32; 11];
        for (i, v) in w.iter_mut().enumerate() {
            *v = self.sides[s].att.occ(b, z, i);
        }
        let slot = self.pick(s, &w, exclude).unwrap_or(0);
        self.sides[s].on[slot] as usize
    }

    fn keeper(&self, s: usize) -> usize {
        let side = &self.sides[s];
        side.keeper_slot().and_then(|k| (side.on[k] != NONE).then_some(side.on[k] as usize)).or_else(|| side.on.iter().find(|&&p| p != NONE).map(|&p| p as usize)).unwrap_or(0)
    }

    fn slot_role(&self, i: usize) -> Role {
        let m = &self.mp[i];
        self.sides[m.side as usize].slots[m.slot as usize].role
    }

    fn slot_pos(&self, i: usize) -> Pos {
        let m = &self.mp[i];
        if m.slot == NONE {
            return Pos::MC;
        }
        self.sides[m.side as usize].slots[m.slot as usize].pos
    }

    fn event(&mut self, side: usize, kind: Ev, player: usize, other: Option<usize>, zone: usize, value: f32) {
        if !self.full && !kind.is_key() {
            return;
        }
        let pid = |i: usize| self.mp[i].sheet.id;
        self.events.push(MatchEvent {
            t: self.clock.min(65_000.0) as u16,
            side: side as u8,
            kind,
            player: if player == usize::MAX { PlayerId::NONE } else { pid(player) },
            other: other.map_or(PlayerId::NONE, pid),
            zone: zone as u8,
            value,
        });
    }

    fn touch(&mut self, i: usize, z: usize) {
        if let Some(zs) = self.mp[i].zones.as_mut() {
            zs[z] = zs[z].saturating_add(1);
        }
    }

    fn live(&mut self, (lo, hi): (f32, f32)) {
        let tempo = Tactics::unit(self.sides[self.poss].tactics.tempo);
        let dt = self.rng.range_f32(lo, hi) * (1.0 - 0.15 * tempo);
        self.clock += dt;
        self.sides[self.poss].poss_time += dt;
    }

    fn dead(&mut self, lo: f32, hi: f32) {
        self.clock += self.rng.range_f32(lo, hi);
    }

    /// Worth of holding the ball at `z` (own frame): threat plus the baseline
    /// value of simply having possession.
    #[inline]
    fn value(&self, s: usize, z: usize) -> f32 {
        let tempo = Tactics::unit(self.sides[s].tactics.tempo);
        xt(z) + 0.013 * (1.0 - 0.3 * tempo)
    }

    fn injury_check(&mut self, i: usize, exposure: f32) {
        let m = &self.mp[i];
        if m.line.injured || !m.on_pitch() {
            return;
        }
        let p = self.inp.tuning.injury_exposure * exposure * m.sheet.injury_risk * (1.0 + (100.0 - m.cond) / 100.0);
        if self.rng.chance(p) {
            let s = m.side as usize;
            self.mp[i].line.injured = true;
            self.mp[i].line.injury_noncontact = self.rng.chance(0.5);
            let z = self.ball;
            self.event(s, Ev::Injury, i, None, z, 0.0);
            self.dead(45.0, 110.0);
            self.half_events += 1;
            let slot = self.mp[i].slot as usize;
            if !self.substitute(s, slot, true) {
                self.mp[i].cond = self.mp[i].cond.min(35.0);
                self.mp[i].refresh();
            }
        }
    }

    // ------------------------------------------------------------ flow

    fn play(&mut self) {
        self.kickoff(0);
        self.event(0, Ev::KickOff, usize::MAX, None, 0, 0.0);
        self.period(0.0, HALF, true);
        self.ht = (self.sides[0].goals, self.sides[1].goals);
        self.event(0, Ev::HalfTime, usize::MAX, None, 0, 0.0);
        self.manage(0, true);
        self.manage(1, true);
        self.clock = HALF;
        self.kickoff(1);
        self.period(HALF, 2.0 * HALF, false);

        if self.inp.decisive && self.level() {
            self.extra_time = true;
            self.event(0, Ev::ExtraTimeStart, usize::MAX, None, 0, 0.0);
            self.clock = 2.0 * HALF;
            self.kickoff(0);
            self.period(2.0 * HALF, 2.0 * HALF + EXTRA_HALF, true);
            self.clock = 2.0 * HALF + EXTRA_HALF;
            self.kickoff(1);
            self.period(2.0 * HALF + EXTRA_HALF, 2.0 * (HALF + EXTRA_HALF), true);
        }
        self.event(0, Ev::FullTime, usize::MAX, None, 0, 0.0);
    }

    /// Level on the day, or on aggregate with the away-goals rule applied.
    fn level(&self) -> bool {
        let (h, a) = (self.sides[0].goals, self.sides[1].goals);
        match self.inp.first_leg {
            None => h == a,
            Some((fh, fa)) => {
                if u16::from(h) + u16::from(fh) != u16::from(a) + u16::from(fa) {
                    return false;
                }
                !self.inp.away_goals_rule || a == fh
            }
        }
    }

    fn period(&mut self, start: f32, end: f32, short: bool) {
        self.half_events = 0;
        let mut next_cp = start + CHECKPOINT;
        let mut stoppage: Option<f32> = None;
        loop {
            if self.clock >= next_cp {
                self.checkpoint(CHECKPOINT);
                next_cp += CHECKPOINT;
            }
            if self.clock >= end {
                let st = *stoppage.get_or_insert_with(|| (if short { 45.0 } else { 140.0 }) + 26.0 * self.half_events as f32 + self.rng.range_f32(0.0, 75.0));
                if self.clock >= end + st {
                    break;
                }
            }
            self.step();
        }
    }

    fn checkpoint(&mut self, elapsed: f32) {
        self.fatigue(elapsed);
        let mut exposed: SmallVec<[usize; 22]> = SmallVec::new();
        for s in 0..2 {
            for i in 0..11 {
                let p = self.sides[s].on[i];
                if p != NONE && self.sides[s].slots[i].pos != Pos::GK {
                    exposed.push(p as usize);
                }
            }
        }
        for p in exposed {
            self.injury_check(p, 0.5);
        }
        if self.clock >= 3300.0 {
            self.manage(0, false);
            self.manage(1, false);
        }
    }

    fn kickoff(&mut self, s: usize) {
        let b = zone_id(2, 2);
        let c = self.attacker_near(s, b, b, usize::MAX);
        self.possess(s, b, Some(c));
    }

    /// Give side `s` the ball at `z` (s frame).
    fn possess(&mut self, s: usize, z: usize, carrier: Option<usize>) {
        #[cfg(test)]
        if zone_xy(z).0 == 5 {
            diag::hit(&diag::ROW5, 7);
        }
        self.poss = s;
        self.ball = z;
        self.carrier = match carrier {
            Some(c) => c,
            None => self.attacker_near(s, z, z, usize::MAX),
        };
        self.passer = NONE;
        self.carries_since_pass = 0;
        self.through = false;
        if self.full {
            let c = self.carrier;
            self.event(s, Ev::Chain, c, None, z, 0.0);
        }
    }

    fn goal_kick(&mut self, s: usize) {
        self.dead(22.0, 38.0);
        let gk = self.keeper(s);
        self.possess(s, zone_id(0, 2), Some(gk));
    }

    // ---------------------------------------------------------- decision

    fn step(&mut self) {
        let s = self.poss;
        let o = 1 - s;
        let c = self.carrier;
        let z = self.ball;
        if !self.mp[c].on_pitch() {
            self.possess(s, z, None);
            return;
        }
        let bd = mirror(z);
        let (zx, zy) = zone_xy(z);
        let press = self.pressure(o, bd);
        let role = self.slot_role(c);
        let prof = role.profile();
        let traits = self.mp[c].sheet.traits;
        let keeper = self.slot_pos(c) == Pos::GK;
        let tac = self.sides[s].tactics;
        let dir = Tactics::unit(tac.directness);
        let width = Tactics::unit(tac.width);
        let lose_here = self.value(o, bd);
        self.touch(c, z);

        let mut acts: SmallVec<[(Act, f32); 16]> = SmallVec::new();
        let skills = [
            self.mp[c].sk(if keeper { PASS_GK } else { PASS_SHORT }),
            self.mp[c].sk(if keeper { PASS_GK } else { PASS_MEDIUM }),
            self.mp[c].sk(if keeper { PASS_GK } else { PASS_LONG }),
            self.mp[c].sk(PASS_THROUGH),
            self.mp[c].sk(CROSS),
        ];

        for slot in 0..11 {
            let r = self.sides[s].on[slot];
            if r == NONE || r as usize == c {
                continue;
            }
            let (tz, avail) = self.sides[s].target[z][slot];
            if avail < 0.05 {
                continue;
            }
            let tz = tz as usize;
            let kind = self.classify(s, slot, z, tz);
            let tzd = mirror(tz);
            let (dens, dsk) = self.cover(o, bd, tzd);
            let recv = self.mp[r as usize].sk(if kind == PassKind::Through { RECEIVE_RUN } else { RECEIVE });
            let p = self.pass_success(s, kind, z, tz, press, skills[kind as usize], recv, dsk) * (1.0 - self.offside_risk(kind, tz, o, None));
            let space = 1.0 - 0.25 * (dens - 0.4).clamp(0.0, 1.5);
            let mut gain = p * self.value(s, tz) * (0.6 + 0.4 * avail) * space;
            gain *= match kind {
                PassKind::Short => 1.0 - 0.2 * dir,
                PassKind::Medium => 1.0,
                PassKind::Long => (1.0 + 0.45 * dir) * if keeper { 1.2 } else { 1.0 },
                PassKind::Through => prof.risk * (1.0 + 0.35 * dir) * if traits.contains(PlayerTraits::KILLER_BALLS) { 1.4 } else { 1.0 },
                PassKind::Cross => prof.cross * (1.0 + 0.3 * width),
            };
            if traits.contains(PlayerTraits::SIMPLE_PASSES) {
                gain *= if kind == PassKind::Short { 1.2 } else { 0.8 };
            }
            let loss = (1.0 - p) * self.value(o, tzd) * 1.4;
            acts.push((Act::Pass { slot: slot as u8, zone: tz as u8, kind }, gain - loss));
        }

        if zx < ZONES_X - 1 && !keeper {
            let tz = zone_id(zx + 1, drift_y(zy, role, traits));
            let (_, dsk) = self.cover(o, bd, mirror(tz));
            let (engage, win) = self.carry_parts(s, z, tz, press, self.mp[c].sk(DRIBBLE), dsk);
            let p = 1.0 - engage * (1.0 - win);
            let mut m = prof.dribble * (0.8 + self.mp[c].eff(Attr::Flair) / 50.0);
            if traits.contains(PlayerTraits::RUNS_WITH_BALL) {
                m *= 1.3;
            }
            if traits.contains(PlayerTraits::TRIES_TRICKS) {
                m *= 1.15;
            }
            acts.push((Act::Carry { zone: tz as u8 }, p * self.value(s, tz) * m - (1.0 - p) * lose_here * 1.4));
        }

        if zx >= 3 && !keeper {
            let xg = self.open_xg(z, press);
            let mut m = prof.shoot * self.inp.tuning.shot_bias;
            if zx <= 4 && traits.contains(PlayerTraits::SHOOTS_FROM_DISTANCE) {
                m *= 1.6;
            }
            if traits.contains(PlayerTraits::PENALTY_BOX_PLAYER) && in_box(z) {
                m *= 1.2;
            }
            // A shot usually ends the possession deep in the opponent's half.
            acts.push((Act::Shoot, xg * m - (1.0 - xg) * self.value(o, zone_id(0, 2)) * 0.9));
        }

        if zx <= 1 && press > 0.5 {
            let tz = zone_id(3, 2);
            let v = 0.42 * self.value(s, tz) - 0.58 * self.value(o, mirror(tz));
            acts.push((Act::Clear, v * prof.defend * if keeper { 1.5 } else { 1.0 }));
        }

        match self.choose(c, &acts, press) {
            Some(Act::Pass { slot, zone, kind }) => self.pass(slot as usize, zone as usize, kind, press),
            Some(Act::Carry { zone }) => self.carry(zone as usize, press),
            Some(Act::Shoot) => {
                let xg = self.open_xg(z, press);
                self.shoot(ShotKind::Open, xg);
            }
            Some(Act::Clear) | None => self.clear(),
        }
    }

    fn classify(&self, s: usize, slot: usize, z: usize, tz: usize) -> PassKind {
        let (zx, _) = zone_xy(z);
        let (tzx, _) = zone_xy(tz);
        let d = zone_dist(z, tz);
        if zx >= 4 && is_wide(z) && in_box(tz) {
            PassKind::Cross
        } else if tzx >= 4 && tzx >= zx + 2 && self.sides[s].slots[slot].role.profile().push >= 0.08 {
            PassKind::Through
        } else if d <= 1.5 {
            PassKind::Short
        } else if d <= 3.0 {
            PassKind::Medium
        } else {
            PassKind::Long
        }
    }

    /// Expected outfield defenders in the passing lane (midpoint zone).
    fn lane(&self, o: usize, z: usize, tz: usize) -> f32 {
        if zone_dist(z, tz) < 1.9 {
            return 0.0;
        }
        let ((zx, zy), (tx, ty)) = (zone_xy(z), zone_xy(tz));
        let mz = zone_id((zx + tx) / 2, (zy + ty) / 2);
        if mz == z || mz == tz { 0.0 } else { self.cover(o, mirror(z), mirror(mz)).0 }
    }

    /// (technical error, a defender engages, the pass beats him).
    #[allow(clippy::too_many_arguments)]
    fn pass_parts(&self, s: usize, kind: PassKind, z: usize, tz: usize, press: f32, skill: f32, recv: f32, def: f32) -> (f32, f32, f32) {
        let o = 1 - s;
        let (dt, _) = self.cover(o, mirror(z), mirror(tz));
        let dl = self.lane(o, z, tz);
        let err = sigmoid(-0.3 * (skill - 10.0) - kind.accuracy() + 0.5 * press.min(1.6) + 0.3 * (zone_dist(z, tz) - 1.5).max(0.0));
        // Defenders inside their own box are set and marking: entries are contested hard.
        let entry = in_box(tz) && !in_box(z);
        let engage = 1.0 - exp(-((if entry { 1.8 } else { 1.1 }) * dt + 0.6 * dl));
        let win = self.contest(s, 0.75 * skill + 0.25 * recv, def, kind.duel() - if entry { 1.1 } else { 0.0 });
        (err, engage, win)
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn pass_success(&self, s: usize, kind: PassKind, z: usize, tz: usize, press: f32, skill: f32, recv: f32, def: f32) -> f32 {
        let (err, engage, win) = self.pass_parts(s, kind, z, tz, press, skill, recv, def);
        (1.0 - err) * (1.0 - engage * (1.0 - win))
    }

    /// (a defender engages the carrier, the carrier beats him).
    fn carry_parts(&self, s: usize, z: usize, tz: usize, press: f32, dribble: f32, tackle: f32) -> (f32, f32) {
        let (dt, _) = self.cover(1 - s, mirror(z), mirror(tz));
        let entry = in_box(tz) && !in_box(z);
        let engage = 1.0 - exp(-((if entry { 1.5 } else { 0.85 }) * dt + 0.35 * press.min(1.6)));
        (engage, self.contest(s, dribble, tackle, if entry { -0.4 } else { 0.3 }))
    }

    /// Chance a forward ball is flagged offside. `receiver` refines it with
    /// the runner's timing; without one it is the passer's expectation.
    fn offside_risk(&self, kind: PassKind, tz: usize, o: usize, receiver: Option<usize>) -> f32 {
        let base = match kind {
            PassKind::Through => 0.2,
            PassKind::Long | PassKind::Medium if zone_xy(tz).0 >= 4 => 0.05,
            _ => return 0.0,
        };
        let line_u = Tactics::unit(self.sides[o].tactics.line);
        let mut p = base * (1.0 + 0.6 * line_u);
        if let Some(r) = receiver {
            p *= 1.0 - 0.025 * (self.mp[r].eff(Attr::Anticipation) - 10.0);
            if self.mp[r].sheet.traits.contains(PlayerTraits::BEATS_OFFSIDE_TRAP) {
                p *= 1.3;
            }
        }
        p.clamp(0.01, 0.4)
    }

    fn choose(&mut self, c: usize, acts: &[(Act, f32)], press: f32) -> Option<Act> {
        if acts.is_empty() {
            return None;
        }
        let best = acts.iter().map(|a| a.1).fold(f32::MIN, f32::max);
        let dec = self.mp[c].eff(Attr::Decisions);
        let comp = self.mp[c].eff(Attr::Composure);
        let tau = ((0.3 - 0.012 * dec) * (1.0 + 0.5 * press * (1.0 - comp / 20.0))).clamp(0.04, 0.35);
        let norm = best.abs().max(0.004);
        let mut w: SmallVec<[f32; 16]> = SmallVec::new();
        for a in acts {
            w.push(exp((a.1 - best) / (tau * norm)));
        }
        Some(acts[self.rng.weighted(&w)].0)
    }

    fn open_xg(&self, z: usize, press: f32) -> f32 {
        let mut xg = base_xg(z) * (1.0 - 0.3 * press.min(1.2));
        if self.through {
            xg *= 1.8;
        } else if self.passer != NONE && self.carries_since_pass == 0 {
            xg *= 1.1;
        }
        let (dens, _) = self.cover(1 - self.poss, mirror(z), mirror(z));
        xg *= 1.0 - 0.22 * (dens - 0.5).clamp(0.0, 2.0);
        xg.clamp(0.004, 0.7)
    }

    // ----------------------------------------------------------- actions

    fn pass(&mut self, slot: usize, tz: usize, kind: PassKind, press: f32) {
        let s = self.poss;
        let o = 1 - s;
        let c = self.carrier;
        let r = self.sides[s].on[slot] as usize;
        let z = self.ball;
        let bd = mirror(z);
        let tzd = mirror(tz);
        self.mp[c].line.passes += 1;
        self.sides[s].stats.passes += 1;
        if kind == PassKind::Cross {
            self.mp[c].line.crosses += 1;
        }
        self.live(kind.duration());

        {
            let p_off = self.offside_risk(kind, tz, o, Some(r));
            if p_off > 0.0 && self.rng.chance(p_off) {
                self.mp[r].line.offsides += 1;
                self.sides[s].stats.offsides += 1;
                self.event(s, Ev::Offside, r, None, tz, 0.0);
                self.dead(18.0, 30.0);
                let d = self.defender(o, tzd, tzd);
                self.possess(o, tzd, Some(d));
                return;
            }
        }

        let d = self.defender(o, bd, tzd);
        if press > 0.35 && self.rng.chance(0.02 * press * self.aggression(d)) {
            self.foul(d, c, z, false);
            return;
        }

        let skill = self.mp[c].sk(match (kind, self.slot_pos(c) == Pos::GK) {
            (PassKind::Short | PassKind::Medium | PassKind::Long, true) => PASS_GK,
            (PassKind::Short, _) => PASS_SHORT,
            (PassKind::Medium, _) => PASS_MEDIUM,
            (PassKind::Long, _) => PASS_LONG,
            (PassKind::Through, _) => PASS_THROUGH,
            (PassKind::Cross, _) => CROSS,
        });
        let recv = self.mp[r].sk(if kind == PassKind::Through { RECEIVE_RUN } else { RECEIVE });
        let (err, engage, win) = self.pass_parts(s, kind, z, tz, press, skill, recv, self.mp[d].sk(INTERCEPT));
        let misplaced = self.rng.chance(err);
        let completed = !misplaced && (!self.rng.chance(engage) || self.rng.chance(win));
        #[cfg(test)]
        {
            diag::hit(&diag::TRIED, kind as usize);
            if completed {
                diag::hit(&diag::DONE, kind as usize);
                if zone_xy(tz).0 == 5 && zone_xy(z).0 < 5 {
                    diag::hit(&diag::ROW5, kind as usize);
                }
            }
        }
        if completed {
            self.mp[c].line.passes_completed += 1;
            self.sides[s].stats.passes_completed += 1;
            self.mp[c].va += (xt(tz) - xt(z)).max(0.0);
            if zone_xy(tz).0 >= 4 && zone_xy(z).0 < 4 {
                self.mp[c].line.progressive_passes += 1;
            }
            if kind == PassKind::Through {
                self.event(s, Ev::ThroughBall, c, Some(r), tz, 0.0);
            }
            self.passer = c as u8;
            self.carries_since_pass = 0;
            self.through = kind == PassKind::Through;
            self.carrier = r;
            self.ball = tz;
            if kind == PassKind::Cross {
                self.mp[c].line.crosses_completed += 1;
                self.event(s, Ev::Cross, c, None, tz, 0.0);
                self.aerial(c, false);
            }
            return;
        }

        self.mp[c].va -= if zone_xy(z).0 <= 2 { 0.02 } else { 0.008 };
        let out_p = match kind {
            PassKind::Cross => 0.55,
            PassKind::Long => 0.5,
            PassKind::Through => 0.35,
            _ => 0.3,
        };
        if misplaced && self.rng.chance(out_p) {
            if kind == PassKind::Cross && self.rng.chance(0.45) {
                self.corner(s);
            } else if zone_xy(tz).0 == ZONES_X - 1 && !is_wide(tz) {
                self.goal_kick(o);
            } else {
                self.dead(14.0, 26.0);
                self.possess(o, tzd, None);
            }
        } else {
            self.mp[d].line.interceptions += 1;
            self.mp[d].va += 0.015;
            self.event(o, Ev::Interception, d, None, tzd, 0.0);
            self.possess(o, tzd, Some(d));
        }
    }

    fn carry(&mut self, tz: usize, press: f32) {
        let s = self.poss;
        let o = 1 - s;
        let c = self.carrier;
        let z = self.ball;
        let bd = mirror(z);
        self.live((3.0, 6.0));
        let d = self.defender(o, bd, mirror(tz));
        let (engage, win) = self.carry_parts(s, z, tz, press, self.mp[c].sk(DRIBBLE), self.mp[d].sk(TACKLE));
        if !self.rng.chance(engage) {
            #[cfg(test)]
            if zone_xy(tz).0 == 5 {
                diag::hit(&diag::ROW5, 5);
            }
            self.mp[c].va += (xt(tz) - xt(z)).max(0.0);
            self.carries_since_pass += 1;
            self.through = false;
            self.ball = tz;
            return;
        }
        self.mp[c].line.dribbles += 1;
        self.mp[d].line.tackles += 1;
        self.injury_check(c, 0.6);
        self.injury_check(d, 0.5);
        if !self.mp[c].on_pitch() || !self.mp[d].on_pitch() || self.poss != s {
            return;
        }
        let won = self.rng.chance(win);
        let foul_p = if won { 0.24 } else { 0.07 } * self.aggression(d) * self.inp.tuning.foul_rate / 0.12;
        if self.rng.chance(foul_p) {
            self.foul(d, c, if won { tz } else { z }, true);
            return;
        }
        if won {
            #[cfg(test)]
            if zone_xy(tz).0 == 5 {
                diag::hit(&diag::ROW5, 6);
            }
            self.mp[c].line.dribbles_won += 1;
            self.mp[c].va += (xt(tz) - xt(z)).max(0.0) + 0.006;
            if zone_xy(tz).0 >= 4 {
                self.event(s, Ev::Dribble, c, Some(d), tz, 0.0);
            }
            self.carries_since_pass += 1;
            self.through = false;
            self.ball = tz;
        } else {
            self.mp[d].line.tackles_won += 1;
            self.sides[o].stats.tackles += 1;
            self.mp[d].va += 0.02;
            self.mp[c].va -= 0.01;
            if zone_xy(bd).0 <= 1 {
                self.event(o, Ev::Tackle, d, Some(c), bd, 0.0);
            }
            self.possess(o, bd, Some(d));
        }
    }

    fn clear(&mut self) {
        let s = self.poss;
        let o = 1 - s;
        let c = self.carrier;
        self.mp[c].line.clearances += 1;
        self.mp[c].va += 0.005;
        let z = self.ball;
        self.event(s, Ev::Clearance, c, None, z, 0.0);
        self.live((2.5, 4.0));
        let tz = zone_id(3, self.rng.below(ZONES_Y as u32) as usize);
        if self.rng.chance(0.4) {
            self.possess(s, tz, None);
        } else {
            if self.rng.chance(0.3) {
                self.dead(14.0, 24.0);
            }
            self.possess(o, mirror(tz), None);
        }
    }

    fn aggression(&self, i: usize) -> f32 {
        let m = &self.mp[i];
        (1.0 + (m.eff(Attr::Aggression) - 10.0) / 20.0 + (m.hidden(Hidden::Dirtiness) - 10.0) / 20.0 - (m.hidden(Hidden::Sportsmanship) - 10.0) / 30.0).clamp(0.4, 2.0)
    }

    fn foul(&mut self, offender: usize, victim: usize, zone: usize, tackle: bool) {
        let s = self.mp[victim].side as usize;
        let o = 1 - s;
        self.mp[offender].line.fouls += 1;
        self.mp[victim].line.fouled += 1;
        self.sides[o].stats.fouls += 1;
        self.mp[offender].va -= 0.01;
        self.event(o, Ev::Foul, offender, Some(victim), mirror(zone), 0.0);

        let strict = self.inp.referee_strictness;
        let tactical = if zone_xy(zone).0 >= 3 { 1.35 } else { 1.0 };
        let ag = self.aggression(offender);
        let temper = 1.0 + (10.0 - self.mp[offender].hidden(Hidden::Temperament)).max(0.0) / 15.0;
        let p_red = self.inp.tuning.red_rate * strict * ag * temper * if tackle { 1.0 } else { 0.4 };
        let p_yellow = self.inp.tuning.yellow_rate * strict * ag * tactical;
        if self.rng.chance(p_red) {
            self.card(offender, true);
        } else if self.rng.chance(p_yellow) {
            self.card(offender, false);
        }

        if in_box(zone) && self.rng.chance(0.85) {
            self.penalty(s);
            return;
        }
        self.dead(22.0, 45.0);
        let (zx, zy) = zone_xy(zone);
        if zx >= 4 && (1..=3).contains(&zy) && self.rng.chance(0.55) {
            let taker = self.best_taker(s, Attr::FreeKicks);
            let fk = self.mp[taker].eff(Attr::FreeKicks);
            self.possess(s, zone, Some(taker));
            let xg = (0.035 + 0.004 * (fk - 10.0)).clamp(0.015, 0.1);
            self.event(s, Ev::FreeKick, taker, None, zone, xg);
            self.shoot(ShotKind::FreeKick, xg);
            return;
        }
        if zx >= 4 && is_wide(zone) {
            let taker = self.best_taker(s, Attr::Crossing);
            self.possess(s, zone, Some(taker));
            self.event(s, Ev::FreeKick, taker, None, zone, 0.0);
            self.aerial(taker, true);
            return;
        }
        let v = self.mp[victim].on_pitch().then_some(victim);
        self.possess(s, zone, v);
    }

    fn card(&mut self, i: usize, straight_red: bool) {
        let s = self.mp[i].side as usize;
        let z = self.ball;
        self.half_events += 1;
        if straight_red || self.mp[i].yellow >= 1 {
            if straight_red {
                self.event(s, Ev::Red, i, None, z, 0.0);
            } else {
                self.mp[i].yellow += 1;
                self.mp[i].line.yellows += 1;
                self.sides[s].stats.yellows += 1;
                self.event(s, Ev::SecondYellow, i, None, z, 0.0);
            }
            self.mp[i].line.reds += 1;
            self.sides[s].stats.reds += 1;
            self.mp[i].va -= 0.3;
            self.send_off(i);
        } else {
            self.mp[i].yellow = 1;
            self.mp[i].line.yellows += 1;
            self.sides[s].stats.yellows += 1;
            self.mp[i].va -= 0.03;
            self.event(s, Ev::Yellow, i, None, z, 0.0);
        }
    }

    fn send_off(&mut self, i: usize) {
        let s = self.mp[i].side as usize;
        let slot = self.mp[i].slot as usize;
        self.mp[i].gone = true;
        self.mp[i].off_t = Some(self.clock);
        self.sides[s].on[slot] = NONE;
        if self.sides[s].slots[slot].pos == Pos::GK {
            let bench_gk = self.sides[s].bench.iter().copied().find(|&b| is_natural_keeper(&self.mp[b as usize].sheet));
            match bench_gk {
                Some(b) if self.sides[s].subs_left > 0 => {
                    if let Some(out_slot) = (0..11).rev().find(|&k| self.sides[s].on[k] != NONE && self.sides[s].slots[k].pos != Pos::GK) {
                        let out = self.sides[s].on[out_slot] as usize;
                        self.sides[s].slots.swap(slot, out_slot);
                        self.bring_on(s, out_slot, out, b as usize);
                    }
                }
                _ => {
                    if let Some(k) = (0..11).rev().find(|&k| self.sides[s].on[k] != NONE) {
                        self.sides[s].on[slot] = self.sides[s].on[k];
                        self.sides[s].on[k] = NONE;
                        let moved = self.sides[s].on[slot] as usize;
                        self.mp[moved].slot = slot as u8;
                    }
                }
            }
        }
        self.reshape(s);
        if self.carrier == i {
            let z = mirror(self.ball);
            self.possess(1 - s, z, None);
        }
    }

    fn best_taker(&self, s: usize, a: Attr) -> usize {
        self.sides[s]
            .on
            .iter()
            .filter(|&&p| p != NONE)
            .map(|&p| p as usize)
            .filter(|&p| self.slot_pos(p) != Pos::GK)
            .max_by(|&x, &y| self.mp[x].eff(a).total_cmp(&self.mp[y].eff(a)))
            .unwrap_or(self.carrier)
    }

    fn corner(&mut self, s: usize) {
        self.sides[s].stats.corners += 1;
        self.dead(28.0, 45.0);
        let taker = self.best_taker(s, Attr::Corners);
        let z = zone_id(ZONES_X - 1, if self.rng.chance(0.5) { 0 } else { ZONES_Y - 1 });
        self.possess(s, z, Some(taker));
        self.event(s, Ev::Corner, taker, None, z, 0.0);
        if self.rng.chance(0.14) {
            return;
        }
        if self.rng.chance(sigmoid(0.25 * (self.mp[taker].eff(Attr::Corners) - 10.0) + 0.6)) {
            self.aerial(taker, true);
        } else {
            self.possess(1 - s, zone_id(0, 2), None);
        }
    }

    /// Ball delivered into the box: keeper claim, aerial duel, header or clearance.
    fn aerial(&mut self, deliverer: usize, set_piece: bool) {
        let s = self.poss;
        let o = 1 - s;
        let box_zones = [zone_id(5, 1), zone_id(5, 2), zone_id(5, 3)];
        let b = self.ball;
        let bd = mirror(b);
        let gk = self.keeper(o);
        let claim = 0.06 + 0.012 * (self.mp[gk].eff(Attr::CommandOfArea) + self.mp[gk].eff(Attr::AerialReach) - 20.0);
        self.live((1.5, 3.0));
        if self.rng.chance(claim.clamp(0.02, 0.25)) {
            self.possess(o, zone_id(0, 2), Some(gk));
            return;
        }
        let bonus = if set_piece { 0.08 } else { 0.0 };
        let mut wa = [0.0f32; 11];
        let mut wd = [0.0f32; 11];
        for i in 0..11 {
            let pa = self.sides[s].on[i];
            if pa != NONE && pa as usize != deliverer && self.sides[s].slots[i].pos != Pos::GK {
                let presence: f32 = box_zones.iter().map(|&z| self.sides[s].att.occ(b, z, i)).sum::<f32>() + bonus;
                wa[i] = presence * aerial_power(&self.mp[pa as usize], AERIAL_ATT).max(1.0).powi(2);
            }
            let pd = self.sides[o].on[i];
            if pd != NONE && self.sides[o].outfield[i] {
                let presence: f32 = box_zones.iter().map(|&z| self.sides[o].def.occ(bd, mirror(z), i)).sum::<f32>() + bonus;
                wd[i] = presence * aerial_power(&self.mp[pd as usize], AERIAL_DEF).max(1.0).powi(2);
            }
        }
        let (Some(ai), Some(di)) = (self.pick(s, &wa, deliverer), self.pick(o, &wd, usize::MAX)) else {
            self.possess(o, zone_id(1, 2), None);
            return;
        };
        let a = self.sides[s].on[ai] as usize;
        let d = self.sides[o].on[di] as usize;
        let p = self.contest(s, aerial_power(&self.mp[a], AERIAL_ATT), aerial_power(&self.mp[d], AERIAL_DEF), -0.2);
        self.injury_check(a, 0.4);
        self.injury_check(d, 0.4);
        if !self.mp[a].on_pitch() || !self.mp[d].on_pitch() || self.poss != s {
            return;
        }
        if self.rng.chance(p) {
            self.mp[a].line.aerials_won += 1;
            self.mp[d].line.aerials_lost += 1;
            self.mp[a].va += 0.01;
            let hz = box_zones[self.rng.weighted(&[1.0, 1.6, 1.0])];
            self.carrier = a;
            self.ball = hz;
            self.passer = deliverer as u8;
            self.carries_since_pass = 0;
            self.through = false;
            let xg = header_xg(hz) * (0.75 + 0.03 * (self.mp[a].eff(Attr::Heading) - 10.0)).clamp(0.4, 1.4);
            self.shoot(ShotKind::Header, xg);
        } else {
            self.mp[d].line.aerials_won += 1;
            self.mp[d].line.clearances += 1;
            self.mp[a].line.aerials_lost += 1;
            self.mp[d].va += 0.02;
            if self.rng.chance(0.004) {
                self.own_goal(d);
                return;
            }
            self.event(o, Ev::Clearance, d, None, zone_id(0, 2), 0.0);
            if self.rng.chance(0.38) {
                let y = 1 + self.rng.below(3) as usize;
                self.possess(s, zone_id(4, y), None);
            } else if self.rng.chance(0.18) {
                self.corner(s);
            } else {
                self.possess(o, zone_id(1, 2), Some(d));
            }
        }
    }

    // ------------------------------------------------------------ shots

    fn finish_skill(&mut self, i: usize, kind: ShotKind) -> f32 {
        let long = zone_xy(self.ball).0 <= 4;
        let weak_roll = self.rng.f32();
        let m = &self.mp[i];
        let f = match kind {
            ShotKind::Header => 1.0 + 0.05 * (m.eff(Attr::Heading) - 11.0) + 0.02 * (m.eff(Attr::JumpingReach) - 11.0),
            ShotKind::FreeKick => 1.0 + 0.06 * (m.eff(Attr::FreeKicks) - 11.0) + 0.015 * (m.eff(Attr::Technique) - 11.0),
            ShotKind::Open | ShotKind::Rebound => {
                let main = if long { 0.6 * m.eff(Attr::LongShots) + 0.4 * m.eff(Attr::Finishing) } else { m.eff(Attr::Finishing) };
                let mut f = 1.0 + 0.05 * (main - 11.0) + 0.02 * (m.eff(Attr::Composure) - 11.0) + 0.01 * (m.eff(Attr::Technique) - 11.0);
                let weak = f32::from(m.sheet.left_foot.min(m.sheet.right_foot));
                if weak_roll < if weak >= 15.0 { 0.12 } else { 0.3 } {
                    f *= 0.7 + 0.015 * weak;
                }
                f
            }
        };
        (f * self.inp.tuning.finish_bias).clamp(0.4, 1.8)
    }

    fn keeper_factor(&self, s: usize) -> f32 {
        let gk = self.keeper(s);
        let g = if self.slot_pos(gk) == Pos::GK && is_natural_keeper(&self.mp[gk].sheet) { self.mp[gk].sk(KEEPER) } else { 3.0 };
        (1.0 - 0.035 * (g - 11.0)).clamp(0.55, 1.45)
    }

    fn shoot(&mut self, kind: ShotKind, xg: f32) {
        let s = self.poss;
        let o = 1 - s;
        let c = self.carrier;
        let z = self.ball;
        self.live((1.2, 2.2));
        self.mp[c].line.shots += 1;
        self.mp[c].line.xg += xg;
        self.sides[s].stats.shots += 1;
        self.sides[s].stats.xg += xg;
        if xg >= 0.3 {
            self.sides[s].stats.big_chances += 1;
        }
        let assister = (self.passer != NONE && self.carries_since_pass <= 1 && self.passer as usize != c && kind != ShotKind::FreeKick).then_some(self.passer as usize);
        if let Some(a) = assister {
            self.mp[a].line.key_passes += 1;
            self.mp[a].line.xa += xg;
            self.mp[a].va += 0.3 * xg;
            self.event(s, Ev::KeyPass, a, Some(c), z, xg);
        }

        let (dens, _) = self.cover(o, mirror(z), mirror(z));
        let block_p = match kind {
            ShotKind::Header => 0.05,
            ShotKind::FreeKick => 0.22,
            _ => (0.12 * dens).clamp(0.05, 0.4),
        };
        let p_goal = (xg * self.finish_skill(c, kind) * self.keeper_factor(o)).clamp(0.002, 0.9);
        let p_on = (0.33 + 0.02 * (self.mp[c].eff(Attr::Composure) - 10.0) + 0.6 * xg).clamp(0.15, 0.85);
        let p_save = (p_on - p_goal).max(0.04);
        let r = self.rng.f32();
        let gk = self.keeper(o);

        if r < block_p {
            let blocker = self.defender(o, mirror(z), mirror(z));
            self.mp[blocker].line.blocks += 1;
            self.mp[blocker].va += 0.03;
            self.mp[c].va -= 0.1 * xg;
            self.event(s, Ev::ShotBlocked, c, Some(blocker), z, xg);
            let r2 = self.rng.f32();
            if r2 < 0.3 {
                self.corner(s);
            } else if r2 < 0.6 {
                let y = 1 + self.rng.below(3) as usize;
                self.possess(s, zone_id(4, y), None);
            } else {
                self.possess(o, mirror(z), Some(blocker));
            }
        } else if r < block_p + p_goal * (1.0 - block_p) {
            self.mp[c].line.on_target += 1;
            self.sides[s].stats.on_target += 1;
            self.score(s, c, assister, Ev::Goal, xg);
        } else if r < block_p + (p_goal + p_save) * (1.0 - block_p) {
            self.mp[c].line.on_target += 1;
            self.sides[s].stats.on_target += 1;
            self.sides[o].stats.saves += 1;
            self.mp[gk].line.saves += 1;
            self.mp[gk].va += 0.9 * xg + 0.02;
            self.mp[c].va -= 0.2 * xg;
            self.event(s, Ev::ShotSaved, c, Some(gk), z, xg);
            let r2 = self.rng.f32();
            if r2 < 0.1 && kind != ShotKind::Rebound {
                let b = zone_id(5, 2);
                let follow = self.attacker_near(s, b, b, usize::MAX);
                if self.rng.chance(0.45) {
                    self.carrier = follow;
                    self.ball = b;
                    self.passer = NONE;
                    self.through = false;
                    let xg2 = 0.25 * self.rng.range_f32(0.6, 1.3);
                    self.shoot(ShotKind::Rebound, xg2);
                    return;
                }
                self.possess(o, zone_id(0, 2), None);
            } else if r2 < 0.36 {
                self.corner(s);
            } else {
                self.possess(o, zone_id(0, 2), Some(gk));
            }
        } else {
            self.mp[c].va -= 0.3 * xg;
            let post = self.rng.chance(0.05);
            self.event(s, if post { Ev::ShotPost } else { Ev::ShotWide }, c, None, z, xg);
            if post && self.rng.chance(0.35) {
                let y = 1 + self.rng.below(3) as usize;
                self.possess(s, zone_id(5, y), None);
            } else {
                self.goal_kick(o);
            }
        }
    }

    fn penalty(&mut self, s: usize) {
        let o = 1 - s;
        self.dead(60.0, 95.0);
        let taker = self.best_taker(s, Attr::PenaltyTaking);
        let gk = self.keeper(o);
        self.poss = s;
        self.carrier = taker;
        self.ball = zone_id(5, 2);
        self.passer = NONE;
        let xg = 0.76;
        self.mp[taker].line.shots += 1;
        self.mp[taker].line.xg += xg;
        self.sides[s].stats.shots += 1;
        self.sides[s].stats.xg += xg;
        self.sides[s].stats.big_chances += 1;
        if self.rng.chance(self.pen_prob(taker, gk, 0.0)) {
            self.mp[taker].line.on_target += 1;
            self.sides[s].stats.on_target += 1;
            self.score(s, taker, None, Ev::PenaltyGoal, xg);
        } else {
            self.mp[taker].va -= 0.25;
            let z = self.ball;
            self.event(s, Ev::PenaltyMiss, taker, Some(gk), z, xg);
            if self.rng.chance(0.6) {
                self.mp[gk].line.saves += 1;
                self.mp[gk].va += 0.35;
                self.sides[o].stats.saves += 1;
                self.mp[taker].line.on_target += 1;
                self.sides[s].stats.on_target += 1;
            }
            self.goal_kick(o);
        }
    }

    fn pen_prob(&self, taker: usize, gk: usize, pressure: f32) -> f32 {
        let t = &self.mp[taker];
        let g = &self.mp[gk];
        let nerves = (10.0 - t.eff(Attr::Composure)).max(0.0) * pressure;
        (0.76 + 0.012 * (t.eff(Attr::PenaltyTaking) - 12.0) + 0.008 * (t.eff(Attr::Composure) - 12.0) - 0.01 * (g.eff(Attr::Reflexes) - 12.0) - 0.012 * nerves - 0.04 * pressure).clamp(0.5, 0.93)
    }

    fn own_goal(&mut self, i: usize) {
        let defending = self.mp[i].side as usize;
        self.mp[i].va -= 0.4;
        self.sides[1 - defending].goals += 1;
        let z = self.ball;
        self.event(defending, Ev::OwnGoal, i, None, z, 0.0);
        self.concede(defending);
        self.half_events += 1;
        self.dead(50.0, 75.0);
        self.kickoff(defending);
    }

    fn score(&mut self, s: usize, scorer: usize, assister: Option<usize>, ev: Ev, xg: f32) {
        let o = 1 - s;
        self.sides[s].goals += 1;
        self.mp[scorer].line.goals += 1;
        self.mp[scorer].va += 0.55 + 0.35 * (1.0 - xg);
        if let Some(a) = assister {
            self.mp[a].line.assists += 1;
            self.mp[a].va += 0.3;
        }
        let z = self.ball;
        self.event(s, ev, scorer, assister, z, xg);
        self.concede(o);
        self.half_events += 1;
        self.dead(55.0, 80.0);
        self.kickoff(o);
    }

    fn concede(&mut self, o: usize) {
        for i in 0..11 {
            let p = self.sides[o].on[i];
            if p == NONE {
                continue;
            }
            let p = p as usize;
            self.mp[p].conceded += 1;
            match self.sides[o].slots[i].pos.group() {
                PosGroup::Gk => {
                    self.mp[p].line.conceded += 1;
                    self.mp[p].va -= 0.07;
                }
                PosGroup::Def => self.mp[p].va -= 0.04,
                _ => self.mp[p].va -= 0.012,
            }
        }
    }

    // ------------------------------------------------- fatigue & manager

    fn fatigue(&mut self, elapsed: f32) {
        let minutes = elapsed / 60.0;
        for s in 0..2 {
            let press_u = Tactics::unit(self.sides[s].tactics.press);
            for i in 0..11 {
                let p = self.sides[s].on[i];
                if p == NONE {
                    continue;
                }
                let role = self.sides[s].slots[i].role.profile();
                let keeper = self.sides[s].slots[i].pos == Pos::GK;
                let m = &mut self.mp[p as usize];
                let stamina = m.sheet.attrs.get(Attr::Stamina);
                let nf = m.sheet.attrs.get(Attr::NaturalFitness);
                let mut drain = 0.5 * minutes * (1.35 - stamina / 25.0) * (0.85 + 0.12 * press_u + 0.08 * role.press) * (1.1 - nf / 50.0);
                if keeper {
                    drain *= 0.3;
                }
                m.cond = (m.cond - drain.max(0.0)).max(15.0);
                m.refresh();
            }
            self.reskill(s);
        }
    }

    /// In-game management: mentality shifts and substitutions.
    fn manage(&mut self, s: usize, half_time: bool) {
        let o = 1 - s;
        let minute = self.clock / 60.0;
        let diff = i32::from(self.sides[s].goals) - i32::from(self.sides[o].goals);

        if !half_time && minute >= 68.0 && self.clock - self.sides[s].last_change > 600.0 {
            let react = self.sides[s].reactivity;
            let cur = self.sides[s].mentality;
            let next = if diff < 0 && cur != Mentality::Attacking && self.rng.chance(0.5 + 0.5 * react) {
                Some(cur.shift(1))
            } else if diff > 0 && minute >= 78.0 && cur != Mentality::VeryDefensive && self.rng.chance(0.3 + 0.5 * react) {
                Some(cur.shift(-1))
            } else {
                None
            };
            if let Some(m) = next {
                self.sides[s].mentality = m;
                self.sides[s].last_change = self.clock;
                self.event(s, Ev::TacticChange, usize::MAX, None, 0, m.level() as f32);
                self.reshape(s);
            }
        }

        if self.sides[s].subs_left == 0 || (!half_time && (self.sides[s].windows_left == 0 || minute < 55.0)) {
            return;
        }
        let mut outs: SmallVec<[(usize, f32); 11]> = SmallVec::new();
        for i in 0..11 {
            let p = self.sides[s].on[i];
            if p == NONE || self.sides[s].slots[i].pos == Pos::GK {
                continue;
            }
            let m = &self.mp[p as usize];
            let want = if half_time {
                (if m.va < -0.15 { 14.0 } else { 0.0 }) + (80.0 - m.cond).max(0.0)
            } else {
                let mut w = (82.0 - m.cond).max(0.0) * 1.1 + (minute - 55.0).max(0.0) * 0.3;
                if minute >= 58.0 && m.va < -0.08 {
                    w += 6.0;
                }
                if m.yellow > 0 && self.sides[s].slots[i].pos.group() == PosGroup::Def && minute > 60.0 {
                    w += 4.0;
                }
                if diff < 0 && minute >= 60.0 && self.sides[s].slots[i].pos.group() != PosGroup::Att {
                    w += 3.0 * self.sides[s].reactivity;
                }
                w
            };
            if want >= 12.0 {
                outs.push((i, want));
            }
        }
        if outs.is_empty() {
            return;
        }
        outs.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut made = 0;
        for (slot, _) in outs.into_iter().take(if half_time { 2 } else { 3 }) {
            if self.sides[s].subs_left == 0 {
                break;
            }
            if self.substitute(s, slot, false) {
                made += 1;
            }
        }
        if made > 0 && !half_time {
            self.sides[s].windows_left -= 1;
        }
    }

    fn substitute(&mut self, s: usize, slot: usize, forced: bool) -> bool {
        if self.sides[s].subs_left == 0 {
            return false;
        }
        let out = self.sides[s].on[slot];
        if out == NONE {
            return false;
        }
        let Slot { pos, role } = self.sides[s].slots[slot];
        let best = self.sides[s]
            .bench
            .iter()
            .copied()
            .filter(|&b| !self.mp[b as usize].gone && self.mp[b as usize].slot == NONE)
            .filter(|&b| (pos == Pos::GK) == is_natural_keeper(&self.mp[b as usize].sheet) || (forced && pos != Pos::GK))
            .max_by(|&a, &b| sub_score(&self.mp[a as usize], pos, role).total_cmp(&sub_score(&self.mp[b as usize], pos, role)));
        let Some(b) = best else { return false };
        self.bring_on(s, slot, out as usize, b as usize);
        true
    }

    fn bring_on(&mut self, s: usize, slot: usize, out: usize, inn: usize) {
        self.mp[out].gone = true;
        self.mp[out].off_t = Some(self.clock);
        self.mp[out].slot = NONE;
        self.mp[inn].slot = slot as u8;
        self.mp[inn].on_t = self.clock;
        self.mp[inn].line.pos = Some(self.sides[s].slots[slot].pos);
        self.sides[s].on[slot] = inn as u8;
        self.sides[s].subs_left = self.sides[s].subs_left.saturating_sub(1);
        self.sides[s].bench.retain(|b| *b as usize != inn);
        self.half_events += 1;
        let z = self.ball;
        self.event(s, Ev::Sub, out, Some(inn), z, 0.0);
        if self.carrier == out {
            self.carrier = inn;
        }
        self.dead(20.0, 35.0);
        self.reshape(s);
    }

    // ------------------------------------------------------------ result

    fn shootout(&mut self) -> (u8, u8) {
        let order = |e: &Self, s: usize| -> SmallVec<[usize; 11]> {
            let mut v: SmallVec<[usize; 11]> = e.sides[s].on.iter().filter(|&&p| p != NONE).map(|&p| p as usize).collect();
            let sc = |i: usize| e.mp[i].eff(Attr::PenaltyTaking) + e.mp[i].eff(Attr::Composure) * 0.5;
            v.sort_by(|&a, &b| sc(b).total_cmp(&sc(a)));
            v
        };
        let takers = [order(self, 0), order(self, 1)];
        let keepers = [self.keeper(0), self.keeper(1)];
        let mut score = [0u8; 2];
        let mut taken = [0u8; 2];
        for round in 0..40usize {
            for s in 0..2 {
                let t = takers[s][round % takers[s].len().max(1)];
                let scored = self.rng.chance(self.pen_prob(t, keepers[1 - s], 0.3 + 0.1 * round as f32));
                taken[s] += 1;
                score[s] += u8::from(scored);
                self.event(s, if scored { Ev::ShootoutGoal } else { Ev::ShootoutMiss }, t, Some(keepers[1 - s]), zone_id(5, 2), 0.0);
                if round < 5 {
                    let left = |x: usize| 5 - i32::from(taken[x]);
                    let (a, b) = (i32::from(score[0]), i32::from(score[1]));
                    if a + left(0) < b || b + left(1) < a {
                        return (score[0], score[1]);
                    }
                }
            }
            if round >= 4 && score[0] != score[1] {
                return (score[0], score[1]);
            }
        }
        (score[0] + 1, score[1])
    }

    fn finish(mut self) -> MatchResult {
        let pens = (self.inp.decisive && self.level()).then(|| self.shootout());
        let end = self.clock;
        let (hg, ag) = (self.sides[0].goals, self.sides[1].goals);
        let total_poss = (self.sides[0].poss_time + self.sides[1].poss_time).max(1.0);
        self.sides[0].stats.possession = (self.sides[0].poss_time / total_poss * 100.0).round() as u8;
        self.sides[1].stats.possession = 100 - self.sides[0].stats.possession;
        let scale = self.inp.tuning.rating_scale;
        let sign = [f32::from(i8::from(hg > ag) - i8::from(hg < ag)), f32::from(i8::from(ag > hg) - i8::from(ag < hg))];

        let mut lines = Vec::with_capacity(self.mp.len());
        let mut best: Option<(f32, u8, PlayerId)> = None;
        for m in &mut self.mp {
            let played = m.line.started || m.on_t > 0.0;
            let minutes = if played { ((m.off_t.unwrap_or(end) - m.on_t) / 60.0).round().clamp(1.0, 130.0) as u8 } else { 0 };
            m.line.minutes = minutes;
            m.line.on_at = (m.on_t / 60.0) as u8;
            m.line.off_at = m.off_t.map_or(0, |t| (t / 60.0).min(130.0) as u8);
            m.line.condition_end = m.cond.round() as u8;
            m.line.zones = m.zones.take();
            let group = m.line.pos.map(Pos::group);
            m.line.is_keeper = group == Some(PosGroup::Gk);
            if minutes > 0 {
                let mut va = m.va;
                if matches!(group, Some(PosGroup::Gk | PosGroup::Def)) && m.conceded == 0 && minutes >= 60 {
                    va += 0.15;
                }
                let base = match group {
                    Some(PosGroup::Gk) => 0.55,
                    Some(PosGroup::Def) => 0.45,
                    Some(PosGroup::Mid) => 0.35,
                    _ => 0.3,
                };
                let r = 6.0 + base + scale * va * (90.0 / f32::from(minutes.max(20))).sqrt() + 0.2 * sign[m.side as usize];
                m.line.rating = (r * 10.0).round().clamp(30.0, 100.0) / 10.0;
                let key = (m.line.rating, m.line.goals, m.line.player);
                if best.is_none_or(|b| (key.0, key.1) > (b.0, b.1)) {
                    best = Some(key);
                }
            }
            lines.push(std::mem::take(&mut m.line));
        }

        MatchResult {
            home: self.sides[0].team,
            away: self.sides[1].team,
            home_goals: hg,
            away_goals: ag,
            ht: self.ht,
            extra_time: self.extra_time,
            pens,
            stats: [self.sides[0].stats, self.sides[1].stats],
            lines,
            events: self.events,
            pom: best.map_or(PlayerId::NONE, |b| b.2),
        }
    }
}

// ------------------------------------------------------------- free fns

fn drift_y(zy: usize, role: Role, traits: PlayerTraits) -> usize {
    let inside = matches!(role, Role::InsideForward | Role::InvertedWinger | Role::InvertedWingBack) || traits.contains(PlayerTraits::CUTS_INSIDE);
    let wide = matches!(role, Role::Winger | Role::WingBack) || traits.contains(PlayerTraits::HUGS_LINE);
    match zy {
        0 if inside => 1,
        4 if inside => 3,
        1 if wide => 0,
        3 if wide => 4,
        y => y,
    }
}

fn aerial_power(m: &Mp, w: W) -> f32 {
    m.sk(w) + ((f32::from(m.sheet.height) - 180.0) / 6.0).clamp(-3.0, 3.0)
}

fn is_natural_keeper(p: &PlayerSheet) -> bool {
    p.familiarity[Pos::GK.idx()] >= 15
}

fn sub_score(m: &Mp, pos: Pos, role: Role) -> f32 {
    let fam = f32::from(m.sheet.familiarity[pos.idx()]);
    m.sheet.attrs.weighted(role.key_attrs()) * (0.5 + fam / 40.0) * (m.cond / 100.0)
}

#[cfg(test)]
mod tests;
