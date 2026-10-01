//! View types for the highest-risk surface: a player as the viewer may know him (locked design 8.9, 9.4).
//!
//! The pages used to read `PlayerCold` and `PlayerHot` and decide, field by field, what to leave out. A [`VisiblePlayer`] turns that
//! around: it is built in this file and only here, through [`Ctx`], and every field the viewer may not know is an `Option` that is
//! `Some` only when the viewer's perspective allows it (the omniscient debug view, or the man himself). A page that builds from a
//! `VisiblePlayer` cannot forget a gate, because there is nothing behind the gate to forget: for a public or inhabited viewer the
//! struct does not hold the true ability, the potential, the personality, the private terms, the body state or the diagnosis.
//!
//! The structs cannot be written as literals anywhere else (they carry a private seal), so `Ctx::visible_player` and
//! `Ctx::visible_ability` are the only ways to get one. This file is on the reviewed list of files that read engine truth
//! (`tests/firewall.rs`); `tables/players.rs` and the attribute readout are not, and that test now keeps them off it.

use pw_core::{Attr, ClubId, Date, Hidden, Money, PersonId, PlayerId, Pos, TeamId};
use pw_world::{Contract, Focus, Intensity, PlayerStatus, SquadStatus};
use serde::Serialize;

use crate::ctx::{AttrView, Ctx};

/// Proof that a view type came out of this module: nothing else can name it.
#[derive(Clone, Copy, Debug)]
struct Seal;

/// What a club's own doctors know. Everyone else can see that a man is out, not why or for how long.
#[derive(Clone, Debug, Serialize)]
pub struct Medical {
    pub diagnosis: String,
    pub days: u16,
}

#[derive(Clone, Debug, Serialize)]
pub struct Injury {
    /// `Some` only for the club's own staff, the man himself and the omniscient view.
    pub medical: Option<Medical>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VisibleLoan {
    pub parent: ClubId,
    pub club: ClubId,
    pub end: Date,
}

/// A contract the viewer may read: the player's own, or any in the omniscient view.
#[derive(Clone, Debug, Serialize)]
pub struct Terms {
    pub contract: Contract,
    pub wage_now: Money,
    pub days_left: i32,
    pub squad_status: SquadStatus,
}

/// The state of the body and mind: the player's own, or the omniscient view.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Body {
    pub condition: u8,
    pub sharpness: u8,
    pub fitness: u8,
    pub fatigue: u8,
    pub morale: u8,
    pub confidence: u8,
    pub wellbeing: u8,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Reputation {
    pub current: u16,
    pub home: u16,
    pub world: u16,
}

/// What the simulation holds and nobody inside the world can read: only the omniscient debug view has it.
#[derive(Clone, Debug, Serialize)]
pub struct Engine {
    pub ability: u8,
    pub potential: u8,
    pub reputation: Reputation,
    pub personality: &'static str,
    pub bio_offset: i8,
    pub focus: Focus,
    pub intensity: Intensity,
}

/// One player as the viewer may know him. Built by [`Ctx::visible_player`] only.
#[derive(Clone, Debug, Serialize)]
pub struct VisiblePlayer {
    pub id: PlayerId,
    pub person: PersonId,
    pub is_me: bool,
    pub club: ClubId,
    pub team: TeamId,
    pub status: PlayerStatus,
    pub best_pos: Pos,
    pub loan: Option<VisibleLoan>,
    /// Out injured, as anyone can see; what it is and for how long is `Injury::medical`.
    pub injured: Option<Injury>,
    /// Matches left on a suspension.
    pub ban: u8,
    pub terms: Option<Terms>,
    /// What the player is worth: his own view of it, or the omniscient view.
    pub value: Option<Money>,
    pub body: Option<Body>,
    pub engine: Option<Engine>,
    #[serde(skip)]
    #[allow(dead_code)]
    seal: Seal,
}

/// How the viewer came to know the attributes.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum Assessment {
    /// The omniscient debug view: the values as the simulation holds them.
    Omniscient,
    /// The coaching staff of the viewer's club has watched him.
    Staff { club: ClubId, last: Date, minutes: u32 },
    /// A man's own view of himself.
    Own,
    /// Nobody the viewer has access to has watched him enough.
    Nobody,
}

