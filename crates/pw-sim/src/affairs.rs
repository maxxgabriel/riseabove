//! Personal affairs (10 §3–§6): study and badges, homes and the cost of
//! living, personal staff, giving back, investing, and working lives after
//! playing. One set of rules for everyone; AI minds choose here from
//! personality and circumstance, humans choose through intents that call the
//! same functions.
//!
//! Cross-system effects: coaching badges gate management jobs (`board`,
//! `staffing`); a trainer and chef lower the injury hazard (`health`); a
//! tutor speeds language learning; a publicist and giving shape public image
//! and local standing (`renown`); a pundit becomes a journalist with sources
//! among former teammates (`media` rumours); an agent becomes a real agency
//! (`agents`); money flows through `life::finances`.

use pw_core::rng::{Rng, hash_key, stream};
use pw_core::{Attr, ClubId, Hidden, Money, NationId, PersonId, PlayerId, StaffAttr};
use pw_world::affairs::{CareerPath, Course, Employer, Helper, Hired, HomeKind, Study, Work};
use pw_world::agent::Agent;
use pw_world::event::{EventKind, Visibility};
use pw_world::media::{Journalist, OutletKind};
use pw_world::{FanReason, Intent, MindKind, PlayerStatus, StaffRole, World};
use smallvec::SmallVec;

use crate::consider;

/// Relative cost of living where someone lives (1.0 ≈ the richest economy).
pub fn cost_index(w: &World, nation: NationId) -> f32 {
    let econ = w.nations.get(nation).map_or(0.5, |n| n.economy);
    (0.35 + 0.65 * econ) * w.economy.global()
}

fn home_of(w: &World, who: PersonId) -> NationId {
    w.lives.get(who).map_or(w.people[who].nation, |l| l.home)
}

// ---------------------------------------------------------------------------
// Money (read by life::finances)
// ---------------------------------------------------------------------------

/// Extra monthly (gross income, spending) from affairs and endorsements.
pub fn money(w: &World, who: PersonId) -> (Money, Money) {
    let mut income: Money = 0;
    let mut spend: Money = 0;
    let ci = cost_index(w, home_of(w, who));
    if let Some(a) = w.affairs.of(who) {
        match a.home.kind {
            HomeKind::Family => {}
            HomeKind::Digs => spend += (400.0 * ci) as Money,
            HomeKind::Rented => spend += (f32::from(a.home.quality) * 900.0 * ci) as Money,
            HomeKind::Owned => spend += a.home.mortgage / 200 + (f32::from(a.home.quality) * 250.0 * ci) as Money,
        }
        spend += a.staff.iter().map(|h| h.cost).sum::<Money>();
        if a.foundation {
            spend += (5_000.0 * ci) as Money;
        }
        if let Some(wk) = a.work {
            income += wk.income;
        }
    }
    for e in w.commerce.active_for(who) {
        income += e.fee_year / 12 * Money::from(100 - e.club_share) / 100;
    }
    // Giving is a share of what comes in.
    if let Some(a) = w.affairs.of(who) {
        let base = w.lives.get(who).map_or(0, |l| l.finances.income) + income;
        spend += base * Money::from(a.giving_pct) / 100;
    }
    (income, spend)
}

// ---------------------------------------------------------------------------
// Monthly
// ---------------------------------------------------------------------------

pub fn monthly(w: &mut World) {
    let people: Vec<PersonId> = w.people.iter_enumerated().filter(|(id, p)| (p.player.is_some() || p.staff.is_some()) && consider::age(w, *id) >= 16.0).map(|(id, _)| id).collect();
    for who in people {
        if w.people[who].mind == MindKind::Ai {
            ai_choices(w, who);
        }
        study(w, who);
        home_effects(w, who);
        helpers(w, who);
        giving(w, who);
        invest(w, who);
        work(w, who);
    }
}

// ---------------------------------------------------------------------------
// Study
// ---------------------------------------------------------------------------

