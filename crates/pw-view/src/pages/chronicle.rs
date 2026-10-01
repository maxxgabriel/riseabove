//! `me.chronicle`: the inhabited person's career as a living timeline. Every line is a sentence with links to the people, clubs,
//! competitions, matches and stories that produced it (`pw_world::chronicle`). Only the person's own story and what is public of the
//! people they crossed paths with: nothing here reads a hidden number.

use pw_core::{ClubId, CompId, Date, NationId, PersonId, RegionId};
use pw_world::chronicle::{Big, Join, Layer, Line, Then, Tie, TieKind};
use pw_world::event::{AwardKind, LifeEventKind, MilestoneKind, RecordKind};
use pw_world::pathway::Why;
use pw_world::recog::Learned;
use serde_json::Value;

use super::pathway::org_words;
use crate::contract::{ChronicleEntry, ChronicleReach, ChronicleTie, ChronicleView};
use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Part, Ref};

/// Shortest shared spell (days) for a teammate to be listed among the people from your past.
const LISTED_DAYS: i32 = 120;

struct S<'a> {
    c: &'a Ctx<'a>,
    v: Vec<Part>,
}

impl<'a> S<'a> {
    fn new(c: &'a Ctx<'a>) -> Self {
        Self { c, v: Vec::new() }
    }
    fn t(mut self, t: impl Into<String>) -> Self {
        self.v.push(Part::t(t));
        self
    }
    fn person(mut self, p: PersonId) -> Self {
        if p.is_some() && self.c.w.people.get(p).is_some() {
            self.v.push(Part::l(Ref::person(p), self.c.person_name(p)));
        } else {
            self.v.push(Part::t("someone"));
        }
        self
    }
    fn club(mut self, k: ClubId) -> Self {
        if k.is_some() && self.c.w.clubs.get(k).is_some() {
            self.v.push(Part::l(Ref::club(k), self.c.club_name(k)));
        } else {
            self.v.push(Part::t("a club"));
        }
        self
    }
    fn comp(mut self, k: CompId) -> Self {
        if k.is_some() && self.c.w.comps.get(k).is_some() {
            self.v.push(Part::l(Ref::comp(k), self.c.comp_name(k)));
        } else {
            self.v.push(Part::t("a competition"));
        }
        self
    }
    fn nation(mut self, n: NationId) -> Self {
        if n.is_some() && self.c.w.nations.get(n).is_some() {
            self.v.push(Part::l(Ref::nation(n), self.c.nation_name(n)));
        } else {
            self.v.push(Part::t("the national side"));
        }
        self
    }
    fn inst(self, i: u32) -> Self {
        let name = inst_name(self.c, i);
        self.t(name)
    }
    fn region(self, r: RegionId) -> Self {
        let name = region_name(self.c, r);
        self.t(name)
    }
    fn date(mut self, d: Date) -> Self {
        self.v.push(Part::date(d));
        self
    }
    fn parts(self, more: Vec<Part>) -> Self {
        let mut s = self;
        s.v.extend(more);
        s
    }
    fn done(self) -> Vec<Part> {
        self.v
    }
}

fn inst_name(c: &Ctx, i: u32) -> String {
    c.w.minor.institutions.get(i as usize).map_or_else(|| "a university".into(), |x| x.name.clone())
}

fn region_name(c: &Ctx, r: RegionId) -> String {
    c.w.ext.ecosystem.regions.get(r).map_or_else(|| "home".into(), |x| x.name.clone())
}

fn tier_words(tier: u8) -> &'static str {
    match tier {
        3 => "a full football scholarship",
        2 => "a scholarship covering tuition and hostel",
        1 => "a tuition scholarship",
        _ => "a place in the football programme",
    }
}

fn learned_you(l: Learned) -> &'static str {
    match l {
        Learned::Watched => "watched you play",
        Learned::Recommended => "heard about you from someone they trusted",
        Learned::Came => "saw you when you came to them",
        Learned::Buzz => "heard the talk about you and came to look",
    }
}

fn award_words(a: AwardKind) -> String {
    a.label()
}

fn milestone_words(k: MilestoneKind, n: u16) -> String {
    match k {
        MilestoneKind::ClubApps => format!("{n} appearances for "),
        MilestoneKind::CareerGoals => format!("{n} career goals, the latest for "),
        MilestoneKind::SeniorApps => format!("{n} senior appearances, the latest for "),
        MilestoneKind::Caps => format!("{n} caps, while at "),
    }
}

fn record_words(k: RecordKind) -> &'static str {
    match k {
        RecordKind::ClubTopScorer => "became the all-time top scorer of ",
        RecordKind::ClubMostApps => "became the most-capped player in the history of ",
        RecordKind::ClubRecordSigning => "became the record signing of ",
        RecordKind::ClubRecordSale => "became the record sale of ",
        RecordKind::ClubBiggestWin => "played in the biggest win in the history of ",
        RecordKind::LeagueGoalsInSeason => "set a league record for goals in a season, at ",
        RecordKind::NationMostCaps => "became your country's most-capped player, while at ",
        RecordKind::NationTopScorer => "became your country's top scorer, while at ",
        RecordKind::WorldRecordFee => "moved for a world-record fee, to ",
    }
}

