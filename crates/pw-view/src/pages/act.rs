//! Things the inhabited person can decide to do. Every action becomes an `Intent` in the world's
//! queue and is applied by the same rules the simulation uses for everyone, on the next simulated
//! day. This module checks the plain preconditions first, so the client can say why an action is
//! not available instead of queuing something the world would silently ignore.

use pw_core::{AgentId, NationId, PersonId};
use pw_world::affairs::{CareerPath, Course, Helper};
use pw_world::interaction::{Tone, Topic};
use pw_world::life::{Lifestyle, Routine};
use pw_world::media::Stance;
use pw_world::socialnet::{Concept, NO_POST};
use pw_world::staff::StaffRole;
use pw_world::{Intent, PartnerAsk, PlayerStatus};
use serde_json::{Value, json};

use super::me::named;
use crate::contract::{ActDone, ActReq};
use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Ref};
use crate::session::Session;

pub const STANCES: [(Stance, &str, &str); 7] = [
    (Stance::Praise, "praise", "Praise"),
    (Stance::Support, "support", "Back someone publicly"),
    (Stance::Loyalty, "loyalty", "Say you are happy here"),
    (Stance::Ambition, "ambition", "Talk up your ambitions"),
    (Stance::Deflect, "deflect", "Say nothing of substance"),
    (Stance::Complain, "complain", "Complain about your role"),
    (Stance::Criticise, "criticise", "Criticise"),
];

/// What a person can say online, in the words the interface offers.
pub const POSTS: [(Concept, &str, &str); 9] = [
    (Concept::Praise, "praise", "Praise"),
    (Concept::Celebrate, "celebrate", "Celebrate"),
    (Concept::Defend, "defend", "Stand up for"),
    (Concept::Agree, "agree", "Agree"),
    (Concept::Statement, "statement", "Make a statement"),
    (Concept::Lament, "lament", "Lament"),
    (Concept::Disagree, "disagree", "Disagree"),
    (Concept::Criticise, "criticise", "Criticise"),
    (Concept::Mock, "mock", "Mock"),
];

pub fn concept_label(c: Concept) -> &'static str {
    POSTS.iter().find(|p| p.0 == c).map_or("Post", |p| p.2)
}

const ROLES: [(StaffRole, &str); 7] = [
    (StaffRole::Coach, "coach"),
    (StaffRole::Assistant, "assistant"),
    (StaffRole::Manager, "manager"),
    (StaffRole::Scout, "scout"),
    (StaffRole::Analyst, "analyst"),
    (StaffRole::Physio, "physio"),
    (StaffRole::FitnessCoach, "fitness"),
];

const COURSES: [Course; 10] = [
    Course::CoachingC,
    Course::CoachingB,
    Course::CoachingA,
    Course::CoachingPro,
    Course::SportsScience,
    Course::Journalism,
    Course::Business,
    Course::DataAnalysis,
    Course::Scouting,
    Course::MediaTraining,
];

const PATHS: [CareerPath; 9] =
    [CareerPath::Coach, CareerPath::Pundit, CareerPath::Journalist, CareerPath::Agent, CareerPath::Analyst, CareerPath::Scout, CareerPath::Director, CareerPath::Ambassador, CareerPath::Business];

fn key_of<T: Copy + PartialEq>(list: &[(T, &'static str, &'static str)], v: T) -> &'static str {
    list.iter().find(|x| x.0 == v).map_or("", |x| x.1)
}

fn tone_from(s: &str) -> Option<Tone> {
    Some(match s {
        "calm" => Tone::Calm,
        "assertive" => Tone::Assertive,
        "aggressive" => Tone::Aggressive,
        "humble" => Tone::Humble,
        "joking" | "light-hearted" => Tone::Joking,
        _ => return None,
    })
}

fn topic_key(t: Topic) -> &'static str {
    match t {
        Topic::PlayingTime => "playing_time",
        Topic::Feedback => "feedback",
        Topic::Position => "position",
        Topic::NewContract => "new_contract",
        Topic::LoanRequest => "loan",
        Topic::WantAway => "want_away",
        Topic::Attitude => "attitude",
        Topic::Discipline => "discipline",
        Topic::Dropped => "dropped",
        Topic::Encouragement => "encouragement",
        Topic::PromiseFollowUp => "promise",
        Topic::TeammateIssue => "teammate",
        Topic::Apology => "apology",
        Topic::AgentReview => "agent_review",
    }
}

fn topic_from(s: &str) -> Option<Topic> {
    Topic::PLAYER_RAISES.iter().copied().chain([Topic::AgentReview]).find(|&t| topic_key(t) == s)
}