/// Start a course (intent or AI). Fees are paid up front.
pub fn enrol(w: &mut World, who: PersonId, course: Course) {
    let today = w.date;
    let a = w.affairs.entry(who);
    if a.studying.is_some() || a.has(course) || course.requires().is_some_and(|r| !a.has(r)) {
        return;
    }
    a.studying = Some(Study { course, started: today, done: 0 });
    let f = &mut w.lives[who].finances;
    f.savings -= course.cost();
    if f.savings < 0 {
        f.debt += -f.savings;
        f.savings = 0;
    }
    w.events.push(today, Visibility::Person(who), EventKind::EnrolledCourse { person: who, course });
}

fn study(w: &mut World, who: PersonId) {
    let today = w.date;
    let Some(s) = w.affairs.of(who).and_then(|a| a.studying) else { return };
    let hours = w.lives.get(who).map_or(0, |l| l.routine.study);
    let aptitude = 0.7 + consider::hid(w, who, Hidden::Professionalism) / 40.0 + f32::from(w.lives[who].education) * 0.05;
    let done = s.done + (f32::from(hours) * aptitude).round() as u16;
    let a = w.affairs.entry(who);
    if done >= s.course.effort() {
        a.studying = None;
        a.quals.push((s.course, today));
        w.events.push(today, Visibility::Public, EventKind::Qualified { person: who, course: s.course });
        on_qualified(w, who, s.course);
    } else if today.days_until(s.started) < -1100 && done < s.course.effort() / 3 {
        // Three years in and barely started: dropped out.
        a.studying = None;
    } else {
        a.studying = Some(Study { done, ..s });
    }
}

/// What a qualification changes for people already working in football.
fn on_qualified(w: &mut World, who: PersonId, course: Course) {
    let s = w.people[who].staff;
    if s.is_none() {
        return;
    }
    let bump = |w: &mut World, a: StaffAttr| {
        let v = w.staff[s].attrs.get(a);
        w.staff[s].attrs.set(a, (v + 1).min(20));
    };
    match course {
        Course::CoachingC | Course::CoachingB | Course::CoachingA | Course::CoachingPro => {
            bump(w, StaffAttr::TacticalKnowledge);
            bump(w, StaffAttr::Tactical);
            w.staff[s].reputation = w.staff[s].reputation.saturating_add(150);
        }
        Course::SportsScience => bump(w, StaffAttr::SportsScience),
        Course::DataAnalysis => bump(w, StaffAttr::TacticalKnowledge),
        Course::Scouting => {
            bump(w, StaffAttr::JudgingAbility);
            bump(w, StaffAttr::JudgingPotential);
        }
        _ => {}
    }
}

/// Coaching level for hiring decisions. Experienced managers and people who
/// never played professionally (imported staff) are treated as qualified.
pub fn coaching_level(w: &World, who: PersonId) -> u8 {
    let quals = w.affairs.of(who).map_or(0, |a| a.coaching_level());
    let s = w.people[who].staff;
    let experienced = s.is_some() && w.staff[s].record.games >= 100;
    let imported = w.people[who].player.is_none();
    if experienced || imported { quals.max(4) } else { quals }
}

/// The licence a club of this reputation requires of its manager.
pub fn required_level(club_rep: u16) -> u8 {
    match club_rep {
        6000.. => 4,
        3000..=5999 => 3,
        1200..=2999 => 2,
        _ => 1,
    }
}

// ---------------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------------

/// Move home: rent or buy at a quality.
pub fn move_home(w: &mut World, who: PersonId, buy: bool, quality: u8) {
    let today = w.date;
    let quality = quality.clamp(1, 5);
    let ci = cost_index(w, home_of(w, who));
    let value = (f32::from(quality) * 180_000.0 * ci) as Money;
    let a = w.affairs.entry(who);
    a.home.quality = quality;
    a.home.since = today;
    if buy {
        let deposit = value / 4;
        let f = &mut w.lives[who].finances;
        if f.savings < deposit {
            return;
        }
        f.savings -= deposit;
        let a = w.affairs.entry(who);
        a.home.kind = HomeKind::Owned;
        a.home.value = value;
        a.home.mortgage = value - deposit;
    } else {
        a.home.kind = HomeKind::Rented;
        a.home.value = 0;
        a.home.mortgage = 0;
    }
    w.events.push(today, Visibility::Person(who), EventKind::MovedHome { person: who, bought: buy });
}