fn result_words(r: i8) -> &'static str {
    match r {
        1.. => " (won)",
        0 => " (drawn)",
        _ => " (lost)",
    }
}

/// Whether the crossing is still going on: a teammate still at the same club, a manager still in charge of you.
fn current(c: &Ctx, me: PersonId, t: &Tie) -> bool {
    let w = c.w;
    let mine = w.people.get(me).map_or(pw_core::PlayerId::NONE, |x| x.player);
    let theirs = w.people.get(t.person).map_or(pw_core::PlayerId::NONE, |x| x.player);
    match t.kind {
        TieKind::Teammate { club } => mine.is_some() && theirs.is_some() && w.players.hot[mine].club == club && w.players.hot[theirs].club == club,
        TieKind::Classmate { inst } => mine.is_some() && theirs.is_some() && w.minor.member_of.get(&mine) == Some(&inst) && w.minor.member_of.get(&theirs) == Some(&inst),
        TieKind::Coach { club } => {
            mine.is_some() && w.players.hot[mine].club == club && w.clubs.get(club).is_some_and(|k| k.manager.is_some() && w.staff[k.manager].person == t.person)
        }
        _ => false,
    }
}

/// "your former teammate at Odisha FC", "the first at KIIT to take you seriously" ...
pub(crate) fn tie_parts(c: &Ctx, t: &Tie, now: bool) -> Vec<Part> {
    let s = S::new(c);
    match t.kind {
        TieKind::Teammate { club } => s.t(if now { "your teammate at " } else { "your former teammate at " }).club(club),
        TieKind::Classmate { inst } => s.t(if now { "your teammate at " } else { "your former teammate at " }).inst(inst),
        TieKind::Coach { club } => s.t(if now { "your manager at " } else { "your former manager at " }).club(club),
        TieKind::Scout { org } => s.t(format!("the first at {} to take you seriously", org_words(c, org))),
        TieKind::LetGo { club } => s.t("who let you go at ").club(club),
        TieKind::Mentor => s.t("your old mentor"),
        TieKind::Finder => s.t("the first person outside your family to take you seriously"),
    }
    .done()
}

fn then_parts(c: &Ctx, t: Then) -> Vec<Part> {
    let s = S::new(c);
    match t {
        Then::BecameManager { club } => s.t("became manager of ").club(club),
        Then::JoinedStaff { club, role } => s.t(format!("joined the staff of {} as {}", c.club_name(club), role.label().to_lowercase())),
        Then::Moved { club } => s.t("moved to ").club(club),
        Then::Capped { nation } => s.t("won a first cap for ").nation(nation),
        Then::Honoured { award } => s.t(format!("won {}", award_words(award).to_lowercase())),
        Then::Retired => s.t("retired"),
        Then::JoinedYourClub { club } => s.t("joined you at ").club(club),
        Then::ManagesYou { club } => s.t("became your manager at ").club(club),
    }
    .done()
}

fn life_parts(c: &Ctx, k: LifeEventKind) -> Vec<Part> {
    let s = S::new(c);
    match k {
        LifeEventKind::StartedDating { partner } => s.t("Started seeing ").person(partner),
        LifeEventKind::MovedIn { partner } => s.t("Moved in with ").person(partner),
        LifeEventKind::Married { partner } => s.t("Married ").person(partner),
        LifeEventKind::Separated { partner } => s.t("Separated from ").person(partner),
        LifeEventKind::ChildBorn => s.t("Became a parent"),
        LifeEventKind::ParentUnwell => s.t("A parent fell ill"),
        LifeEventKind::ParentRecovered => s.t("A parent recovered"),
        LifeEventKind::Bereavement => s.t("Lost someone close"),
        LifeEventKind::Relocated { nation } => {
            // Abroad, the clock is part of the distance: how far home is ahead or behind.
            let family = c.me().map_or(NationId::NONE, |me| pw_world::chat::family_nation(c.w, me));
            let s = s.t("Moved your life to ").nation(nation);
            match clock_words(c, family, nation) {
                Some(w) => s.t(format!(". {w}")),
                None => s,
            }
        }
        LifeEventKind::PartnerJoinedMove { partner } => s.person(partner).t(" came with you"),
        LifeEventKind::PartnerStayedBehind { partner } => s.person(partner).t(" stayed behind"),
        LifeEventKind::FinancialTrouble => s.t("Money became a worry"),
        LifeEventKind::Graduated => s.t("Completed another stage of your education"),
    }
    .done()
}

fn hours(n: i8) -> String {
    match n.unsigned_abs() {
        1 => "an hour".into(),
        h => format!("{h} hours"),
    }
}

/// "Home is 4 hours ahead": how the clock at home (`home`, where the family is) stands against `here`, said as roughly when either
/// nation's hours are inferred rather than known. `None` when they keep the same hours or either is unknown.
pub(crate) fn clock_words(c: &Ctx, home: NationId, here: NationId) -> Option<String> {
    if home.is_none() || here.is_none() || home == here {
        return None;
    }
    let (a, b) = (&c.w.nations.get(home)?.env, &c.w.nations.get(here)?.env);
    let ahead = a.tz - b.tz;
    if ahead == 0 {
        return None;
    }
    let about = if a.known && b.known { "" } else { "about " };
    Some(format!("Home is {about}{} {}", hours(ahead), if ahead > 0 { "ahead" } else { "behind" }))
}