fn course_key(c: Course) -> String {
    format!("{c:?}")
}

fn helper_key(h: Helper) -> String {
    format!("{h:?}")
}

fn path_key(p: CareerPath) -> String {
    format!("{p:?}")
}

/// "a coach", "an assistant manager", and no article for a mass noun such as "security".
fn with_article(noun: &str) -> String {
    if noun == "security" {
        return noun.to_string();
    }
    let vowel = noun.chars().next().is_some_and(|c| "aeiou".contains(c));
    format!("{} {noun}", if vowel { "an" } else { "a" })
}

/// A sentence for something queued but not yet applied.
pub fn intent_text(c: &Ctx, i: &Intent) -> String {
    let w = c.w;
    match *i {
        Intent::RequestMeeting { with, topic, .. } => format!("Ask {} for a meeting about {}", c.person_name(with), topic.label()),
        Intent::TransferRequest => "Hand in a transfer request".into(),
        Intent::WithdrawTransferRequest => "Withdraw your transfer request".into(),
        Intent::SetTraining(_) => "Change your training plan".into(),
        Intent::SetRoutine(_) => "Change how you spend your week".into(),
        Intent::SetLifestyle(l) => format!("Live a {} lifestyle", l.label()),
        Intent::HireAgent(a) => format!("Ask {} to represent you", c.person_name(w.agents.list[a].person)),
        Intent::DropAgent => "Part ways with your agent".into(),
        Intent::Retire => "Retire from playing".into(),
        Intent::SeekStaffJob(r) => format!("Look for work as {}", with_article(&r.label().to_lowercase())),
        Intent::Unretire => "Come out of retirement".into(),
        Intent::AskPartner(a) => match a {
            PartnerAsk::MoveIn => "Ask your partner to move in".into(),
            PartnerAsk::Marry => "Ask your partner to marry you".into(),
            PartnerAsk::Separate => "Tell your partner you should separate".into(),
        },
        Intent::OpenToDating(true) => "Open yourself to meeting someone".into(),
        Intent::OpenToDating(false) => "Stop looking for a relationship".into(),
        Intent::JoinAmateurFootball => "Sign up for amateur football".into(),
        Intent::DeclareForNation(n) => format!("Declare for {}", c.nation_name(n)),
        Intent::RetireFromInternational => "Retire from international football".into(),
        Intent::PlayThroughPain(true) => "Tell the medical staff you will play through pain".into(),
        Intent::PlayThroughPain(false) => "Tell the medical staff you will not play through pain".into(),
        Intent::Mentor(p) => format!("Offer to mentor {}", c.person_name(p)),
        Intent::SpeakToPress { about, stance } => {
            format!("Speak to the press about {} ({})", if Some(about) == c.me() { "yourself".to_string() } else { c.person_name(about) }, key_of(&STANCES, stance))
        }
        Intent::Enrol(course) => format!("Enrol on the {}", course.label()),
        Intent::MoveHome { buy, .. } => (if buy { "Buy a home" } else { "Rent a home" }).into(),
        Intent::HireHelper(h, _) => format!("Hire {}", with_article(h.label())),
        Intent::DismissHelper(h) => format!("Let your {} go", h.label()),
        Intent::SetGiving { pct, community } => format!("Give {pct}% of income and {community} hours of community work a month"),
        Intent::StartFoundation => "Start a foundation".into(),
        Intent::Invest { .. } => "Invest money".into(),
        Intent::PursueCareer(p) => format!("Start working in {}", p.label()),
        Intent::LeaveCareer => "Leave your current work".into(),
        Intent::Thank { to } => format!("Thank {}", c.person_name(to)),
        Intent::Tell { to, .. } => format!("Pass what you heard on to {}", c.person_name(to)),
        Intent::Post { about, concept, reply_to, .. } => {
            let who = if Some(about) == c.me() { "yourself".to_string() } else { c.person_name(about) };
            let what = concept_label(concept).to_lowercase();
            if reply_to != pw_world::socialnet::NO_POST { format!("Reply online ({what}) about {who}") } else { format!("Post online ({what}) about {who}") }
        }
    }
}

fn hours(h: Option<u64>, fallback: u8) -> u8 {
    h.map_or(fallback, |h| h.min(60) as u8)
}

fn person_id(w: &pw_world::World, id: u32) -> ApiResult<PersonId> {
    let p = PersonId(id);
    if w.people.get(p).is_none() {
        return Err(ApiError::NotFound("person".into()));
    }
    Ok(p)
}

