//! Life-to-football state (locked design 7.35-7.54).
//!
//! An event never becomes a bonus or a penalty. It is *interpreted* by the person who lives it (`interpret`): the same news makes one man
//! ruminate, another angry, a third more driven, a fourth indifferent, depending on his temperament, his way of coping, how much
//! support he has and what the news is. What comes out is a set of psychological channels with a shape in time (onset, peak, a decay that
//! is sharp, steady or lingering; some things come back). Football sees them only as behaviour (`mind_for`): attention, composure,
//! appetite for risk, energy, belief. Good and bad news are the same machinery and can be present together.
//!
//! Around it: major moments that stay as scars and can return with the place or the situation; a support network that helps recovery
//! or, mishandled, makes it worse; managers who learn of a player's state only through what he tells them and what they can see,
//! and who decide what to do about it with football and the man both in mind; the match feeding back into all of it.

use pw_core::rng::{Rng, stream};
use pw_core::{ClubId, Hidden, PersonId, PlayerId, StaffAttr};
use pw_match::{Ev, MatchResult, Mind};
use pw_world::attention::Cause as Wave;
use pw_world::event::{Cause, Causes, EventKind, Fact, LifeEventKind, Visibility};
use pw_world::lifestate::*;
use pw_world::{FxHashMap, MemoryKind, SquadStatus, StaffRole, World};

use crate::consider;
use crate::selection::Selection;
use crate::tactics::MatchCtx;

// ------------------------------------------------------------------ who the person is

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Coping {
    /// Switches off; little gets in, and little is worked through.
    Avoidant,
    /// Turns pressure into work.
    Driven,
    /// Feels it out loud.
    Expressive,
    Steady,
}

/// The stable side of a person: traits, as against the state they are in.
#[derive(Clone, Copy, Debug)]
pub struct Temper {
    pub resilience: f32,
    pub coping: Coping,
    pub ambition: f32,
    pub professionalism: f32,
}

pub fn temper(w: &World, who: PersonId) -> Temper {
    let h = &w.people[who].hidden;
    let (pr, am, pf, te, co) = (h.f(Hidden::Pressure), h.f(Hidden::Ambition), h.f(Hidden::Professionalism), h.f(Hidden::Temperament), h.f(Hidden::Controversy));
    let resilience = ((pr + pf + te) / 60.0).clamp(0.0, 1.0);
    let coping = if am >= 14.0 && pf >= 12.0 {
        Coping::Driven
    } else if te <= 8.0 || co >= 14.0 {
        Coping::Expressive
    } else if pf <= 8.0 && am <= 9.0 {
        Coping::Avoidant
    } else {
        match pw_core::rng::hash_key(&[w.seed, u64::from(who.0), 0xc09e]) % 3 {
            0 => Coping::Steady,
            1 => Coping::Avoidant,
            _ => Coping::Driven,
        }
    };
    Temper { resilience, coping, ambition: am / 20.0, professionalism: pf / 20.0 }
}

// ------------------------------------------------------------------ support

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    Partner,
    Family,
    Teammates,
    Captain,
    Manager,
    Nobody,
}

#[derive(Clone, Copy, Debug)]
pub struct Support {
    /// 0..1: how much there is around him.
    pub strength: f32,
    pub best: Source,
    /// Far from home, without the language, without anyone.
    pub isolated: bool,
}

/// The people around a person, weighed (7.43). Partner and family count for most when they are near; teammates, the captain and the
/// manager count for what he thinks of them, not what they think of him.
pub fn support(w: &World, who: PersonId) -> Support {
    let life = &w.lives[who];
    let partner = life.partner().map_or(0.0, |p| f32::from(p.bond) / 100.0 * if p.lives == life.home { 1.0 } else { 0.4 });
    let par = life.household.parents;
    let family = if par.alive > 0 { f32::from(par.closeness) / 100.0 * if par.nation == life.home { 1.0 } else { 0.5 } * 0.8 } else { 0.0 };
    let p = w.people[who].player;
    let (mut mates, mut captain, mut manager) = (0.0, 0.0, 0.0);
    if p.is_some() {
        let team = w.players.hot[p].team;
        if team.is_some() {
            let mut a: Vec<f32> = w.teams[team].squad.iter().filter(|&&x| x != p).map(|&x| consider::affinity(w, w.players.cold[x].person, who).max(0.0)).collect();
            a.sort_by(|x, y| y.total_cmp(x));
            mates = a.iter().take(3).sum::<f32>() / 3.0;
            let cap = w.teams[team].captain;
            if cap.is_some() && cap != p {
                captain = consider::trust(w, who, w.players.cold[cap].person);
            }
        }
        if let Some(m) = w.manager_of_player(p) {
            manager = consider::trust(w, who, m);
        }
    }
    let parts = [(Source::Partner, partner), (Source::Family, family), (Source::Teammates, mates), (Source::Captain, captain), (Source::Manager, manager)];
    let strength = (0.3 * partner + 0.2 * family + 0.2 * mates + 0.15 * captain + 0.15 * manager).clamp(0.0, 1.0);
    let best = parts.iter().copied().filter(|x| x.1 > 0.3).max_by(|a, b| a.1.total_cmp(&b.1)).map_or(Source::Nobody, |x| x.0);
    let abroad = life.home != w.people[who].nation && w.people[who].nation.is_some();
    let isolated = strength < 0.25 || (abroad && life.fluency(life.home) < 40 && partner < 0.4);
    Support { strength, best, isolated }
}

// ------------------------------------------------------------------ interpretation