/// A home's quality (1–5) as a word, before "rented place" or "home".
pub(crate) fn quality_word(q: u8) -> &'static str {
    match q {
        1 => "small ",
        2 => "modest ",
        3 => "comfortable ",
        4 => "spacious ",
        5 => "luxurious ",
        _ => "",
    }
}

fn why_parts(c: &Ctx, why: Why) -> Option<Vec<Part>> {
    let s = S::new(c);
    Some(
        match why {
            Why::AcademyInvite { club } => s.t("Invited into the academy at ").club(club),
            Why::ReleasedByAcademy { club } => s.t("Released by the academy at ").club(club),
            Why::UniversityScholarship { inst, tier } => s.t("Joined ").inst(inst).t(format!(" on {}", tier_words(tier))),
            Why::StateSelection { state } => s.t("Selected by ").region(state),
            Why::DistrictSelection { district } => s.t("Picked by the selectors of ").region(district),
            Why::CampCall => s.t("Called to a national identification camp"),
            Why::ProfessionalTrial { club } => s.t("Invited to a trial at ").club(club),
            Why::ScoutRecommendation { scout, club } => s.person(scout).t(" recommended you to ").club(club),
            Why::CoachRecommendation { .. } => s.t("Recommended by your coach"),
            Why::Promotion { club } => s.t("Went up with ").club(club),
            Why::Emerged | Why::Enrolled | Why::Signed { .. } | Why::Transfer { .. } | Why::ChosenStart => return None,
        }
        .done(),
    )
}