/// A move abroad ends the old arrangement (called from `life::relocate`).
pub fn on_relocate(w: &mut World, who: PersonId) {
    if let Some(a) = w.affairs.people.get_mut(&who) {
        if a.home.kind == HomeKind::Owned {
            // Sold: equity back to savings.
            let equity = a.home.value - a.home.mortgage;
            a.home = Default::default();
            w.lives[who].finances.savings += equity.max(0);
        } else {
            a.home = Default::default();
        }
        // Local helpers do not move with you (tutors and advisers do).
        a.staff.retain(|h| matches!(h.helper, Helper::Adviser | Helper::Tutor | Helper::Publicist));
    }
}

fn home_effects(w: &mut World, who: PersonId) {
    let Some(a) = w.affairs.people.get_mut(&who) else { return };
    if a.home.kind == HomeKind::Owned {
        // Pay down and appreciate.
        a.home.mortgage = (a.home.mortgage - a.home.value / 300).max(0);
        a.home.value += a.home.value / 400;
    }
    let q = i32::from(a.home.quality);
    let kind = a.home.kind;
    if let Some(l) = w.lives.get_mut(who) {
        let adj = match kind {
            HomeKind::Family | HomeKind::Digs => 0,
            _ => (q - 3) * 2,
        };
        l.sleep = (i32::from(l.sleep) + adj).clamp(10, 100) as u8;
    }
}

// ---------------------------------------------------------------------------
// Personal staff
// ---------------------------------------------------------------------------

pub fn hire_helper(w: &mut World, who: PersonId, helper: Helper, quality: u8) {
    let today = w.date;
    let quality = quality.clamp(1, 20);
    let ci = cost_index(w, home_of(w, who));
    let cost = (helper.base_cost() as f32 * f32::from(quality) / 10.0 * ci) as Money;
    let a = w.affairs.entry(who);
    a.staff.retain(|h| h.helper != helper);
    a.staff.push(Hired { helper, quality, cost, since: today });
    w.events.push(today, Visibility::Person(who), EventKind::HiredHelper { person: who, helper });
}

pub fn dismiss_helper(w: &mut World, who: PersonId, helper: Helper) {
    if let Some(a) = w.affairs.people.get_mut(&who) {
        a.staff.retain(|h| h.helper != helper);
    }
}

/// Injury-hazard multiplier from a trainer and chef (read by `health`).
pub fn body_care(w: &World, who: PersonId) -> f32 {
    let Some(a) = w.affairs.of(who) else { return 1.0 };
    let t = a.helper(Helper::Trainer).map_or(0.0, |h| f32::from(h.quality) / 20.0 * 0.08);
    let c = a.helper(Helper::Chef).map_or(0.0, |h| f32::from(h.quality) / 20.0 * 0.05);
    1.0 - t - c
}