/// What an experience does to each channel at its peak for an ordinary person, before he interprets it.
fn template(kind: LoadKind) -> &'static [(Chan, i32)] {
    use Chan::*;
    match kind {
        LoadKind::Bereavement => &[(Grief, 70), (Rumination, 40), (Sleep, -40), (Focus, -40), (Motivation, -25), (Calm, -15), (Confidence, -10)],
        LoadKind::ParentIll => &[(Rumination, 45), (Sleep, -30), (Focus, -25), (Grief, 20), (Motivation, -10), (Calm, -10)],
        LoadKind::Breakup => &[(Rumination, 50), (Grief, 30), (Anger, 20), (Sleep, -30), (Focus, -30), (Confidence, -15), (Motivation, -10)],
        LoadKind::NewRelationship => &[(Excitement, 40), (Motivation, 20), (Confidence, 15), (Focus, -10), (Sleep, -10), (Risk, 10)],
        LoadKind::NewChild => &[(Excitement, 45), (Motivation, 30), (Sleep, -45), (Belonging, 20), (Focus, -10)],
        LoadKind::Marriage => &[(Excitement, 40), (Confidence, 10), (Focus, -15), (Belonging, 20)],
        LoadKind::Loneliness => &[(Rumination, 30), (Belonging, -50), (Motivation, -25), (Confidence, -15), (Sleep, -15)],
        LoadKind::FinancialTrouble => &[(Rumination, 45), (Sleep, -25), (Focus, -25), (Risk, 15), (Calm, -10)],
        LoadKind::OnlineAbuse => &[(Rumination, 45), (Anger, 30), (Confidence, -30), (Sleep, -25), (Calm, -20), (Focus, -15)],
        LoadKind::Hype => &[(Excitement, 40), (Motivation, 30), (Risk, 20), (Rumination, 10), (Sleep, -10), (Confidence, 10), (Focus, -10)],
        LoadKind::Scandal => &[(Rumination, 40), (Anger, 25), (Confidence, -25), (Focus, -25), (Belonging, -20), (Calm, -15)],
        LoadKind::CallUp => &[(Excitement, 50), (Motivation, 35), (Confidence, 20), (Belonging, 15), (Focus, -5), (Sleep, -5)],
        LoadKind::NewContract => &[(Motivation, 25), (Confidence, 20), (Belonging, 15), (Rumination, -15)],
        LoadKind::ContractWorry => &[(Rumination, 30), (Motivation, -15), (Focus, -15), (Sleep, -10)],
        LoadKind::LongInjury => &[(Motivation, 10), (Confidence, -20), (Rumination, 25), (Belonging, -10), (Risk, -15)],
        LoadKind::PublicMistake => &[(Rumination, 35), (Confidence, -30), (Risk, -30), (Focus, -15), (Anger, 10)],
        LoadKind::MissedPenalty => &[(Rumination, 50), (Confidence, -40), (Risk, -25), (Anger, 10), (Sleep, -20)],
        LoadKind::RedCard => &[(Anger, 30), (Rumination, 30), (Calm, -25), (Confidence, -15), (Motivation, 10)],
        LoadKind::Humiliation => &[(Anger, 25), (Rumination, 30), (Confidence, -25), (Motivation, 20), (Calm, -10)],
        LoadKind::Triumph => &[(Confidence, 40), (Excitement, 30), (Motivation, 20), (Risk, 15), (Focus, 10), (Calm, 10)],
    }
}

const fn onset_days(kind: LoadKind) -> u8 {
    match kind {
        LoadKind::Bereavement | LoadKind::ParentIll => 4,
        LoadKind::NewChild => 6,
        LoadKind::Hype | LoadKind::Loneliness | LoadKind::ContractWorry | LoadKind::FinancialTrouble => 5,
        LoadKind::LongInjury | LoadKind::Breakup => 2,
        _ => 1,
    }
}

/// Is a channel's movement in this direction a burden?
fn adverse(c: Chan, v: i32) -> bool {
    match c {
        Chan::Rumination | Chan::Anger | Chan::Grief => v > 0,
        Chan::Sleep | Chan::Focus | Chan::Confidence | Chan::Calm | Chan::Belonging | Chan::Motivation => v < 0,
        Chan::Excitement | Chan::Risk => false,
    }
}

/// What this person makes of this experience (7.36): the template, bent by who he is and who is around him. Nothing here is a
/// football statistic.
pub fn interpret(kind: LoadKind, mag: f32, t: &Temper, sup: &Support) -> Effects {
    let mut e = Effects::default();
    for &(c, v) in template(kind) {
        e.add(c, (v as f32 * mag).round() as i32);
    }
    let burden = ((1.3 - 0.8 * t.resilience) * (1.15 - 0.5 * sup.strength)).clamp(0.35, 1.5);
    for c in Chan::ALL {
        let v = i32::from(e.get(c));
        if v != 0 && adverse(c, v) {
            e.0[c.idx()] = (v as f32 * burden).round().clamp(-100.0, 100.0) as i8;
        }
    }
    match t.coping {
        Coping::Driven => {
            // What angers him drives him.
            let anger = i32::from(e.get(Chan::Anger)).max(0);
            e.add(Chan::Motivation, (anger as f32 * 0.9) as i32);
            e.0[Chan::Anger.idx()] = (i32::from(e.get(Chan::Anger)) / 2) as i8;
            e.0[Chan::Rumination.idx()] = (f32::from(e.get(Chan::Rumination)) * 0.7) as i8;
        }
        Coping::Avoidant => {
            // Indifference: most of it does not land, and less is worked through.
            for c in Chan::ALL {
                let v = i32::from(e.get(c));
                if v != 0 && adverse(c, v) {
                    e.0[c.idx()] = (v as f32 * 0.55) as i8;
                }
            }
            e.add(Chan::Belonging, -5);
        }
        Coping::Expressive => {
            e.0[Chan::Anger.idx()] = (f32::from(e.get(Chan::Anger)) * 1.3).clamp(-100.0, 100.0) as i8;
            e.0[Chan::Calm.idx()] = (f32::from(e.get(Chan::Calm)) * 1.3).clamp(-100.0, 100.0) as i8;
            e.0[Chan::Rumination.idx()] = (f32::from(e.get(Chan::Rumination)) * 0.8) as i8;
        }
        Coping::Steady => {}
    }
    if kind == LoadKind::Hype {
        // Fame itself is not the effect; the reading of it is (7.50).
        if t.resilience < 0.4 && sup.strength < 0.5 {
            e.add(Chan::Rumination, 30);
            e.add(Chan::Confidence, -20);
            e.0[Chan::Excitement.idx()] = (f32::from(e.get(Chan::Excitement)) * 0.5) as i8;
        } else if t.ambition > 0.65 && t.professionalism < 0.45 {
            e.add(Chan::Risk, 20);
            e.add(Chan::Focus, -15);
            e.add(Chan::Calm, -10);
        } else if sup.strength > 0.7 {
            e.0[Chan::Rumination.idx()] = (f32::from(e.get(Chan::Rumination)) * 0.3) as i8;
        }
    }
    e
}