/// The sentence, its category and the keepsake it left.
fn render(c: &Ctx, life: &pw_world::chronicle::Life, line: Line) -> Option<(Vec<Part>, &'static str, Option<&'static str>)> {
    let s = S::new(c);
    let w = c.w;
    Some(match line {
        Line::Began { region, institution, finder } => {
            let mut s = s.t("Grew up in ").region(region);
            if let Some(i) = institution {
                s = s.t(", playing for ").inst(i);
            }
            if finder.is_some() {
                s = s.t(". ").person(finder).t(" was the first outside your family to take you seriously");
            }
            (s.done(), "people", None)
        }
        Line::Step { why, .. } => {
            let cat = match why {
                Why::StateSelection { .. } | Why::DistrictSelection { .. } | Why::CampCall => "international",
                _ => "moves",
            };
            let keep = match why {
                Why::UniversityScholarship { .. } => Some("scholarship"),
                Why::ProfessionalTrial { .. } => Some("trial"),
                Why::CampCall | Why::StateSelection { .. } => Some("call_up"),
                _ => None,
            };
            (why_parts(c, why)?, cat, keep)
        }
        Line::Joined { club, how } => {
            let s = match how {
                Join::Transfer => s.t("Joined ").club(club),
                Join::Loan => s.t("Joined ").club(club).t(" on loan"),
                Join::LoanReturn => s.t("Back at ").club(club).t(" after the loan"),
                Join::Academy => s.t("Joined the academy at ").club(club),
                Join::Signed => s.t("Signed for ").club(club),
            };
            (s.done(), "moves", (how == Join::Transfer).then_some("transfer"))
        }
        Line::Released { club } => (s.t("Released by ").club(club).done(), "moves", None),
        Line::Contract { club, first, renewal, until } => {
            let s = if first {
                s.t("Signed your first professional contract, with ").club(club).t(", until ")
            } else if renewal {
                s.t("Extended your contract with ").club(club).t(" until ")
            } else {
                s.t("Signed a contract with ").club(club).t(" until ")
            };
            let mut s = s.date(until);
            // Your own terms, kept when the contract was signed.
            if let Some(wage) = life.entries.iter().find_map(|e| match e.line {
                Line::Terms { club: k, until: u, wage } if k == club && u == until => Some(wage),
                _ => None,
            }) && wage > 0
            {
                s = s.t(format!(", on {} a week", pw_narrate::lang::money_text(w, wage)));
            }
            (s.done(), "moves", Some("contract"))
        }
        Line::Trial { club } => (s.t("Began a trial at ").club(club).done(), "moves", Some("trial")),
        Line::TrialOutcome { club, offered } => {
            let s = if offered { s.club(club).t(" wanted you after the trial") } else { s.club(club).t(" let you go after the trial") };
            (s.done(), "moves", None)
        }
        Line::Scholarship { inst, tier, contested } => {
            let mut s = s.t("Joined ").inst(inst).t(format!(" on {}", tier_words(tier)));
            if contested {
                s = s.t(", ahead of other programmes that wanted you");
            }
            (s.done(), "moves", Some("scholarship"))
        }
        Line::Enrolled { inst } => (s.t("Enrolled at ").inst(inst).done(), "moves", None),
        Line::Season { year, club, apps, starts, goals, assists, rating10, benched, omitted } => {
            let plural = |n: u16, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
            let mut text = format!(": {}", plural(apps, "appearance", "appearances"));
            if apps > 0 {
                text.push_str(&format!(" ({} as a starter)", starts));
                text.push_str(&format!(", {}, {}", plural(goals, "goal", "goals"), plural(assists, "assist", "assists")));
                if rating10 > 0 {
                    text.push_str(&format!(", average rating {:.1}", f32::from(rating10) / 10.0));
                }
            }
            if benched + omitted > apps {
                let mut idle = Vec::new();
                if benched > 0 {
                    idle.push(format!("on the bench without playing {}", plural(benched, "time", "times")));
                }
                if omitted > 0 {
                    idle.push(format!("left out of the squad {}", plural(omitted, "time", "times")));
                }
                text.push_str(&format!("; {}", idle.join(", ")));
            }
            // How far the season took you, when the clubs are on the map.
            if let Some(t) = c.me().and_then(|me| w.ext.journeys.of(me, year, club)).filter(|t| t.trips > 0) {
                text.push_str(&format!("; {} km on the road to {} away {}", crate::fmt::thousands(t.km), t.trips, if t.trips == 1 { "match" } else { "matches" }));
            }
            (s.t(format!("{year} at ")).club(club).t(text).done(), "football", None)
        }
        Line::Exams { passed } => (s.t(if passed { "Passed your school exams" } else { "Did not pass your school exams" }).done(), "life", passed.then_some("certificate")),
        Line::MinorSeason { history, apps, goals, won, runner_up, top_scorer, best } => {
            let h = w.minor.history.get(history as usize)?;
            let name = pw_narrate::history::comp_name(w, h.kind, h.nation, &h.region);
            let side = pw_narrate::history::entrant(w, h.winner);
            let season = format!("{}/{:02}", h.season, (h.season + 1) % 100);
            let mut text = format!("{name} {season}: {apps} appearance{}, {goals} goal{}", if apps == 1 { "" } else { "s" }, if goals == 1 { "" } else { "s" });
            if won {
                text.push_str(&format!(". Champions with {side}"));
            } else if runner_up {
                text.push_str(". Runners-up");
            }
            if top_scorer {
                text.push_str(". Top scorer of the competition");
            }
            if best {
                text.push_str(". Its best player");
            }
            (s.t(text).done(), if won || top_scorer || best { "honours" } else { "football" }, (won || top_scorer || best).then_some("medal"))
        }
        Line::Graduated { inst } => (s.t("Graduated from ").inst(inst).done(), "life", Some("certificate")),
        Line::Noticed { org, by, how } => {
            let s = if by.is_some() { s.person(by).t(format!(" of {} first {}", org_words(c, org), learned_you(how))) } else { s.t(format!("{} first {}", org_words(c, org), learned_you(how))) };
            (s.done(), "recognition", None)
        }
        Line::Debut { club, comp, .. } => (s.t("Senior debut, for ").club(club).t(" in the ").comp(comp).done(), "football", Some("team_sheet")),
        Line::FirstGoal { club, comp, .. } => (s.t("First senior goal, for ").club(club).t(" in the ").comp(comp).done(), "football", None),
        Line::Match { opp, comp, goals, result, big, .. } => {
            let lead = match big {
                Big::HatTrick => s.t("Hat-trick against "),
                Big::Winner => s.t("Scored the late winner against "),
                Big::Decisive => s.t(if goals >= 2 { format!("Scored {goals} in a knockout tie against ") } else { "Scored in a knockout tie against ".into() }),
                Big::Derby if goals > 0 => s.t(if goals >= 2 { format!("Scored {goals} in the derby against ") } else { "Scored in the derby against ".into() }),
                Big::Derby => s.t("Best on the pitch in the derby against "),
                Big::Brace => s.t(format!("Scored {goals} against ")),
                Big::BigOccasion => s.t("Scored in a match that mattered, against "),
                Big::BestOnPitch => s.t("Best player on the pitch against "),
                Big::Final => s.t("Played in the final against "),
                Big::Decider => s.t(if goals >= 2 { format!("Scored {goals}, the last of them the winner, against ") } else { "Scored the winner against ".into() }),
            };
            (lead.club(opp).t(", ").comp(comp).t(result_words(result)).done(), "football", None)
        }
        Line::StateSide { state } => (s.t("Selected for ").region(state).t("'s state team").done(), "international", Some("call_up")),
        Line::NationalSquad { nation } => (s.t("Called up to the ").nation(nation).t(" squad").done(), "international", Some("call_up")),
        Line::Capped { nation } => (s.t("First cap for ").nation(nation).done(), "international", Some("cap")),
        Line::WithdrewFromSquad { nation } => (s.t("Withdrew from the ").nation(nation).t(" squad").done(), "international", None),
        Line::Injury { injury, days } => {
            let name = pw_sim::health::injury_name(w, injury).to_lowercase();
            let weeks = (u32::from(days) + 6) / 7;
            (s.t(format!("Injured: {name}, about {weeks} week{} out", if weeks == 1 { "" } else { "s" })).done(), "injury", None)
        }
        Line::Setback { days } => (s.t(format!("A setback in your recovery: about {days} more days")).done(), "injury", None),
        Line::Recovered => (s.t("Back in full training after the injury").done(), "injury", None),
        Line::Honour { award, comp, season } => (s.t(format!("{}, ", award_words(award))).comp(comp).t(format!(" {season}")).done(), "honours", Some("medal")),
        Line::Title { comp, club, .. } => (s.t("Won the ").comp(comp).t(" with ").club(club).done(), "honours", Some("medal")),
        Line::Promoted { comp, club } => (s.t("Promoted with ").club(club).t(" from the ").comp(comp).done(), "honours", None),
        Line::Relegated { comp, club } => (s.t("Relegated with ").club(club).t(" from the ").comp(comp).done(), "football", None),
        Line::Captain { club } => (s.t("Named captain of ").club(club).done(), "football", None),
        Line::Mentor { mentor } => (s.person(mentor).t(" began to look out for you").done(), "people", None),
        Line::Milestone { kind, count, club } => (s.t(milestone_words(kind, count)).club(club).done(), "football", None),
        Line::Record { kind, club, .. } => (s.t("You ").t(record_words(kind)).club(club).done(), "honours", None),
        Line::Breakout => (s.t("Broke through: people in football started to talk about you").done(), "recognition", None),
        Line::Press { story, layer, first } => {
            let st = w.media.stories.get(story)?;
            let outlet = pw_narrate::press::outlet_name(w, st);
            let head = c.headline(st);
            if st.kind == pw_world::media::StoryKind::TransferRumour && st.other_club.is_some() {
                let s = s.t(format!("{outlet} linked you with ")).club(st.other_club).t(format!(": \u{201c}{head}\u{201d}"));
                return Some((s.done(), "recognition", Some("clipping")));
            }
            let lead = match (first, layer) {
                (true, Layer::Local) => "First written about, in ".to_string(),
                (true, Layer::National) => "First national coverage, in ".to_string(),
                (true, Layer::Abroad) => "First coverage abroad, in ".to_string(),
                (false, _) => "Written about in ".to_string(),
            };
            (s.t(format!("{lead}{outlet}: \u{201c}{head}\u{201d}")).done(), "recognition", Some("clipping"))
        }
        Line::Endorsed { brand } => {
            let name = w.commerce.brands.get(brand as usize).map_or_else(|| "a brand".into(), |b| b.name.clone());
            (s.t(format!("Signed an endorsement with {name}")).done(), "recognition", None)
        }
        Line::Life { kind } => (life_parts(c, kind), "life", None),
        Line::MovedHome { bought } => (s.t(if bought { "Bought a home" } else { "Moved home" }).done(), "life", None),
        Line::Retired => (s.t("Retired from playing").done(), "football", None),
        Line::Meanwhile { who, tie, then } => {
            let t = life.ties.get(usize::from(tie))?;
            // Someone who stops before thirty gave the game up; "retired" is for a career.
            let young = then == Then::Retired && w.people.get(who).is_some_and(|x| x.age(c.w.date) < 30);
            let then = if young { S::new(c).t("gave up playing").done() } else { then_parts(c, then) };
            (s.person(who).t(", ").parts(tie_parts(c, t, false)).t(", ").parts(then).done(), "people", None)
        }
        Line::AtClub { club, news } => {
            use pw_world::chronicle::ClubNews as N;
            let s = match news {
                N::NewManager { who } => s.person(who).t(" became manager of ").club(club),
                N::ManagerSacked { who } => s.club(club).t(" sacked ").person(who),
                N::Takeover { owner } => s.person(owner).t(" took over ").club(club),
                N::Administration => s.club(club).t(" went into administration"),
                N::PointsDeducted { points } => s.club(club).t(format!(" were docked {points} points")),
                N::Investment => s.t("The owner put money into ").club(club),
                N::Facility { kind } => s.club(club).t(format!(" opened their new {}", kind.label())),
            };
            (s.done(), "club", None)
        }
        Line::FirstGoalFor { club, .. } => (s.t("First goal for ").club(club).done(), "football", None),
        Line::BackWithGroup => (s.t("Back training with the group, part of the way back").done(), "injury", None),
        Line::AskedIfReady { by } => (s.person(by).t(" asked whether you could play before the medical staff had cleared you").done(), "injury", None),
        Line::Comeback { club, opp, days, .. } => {
            let weeks = (u32::from(days) + 3) / 7;
            let away = if weeks >= 2 { format!(", after {weeks} weeks out") } else { String::new() };
            (s.t("Back on the pitch for ").club(club).t(" against ").club(opp).t(away).done(), "injury", None)
        }
        Line::NothingCameOfIt { club, .. } => (s.t("Nothing came of the talk linking you with ").club(club).done(), "recognition", None),
        Line::Answered { story, answer, corrected } => {
            let st = w.media.stories.get(story)?;
            let outlet = pw_narrate::press::outlet_name(w, st);
            let head = c.headline(st);
            let tail = if corrected {
                ". They later corrected it".to_string()
            } else {
                let by = w.media.stories.get(answer).map(|a| pw_narrate::press::outlet_name(w, a)).filter(|o| *o != outlet);
                by.map_or_else(|| ". It was denied".to_string(), |o| format!(". It was denied, in {o}"))
            };
            (s.t(format!("{outlet} wrote \u{201c}{head}\u{201d}{tail}")).done(), "recognition", None)
        }
        Line::Terms { .. } => return None,
        Line::TrialViews { club, keen, doubtful } => {
            let role = |q: PersonId| w.people.get(q).filter(|x| x.staff.is_some()).map(|x| w.staff[x.staff].role.label().to_lowercase()).unwrap_or_else(|| "staff".into());
            let s = s.club(club).t(" told you their people had not seen you the same way: ").person(keen).t(format!(", their {}, rated you higher than ", role(keen))).person(doubtful).t(format!(", their {}", role(doubtful)));
            (s.done(), "recognition", None)
        }
        Line::Language { nation, level } => {
            // Nations carry a language family, not a name for it: say what it is to live in it.
            let how = match level {
                1 => "Getting by in the language in ",
                2 => "Comfortable in the language in ",
                _ => "Fluent in the language in ",
            };
            (s.t(how).nation(nation).done(), "life", None)
        }
        Line::Talked { reach, club, nation, region } => {
            use pw_world::chronicle::FanReach as R;
            let s = match reach {
                R::OwnClub => s.club(club).t(" supporters started talking about you online"),
                R::OtherClub => s.t("Supporters of ").club(club).t(" started talking about you too"),
                R::OtherState if region.is_some() => s.t("People in ").region(region).t(" started talking about you"),
                R::OtherState => s.t("People beyond your home state started talking about you"),
                R::Abroad => s.t("Fans in ").nation(nation).t(" started talking about you"),
            };
            (s.done(), "recognition", None)
        }
        Line::Spotted { spot, club, nation } => {
            use pw_world::chronicle::Spot;
            let s = match spot {
                Spot::Street => s.t("Asked for your autograph in the street, the first time a stranger had"),
                Spot::Training => s.t("People waiting outside training at ").club(club).t(" asked for your autograph, the first time anyone had"),
                Spot::AwayTrip => s.t("Photographed at the airport for the first time, on the way to play ").club(club),
                Spot::SquadTrip => s.t("Photographed at the airport for the first time, on the way to join the ").nation(nation).t(" squad"),
                Spot::Moving => s.t("Photographed at the airport for the first time, on the way to ").club(club),
            };
            (s.done(), "recognition", None)
        }
        Line::NewPlace { club, home, quality } => {
            use pw_world::affairs::HomeKind as H;
            let s = match home {
                H::Family => s.t("Moved in with a host family the club found, near ").club(club),
                H::Digs => s.t("Settled into the club's digs at ").club(club).t(", while looking for a place of your own"),
                H::Rented => s.t(format!("Found a {}rented place near ", quality_word(quality))).club(club),
                H::Owned => s.t(format!("Bought a {}home near ", quality_word(quality))).club(club),
            };
            (s.done(), "life", None)
        }
        Line::Faced { who, tie, club, .. } => {
            let t = life.ties.get(usize::from(tie))?;
            let role = if matches!(t.kind, TieKind::Coach { .. } | TieKind::LetGo { .. }) { ", in charge of " } else { ", now at " };
            (s.t("Came up against ").person(who).t(", ").parts(tie_parts(c, t, false)).t(role).club(club).done(), "people", None)
        }
    })
}