fn helpers(w: &mut World, who: PersonId) {
    let Some(a) = w.affairs.of(who) else { return };
    let publicist = a.helper(Helper::Publicist).map(|h| h.quality);
    let security = a.helper(Helper::Security).is_some();
    let tutor = a.helper(Helper::Tutor).map(|h| h.quality);
    if let Some(q) = publicist {
        // Good PR pulls image up slowly and softens the bad.
        let img = w.media.image.get(&who).copied().unwrap_or(0);
        let by = if img < 0 { i16::from(q) } else { i16::from(q) / 3 };
        w.media.nudge_image(who, by);
    }
    if security {
        if let Some(l) = w.lives.get_mut(who) {
            l.stress = l.stress.saturating_sub(2);
        }
    }
    if let Some(q) = tutor {
        let home = home_of(w, who);
        if let Some(l) = w.lives.get_mut(who) {
            if let Some(entry) = l.languages.iter_mut().find(|x| x.0 == home) {
                entry.1 = (entry.1 + 2 + q / 5).min(100);
            } else {
                l.languages.push((home, 5 + q / 4));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Giving back
// ---------------------------------------------------------------------------

pub fn set_giving(w: &mut World, who: PersonId, pct: u8, community: u8) {
    let first = w.affairs.of(who).is_none_or(|a| a.giving_pct == 0 && a.community == 0);
    let a = w.affairs.entry(who);
    a.giving_pct = pct.min(50);
    a.community = community.min(40);
    if first && (pct > 0 || community > 0) {
        w.events.push(w.date, Visibility::Public, EventKind::GaveBack { person: who, foundation: false });
    }
}

pub fn start_foundation(w: &mut World, who: PersonId) {
    let a = w.affairs.entry(who);
    if a.foundation {
        return;
    }
    a.foundation = true;
    w.events.push(w.date, Visibility::Public, EventKind::GaveBack { person: who, foundation: true });
}

fn giving(w: &mut World, who: PersonId) {
    let Some(a) = w.affairs.of(who) else { return };
    let (pct, hours, foundation) = (a.giving_pct, a.community, a.foundation);
    if pct == 0 && hours == 0 && !foundation {
        return;
    }
    let img = i16::from(pct) + i16::from(hours) / 4 + if foundation { 10 } else { 0 };
    w.media.nudge_image(who, img);
    let club = w.club_of_person(who);
    if hours > 0 || foundation {
        w.media.move_fans(club, who, i16::from(hours) / 2 + if foundation { 8 } else { 0 }, FanReason::Loyalty, w.date);
        let r = w.renown.people.entry(who).or_default();
        r.local = r.local.saturating_add(u16::from(hours) * 5 + if foundation { 40 } else { 0 }).min(10_000);
    }
    if let Some(l) = w.lives.get_mut(who) {
        l.fulfilment = (l.fulfilment + 1 + hours / 10 + u8::from(foundation) * 2).min(100);
    }
}

// ---------------------------------------------------------------------------
// Investing
// ---------------------------------------------------------------------------

pub fn invest_money(w: &mut World, who: PersonId, amount: Money, risk: u8) {
    let f = &mut w.lives[who].finances;
    let amount = amount.min(f.savings).max(0);
    f.savings -= amount;
    let a = w.affairs.entry(who);
    a.invested += amount;
    a.risk = risk.clamp(1, 20);
}

fn invest(w: &mut World, who: PersonId) {
    let today = w.date;
    let Some(a) = w.affairs.of(who) else { return };
    if a.invested <= 0 {
        return;
    }
    let risk = f32::from(a.risk) / 20.0;
    let adviser = a.helper(Helper::Adviser).map_or(0.0, |h| f32::from(h.quality) / 20.0);
    let noise = pw_core::rng::noise(&[w.seed, stream::LIFE, u64::from(who.0), today.0 as u64, 0x1a7]);
    // Drift with the economy; spread with risk; a good adviser trims the downside.
    let mean = 0.004 + 0.003 * risk + 0.002 * adviser;
    let mut r = mean + noise * (0.01 + 0.05 * risk);
    if r < 0.0 {
        r *= 1.0 - 0.5 * adviser;
    }
    let invested = a.invested;
    let gain = (invested as f32 * r) as Money;
    w.affairs.entry(who).invested = (invested + gain).max(0);
    if gain.abs() > invested / 10 && gain.abs() > 50_000 {
        w.events.push(today, Visibility::Person(who), EventKind::Investment { person: who, gain });
    }
}

// ---------------------------------------------------------------------------
// Working lives after playing
// ---------------------------------------------------------------------------

/// Whether a path is open to a person right now, and why not.
pub fn can_pursue(w: &World, who: PersonId, path: CareerPath) -> Result<(), &'static str> {
    let p = w.people[who].player;
    let fame = w.renown.of(who).fame;
    let world_rep = if p.is_some() { w.players.cold[p].rep.world } else { 0 };
    let insider = w.media.insider.get(&who).copied().unwrap_or(0);
    let a = w.affairs.of(who);
    let has = |c: Course| a.is_some_and(|x| x.has(c));
    match path {
        CareerPath::Coach => {
            if coaching_level(w, who) >= 1 {
                Ok(())
            } else {
                Err("needs at least a C licence")
            }
        }
        CareerPath::Pundit => {
            if fame >= 2500 || world_rep >= 3500 {
                Ok(())
            } else {
                Err("not well known enough")
            }
        }
        CareerPath::Journalist => {
            if has(Course::Journalism) || w.lives.get(who).is_some_and(|l| l.education >= 4) {
                Ok(())
            } else {
                Err("needs a journalism course or a degree")
            }
        }
        CareerPath::Agent => {
            if insider >= 100 || world_rep >= 3000 || has(Course::Business) {
                Ok(())
            } else {
                Err("needs contacts in the game")
            }
        }
        CareerPath::Analyst => {
            if has(Course::DataAnalysis) || has(Course::SportsScience) {
                Ok(())
            } else {
                Err("needs an analysis qualification")
            }
        }
        CareerPath::Scout => Ok(()),
        CareerPath::Director => {
            if has(Course::Business) || coaching_level(w, who) >= 3 || insider >= 300 {
                Ok(())
            } else {
                Err("needs a business degree, an A licence or standing in the game")
            }
        }
        CareerPath::Ambassador => {
            let legend = w.honours.clubs.iter().any(|(_, r)| r.legends.contains(&who));
            if legend { Ok(()) } else { Err("clubs only make legends ambassadors") }
        }
        CareerPath::Business => {
            if w.lives.get(who).is_some_and(|l| l.finances.savings >= 200_000) {
                Ok(())
            } else {
                Err("needs capital")
            }
        }
    }
}

/// Begin a path (intent; AI minds submit the same intent).
pub fn pursue(w: &mut World, who: PersonId, path: CareerPath) {
    let today = w.date;
    if can_pursue(w, who, path).is_err() {
        return;
    }
    if let Some(prev) = w.affairs.of(who).and_then(|a| a.work) {
        end_work(w, who, prev);
    }
    let home = home_of(w, who);
    let ci = cost_index(w, home);
    let fame = f32::from(w.renown.of(who).fame);
    let (employer, income) = match path {
        CareerPath::Coach | CareerPath::Analyst | CareerPath::Scout | CareerPath::Director => {
            let role = match path {
                CareerPath::Coach => {
                    let p = w.people[who].player;
                    if p.is_some() { crate::mind::preferred_staff_role(w, p) } else { StaffRole::Coach }
                }
                CareerPath::Analyst => StaffRole::Analyst,
                CareerPath::Scout => StaffRole::Scout,
                _ => StaffRole::DirectorOfFootball,
            };
            // Clubs hire from the staff pool; income comes as a wage then.
            if crate::people::enter_staff_pool(w, who, role).is_none() {
                return;
            }
            (Employer::None, 0)
        }
        CareerPath::Pundit | CareerPath::Journalist => {
            let want = if path == CareerPath::Pundit { OutletKind::Broadcaster } else { OutletKind::National };
            let outlet = w.media.outlets.iter_enumerated().filter(|(_, o)| o.nation == home && o.kind == want).map(|(id, _)| id).next();
            let Some(outlet) = outlet else { return };
            // The beat: clubs they played for. Sources: people who like them.
            let p = w.people[who].player;
            let mut beat: SmallVec<[ClubId; 4]> = SmallVec::new();
            if p.is_some() {
                for s in w.history.spells.get(&p).into_iter().flatten().rev() {
                    if !beat.contains(&s.club) && beat.len() < 4 {
                        beat.push(s.club);
                    }
                }
            }
            let mut sources: SmallVec<[PersonId; 8]> = SmallVec::new();
            let mut liked: Vec<(PersonId, i8)> = w.social.toward(who).filter(|(a, r)| *a != who && r.affinity > 20).map(|(a, r)| (a, r.affinity)).collect();
            liked.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(&y.0)));
            sources.extend(liked.into_iter().take(8).map(|x| x.0));
            w.media.journalists.insert(who, Journalist { person: who, outlet, beat, sources, credibility: if path == CareerPath::Pundit { 55 } else { 45 } });
            let pay = if path == CareerPath::Pundit { 3_000.0 + fame * 4.0 } else { 3_500.0 };
            (Employer::Outlet(outlet), (pay * ci) as Money)
        }
        CareerPath::Agent => {
            let p = w.people[who].player;
            let rep = if p.is_some() { w.players.cold[p].rep.world / 3 } else { 500 };
            let neg = consider::hid(w, who, Hidden::Ambition).max(6.0) as u8;
            let honesty = consider::hid(w, who, Hidden::Sportsmanship).max(4.0) as u8;
            let id = w.agents.list.push(Agent {
                person: who,
                base: home,
                reach: Vec::new(),
                negotiating: neg,
                network: (4 + rep / 800).min(20) as u8,
                diligence: consider::hid(w, who, Hidden::Professionalism) as u8,
                greed: (20.0 - consider::hid(w, who, Hidden::Loyalty)).clamp(3.0, 18.0) as u8,
                honesty,
                reputation: rep.max(300),
                clients: Vec::new(),
                capacity: 6,
                active: true,
            });
            (Employer::Agency(id), 0)
        }
        CareerPath::Ambassador => {
            let club = w.honours.clubs.iter().filter(|(_, r)| r.legends.contains(&who)).map(|(&c, _)| c).max_by_key(|&c| w.clubs[c].reputation);
            let Some(club) = club else { return };
            let pay = (2_000.0 + f32::from(w.clubs[club].reputation) * 0.8) * ci;
            (Employer::Club(club), pay as Money)
        }
        CareerPath::Business => {
            let stake = w.lives[who].finances.savings / 2;
            w.lives[who].finances.savings -= stake;
            let a = w.affairs.entry(who);
            a.invested += stake;
            a.risk = a.risk.max(12);
            (Employer::Own, 0)
        }
    };
    w.affairs.entry(who).work = Some(Work { path, employer, since: today, income, standing: 50 });
    w.events.push(today, Visibility::Public, EventKind::NewCareer { person: who, path });
}

pub fn leave_work(w: &mut World, who: PersonId) {
    if let Some(wk) = w.affairs.of(who).and_then(|a| a.work) {
        end_work(w, who, wk);
    }
}

fn end_work(w: &mut World, who: PersonId, wk: Work) {
    let today = w.date;
    match wk.employer {
        Employer::Outlet(_) => {
            w.media.journalists.remove(&who);
        }
        Employer::Agency(a) => {
            w.agents.list[a].active = false;
        }
        _ => {}
    }
    let a = w.affairs.entry(who);
    a.past_work.push((wk.path, wk.since, today));
    a.work = None;
    w.events.push(today, Visibility::Public, EventKind::CareerEnded { person: who, path: wk.path });
}

/// How the work is going.
fn work(w: &mut World, who: PersonId) {
    let Some(wk) = w.affairs.of(who).and_then(|a| a.work) else { return };
    let today = w.date;
    let mut rng = Rng::keyed(&[w.seed, stream::LIFE, u64::from(who.0), today.0 as u64, 0x3a1]);
    let mut standing = f32::from(wk.standing);
    let mut income = wk.income;
    match wk.path {
        CareerPath::Pundit | CareerPath::Journalist => {
            // Audiences like the famous and the outspoken; credibility counts.
            let fame = f32::from(w.renown.of(who).fame) / 10_000.0;
            let spice = consider::hid(w, who, Hidden::Controversy) / 20.0;
            let cred = w.media.journalists.get(&who).map_or(40.0, |j| f32::from(j.credibility)) / 100.0;
            let target = 30.0 + fame * 40.0 + spice * 15.0 + cred * 25.0;
            standing += 0.2 * (target - standing) + rng.normal() * 3.0;
            income = (income as f32 * (0.99 + (standing - 50.0) / 2000.0)) as Money;
        }
        CareerPath::Coach | CareerPath::Analyst | CareerPath::Scout | CareerPath::Director => {
            // Employed by a club → standing rises; waiting for too long → drift away.
            let s = w.people[who].staff;
            let employed = s.is_some() && w.staff[s].employed();
            let club = if employed { w.staff[s].club } else { ClubId::NONE };
            income = if employed { w.staff[s].wage * 52 / 12 } else { 0 };
            standing += if employed { 2.0 } else { -2.5 };
            if let Some(a) = w.affairs.people.get_mut(&who) {
                if let Some(x) = a.work.as_mut() {
                    x.employer = if employed { Employer::Club(club) } else { Employer::None };
                }
            }
        }
        CareerPath::Agent => {
            if let Employer::Agency(a) = wk.employer {
                let clients = w.agents.list[a].clients.clone();
                let fees: Money = clients.iter().map(|&p| w.players.cold[p].contract.current_wage(today) * 52 / 12 * 5 / 100).sum();
                income = fees;
                standing += (clients.len() as f32 - 3.0) * 1.5 - 0.5;
            }
        }
        CareerPath::Ambassador => {
            if let Employer::Club(c) = wk.employer {
                w.media.move_fans(c, who, 5, FanReason::Loyalty, today);
                w.clubs[c].finance.balance -= income;
            }
            standing += 0.5;
        }
        CareerPath::Business => {
            let skill = if w.affairs.of(who).is_some_and(|a| a.has(Course::Business)) { 0.3 } else { 0.0 };
            standing += rng.normal() * 6.0 + skill * 3.0 - 0.3;
        }
    }
    let standing = standing.clamp(0.0, 100.0);
    if let Some(a) = w.affairs.people.get_mut(&who) {
        if let Some(x) = a.work.as_mut() {
            x.standing = standing as u8;
            x.income = income.max(0);
        }
    }
    if let Some(l) = w.lives.get_mut(who) {
        l.fulfilment = (f32::from(l.fulfilment) + (standing - 40.0) * 0.05).clamp(5.0, 100.0) as u8;
    }
    if standing < 8.0 {
        end_work(w, who, wk);
    }
}

// ---------------------------------------------------------------------------
// AI minds
// ---------------------------------------------------------------------------

fn ai_choices(w: &mut World, who: PersonId) {
    let today = w.date;
    let key = hash_key(&[w.seed, stream::MIND, u64::from(who.0), today.0 as u64, 0xaf]);
    let roll = (key % 1000) as f32 / 1000.0;
    let p = w.people[who].player;
    let playing = p.is_some() && w.players.hot[p].status == PlayerStatus::Active;
    let age = consider::age(w, who);
    let prof = consider::hid(w, who, Hidden::Professionalism);
    let sport = consider::hid(w, who, Hidden::Sportsmanship);
    let (income, savings, spending, lifestyle) =
        w.lives.get(who).map_or((0, 0, 0, pw_world::Lifestyle::Modest), |l| (l.finances.income, l.finances.savings, l.finances.spending, l.finances.lifestyle));
    let fame = w.renown.of(who).fame;
    let a = w.affairs.of(who).cloned().unwrap_or_default();

    // Where to live.
    if age >= 19.0 && matches!(a.home.kind, HomeKind::Family) && income > 2_000 {
        let q = match lifestyle {
            pw_world::Lifestyle::Frugal => 2,
            pw_world::Lifestyle::Modest => 3,
            pw_world::Lifestyle::Comfortable => 4,
            pw_world::Lifestyle::Lavish => 5,
        };
        move_home(w, who, false, q);
        return;
    }
    let settled = w.lives.get(who).is_some_and(|l| l.home_since.days_until(today) > 365);
    if a.home.kind == HomeKind::Rented && settled && savings > spending.max(1) * 30 && roll < 0.1 {
        move_home(w, who, true, a.home.quality.max(3));
        return;
    }
    // Helpers.
    if playing && income > 40_000 && prof >= 13.0 && a.helper(Helper::Trainer).is_none() && roll < 0.15 {
        hire_helper(w, who, Helper::Trainer, (prof as u8).min(18));
    } else if playing && income > 60_000 && a.helper(Helper::Chef).is_none() && roll < 0.1 {
        hire_helper(w, who, Helper::Chef, 12);
    } else if fame >= 6000 && a.helper(Helper::Security).is_none() && roll < 0.2 {
        hire_helper(w, who, Helper::Security, 12);
    } else if fame >= 5000 && a.helper(Helper::Publicist).is_none() && w.media.image.get(&who).copied().unwrap_or(0) < 0 && roll < 0.3 {
        hire_helper(w, who, Helper::Publicist, 12);
    } else if savings >= 1_000_000 && a.helper(Helper::Adviser).is_none() && roll < 0.2 {
        hire_helper(w, who, Helper::Adviser, (prof as u8).clamp(6, 18));
    }
    // Giving back.
    if a.giving_pct == 0 && income > 30_000 && sport >= 14.0 && roll > 0.9 {
        set_giving(w, who, ((sport - 12.0) as u8).max(1), (sport / 2.0) as u8);
    }
    if !a.foundation && fame >= 7000 && sport >= 13.0 && savings > 2_000_000 && roll > 0.97 {
        start_foundation(w, who);
    }
    // Investing surplus.
    if savings > spending.max(1) * 18 && roll < 0.08 {
        let ambition = consider::hid(w, who, Hidden::Ambition);
        invest_money(w, who, savings / 3, (ambition * 0.8) as u8);
    }
    // Preparing for later: the professional thirty-somethings take badges.
    if playing && age >= 28.0 && a.studying.is_none() && roll < 0.05 {
        let lead = w.players.cold[p].attrs.get(Attr::Leadership);
        let course = if lead >= 12.0 || prof >= 14.0 {
            [Course::CoachingC, Course::CoachingB, Course::CoachingA].into_iter().find(|c| !a.has(*c))
        } else if fame >= 3000 {
            (!a.has(Course::MediaTraining)).then_some(Course::MediaTraining)
        } else {
            None
        };
        if let Some(c) = course {
            if w.lives.get(who).is_some_and(|l| l.routine.study == 0) {
                w.lives[who].routine.study = 2;
                w.lives[who].routine = w.lives[who].routine.normalised();
            }
            enrol(w, who, c);
        }
    }
}

/// What a retired player does next, as an intent (called by `mind`).
pub fn ai_next_step(w: &World, who: PersonId, p: PlayerId) -> Option<Intent> {
    let fame = f32::from(w.renown.of(who).fame) / 10_000.0;
    let c = &w.players.cold[p];
    let lead = c.attrs.get(Attr::Leadership) / 20.0;
    let prof = consider::hid(w, who, Hidden::Professionalism) / 20.0;
    let spice = consider::hid(w, who, Hidden::Controversy) / 20.0;
    let ambition = consider::hid(w, who, Hidden::Ambition) / 20.0;
    let vision = c.attrs.get(Attr::Anticipation) / 20.0;
    let legend = w.honours.clubs.values().any(|r| r.legends.contains(&who));
    let options: [(CareerPath, f32); 7] = [
        (CareerPath::Coach, lead * 0.6 + prof * 0.4),
        (CareerPath::Pundit, fame * 1.2 + spice * 0.4),
        (CareerPath::Agent, ambition * 0.5 + (1.0 - prof) * 0.2),
        (CareerPath::Scout, vision * 0.6),
        (CareerPath::Ambassador, if legend { 0.9 } else { 0.0 }),
        (CareerPath::Business, if w.lives.get(who).is_some_and(|l| l.finances.savings > 500_000) { ambition * 0.6 } else { 0.0 }),
        (CareerPath::Director, if coaching_level(w, who) >= 3 { prof * 0.6 } else { 0.0 }),
    ];
    let mut best: Vec<(CareerPath, f32)> = options.into_iter().filter(|x| x.1 > 0.35).collect();
    best.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (path, _) in best {
        if can_pursue(w, who, path).is_ok() {
            return Some(Intent::PursueCareer(path));
        }
        if path == CareerPath::Coach && w.affairs.of(who).is_none_or(|a| a.studying.is_none()) {
            let next = [Course::CoachingC, Course::CoachingB, Course::CoachingA, Course::CoachingPro].into_iter().find(|c| w.affairs.of(who).is_none_or(|a| !a.has(*c)));
            if let Some(c) = next {
                return Some(Intent::Enrol(c));
            }
        }
    }
    None
}