/// Something happens to a person and he lives it. The same kind merges into a live one rather than stacking.
pub fn add_load(w: &mut World, who: PersonId, kind: LoadKind, mag: f32, cause: pw_core::EventId) {
    if mag < 0.1 {
        return;
    }
    let today = w.date;
    let t = temper(w, who);
    let sup = support(w, who);
    let effects = interpret(kind, mag.min(1.3), &t, &sup);
    if effects.is_zero() {
        return;
    }
    let expected = kind.expected_days();
    let mut actual = f32::from(expected) * (0.6 + 0.8 * (1.0 - t.resilience)) * (1.25 - 0.6 * sup.strength);
    actual *= match t.coping {
        Coping::Avoidant => 0.8,
        Coping::Expressive => 1.1,
        _ => 1.0,
    };
    let load = Load { kind, cause, since: today, onset: onset_days(kind), expected_days: expected, actual_days: actual.clamp(1.0, 900.0) as u16, tail: kind.tail(), effects, helped: 0, misfired: 0, reminded: 0 };
    let st = w.lifestate.by.entry(who).or_default();
    st.checked = today;
    if let Some(l) = st.loads.iter_mut().find(|l| l.kind == kind && l.live(today)) {
        // Again: it builds on what is left, and reminds him of the last.
        let left = l.intensity(today);
        if mag * 0.8 > left {
            let (helped, reminded) = (l.helped, l.reminded.saturating_add(1));
            *l = Load { helped, reminded, ..load };
        } else {
            l.reminded = l.reminded.saturating_add(1);
        }
        return;
    }
    if st.loads.len() >= 4 {
        // The weakest one fades out of the picture.
        if let Some(i) = st.loads.iter().enumerate().min_by(|a, b| a.1.intensity(today).total_cmp(&b.1.intensity(today))).map(|(i, _)| i) {
            st.loads.remove(i);
        }
    }
    st.loads.push(load);
}

fn add_scar(w: &mut World, who: PersonId, kind: ScarKind, venue: ClubId, weight: u8, cause: pw_core::EventId) {
    let today = w.date;
    let st = w.lifestate.by.entry(who).or_default();
    if let Some(s) = st.scars.iter_mut().find(|s| s.kind == kind && s.venue == venue) {
        s.weight = s.weight.max(weight);
        return;
    }
    if st.scars.len() >= 3
        && let Some(i) = st.scars.iter().enumerate().min_by_key(|(_, s)| s.weight).map(|(i, _)| i)
    {
        st.scars.remove(i);
    }
    st.scars.push(Scar { kind, cause, date: today, venue, weight, reactivated: 0, last: today });
}

// ------------------------------------------------------------------ state now

/// What he is carrying today, per channel: the loads in force at their present strength, on top of what his sleep, stress, confidence
/// and morale already say.
pub fn channels(w: &World, who: PersonId) -> [f32; N_CHAN] {
    let today = w.date;
    let mut c = [0.0f32; N_CHAN];
    if let Some(st) = w.lifestate.by.get(&who) {
        for l in &st.loads {
            let k = l.intensity(today);
            if k > 0.0 {
                for ch in Chan::ALL {
                    c[ch.idx()] += f32::from(l.effects.get(ch)) * k;
                }
            }
        }
    }
    let life = &w.lives[who];
    let p = w.people[who].player;
    c[Chan::Sleep.idx()] += (f32::from(life.sleep) - 70.0) * 0.5;
    c[Chan::Rumination.idx()] += (f32::from(life.stress) - 30.0).max(0.0) * 0.3;
    if p.is_some() {
        let h = &w.players.hot[p];
        c[Chan::Confidence.idx()] += (f32::from(h.confidence) - 60.0) * 0.6;
        c[Chan::Motivation.idx()] += (f32::from(h.morale) - 60.0) * 0.4;
    }
    for v in &mut c {
        *v = v.clamp(-120.0, 120.0);
    }
    c
}

/// How heavily the adverse channels weigh on him, 0..100, and the load doing most of it.
pub fn strain(w: &World, who: PersonId) -> (f32, Option<LoadKind>) {
    let Some(st) = w.lifestate.by.get(&who) else { return (0.0, None) };
    let today = w.date;
    let (mut total, mut top): (f32, Option<(f32, LoadKind)>) = (0.0, None);
    for l in &st.loads {
        let k = l.intensity(today);
        let mut adv: Vec<f32> = Chan::ALL.iter().filter_map(|&c| {
            let v = i32::from(l.effects.get(c));
            adverse(c, v).then(|| v.unsigned_abs() as f32)
        }).collect();
        adv.sort_by(|a, b| b.total_cmp(a));
        let s = k * adv.iter().take(3).sum::<f32>() / 3.0;
        total += s;
        if top.is_none_or(|t| s > t.0) && s > 0.0 {
            top = Some((s, l.kind));
        }
    }
    (total.clamp(0.0, 100.0), top.map(|t| t.1))
}