/// One line of a person's story in words, for other pages (Today's "on this day").
pub(crate) fn line_parts(c: &Ctx, life: &pw_world::chronicle::Life, line: Line) -> Option<Vec<Part>> {
    render(c, life, line).map(|(parts, _, _)| parts)
}

fn line_uid(l: Line) -> Option<u64> {
    match l {
        Line::Debut { uid, .. } | Line::FirstGoal { uid, .. } | Line::Match { uid, .. } | Line::Faced { uid, .. } | Line::FirstGoalFor { uid, .. } | Line::Comeback { uid, .. } if uid != u64::MAX => Some(uid),
        _ => None,
    }
}

/// Where someone is now, in a few words.
fn now_words(c: &Ctx, who: PersonId) -> Option<String> {
    let w = c.w;
    let p = w.people.get(who)?;
    if p.staff.is_some() {
        let s = &w.staff[p.staff];
        if s.employed() {
            return Some(format!("{} at {}", s.role.label(), c.club_name(s.club)));
        }
    }
    if p.player.is_some() {
        let h = &w.players.hot[p.player];
        // Senior caps are public: what became of them includes how far they went.
        let caps = w.intl.caps.get(&p.player).and_then(|v| v.iter().filter(|x| x.level == pw_world::intl::Level::Senior).max_by_key(|x| x.caps)).map(|x| {
            format!("; {} {} for {}", x.caps, if x.caps == 1 { "cap" } else { "caps" }, c.nation_name(x.nation))
        });
        let caps = caps.unwrap_or_default();
        if h.status == pw_world::PlayerStatus::Retired {
            return Some(format!("Retired{caps}"));
        }
        if h.club.is_some() {
            return Some(format!("{}{caps}", c.club_name(h.club)));
        }
        return Some(format!("Without a club{caps}"));
    }
    None
}