/// A player's attributes as the viewer may know them, and the true values behind them only in the omniscient view.
#[derive(Clone, Debug, Serialize)]
pub struct VisibleAbility {
    pub player: PlayerId,
    pub assessment: Assessment,
    /// Every attribute, each exact, a range, or unknown (`AttrView`): never the stored value for a viewer who has not earned it.
    pub attrs: Vec<(Attr, AttrView)>,
    /// A goalkeeper's group is shown for goalkeepers, and in the omniscient view.
    pub shows_goalkeeping: bool,
    pub positions: Vec<(Pos, u8)>,
    pub engine: Option<Engine>,
    /// The hidden attributes behind the personality (omniscient view only).
    pub hidden: Option<Vec<(&'static str, u8)>>,
    #[serde(skip)]
    #[allow(dead_code)]
    seal: Seal,
}

impl Ctx<'_> {
    /// The player as this viewer may know him. Everything private is `None` unless the viewer is allowed it.
    pub fn visible_player(&self, p: PlayerId) -> VisiblePlayer {
        let w = self.w;
        let (h, cold) = (&w.players.hot[p], &w.players.cold[p]);
        let injured = (h.injury != 0).then(|| Injury {
            medical: self.sees_medical(p).then(|| Medical { diagnosis: pw_sim::health::injury_name(w, h.injury).to_string(), days: h.injury_days }),
        });
        let terms = self.sees_contract(p).then(|| Terms { contract: cold.contract.clone(), wage_now: cold.contract.current_wage(w.date), days_left: cold.contract.days_left(w.date), squad_status: cold.status });
        let body = self.sees_condition(p).then_some(Body {
            condition: h.condition,
            sharpness: h.sharpness,
            fitness: h.fitness,
            fatigue: h.fatigue,
            morale: h.morale,
            confidence: h.confidence,
            wellbeing: h.wellbeing,
        });
        VisiblePlayer {
            id: p,
            person: cold.person,
            is_me: self.is_me(p),
            club: h.club,
            team: h.team,
            status: h.status,
            best_pos: cold.best_pos,
            loan: cold.loan.as_ref().map(|l| VisibleLoan { parent: l.parent, club: l.club, end: l.end }),
            injured,
            ban: h.ban,
            terms,
            value: self.sees_value(p).then_some(cold.value),
            body,
            engine: self.sees_internal_state().then(|| Engine {
                ability: cold.ca,
                potential: cold.pa,
                reputation: Reputation { current: cold.rep.current, home: cold.rep.home, world: cold.rep.world },
                personality: w.people[cold.person].hidden.personality_label(),
                bio_offset: cold.bio_offset,
                focus: cold.plan.focus,
                intensity: cold.plan.intensity,
            }),
            seal: Seal,
        }
    }

    /// The attribute readout as this viewer may know it.
    pub fn visible_ability(&self, p: PlayerId) -> VisibleAbility {
        let w = self.w;
        let cold = &w.players.cold[p];
        let person = &w.people[cold.person];
        let assessment = if self.observer() {
            Assessment::Omniscient
        } else {
            let club = self.my_club();
            match if club.is_some() { w.knowledge.seen(club, p) } else { None } {
                Some(s) => Assessment::Staff { club, last: s.last, minutes: u32::from(s.minutes) },
                None if self.is_me(p) => Assessment::Own,
                None => Assessment::Nobody,
            }
        };
        let engine = self.visible_player(p).engine;
        VisibleAbility {
            player: p,
            assessment,
            attrs: Attr::ALL.iter().map(|&a| (a, self.attr_view(p, a))).collect(),
            shows_goalkeeping: cold.best_pos == Pos::GK || self.observer(),
            positions: Pos::ALL.iter().map(|ps| (*ps, cold.familiarity[ps.idx()])).collect(),
            hidden: engine.is_some().then(|| Hidden::ALL.iter().map(|h| (h.label(), person.hidden.get(*h))).collect()),
            engine,
            seal: Seal,
        }
    }
}
