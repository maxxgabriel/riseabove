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
        Intent::SeekStaffJob(r) => format!("Look for work as a {}", r.label().to_lowercase()),
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
        Intent::HireHelper(h, _) => format!("Hire a {}", h.label()),
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

fn hours(v: &Value, key: &str, fallback: u8) -> u8 {
    v.get(key).and_then(Value::as_u64).map_or(fallback, |h| h.min(60) as u8)
}

fn person_arg(args: &Value, key: &str) -> ApiResult<PersonId> {
    args.get(key).and_then(Value::as_u64).map(|n| PersonId(n as u32)).ok_or_else(|| ApiError::Bad(format!("missing {key}")))
}

fn text_arg<'a>(args: &'a Value, key: &str) -> ApiResult<&'a str> {
    args.get(key).and_then(Value::as_str).ok_or_else(|| ApiError::Bad(format!("missing {key}")))
}

fn build(s: &Session, args: &Value) -> ApiResult<Intent> {
    let me = s.my_person().ok_or_else(|| ApiError::State("You are observing the world; there is nobody to act for.".into()))?;
    let w = s.w();
    let p = w.people[me].player;
    let status = (p.is_some()).then(|| w.players.hot[p].status);
    let playing = status == Some(PlayerStatus::Active);
    let action = text_arg(args, "action")?;
    let need_player = || if p.is_some() { Ok(()) } else { Err(ApiError::State("Only a player can do that.".into())) };
    let need_playing = || if playing { Ok(()) } else { Err(ApiError::State("You need to be playing for a club to do that.".into())) };
    Ok(match action {
        "meet" => {
            let with = person_arg(args, "with")?;
            let topic = topic_from(text_arg(args, "topic")?).ok_or_else(|| ApiError::Bad("Unknown subject.".into()))?;
            let tone = tone_from(args.get("tone").and_then(Value::as_str).unwrap_or("calm")).ok_or_else(|| ApiError::Bad("Unknown tone.".into()))?;
            if with == me || w.people.get(with).is_none() {
                return Err(ApiError::Bad("Choose someone else to talk to.".into()));
            }
            let queued = w.intents.queue.iter().any(|pi| pi.person == me && matches!(pi.intent, Intent::RequestMeeting { with: x, .. } if x == with));
            if queued || w.meetings.has_pending(me, with) {
                return Err(ApiError::State("A conversation with them is already waiting to happen.".into()));
            }
            Intent::RequestMeeting { with, topic, tone }
        }
        "transfer_request" => {
            need_playing()?;
            if w.market.has_requested(p) {
                return Err(ApiError::State("You have already handed in a transfer request.".into()));
            }
            Intent::TransferRequest
        }
        "withdraw_request" => {
            need_playing()?;
            if !w.market.has_requested(p) {
                return Err(ApiError::State("You have no transfer request to withdraw.".into()));
            }
            Intent::WithdrawTransferRequest
        }
        "routine" => {
            let cur = w.lives[me].routine;
            let h = args.get("hours").cloned().unwrap_or(Value::Null);
            Intent::SetRoutine(Routine {
                rest: hours(&h, "rest", cur.rest),
                recovery: hours(&h, "recovery", cur.recovery),
                family: hours(&h, "family", cur.family),
                partner: hours(&h, "partner", cur.partner),
                social: hours(&h, "social", cur.social),
                study: hours(&h, "study", cur.study),
                hobbies: hours(&h, "hobbies", cur.hobbies),
                media: hours(&h, "media", cur.media),
                nightlife: hours(&h, "nightlife", cur.nightlife),
                language: hours(&h, "language", cur.language),
            })
        }
        "lifestyle" => {
            let v = text_arg(args, "value")?;
            Intent::SetLifestyle(Lifestyle::ALL.iter().copied().find(|l| l.label() == v).ok_or_else(|| ApiError::Bad("Unknown lifestyle.".into()))?)
        }
        "hire_agent" => {
            need_player()?;
            let a = args.get("agent").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing agent".into()))? as u32;
            if a as usize >= w.agents.list.len() {
                return Err(ApiError::NotFound("agent".into()));
            }
            if w.agents.of_player.contains_key(&p) {
                return Err(ApiError::State("You already have an agent. Part ways first.".into()));
            }
            Intent::HireAgent(AgentId(a))
        }
        "drop_agent" => {
            need_player()?;
            if !w.agents.of_player.contains_key(&p) {
                return Err(ApiError::State("You do not have an agent.".into()));
            }
            Intent::DropAgent
        }
        "retire" => {
            need_player()?;
            if status == Some(PlayerStatus::Retired) {
                return Err(ApiError::State("You have already retired from playing.".into()));
            }
            Intent::Retire
        }
        "unretire" => {
            if status != Some(PlayerStatus::Retired) {
                return Err(ApiError::State("Only a retired player can come back.".into()));
            }
            Intent::Unretire
        }
        "seek_job" => {
            let v = text_arg(args, "role")?;
            Intent::SeekStaffJob(ROLES.iter().find(|r| r.1 == v).map(|r| r.0).ok_or_else(|| ApiError::Bad("Unknown role.".into()))?)
        }
        "dating" => Intent::OpenToDating(args.get("open").and_then(Value::as_bool).unwrap_or(true)),
        "partner" => {
            if w.lives[me].partner().is_none() {
                return Err(ApiError::State("You do not have a partner.".into()));
            }
            Intent::AskPartner(match text_arg(args, "ask")? {
                "movein" => PartnerAsk::MoveIn,
                "marry" => PartnerAsk::Marry,
                "separate" => PartnerAsk::Separate,
                _ => return Err(ApiError::Bad("Unknown question.".into())),
            })
        }
        "amateur" => {
            if status != Some(PlayerStatus::FreeAgent) {
                return Err(ApiError::State("Only a player without a club can sign up for amateur football.".into()));
            }
            Intent::JoinAmateurFootball
        }
        "nation" => {
            need_player()?;
            let n = NationId(args.get("nation").and_then(Value::as_u64).ok_or_else(|| ApiError::Bad("missing nation".into()))? as u32);
            if !pw_sim::intl::eligible_nations(w, p).contains(&n) {
                return Err(ApiError::State("You are not eligible for that nation.".into()));
            }
            Intent::DeclareForNation(n)
        }
        "retire_international" => {
            need_player()?;
            Intent::RetireFromInternational
        }
        "pain" => {
            need_player()?;
            Intent::PlayThroughPain(args.get("on").and_then(Value::as_bool).unwrap_or(false))
        }
        "mentor" => {
            need_playing()?;
            Intent::Mentor(person_arg(args, "person")?)
        }
        "press" => {
            let about = person_arg(args, "about")?;
            let v = text_arg(args, "stance")?;
            Intent::SpeakToPress { about, stance: STANCES.iter().find(|x| x.1 == v).map(|x| x.0).ok_or_else(|| ApiError::Bad("Unknown stance.".into()))? }
        }
        "enrol" => {
            let v = text_arg(args, "course")?;
            Intent::Enrol(COURSES.iter().copied().find(|c| course_key(*c) == v).ok_or_else(|| ApiError::Bad("Unknown course.".into()))?)
        }
        "move_home" => Intent::MoveHome { buy: args.get("buy").and_then(Value::as_bool).unwrap_or(false), quality: args.get("quality").and_then(Value::as_u64).map_or(3, |q| q.clamp(1, 5) as u8) },
        "helper" => {
            let v = text_arg(args, "helper")?;
            let h = Helper::ALL.iter().copied().find(|h| helper_key(*h) == v).ok_or_else(|| ApiError::Bad("Unknown kind of help.".into()))?;
            Intent::HireHelper(h, args.get("quality").and_then(Value::as_u64).map_or(10, |q| q.clamp(1, 20) as u8))
        }
        "dismiss_helper" => {
            let v = text_arg(args, "helper")?;
            Intent::DismissHelper(Helper::ALL.iter().copied().find(|h| helper_key(*h) == v).ok_or_else(|| ApiError::Bad("Unknown kind of help.".into()))?)
        }
        "giving" => {
            Intent::SetGiving { pct: args.get("pct").and_then(Value::as_u64).map_or(0, |v| v.min(60) as u8), community: args.get("community").and_then(Value::as_u64).map_or(0, |v| v.min(40) as u8) }
        }
        "foundation" => Intent::StartFoundation,
        "invest" => Intent::Invest {
            amount: args.get("amount").and_then(Value::as_i64).filter(|a| *a > 0).ok_or_else(|| ApiError::Bad("Choose an amount to invest.".into()))?,
            risk: args.get("risk").and_then(Value::as_u64).map_or(8, |r| r.clamp(1, 20) as u8),
        },
        "career" => {
            if playing {
                return Err(ApiError::State("A second career can only start once you are no longer playing for a club.".into()));
            }
            let v = text_arg(args, "path")?;
            Intent::PursueCareer(PATHS.iter().copied().find(|c| path_key(*c) == v).ok_or_else(|| ApiError::Bad("Unknown kind of work.".into()))?)
        }
        "leave_career" => Intent::LeaveCareer,
        "post" => {
            let v = text_arg(args, "concept")?;
            let concept = POSTS.iter().find(|p| p.1 == v).map(|p| p.0).ok_or_else(|| ApiError::Bad("Unknown kind of post.".into()))?;
            let post_id = |key: &str| args.get(key).and_then(Value::as_u64).map_or(NO_POST, |n| n as u32);
            let (reply_to, quote_of) = (post_id("reply_to"), post_id("quote_of"));
            let mut about = args.get("about").and_then(Value::as_u64).map(|n| PersonId(n as u32)).unwrap_or(me);
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
        _ => return Err(ApiError::Bad("Unknown action.".into())),
    })
}

/// Queue an action for the inhabited person. The reply says what was queued and when it takes effect.
pub fn act(s: &mut Session, args: &Value) -> ApiResult<Value> {
    let intent = build(s, args)?;
    let text = {
        let c = Ctx::new(s);
        intent_text(&c, &intent)
    };
    s.act(intent)?;
    Ok(json!({"ok": true, "text": text, "applies": "next day"}))
}

/// The choices behind each action's picker, drawn from the world as this person can know it.
pub fn options(c: &Ctx) -> ApiResult<Value> {
    let me = c.me().ok_or_else(|| ApiError::State("You are observing the world. Inhabit someone first.".into()))?;
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