pub fn chronicle(c: &Ctx) -> ApiResult<Value> {
    let me = c.me().ok_or_else(|| ApiError::Unauthorized("You are observing the world. Inhabit someone to read their story.".into()))?;
    let w = c.w;
    let empty = pw_world::chronicle::Life::default();
    let life = w.ext.chronicle.of(me).unwrap_or(&empty);
    let mut entries: Vec<ChronicleEntry> = Vec::with_capacity(life.entries.len());
    for e in &life.entries {
        let Some((parts, cat, keep)) = render(c, life, e.line) else { continue };
        entries.push(ChronicleEntry {
            date: e.date.0,
            cat: cat.into(),
            parts,
            uid: line_uid(e.line),
            story: match e.line {
                Line::Press { story, .. } => Some(story.0),
                _ => None,
            },
            learned: e.learned.map(|d| d.0),
            keepsake: keep.map(Into::into),
        });
    }
    entries.sort_by_key(|e| e.date);

    // The people from your past: anyone but a brief teammate, the most significant crossing for each person, latest first.
    let mut seen: std::collections::HashSet<PersonId> = std::collections::HashSet::new();
    let mut people: Vec<(i32, ChronicleTie)> = Vec::new();
    for (i, t) in life.ties.iter().enumerate() {
        if t.person == me || seen.contains(&t.person) || w.people.get(t.person).is_none() {
            continue;
        }
        let Some(best) = life.best_tie(t.person) else { continue };
        if best != i {
            continue;
        }
        if matches!(t.kind, TieKind::Teammate { .. } | TieKind::Classmate { .. }) && t.days() < LISTED_DAYS {
            continue;
        }
        seen.insert(t.person);
        let now = current(c, me, t);
        people.push((t.to.0, ChronicleTie { who: Named::new(Ref::person(t.person), c.person_name(t.person)), how: tie_parts(c, t, now), from: t.from.0, to: t.to.0, now: now_words(c, t.person) }));
    }
    people.sort_by_key(|(to, _)| std::cmp::Reverse(*to));

    // How far the name has travelled: every kept story about you, by reach.
    let home = w.people[me].nation;
    let p = w.people[me].player;
    let mut reach: Vec<ChronicleReach> = Layer::ALL.iter().map(|l| ChronicleReach { layer: layer_key(*l).into(), first: None, outlet: None, stories: 0 }).collect();
    for s in w.media.stories.iter() {
        if s.person != me && !(p.is_some() && s.player == p) {
            continue;
        }
        let layer = pw_sim::chronicle::layer_of(w, s.outlet, home);
        let r = &mut reach[Layer::ALL.iter().position(|l| *l == layer).unwrap_or(0)];
        r.stories += 1;
        if r.first.is_none_or(|d| s.date.0 < d) {
            r.first = Some(s.date.0);
            r.outlet = Some(pw_narrate::press::outlet_name(w, s));
        }
    }
    // The chronicle remembers firsts the story list has since forgotten.
    for e in &life.entries {
        if let Line::Press { story, layer, first: true } = e.line {
            let r = &mut reach[Layer::ALL.iter().position(|l| *l == layer).unwrap_or(0)];
            if r.first.is_none_or(|d| e.date.0 < d) {
                r.first = Some(e.date.0);
                r.outlet = w.media.stories.get(story).map(|st| pw_narrate::press::outlet_name(w, st));
            }
        }
    }

    let view = ChronicleView {
        person: Named::new(Ref::person(me), c.person_name(me)),
        since: life.since.0,
        entries,
        people: people.into_iter().map(|(_, t)| t).collect(),
        reach,
        born: w.people[me].dob.0,
    };
    serde_json::to_value(view).map_err(|e| ApiError::Internal(e.to_string()))
}