fn build(s: &Session, req: ActReq) -> ApiResult<Intent> {
    let me = s.my_person().ok_or_else(|| ApiError::Unauthorized("You are observing the world; there is nobody to act for.".into()))?;
    let w = s.w();
    let p = w.people[me].player;
    let status = (p.is_some()).then(|| w.players.hot[p].status);
    let playing = status == Some(PlayerStatus::Active);
    let need_player = || if p.is_some() { Ok(()) } else { Err(ApiError::State("Only a player can do that.".into())) };
    let need_playing = || if playing { Ok(()) } else { Err(ApiError::State("You need to be playing for a club to do that.".into())) };
    Ok(match req {
        ActReq::Meet { with, topic, tone } => {
            let with = person_id(w, with)?;
            let topic = topic_from(&topic).ok_or_else(|| ApiError::Bad("Unknown subject.".into()))?;
            let tone = tone_from(tone.as_deref().unwrap_or("calm")).ok_or_else(|| ApiError::Bad("Unknown tone.".into()))?;
            if with == me {
                return Err(ApiError::Bad("Choose someone else to talk to.".into()));
            }
            let queued = w.intents.queue.iter().any(|pi| pi.person == me && matches!(pi.intent, Intent::RequestMeeting { with: x, .. } if x == with));
            if queued || w.meetings.has_pending(me, with) {
                return Err(ApiError::State("A conversation with them is already waiting to happen.".into()));
            }
            Intent::RequestMeeting { with, topic, tone }
        }
        ActReq::TransferRequest {} => {
            need_playing()?;
            if w.market.has_requested(p) {
                return Err(ApiError::State("You have already handed in a transfer request.".into()));
            }
            Intent::TransferRequest
        }
        ActReq::WithdrawRequest {} => {
            need_playing()?;
            if !w.market.has_requested(p) {
                return Err(ApiError::State("You have no transfer request to withdraw.".into()));
            }
            Intent::WithdrawTransferRequest
        }
        ActReq::Routine { hours: h } => {
            let cur = w.lives[me].routine;
            let h = h.unwrap_or_default();
            Intent::SetRoutine(Routine {
                rest: hours(h.rest, cur.rest),
                recovery: hours(h.recovery, cur.recovery),
                family: hours(h.family, cur.family),
                partner: hours(h.partner, cur.partner),
                social: hours(h.social, cur.social),
                study: hours(h.study, cur.study),
                hobbies: hours(h.hobbies, cur.hobbies),
                media: hours(h.media, cur.media),
                nightlife: hours(h.nightlife, cur.nightlife),
                language: hours(h.language, cur.language),
            })
        }
        ActReq::Lifestyle { value } => Intent::SetLifestyle(Lifestyle::ALL.iter().copied().find(|l| l.label() == value).ok_or_else(|| ApiError::Bad("Unknown lifestyle.".into()))?),
        ActReq::HireAgent { agent } => {
            need_player()?;
            if agent as usize >= w.agents.list.len() {
                return Err(ApiError::NotFound("agent".into()));
            }
            if w.agents.of_player.contains_key(&p) {
                return Err(ApiError::State("You already have an agent. Part ways first.".into()));
            }
            Intent::HireAgent(AgentId(agent))
        }
        ActReq::DropAgent {} => {
            need_player()?;
            if !w.agents.of_player.contains_key(&p) {
                return Err(ApiError::State("You do not have an agent.".into()));
            }
            Intent::DropAgent
        }
        ActReq::Retire {} => {
            need_player()?;
            if status == Some(PlayerStatus::Retired) {
                return Err(ApiError::State("You have already retired from playing.".into()));
            }
            Intent::Retire
        }
        ActReq::Unretire {} => {
            if status != Some(PlayerStatus::Retired) {
                return Err(ApiError::State("Only a retired player can come back.".into()));
            }
            Intent::Unretire
        }
        ActReq::SeekJob { role } => Intent::SeekStaffJob(ROLES.iter().find(|r| r.1 == role).map(|r| r.0).ok_or_else(|| ApiError::Bad("Unknown role.".into()))?),
        ActReq::Dating { open } => Intent::OpenToDating(open.unwrap_or(true)),
        ActReq::Partner { ask } => {
            if w.lives[me].partner().is_none() {
                return Err(ApiError::State("You do not have a partner.".into()));
            }
            Intent::AskPartner(match ask.as_str() {
                "movein" => PartnerAsk::MoveIn,
                "marry" => PartnerAsk::Marry,
                "separate" => PartnerAsk::Separate,
                _ => return Err(ApiError::Bad("Unknown question.".into())),
            })
        }
        ActReq::Amateur {} => {
            if status != Some(PlayerStatus::FreeAgent) {
                return Err(ApiError::State("Only a player without a club can sign up for amateur football.".into()));
            }
            Intent::JoinAmateurFootball
        }
        ActReq::Nation { nation } => {
            need_player()?;
            let n = NationId(nation);
            if !pw_sim::intl::eligible_nations(w, p).contains(&n) {
                return Err(ApiError::State("You are not eligible for that nation.".into()));
            }
            Intent::DeclareForNation(n)
        }
        ActReq::RetireInternational {} => {
            need_player()?;
            Intent::RetireFromInternational
        }
        ActReq::Pain { on } => {
            need_player()?;
            Intent::PlayThroughPain(on.unwrap_or(false))
        }
        ActReq::Mentor { person } => {
            need_playing()?;
            Intent::Mentor(person_id(w, person)?)
        }
        ActReq::Press { about, stance } => {
            let about = person_id(w, about)?;
            Intent::SpeakToPress { about, stance: STANCES.iter().find(|x| x.1 == stance).map(|x| x.0).ok_or_else(|| ApiError::Bad("Unknown stance.".into()))? }
        }
        ActReq::Enrol { course } => Intent::Enrol(COURSES.iter().copied().find(|c| course_key(*c) == course).ok_or_else(|| ApiError::Bad("Unknown course.".into()))?),
        ActReq::MoveHome { buy, quality } => Intent::MoveHome { buy: buy.unwrap_or(false), quality: quality.map_or(3, |q| q.clamp(1, 5) as u8) },
        ActReq::Helper { helper, quality } => {
            let h = Helper::ALL.iter().copied().find(|h| helper_key(*h) == helper).ok_or_else(|| ApiError::Bad("Unknown kind of help.".into()))?;
            Intent::HireHelper(h, quality.map_or(10, |q| q.clamp(1, 20) as u8))
        }
        ActReq::DismissHelper { helper } => Intent::DismissHelper(Helper::ALL.iter().copied().find(|h| helper_key(*h) == helper).ok_or_else(|| ApiError::Bad("Unknown kind of help.".into()))?),
        ActReq::Giving { pct, community } => Intent::SetGiving { pct: pct.map_or(0, |v| v.min(60) as u8), community: community.map_or(0, |v| v.min(40) as u8) },
        ActReq::Foundation {} => Intent::StartFoundation,
        ActReq::Invest { amount, risk } => {
            Intent::Invest { amount: amount.filter(|a| *a > 0).ok_or_else(|| ApiError::Bad("Choose an amount to invest.".into()))?, risk: risk.map_or(8, |r| r.clamp(1, 20) as u8) }
        }
        ActReq::Career { path } => {
            if playing {
                return Err(ApiError::State("A second career can only start once you are no longer playing for a club.".into()));
            }
            Intent::PursueCareer(PATHS.iter().copied().find(|c| path_key(*c) == path).ok_or_else(|| ApiError::Bad("Unknown kind of work.".into()))?)
        }
        ActReq::LeaveCareer {} => Intent::LeaveCareer,
        ActReq::Post { concept, about, reply_to, quote_of } => {
            let concept = POSTS.iter().find(|p| p.1 == concept).map(|p| p.0).ok_or_else(|| ApiError::Bad("Unknown kind of post.".into()))?;
            let (reply_to, quote_of) = (reply_to.unwrap_or(NO_POST), quote_of.unwrap_or(NO_POST));
            let mut about = about.map_or(me, PersonId);
            if about.0 as usize >= w.people.len() {
                return Err(ApiError::NotFound("person".into()));
            }
            for id in [reply_to, quote_of] {
                if id == NO_POST {
                    continue;
                }
                // You can only answer what you can see, and the subject follows the post.
                let post = w.net.post(id).ok_or_else(|| ApiError::NotFound("post".into()))?;
                if about == me && post.about.is_some() {
                    about = post.about;
                }
            }
            Intent::Post { about, concept, reply_to, quote_of }
        }
    })
}