/// The state as behaviour (7.36): channels to how he plays. Different channels move different things, and pressure is not always
/// poison: a resilient, professional player under heavy stakes can sharpen (7.41).
pub fn football(ch: &[f32; N_CHAN], t: &Temper, stakes: f32) -> Mind {
    let g = |c: Chan| ch[c.idx()];
    let thrive = (t.resilience - 0.6).max(0.0) * 2.5;
    let strain = |v: f32| if v > 0.0 { v * (1.25 - 0.6 * t.resilience) } else { v };
    let mut m = Mind {
        focus: (0.5 * g(Chan::Focus) - 0.35 * strain(g(Chan::Rumination)) - 0.3 * strain(g(Chan::Grief)) + 0.1 * g(Chan::Motivation) + 0.15 * g(Chan::Sleep)) / 100.0,
        calm: (0.5 * g(Chan::Calm) - 0.4 * strain(g(Chan::Anger)) - 0.15 * strain(g(Chan::Rumination)) + 0.15 * g(Chan::Sleep) - 0.1 * g(Chan::Excitement)) / 100.0,
        risk: (0.5 * g(Chan::Risk) + 0.25 * g(Chan::Confidence) + 0.1 * g(Chan::Anger) - 0.1 * g(Chan::Grief) - 0.1 * g(Chan::Rumination)) / 100.0,
        drive: (0.6 * g(Chan::Motivation) + 0.2 * g(Chan::Excitement) - 0.25 * g(Chan::Grief) + 0.3 * g(Chan::Sleep)) / 100.0,
        confidence: (0.8 * g(Chan::Confidence) + 0.15 * g(Chan::Belonging) - 0.1 * g(Chan::Rumination)) / 100.0,
    };
    // The bigger the stake the more it magnifies what is already there, and the resilient turn it into attention and effort.
    m.calm *= 1.0 + 0.4 * stakes;
    m.focus += thrive * stakes * 0.2;
    m.drive += thrive * stakes * 0.15 * (0.5 + t.ambition);
    m.clamped()
}

/// The reactions a returning memory can bring.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reaction {
    Nervous,
    Focused,
    Avoidant,
    Angry,
    Motivated,
    Nothing,
}

impl Reaction {
    fn mind(self, k: f32) -> Mind {
        match self {
            Reaction::Nervous => Mind { calm: -0.35 * k, focus: -0.2 * k, risk: -0.2 * k, ..Mind::NEUTRAL },
            Reaction::Focused => Mind { focus: 0.25 * k, calm: 0.1 * k, ..Mind::NEUTRAL },
            Reaction::Avoidant => Mind { risk: -0.3 * k, drive: -0.15 * k, ..Mind::NEUTRAL },
            Reaction::Angry => Mind { calm: -0.3 * k, drive: 0.15 * k, ..Mind::NEUTRAL },
            Reaction::Motivated => Mind { drive: 0.3 * k, focus: 0.1 * k, ..Mind::NEUTRAL },
            Reaction::Nothing => Mind::NEUTRAL,
        }
    }
}

/// The setting a match is played in, for what it can bring back.
#[derive(Clone, Copy, Debug)]
pub struct Setting {
    pub venue: ClubId,
    pub importance: f32,
    pub uid: u64,
}

/// Which scar this setting touches for a player and how he reacts to it, if it does (7.40). Psychology moves odds, not fates: many
/// returns bring little or nothing.
pub fn returning(w: &World, p: PlayerId, s: &Setting) -> Option<(usize, Reaction, f32)> {
    let who = w.players.cold[p].person;
    let st = w.lifestate.by.get(&who)?;
    let today = w.date;
    let t = temper(w, who);
    for (i, sc) in st.scars.iter().enumerate() {
        let hit = match sc.kind {
            ScarKind::MissedDecisivePenalty => s.importance >= 0.65 && w.players.cold[p].attrs.get(pw_core::Attr::PenaltyTaking) >= 12.0,
            ScarKind::PublicMistake => false,
            _ => sc.venue.is_some() && sc.venue == s.venue,
        };
        if !hit {
            continue;
        }
        let years = sc.date.days_until(today).max(0) as f32 / 365.0;
        let k = f32::from(sc.weight) / 100.0 * (0.5 + 0.5 * pw_core::math::exp(-years / 3.0));
        if k < 0.08 {
            continue;
        }
        let mut rng = Rng::keyed(&[w.seed, stream::LIFESTATE, u64::from(who.0), sc.date.0 as u64, s.uid]);
        let weights = [
            (1.0 - t.resilience) * 1.2,
            t.resilience * t.professionalism * 1.2,
            if t.coping == Coping::Avoidant { 0.7 } else { 0.15 },
            if t.coping == Coping::Expressive { 0.7 } else { 0.15 },
            if t.coping == Coping::Driven { 0.8 } else { 0.2 },
            0.4 + t.resilience * 0.3,
        ];
        let r = [Reaction::Nervous, Reaction::Focused, Reaction::Avoidant, Reaction::Angry, Reaction::Motivated, Reaction::Nothing][rng.weighted(&weights)];
        return Some((i, r, k));
    }
    None
}

/// The state a player walks out in for this match (7.35): everything above, with what the day means to him.
pub fn mind_for(w: &World, p: PlayerId, s: &Setting) -> Mind {
    let who = w.players.cold[p].person;
    let t = temper(w, who);
    let ch = channels(w, who);
    // What he thinks is at stake grows with the match and with his own ambition.
    let stakes = (s.importance * (0.7 + 0.6 * t.ambition)).clamp(0.0, 1.0);
    let mut m = football(&ch, &t, stakes);
    if let Some((_, r, k)) = returning(w, p, s) {
        let d = r.mind(k);
        m = Mind { focus: m.focus + d.focus, calm: m.calm + d.calm, risk: m.risk + d.risk, drive: m.drive + d.drive, confidence: m.confidence + d.confidence }.clamped();
    }
    m
}

/// The states of everyone who may play, for the engine (only people with something to carry differ from an ordinary day).
pub fn minds_for(w: &World, sels: [&Selection; 2], fx: &pw_world::Fixture, importance: f32) -> FxHashMap<PlayerId, Mind> {
    let venue = if fx.neutral { ClubId::NONE } else { w.teams[fx.home].club };
    let s = Setting { venue, importance, uid: fx.uid };
    let mut out = FxHashMap::default();
    for sel in sels {
        for &p in sel.xi.iter().chain(sel.bench.iter()) {
            let m = mind_for(w, p, &s);
            if !m.is_neutral() {
                out.insert(p, m);
            }
        }
    }
    out
}