fn layer_key(l: Layer) -> &'static str {
    match l {
        Layer::Local => "local",
        Layer::National => "national",
        Layer::Abroad => "abroad",
    }
}

/// People from your past on the other side of a fixture against `opp`: its players and its manager, by their most telling tie.
pub(crate) fn known_faces(c: &Ctx, me: PersonId, opp: ClubId) -> Vec<(PersonId, Vec<Part>, &'static str)> {
    let w = c.w;
    let Some(life) = w.ext.chronicle.of(me) else { return Vec::new() };
    let manager = w.clubs.get(opp).map(|k| k.manager).filter(|m| m.is_some()).map(|m| w.staff[m].person);
    let mut out: Vec<(PersonId, Vec<Part>, &'static str)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for t in &life.ties {
        if t.person == me || !seen.insert(t.person) {
            continue;
        }
        let Some(best) = life.best_tie(t.person).map(|i| life.ties[i]) else { continue };
        if matches!(best.kind, TieKind::Teammate { .. } | TieKind::Classmate { .. }) && best.days() < LISTED_DAYS {
            continue;
        }
        let q = w.people.get(t.person).map_or(pw_core::PlayerId::NONE, |x| x.player);
        let role = if manager == Some(t.person) {
            "manager"
        } else if q.is_some() && w.players.hot[q].club == opp && w.players.hot[q].status != pw_world::PlayerStatus::Retired {
            "player"
        } else {
            continue;
        };
        out.push((t.person, tie_parts(c, &best, false), role));
    }
    out
}