/// Queue an action for the inhabited person. The reply says what was queued and when it takes effect.
pub fn act(s: &mut Session, args: &Value) -> ApiResult<Value> {
    let req: ActReq = crate::contract::request(args.clone())?;
    let intent = build(s, req)?;
    // Sending the same thing twice before the world has acted (a double click) would do it twice: say it is already waiting.
    // Settings are the exception: the last one sent wins and repeating one changes nothing.
    let is_setting = matches!(intent, Intent::SetRoutine(_) | Intent::SetLifestyle(_) | Intent::SetTraining(_) | Intent::SetGiving { .. } | Intent::OpenToDating(_) | Intent::PlayThroughPain(_));
    if let Some(me) = s.my_person() {
        if is_setting {
            // The new setting replaces one still waiting, so the list of what is queued never says the same thing twice.
            s.game.sim.world.intents.queue.retain(|pi| pi.person != me || std::mem::discriminant(&pi.intent) != std::mem::discriminant(&intent));
        } else if s.w().intents.queue.iter().any(|pi| pi.person == me && pi.intent == intent) {
            return Err(ApiError::State("That is already waiting to happen.".into()));
        }
    }
    let text = {
        let c = Ctx::new(s);
        intent_text(&c, &intent)
    };
    s.act(intent)?;
    Ok(serde_json::to_value(ActDone { ok: true, text, applies: "next day".into() }).unwrap_or(Value::Null))
}