// ------------------------------------------------------------------ the world's events reach people

/// Daily: what happened in people's lives since the last look becomes something they live through.
pub fn scan(w: &mut World) {
    let cursor = w.lifestate.cursor;
    let new: Vec<(pw_core::EventId, EventKind)> = w.events.after(cursor).iter().filter(|e| matches!(e.kind, EventKind::Life { .. } | EventKind::CallUp { .. } | EventKind::ContractSigned { renewal: true, .. } | EventKind::AdaptationStruggling { .. } | EventKind::MediaGrudge { .. })).map(|e| (e.id, e.kind.clone())).collect();
    w.lifestate.cursor = w.events.last_id();
    for (id, kind) in new {
        match kind {
            EventKind::Life { person, kind } => {
                if w.people[person].player.is_none() {
                    continue;
                }
                let (k, mag) = match kind {
                    LifeEventKind::Bereavement => (LoadKind::Bereavement, 1.0),
                    LifeEventKind::ParentUnwell => (LoadKind::ParentIll, 0.8),
                    LifeEventKind::Separated { .. } => (LoadKind::Breakup, 0.9),
                    LifeEventKind::StartedDating { .. } => (LoadKind::NewRelationship, 0.6),
                    LifeEventKind::ChildBorn => (LoadKind::NewChild, 0.9),
                    LifeEventKind::Married { .. } => (LoadKind::Marriage, 0.7),
                    LifeEventKind::FinancialTrouble => (LoadKind::FinancialTrouble, 0.8),
                    LifeEventKind::Relocated { .. } => {
                        // A move is only lonely when the people and the language did not come along.
                        let sup = support(w, person);
                        if sup.isolated { (LoadKind::Loneliness, 0.8) } else { continue }
                    }
                    _ => continue,
                };
                add_load(w, person, k, mag, id);
            }
            // Not settling is lived as loneliness when it is the people and the place, as pressure when it is the head.
            EventKind::AdaptationStruggling { player, channel, .. } => {
                use pw_world::adaptation::Channel as Ch;
                match channel {
                    Ch::Social | Ch::Mental => add_load(w, w.players.cold[player].person, LoadKind::Loneliness, 0.7, id),
                    _ => {}
                }
            }
            // A story that left a lasting grudge is a scandal for a man whose name was in it.
            EventKind::MediaGrudge { subject, .. } => {
                if w.people[subject].player.is_some() {
                    add_load(w, subject, LoadKind::Scandal, 0.5, id);
                }
            }
            EventKind::CallUp { player } => add_load(w, w.players.cold[player].person, LoadKind::CallUp, 0.8, id),
            EventKind::ContractSigned { player, .. } => add_load(w, w.players.cold[player].person, LoadKind::NewContract, 0.6, id),
            _ => {}
        }
    }
}

/// Attention reaches a person (7.49-7.50): a pile-on is one experience, sudden acclaim another, and each is read by the person.
pub fn on_attention(w: &mut World, who: PersonId, cause: Wave, mag: f32, origin: pw_core::EventId) {
    if w.people[who].player.is_none() || mag < 0.4 {
        return;
    }
    let today = w.date;
    match cause {
        Wave::Controversy | Wave::Meme if mag >= 0.5 => {
            let quiet = w.lifestate.by.get(&who).is_some_and(|s| s.loads.iter().any(|l| l.kind == LoadKind::OnlineAbuse && l.since.days_until(today) < 4));
            if !quiet {
                add_load(w, who, LoadKind::OnlineAbuse, mag, origin);
            }
        }
        Wave::Football | Wave::Emotional | Wave::Personality | Wave::Aesthetic => {
            if crate::attention::level(w, who) >= 0.55 {
                add_load(w, who, LoadKind::Hype, 0.6 + 0.4 * mag, origin);
            }
        }
        _ => {}
    }
}

/// Weekly: long injuries and uncertain contracts weigh; reminders come back; support is offered and sometimes misjudged; managers
/// hear, or do not hear; they decide what to do about it.
pub fn weekly(w: &mut World) {
    let today = w.date;
    // Slow burdens that no single event announces.
    let players: Vec<PlayerId> = w.players.ids().filter(|&p| w.players.hot[p].status == pw_world::PlayerStatus::Active).collect();
    for p in players {
        let who = w.players.cold[p].person;
        let h = &w.players.hot[p];
        if h.injury_days >= 45 && !w.lifestate.by.get(&who).is_some_and(|s| s.loads.iter().any(|l| l.kind == LoadKind::LongInjury && l.live(today))) {
            add_load(w, who, LoadKind::LongInjury, (f32::from(h.injury_days) / 120.0).clamp(0.4, 1.0), pw_core::EventId::NONE);
        }
        let left = consider::contract_days_left(w, p);
        if (1..150).contains(&left) && w.players.cold[p].senior_apps >= 20 && !crate::negotiation::in_talks(w, p) && !w.lifestate.by.get(&who).is_some_and(|s| s.loads.iter().any(|l| l.kind == LoadKind::ContractWorry && l.live(today))) && w.roll(stream::LIFESTATE, &[u64::from(who.0), today.0 as u64, 0xc0]) < 0.2 {
            add_load(w, who, LoadKind::ContractWorry, 0.5, pw_core::EventId::NONE);
        }
    }
    let mut people: Vec<PersonId> = w.lifestate.by.keys().copied().collect();
    people.sort();
    for who in people {
        recover(w, who);
    }
    w.lifestate.by.retain(|_, st| !st.loads.is_empty() || !st.scars.is_empty());
    review(w);
}

