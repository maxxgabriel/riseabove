//! The viewing context: who is looking, and therefore what they may see.
//! Every page reads the world through this type, so information limits are
//! applied in one place before any data reaches the client (plan §19.1).

use pw_core::{Attr, ClubId, CompId, NationId, PersonId, PlayerId, TeamId};
use pw_world::knowledge::{Observer, field, perceive, sigma};
use pw_world::media::{Story, StoryLink};
use pw_world::socialnet::{Frame, Post};
use pw_world::{PlayerStatus, World};

use crate::model::Ref;
use crate::session::Session;

pub struct Ctx<'a> {
    pub s: &'a Session,
    pub w: &'a World,
}

/// How a viewer knows one attribute.
#[derive(Clone, Copy, Debug)]
pub enum AttrView {
    Exact(u8),
    /// Assessed range with a central estimate.
    Range {
        lo: u8,
        hi: u8,
        mid: f32,
    },
    Unknown,
}

impl<'a> Ctx<'a> {
    pub fn new(s: &'a Session) -> Self {
        Self { s, w: s.w() }
    }

    #[inline]
    pub fn observer(&self) -> bool {
        self.s.my_person().is_none()
    }

    pub fn me(&self) -> Option<PersonId> {
        self.s.my_person()
    }

    pub fn my_player(&self) -> Option<PlayerId> {
        self.s.my_player()
    }

    pub fn my_club(&self) -> ClubId {
        self.my_player().map_or(ClubId::NONE, |p| self.w.players.hot[p].club)
    }

    pub fn my_team(&self) -> TeamId {
        self.my_player().map_or(TeamId::NONE, |p| self.w.players.hot[p].team)
    }

    pub fn is_me(&self, p: PlayerId) -> bool {
        self.my_player() == Some(p)
    }

    /// Same club (any of its teams), for information that stays inside a dressing room.
    pub fn same_club(&self, club: ClubId) -> bool {
        club.is_some() && self.my_club() == club
    }

    // ---- names -----------------------------------------------------------------

    pub fn person_name(&self, p: PersonId) -> String {
        self.w.people[p].display_name(&self.w.names).into_owned()
    }

    pub fn person_short(&self, p: PersonId) -> String {
        self.w.people[p].short_name(&self.w.names)
    }

    pub fn player_name(&self, p: PlayerId) -> String {
        self.w.player_name(p)
    }

    pub fn player_short(&self, p: PlayerId) -> String {
        self.w.player_short(p)
    }

    pub fn player_ref(&self, p: PlayerId) -> Ref {
        Ref::person(self.w.players.cold[p].person)
    }

    pub fn club_name(&self, c: ClubId) -> String {
        if c.is_some() { self.w.clubs[c].name.clone() } else { "No club".into() }
    }

    pub fn club_short(&self, c: ClubId) -> String {
        if c.is_some() { self.w.clubs[c].short_name.clone() } else { "No club".into() }
    }

    pub fn comp_name(&self, c: CompId) -> String {
        if c.is_some() { self.w.comps[c].name.clone() } else { "—".into() }
    }

    pub fn comp_short(&self, c: CompId) -> String {
        if c.is_some() { self.w.comps[c].short_name.clone() } else { "—".into() }
    }

    pub fn nation_name(&self, n: NationId) -> String {
        if n.is_some() { self.w.nations[n].name.clone() } else { "—".into() }
    }

    pub fn team_name(&self, t: TeamId) -> String {
        self.w.team_name(t)
    }

    pub fn team_short(&self, t: TeamId) -> String {
        self.w.team_short(t)
    }

    /// Navigation target for a team: youth and reserve sides open their club.
    pub fn team_ref(&self, t: TeamId) -> Ref {
        Ref::club(self.w.teams[t].club)
    }

    // ---- what the viewer may see -------------------------------------------------

    /// Private terms of a contract: the person's own, or everything for an observer.
    pub fn sees_contract(&self, p: PlayerId) -> bool {
        self.observer() || self.is_me(p)
    }

    /// Body state (condition, sharpness, morale): own, or observer.
    pub fn sees_condition(&self, p: PlayerId) -> bool {
        self.observer() || self.is_me(p)
    }

    pub fn sees_value(&self, p: PlayerId) -> bool {
        self.observer() || self.is_me(p)
    }

    pub fn sees_club_internals(&self, _c: ClubId) -> bool {
        self.observer()
    }

    /// Internal engine values (CA/PA, hidden attributes, exact trust).
    pub fn sees_internal_state(&self) -> bool {
        self.observer()
    }