/// What two people shared, for a relationship: "Teammates at Kerala Police FC, Aug 2026 to Mar 2028".
pub(crate) fn shared(c: &Ctx, me: PersonId, t: &Tie) -> Vec<Part> {
    let now = current(c, me, t);
    let s = S::new(c);
    let s = match t.kind {
        TieKind::Teammate { club } => s.t("Teammates at ").club(club),
        TieKind::Classmate { inst } => s.t("Teammates at ").inst(inst),
        TieKind::Coach { club } => s.t("Your manager at ").club(club),
        TieKind::Scout { org } => return s.t(format!("The first at {} to take you seriously, ", org_words(c, org))).date(t.from).done(),
        TieKind::LetGo { club } => return s.t("Let you go at ").club(club).t(", ").date(t.from).done(),
        TieKind::Mentor => return s.t("Looked out for you from ").date(t.from).done(),
        TieKind::Finder => return s.t("The first outside your family to take you seriously").done(),
    };
    if now || t.days() < 7 { s.t(", since ").date(t.from).done() } else { s.t(", ").date(t.from).t(" to ").date(t.to).done() }
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::ctx::Ctx;
    use crate::session::Session;
    use pw_core::{EventId, NationId};
    use pw_world::affairs::HomeKind;
    use pw_world::chat::{ChatMsg, Said, Sender};
    use pw_world::chronicle::{Line, Spot};
    use pw_world::event::LifeEventKind;

    fn text(parts: &[crate::model::Part]) -> String {
        parts.iter().map(|p| p.t.as_str()).collect()
    }

    fn clean(t: &str) {
        assert!(!t.trim().is_empty() && !t.contains("  ") && !t.contains('{') && !t.contains('}'), "clean: {t}");
        assert!(t.starts_with(|c: char| c.is_uppercase()), "a sentence: {t}");
        assert!(![" him", " his ", " he ", " she ", " her "].iter().any(|g| t.contains(g)), "no assumed gender: {t}");
    }

    #[test]
    fn recognition_a_new_place_and_the_hours_between_home_and_here_read_as_sentences() {
        let w = pw_import::synthetic::build(pw_data::DataPack::builtin(), 5, pw_import::synthetic::Scale::SMALL);
        let mut s = Session::new(w, "t".into());
        let p = s.w().players.ids().find(|&q| s.w().players.hot[q].club.is_some()).unwrap();
        let me = s.w().players.cold[p].person;
        s.inhabit(me, 0).unwrap();
        let w = &mut s.game.sim.world;
        let club = w.players.hot[p].club;
        // Home (the family's country) four hours ahead of where the person lives now.
        let own = w.people[me].nation;
        let there = w.nations.ids().find(|&n| n != own).expect("two nations");
        w.lives[me].home = there;
        w.lives[me].household.parents.nation = own;
        w.nations[own].env.tz = 7;
        w.nations[there].env.tz = 3;
        w.nations[own].env.known = true;
        w.nations[there].env.known = true;
        let c = Ctx::new(&s);
        let life = c.w.ext.chronicle.of(me).unwrap();
        let line = |l: Line| {
            let (parts, _, _) = render(&c, life, l).expect("told");
            let t = text(&parts);
            clean(&t);
            t
        };
        assert!(line(Line::Spotted { spot: Spot::Street, club, nation: NationId::NONE }).contains("autograph"));
        assert!(line(Line::Spotted { spot: Spot::Training, club, nation: NationId::NONE }).contains("training"));
        for spot in [Spot::AwayTrip, Spot::Moving] {
            assert!(line(Line::Spotted { spot, club, nation: NationId::NONE }).contains("airport"));
        }
        assert!(line(Line::Spotted { spot: Spot::SquadTrip, club: pw_core::ClubId::NONE, nation: own }).contains("squad"));
        assert!(line(Line::NewPlace { club, home: HomeKind::Family, quality: 0 }).contains("host family"));
        assert!(line(Line::NewPlace { club, home: HomeKind::Digs, quality: 0 }).contains("digs"));
        assert!(line(Line::NewPlace { club, home: HomeKind::Rented, quality: 3 }).starts_with("Found a comfortable rented place near "));
        assert!(line(Line::NewPlace { club, home: HomeKind::Owned, quality: 5 }).starts_with("Bought a luxurious home near "));
        assert!(line(Line::Life { kind: LifeEventKind::Relocated { nation: there } }).ends_with(". Home is 4 hours ahead"));
        assert_eq!(super::clock_words(&c, own, own), None, "no clock at home");
        assert_eq!(super::clock_words(&c, there, own).as_deref(), Some("Home is 4 hours behind"));
        // The chats: each new message in words, the family's from home across the hours.
        let today = c.w.date;
        let msg = |said: Said, family: bool| {
            let m = ChatMsg { date: today, from: Sender::Mother, said, event: EventId::NONE };
            let t = text(&crate::pages::chat::words(&c, me, &m, 0, family));
            clean(&t);
            t
        };
        for spot in [Spot::Street, Spot::Training, Spot::AwayTrip] {
            msg(Said::Spotted { spot }, true);
            msg(Said::Spotted { spot }, false);
        }
        for (coming, abroad) in [(true, true), (true, false), (false, true), (false, false)] {
            msg(Said::FamilyMove { coming, abroad }, true);
        }
        msg(Said::PartnerMove { coming: true }, false);
        msg(Said::PartnerMove { coming: false }, false);
        // The final whistle at nine in the evening here is one in the morning at home, four hours ahead.
        assert!(msg(Said::WatchedFromHome { uid: u64::MAX, result: 1, ahead: 4 }, true).starts_with("It's 1 in the morning here"));
        assert!(msg(Said::WatchedFromHome { uid: u64::MAX, result: -1, ahead: -6 }, true).contains("afternoon"));
        assert!(msg(Said::MissingFar { ahead: 4 }, true).contains("11 in the evening"));
    }
}