/// One person's week: reminders, support attempts, scars fading.
fn recover(w: &mut World, who: PersonId) {
    let today = w.date;
    let sup = support(w, who);
    let Some(mut st) = w.lifestate.by.remove(&who) else { return };
    let mut rng = Rng::keyed(&[w.seed, stream::LIFESTATE, u64::from(who.0), today.0 as u64, 0x5e1f]);
    st.loads.retain(|l| l.live(today));
    for l in st.loads.iter_mut() {
        let k = l.intensity(today);
        // A reminder brings part of it back to life after it had faded (7.39).
        if l.tail == Tail::Recurring && k < 0.3 && l.since.days_until(today) > i32::from(l.actual_days) / 3 && rng.chance(0.08) {
            l.reminded = l.reminded.saturating_add(1);
            l.since = today.add_days(-i32::from(l.onset) - i32::from(l.actual_days) / 5);
            l.effects.scale(0.7);
        }
        // Someone tries to help. Most of it works; some of it does not (7.43).
        if sup.strength > 0.25 && k > 0.2 && rng.chance(0.35) {
            let source_bad = matches!(sup.best, Source::Manager) && w.manager_of_player(w.people[who].player).is_some_and(|m| consider::staff_attr(w, m, StaffAttr::ManManagement) < 9.0);
            let pmisfire = 0.08 + 0.15 * (1.0 - sup.strength) + if source_bad { 0.2 } else { 0.0 };
            if rng.chance(pmisfire) {
                l.misfired = l.misfired.saturating_add(1);
                l.effects.add(Chan::Rumination, 6);
                l.effects.add(Chan::Anger, 4);
                l.actual_days = l.actual_days.saturating_add(5);
            } else {
                l.helped = l.helped.saturating_add(1);
                l.actual_days = (f32::from(l.actual_days) * 0.95).max(2.0) as u16;
            }
        } else if sup.isolated && k > 0.2 && rng.chance(0.15) {
            // Nobody there: it drags.
            l.actual_days = l.actual_days.saturating_add(4);
        }
    }
    for sc in st.scars.iter_mut() {
        if sc.last.days_until(today) > 365 && sc.weight > 0 && rng.chance(0.1) {
            sc.weight = sc.weight.saturating_sub(5);
        }
    }
    st.scars.retain(|s| s.weight > 10);
    st.checked = today;
    w.lifestate.by.insert(who, st);
}

fn status_weight(s: SquadStatus) -> f32 {
    match s {
        SquadStatus::Star => 1.0,
        SquadStatus::Important => 0.85,
        SquadStatus::Regular => 0.7,
        SquadStatus::Squad | SquadStatus::ImpactSub => 0.5,
        _ => 0.3,
    }
}

/// How much the manager knows of each player's state, and what he chooses to do about it (7.44, 7.54). What reaches him is what the
/// player tells him, what is public, and what he sees in training: beliefs, never the truth.
fn review(w: &mut World) {
    let today = w.date;
    let mut people: Vec<PersonId> = w.lifestate.by.keys().copied().collect();
    people.sort();
    let mut keep: Vec<(PersonId, PersonId)> = Vec::new();
    for who in people {
        let p = w.people[who].player;
        if p.is_none() || w.players.hot[p].status != pw_world::PlayerStatus::Active {
            continue;
        }
        let Some(mgr) = w.manager_of_player(p) else { continue };
        let (sev, kind) = strain(w, who);
        let Some(kind) = kind else { continue };
        if sev < 20.0 {
            continue;
        }
        let t = temper(w, who);
        let cause = w.lifestate.by[&who].loads.iter().filter(|l| l.kind == kind).map(|l| l.since.0 as u64).next().unwrap_or(0);
        let disclosure = (0.25 + 0.45 * consider::trust(w, who, mgr) + if kind.private() { 0.0 } else { 0.35 } - if t.coping == Coping::Avoidant { 0.25 } else { 0.0 } + if t.coping == Coping::Expressive { 0.1 } else { 0.0 }).clamp(0.05, 0.95);
        let roll = w.roll(stream::LIFESTATE, &[u64::from(mgr.0), u64::from(who.0), cause, 0x4b]);
        let noticing = (0.6 * consider::staff_attr(w, mgr, StaffAttr::Motivating) / 20.0).clamp(0.0, 0.6);
        let roll2 = w.roll(stream::LIFESTATE, &[u64::from(mgr.0), u64::from(who.0), cause, 0x4c]);
        let noise = 0.75 + 0.5 * w.roll(stream::LIFESTATE, &[u64::from(mgr.0), u64::from(who.0), cause, 0x4d]);
        let (level, severity) = if roll < disclosure {
            (Awareness::Knows, (sev * noise).clamp(0.0, 100.0))
        } else if sev >= 25.0 && roll2 < noticing {
            (Awareness::Dip, (sev * 0.6 * noise).clamp(0.0, 100.0))
        } else {
            continue;
        };
        let prev = w.lifestate.known.get(&(mgr, who)).copied();
        let mut known = Known { level, kind, severity: severity as u8, since: prev.map_or(today, |k| k.since), stance: prev.map_or(Handling::SayNothing, |k| k.stance) };
        // What to do about it.
        let (stance, changed) = decide(w, mgr, who, p, &known);
        known.stance = stance;
        if changed || prev.is_none() {
            handled(w, mgr, who, p, &known);
        }
        w.lifestate.known.insert((mgr, who), known);
        keep.push((mgr, who));
    }
    // What he knew of things that have passed, he no longer needs.
    w.lifestate.known.retain(|k, _| keep.contains(k));
}

/// How much his next match within a week matters, 0..1 (0.4 when none is known).
fn next_stakes(w: &World, p: PlayerId) -> f32 {
    let team = w.players.hot[p].team;
    if team.is_none() {
        return 0.4;
    }
    w.fixtures.next_for(team, w.date, 7).map_or(0.4, |id| {
        let f = w.fixtures.get(id);
        (crate::matchday::importance(w, f.comp, f.decisive) + crate::culture::stakes(w, f)).min(1.0)
    })
}