/// The choices behind each action's picker, drawn from the world as this person can know it.
pub fn options(c: &Ctx) -> ApiResult<Value> {
    let me = c.me().ok_or_else(|| ApiError::Unauthorized("You are observing the world. Inhabit someone first.".into()))?;
    let w = c.w;
    let p = w.people[me].player;
    let aff = w.affairs.of(me);
    let manager = w.club_of_person(me);
    let manager_ref = if manager.is_some() { w.clubs[manager].manager.get().map(|m| w.staff[m].person) } else { None };
    let agent_person = (p.is_some()).then(|| w.agents.of_player.get(&p)).flatten().map(|r| w.agents.list[r.agent].person);
    let mut targets: Vec<Value> = Vec::new();
    if let Some(m) = manager_ref {
        targets.push(json!({"role": "Manager", "who": named(Ref::person(m), c.person_name(m))}));
    }
    if let Some(a) = agent_person {
        targets.push(json!({"role": "Agent", "who": named(Ref::person(a), c.person_name(a))}));
    }
    let agents: Vec<Value> = w
        .agents
        .list
        .iter_enumerated()
        .filter(|(_, a)| a.active)
        .map(|(id, a)| {
            let base = if a.base.is_some() { c.nation_name(a.base) } else { String::new() };
            (id, a, base)
        })
        .take(60)
        .map(|(id, a, base)| json!({"id": id.0, "person": named(Ref::person(a.person), c.person_name(a.person)), "base": base, "reputation": a.reputation, "clients": a.clients.len()}))
        .collect();
    let nations: Vec<Value> = if p.is_some() { pw_sim::intl::eligible_nations(w, p).iter().map(|&n| json!({"id": n.0, "name": c.nation_name(n)})).collect() } else { vec![] };
    let courses: Vec<Value> = COURSES
        .iter()
        .map(|&co| {
            json!({
                "key": course_key(co), "label": co.label(), "cost": co.cost(), "effort": co.effort(),
                "requires": co.requires().map(|r| r.label()),
                "done": aff.is_some_and(|a| a.has(co)),
                "studying": aff.and_then(|a| a.studying).is_some_and(|s| s.course == co),
            })
        })
        .collect();
    let helpers: Vec<Value> = Helper::ALL
        .iter()
        .map(|&h| json!({"key": helper_key(h), "label": h.label(), "base_cost": h.base_cost(), "hired": aff.and_then(|a| a.helper(h)).map(|x| json!({"quality": x.quality, "cost": x.cost}))}))
        .collect();
    Ok(json!({
        "topics": Topic::PLAYER_RAISES.iter().map(|&t| json!({"key": topic_key(t), "label": t.label()})).collect::<Vec<_>>(),
        "tones": Tone::ALL.iter().map(|t| json!({"key": t.label(), "label": t.label()})).collect::<Vec<_>>(),
        "meet_with": targets,
        "stances": STANCES.iter().map(|s| json!({"key": s.1, "label": s.2})).collect::<Vec<_>>(),
        "roles": ROLES.iter().map(|r| json!({"key": r.1, "label": r.0.label()})).collect::<Vec<_>>(),
        "courses": courses, "helpers": helpers,
        "careers": PATHS.iter().map(|&pa| json!({"key": path_key(pa), "label": pa.label()})).collect::<Vec<_>>(),
        "lifestyles": Lifestyle::ALL.iter().map(|l| json!({"key": l.label(), "label": l.label()})).collect::<Vec<_>>(),
        "posts": POSTS.iter().map(|p| json!({"key": p.1, "label": p.2})).collect::<Vec<_>>(),
        "agents": agents, "nations": nations, "routine_budget": Routine::BUDGET,
    }))
}