    /// How the viewer knows an attribute of `p`.
    pub fn attr_view(&self, p: PlayerId, a: Attr) -> AttrView {
        let truth = self.w.players.cold[p].attrs.get(a);
        if self.observer() {
            return AttrView::Exact(self.w.players.cold[p].attrs.display(a));
        }
        let club = self.my_club();
        let me = self.me();
        let t = &self.w.data.tuning.perception;
        let (seen, judging, obs) = if club.is_some() {
            (self.w.knowledge.seen(club, p), self.w.club_manager_judging(club).0, Observer::Club(club))
        } else {
            // Without a club, only a person's own self-knowledge is available.
            let seen = self.is_me(p).then_some(pw_world::knowledge::Seen { minutes: 3000, last: self.w.date });
            (seen, 8.0, Observer::Person(me.map_or(0, |m| m.0)))
        };
        let Some(seen) = seen else { return AttrView::Unknown };
        let famous = self.w.players.cold[p].rep.world >= t.famous_reputation;
        let sg = sigma(t, Some(seen), judging, self.w.date, famous);
        let est = perceive(truth, sg, obs, p, field::ATTR + a.idx() as u64).clamp(1.0, 20.0);
        let half = (sg * 1.5).max(0.0);
        let lo = (est - half).round().clamp(1.0, 20.0) as u8;
        let hi = (est + half).round().clamp(1.0, 20.0) as u8;
        if lo == hi { AttrView::Exact(lo) } else { AttrView::Range { lo, hi, mid: est } }
    }

    /// Fixtures whose result the viewer has chosen not to see yet.
    pub fn concealed_fixtures(&self) -> Vec<&'a pw_world::Fixture> {
        if self.s.meta.concealed.is_empty() {
            return Vec::new();
        }
        self.w.fixtures.iter().map(|(_, f)| f).filter(|f| self.s.meta.concealed.contains(&f.uid) && f.score.is_some()).collect()
    }

    /// A match report that would give away a result the viewer has not revealed yet.
    pub fn story_spoils(&self, s: &Story) -> bool {
        !self.s.meta.concealed.is_empty() && matches!(self.w.media.links.get(&s.id), Some(StoryLink::Fixture { uid, .. }) if self.s.meta.concealed.contains(uid))
    }

    /// A post about a result the viewer has not revealed yet.
    pub fn post_spoils(&self, p: &Post) -> bool {
        if self.s.meta.concealed.is_empty() {
            return false;
        }
        match p.frame {
            Frame::Result { uid } | Frame::LateWinner { uid, .. } | Frame::HatTrick { uid, .. } | Frame::RedCard { uid, .. } => self.s.meta.concealed.contains(&uid),
            Frame::Story { story } => self.w.media.stories.get(story).is_some_and(|s| self.story_spoils(s)),
            _ => false,
        }
    }

    /// The headline of a story, unless it names a result the viewer is keeping unseen.
    pub fn headline(&self, s: &Story) -> String {
        if self.story_spoils(s) { "A match report, held back until you reveal the result".to_string() } else { pw_narrate::press::headline(self.w, s) }
    }

    pub fn story_body(&self, s: &Story) -> String {
        if self.story_spoils(s) { String::new() } else { pw_narrate::press::body(self.w, s) }
    }

    /// The words of a post, unless it talks about a result the viewer is keeping unseen.
    pub fn post_text(&self, p: &Post) -> String {
        if self.post_spoils(p) { "Talking about a result you have not revealed yet".to_string() } else { pw_narrate::social::post(self.w, p) }
    }

    pub fn is_concealed(&self, uid: u64) -> bool {
        self.s.meta.concealed.contains(&uid)
    }

    /// Season label like "2026/27" for a competition's nation.
    pub fn season_label(&self, comp: CompId, season: i32) -> String {
        let crosses = if comp.is_some() {
            let n = self.w.comps[comp].nation;
            n.is_some() && self.w.nations[n].season.end.year() != self.w.nations[n].season.year
        } else {
            false
        };
        if crosses { format!("{}/{:02}", season, (season + 1) % 100) } else { season.to_string() }
    }

    /// Age in years as of the world date.
    pub fn age(&self, person: PersonId) -> u32 {
        self.w.people[person].age(self.w.date)
    }

    pub fn status_label(&self, p: PlayerId) -> &'static str {
        match self.w.players.hot[p].status {
            PlayerStatus::Active => "Active",
            PlayerStatus::FreeAgent => "Free agent",
            PlayerStatus::Retired => "Retired",
            PlayerStatus::Amateur => "Amateur",
        }
    }
}