/// The manager weighs the man and the football: how much he cares, how much the player matters, what the next match is worth.
fn decide(w: &World, mgr: PersonId, who: PersonId, p: PlayerId, k: &Known) -> (Handling, bool) {
    let today = w.date;
    let importance = status_weight(w.players.cold[p].status);
    let stakes = next_stakes(w, p);
    let s = w.people[mgr].staff;
    let (manman, ambition) = if s.is_some() { (w.staff[s].attrs.f(StaffAttr::ManManagement) / 20.0, w.people[mgr].hidden.f(Hidden::Ambition) / 20.0) } else { (0.5, 0.5) };
    let care = (0.4 * manman + 0.3 * (1.0 - ambition) + 0.3 * consider::trust(w, mgr, who)).clamp(0.0, 1.0);
    let sev = f32::from(k.severity) / 100.0;
    let club = w.playing_club(p);
    let has_help = club.is_some() && w.clubs[club].staff.iter().any(|&x| matches!(w.staff[x].role, StaffRole::Assistant | StaffRole::Physio | StaffRole::SportsScientist) && !w.staff[x].retired);
    let mut rng = Rng::keyed(&[w.seed, stream::LIFESTATE, u64::from(mgr.0), u64::from(who.0), today.0 as u64, 0xd3c]);
    let mut opts: Vec<(Handling, f32)> = vec![
        (Handling::Start, 0.35 + 0.6 * importance + 0.5 * stakes - 0.9 * care * sev),
        (Handling::Rest, 0.9 * care * sev - 0.4 * stakes - 0.2 * importance),
    ];
    if k.level == Awareness::Knows {
        opts.push((Handling::Bench, 0.5 * care * sev + 0.2 - 0.6 * stakes));
        opts.push((Handling::SimplifyRole, 0.55 * care * sev * (1.0 - 0.5 * stakes) + 0.1));
        if matches!(k.kind, LoadKind::Bereavement | LoadKind::ParentIll) && sev >= 0.6 {
            opts.push((Handling::SendHome, 1.1 * care * sev - 0.7 * stakes - 0.3 * importance));
        }
        if has_help && sev >= 0.5 {
            opts.push((Handling::SeekHelp, 0.6 * care * sev));
        }
    }
    let best = opts.iter().map(|&(h, s)| (h, s + rng.normal() * 0.1)).max_by(|a, b| a.1.total_cmp(&b.1)).map_or(Handling::Start, |x| x.0);
    (best, best != k.stance)
}

/// A decision about a struggling player is itself an event the world can judge (7.54), with effects on the people involved.
fn handled(w: &mut World, mgr: PersonId, who: PersonId, p: PlayerId, k: &Known) {
    let today = w.date;
    if k.stance == Handling::Start && k.severity < 40 {
        return;
    }
    let vis = if k.kind.private() { Visibility::Person(who) } else { Visibility::Public };
    let mut causes = Causes::new();
    causes.push(Cause::Fact(Fact::Household { person: who }));
    let cause_event = w.lifestate.by.get(&who).and_then(|s| s.loads.iter().find(|l| l.kind == k.kind)).map_or(pw_core::EventId::NONE, |l| l.cause);
    if cause_event.is_some() {
        causes.push(Cause::Event(cause_event));
    }
    let ev = w.events.push_caused(today, vis, EventKind::PersonalMatterHandled { player: p, manager: mgr, handling: k.stance, believed: k.kind }, causes);
    let stakes = next_stakes(w, p);
    w.lifestate.note_handled(Handled { date: today, manager: mgr, player: who, handling: k.stance, believed: k.kind, cause: cause_event, stakes });
    let compat = consider::compat(w, who, mgr);
    match k.stance {
        Handling::Rest | Handling::SendHome | Handling::SeekHelp | Handling::SimplifyRole => w.social.remember(who, mgr, MemoryKind::Supported, today, ev, false, 1.0, compat),
        Handling::Start if k.severity >= 60 && k.level == Awareness::Knows => w.social.remember(who, mgr, MemoryKind::LetDown, today, ev, false, 0.5, compat),
        _ => {}
    }
    match k.stance {
        Handling::SendHome => {
            w.incidents.away.insert(p, today.add_days(3));
        }
        Handling::SeekHelp => {
            if let Some(st) = w.lifestate.by.get_mut(&who) {
                for l in st.loads.iter_mut().filter(|l| l.kind == k.kind) {
                    l.helped = l.helped.saturating_add(1);
                    l.actual_days = (f32::from(l.actual_days) * 0.9) as u16;
                }
            }
        }
        _ => {}
    }
}

/// What the manager's belief about a player's state does to how he ranks him for a match (7.45): a strained player he knows about is
/// worth less today, less so the bigger the match; a confident player in form is given a little more freedom. Beliefs only.
pub fn selection_term(w: &World, manager: Option<PersonId>, p: PlayerId, importance: f32) -> f32 {
    let who = w.players.cold[p].person;
    let mut v = 0.0;
    if let Some(k) = manager.and_then(|m| w.lifestate.known.get(&(m, who))) {
        let sev = f32::from(k.severity) / 100.0;
        let weight = match k.stance {
            Handling::Rest => 0.8,
            Handling::Bench => 1.0,
            Handling::SimplifyRole => 0.15,
            Handling::SayNothing => 0.1,
            _ => 0.0,
        };
        v -= sev * weight * (1.1 - importance).max(0.0);
    }
    let h = &w.players.hot[p];
    if let Some(form) = h.form_avg() {
        v += 0.06 * ((f32::from(h.confidence) - 70.0) / 30.0).clamp(0.0, 1.0) * ((form - 6.8) / 1.5).clamp(0.0, 1.0);
    }
    v
}

// ------------------------------------------------------------------ the match feeds back

/// After a match: what happened on the pitch becomes part of the lives of those it happened to (7.46, 7.42). Mistakes, dismissals, missed
/// penalties, heavy defeats, triumphs, a serious injury; a captain's word softens; a home crowd sharpens; the world notices what a player
/// did *through* what it knew of him.
pub fn after_match(w: &mut World, fx: &pw_world::Fixture, sels: [&Selection; 2], result: &MatchResult, ctx: &MatchCtx) {
    let today = w.date;
    let venue_club = w.teams[fx.home].club;
    let venue = if fx.neutral { ClubId::NONE } else { venue_club };
    let setting = Setting { venue, importance: ctx.importance, uid: fx.uid };
    let goals = [result.home_goals, result.away_goals];
    for (i, sel) in sels.iter().enumerate() {
        let margin = i32::from(goals[i]) - i32::from(goals[1 - i]);
        let crowd = if i == 0 && !fx.neutral && w.clubs[venue_club].fan_mood < 40 { 1.3 } else { 1.0 };
        let captain = sel.captain;
        for line in result.lines.iter().filter(|l| usize::from(l.side) == i && l.minutes > 0) {
            let p = line.player;
            let who = w.players.cold[p].person;
            let started = line.started;
            let rating = line.rating;
            // Softened if the captain is on the pitch, trusted, and steady.
            let encouraged = captain.is_some() && captain != p && sel.xi.contains(&captain) && w.players.cold[captain].attrs.get(pw_core::Attr::Leadership) >= 13.0 && consider::trust(w, who, w.players.cold[captain].person) > 0.55;
            let soften = if encouraged { 0.6 } else { 1.0 };
            let own_goal = result.events.iter().any(|e| e.kind == Ev::OwnGoal && e.player == p);
            let missed = result.events.iter().any(|e| e.kind == Ev::PenaltyMiss && e.player == p);
            let shoot_miss = result.events.iter().any(|e| e.kind == Ev::ShootoutMiss && e.player == p);
            let sent_off = result.events.iter().any(|e| matches!(e.kind, Ev::Red | Ev::SecondYellow) && e.player == p);
            let cause = pw_core::EventId::NONE;
            if own_goal {
                add_load(w, who, LoadKind::PublicMistake, 0.9 * crowd * soften, cause);
            } else if started && rating < 5.0 && line.minutes >= 30 {
                add_load(w, who, LoadKind::PublicMistake, ((5.5 - rating) / 2.0).clamp(0.3, 0.9) * crowd * soften, cause);
            }
            if missed || shoot_miss {
                add_load(w, who, LoadKind::MissedPenalty, if shoot_miss { 1.0 } else { 0.6 + 0.4 * ctx.importance } * soften, cause);
                if shoot_miss || ctx.decisive || ctx.importance >= 0.75 {
                    add_scar(w, who, ScarKind::MissedDecisivePenalty, ClubId::NONE, (60.0 + 40.0 * ctx.importance) as u8, cause);
                }
            }
            if sent_off {
                add_load(w, who, LoadKind::RedCard, 0.6 + 0.4 * ctx.importance, cause);
                if ctx.importance >= 0.7 {
                    add_scar(w, who, ScarKind::RedCardInBigMatch, venue, (50.0 + 40.0 * ctx.importance) as u8, cause);
                }
            }
            if started && (margin <= -4 || (margin <= -3 && ctx.importance >= 0.7)) {
                add_load(w, who, LoadKind::Humiliation, if rating < 6.0 { 0.8 } else { 0.5 } * soften, cause);
                if ctx.importance >= 0.7 && margin <= -4 && rating < 6.0 {
                    add_scar(w, who, ScarKind::Humiliation, venue, 60, cause);
                }
            }
            if rating >= 8.6 || line.goals >= 3 {
                add_load(w, who, LoadKind::Triumph, (0.5 + 0.4 * (rating - 8.5).max(0.0)).min(1.0), cause);
            }
            if line.injured && w.players.hot[p].injury_days >= 42 {
                add_scar(w, who, ScarKind::SeriousInjury, venue, (30.0 + f32::from(w.players.hot[p].injury_days) / 3.0 + 30.0 * ctx.importance).min(100.0) as u8, cause);
            }
            // A memory that came back.
            if let Some((idx, _, _)) = returning(w, p, &setting)
                && let Some(st) = w.lifestate.by.get_mut(&who)
                && let Some(sc) = st.scars.get_mut(idx)
            {
                sc.reactivated = sc.reactivated.saturating_add(1);
                sc.last = today;
                let kind = sc.kind;
                w.events.push(today, Visibility::Public, EventKind::MemoryReturned { player: p, scar: kind });
            }
            // What he did through what he was carrying, and who noticed.
            let (heavy, main) = strain(w, who);
            if heavy >= 30.0 && line.minutes >= 45 && let Some(kind) = main {
                let mgr = w.manager_of_player(p);
                let known = mgr.is_some_and(|m| w.lifestate.known.get(&(m, who)).is_some_and(|k| k.level == Awareness::Knows));
                let visible = !kind.private() || known;
                if visible && rating >= 8.0 {
                    w.events.push(today, Visibility::Public, EventKind::PerformedThroughStrain { player: p, kind, well: true });
                    crate::attention::spark(w, who, Wave::Emotional, 0.5);
                    if let Some(m) = mgr {
                        let compat = consider::compat(w, who, m);
                        w.social.remember(who, m, MemoryKind::PublicPraise, today, pw_core::EventId::NONE, true, 0.8, compat);
                    }
                } else if visible && rating <= 5.5 {
                    w.events.push(today, Visibility::Public, EventKind::PerformedThroughStrain { player: p, kind, well: false });
                    if let Some(m) = mgr {
                        // Criticism or cover: how the manager reads it depends on the man he is.
                        let compat = consider::compat(w, who, m);
                        let care = consider::staff_attr(w, m, StaffAttr::ManManagement) >= 12.0;
                        w.social.remember(who, m, if care { MemoryKind::Protected } else { MemoryKind::Blamed }, today, pw_core::EventId::NONE, false, 0.7, compat);
                    }
                }
            }
        }
    }
}
