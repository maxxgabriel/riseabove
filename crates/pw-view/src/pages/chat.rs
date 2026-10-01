//! `me.chats`, `me.chat`, `me.chat_read`: the group chats and private messages around the inhabited person (`pw_world::chat`),
//! worded here from who spoke and what they reacted to. A message names only what its sender could know: a public result, the
//! person's own news.

use pw_core::PersonId;
use pw_core::rng::hash_key;
use pw_world::chat::{About, Chat, ChatMsg, Room, Said, Sender};
use serde_json::Value;

use crate::contract::{ChatLine, ChatRoomRow, ChatView, ChatsView, Done, IdReq};
use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Part, Ref};
use crate::session::Session;

fn need(c: &Ctx) -> ApiResult<PersonId> {
    c.me().ok_or_else(|| ApiError::Unauthorized("You are observing the world. Inhabit someone to read their chats.".into()))
}

fn first(c: &Ctx, p: PersonId) -> String {
    c.w.people.get(p).map_or_else(|| "mate".into(), |x| c.w.names.get(x.first).to_string())
}

fn title(c: &Ctx, room: Room) -> (String, &'static str, Option<Named>) {
    match room {
        Room::Squad { club } => (format!("{} squad", c.club_name(club)), "squad", None),
        Room::Team { inst } => (format!("{} team", c.w.minor.institutions.get(inst as usize).map_or("University", |i| i.name.as_str())), "team", None),
        Room::Family => ("Family".into(), "family", None),
        Room::Direct { with } => (c.person_name(with), "direct", Some(Named::new(Ref::person(with), c.person_name(with)))),
    }
}

/// One of a few ways to say it, the same each time this message is shown; consecutive messages of a day take different ones.
fn pick<'a>(m: &ChatMsg, i: usize, options: &[&'a str]) -> &'a str {
    let base = hash_key(&[u64::from(m.date.0 as u32), options.len() as u64]);
    options[((base + i as u64) % options.len() as u64) as usize]
}

fn about_words(c: &Ctx, a: About) -> String {
    match a {
        About::CallUp { nation } => format!("the {} call-up", c.nation_name(nation)),
        About::Capped { nation } => format!("your first {} cap", c.nation_name(nation)),
        About::StateSide { state } => format!("the {} squad", c.w.ext.ecosystem.regions.get(state).map_or("state", |r| r.name.as_str())),
        About::Debut => "your debut".into(),
        About::FirstGoal => "your first goal".into(),
        About::Contract { club } => format!("the contract with {}", c.club_name(club)),
        About::Moved { club } => format!("the move to {}", c.club_name(club)),
        About::Scholarship { inst } => format!("the place at {}", c.w.minor.institutions.get(inst as usize).map_or("university", |i| i.name.as_str())),
        About::Award => "the award".into(),
    }
}

fn words(c: &Ctx, me: PersonId, m: &ChatMsg, i: usize, family: bool) -> Vec<Part> {
    let t = |s: String| vec![Part::t(s)];
    match m.said {
        Said::AfterMatch { result, gf, ga, scored, derby, .. } => {
            let scorer = if scored.is_some() && scored != me { first(c, scored) } else { String::new() };
            let line = match (result.signum(), derby) {
                (1, true) => pick(m, i, &["Derby is ours! {s}", "The city is ours tonight. {s}", "{s} Nobody is quieter than their fans right now."]).replace("{s}", &format!("{gf}-{ga}.")),
                (1, false) if !scorer.is_empty() => pick(m, i, &["{n} with the goal! Three points.", "What a finish from {n}. Big win.", "Get in! Take a bow, {n}.", "Three points. {n} was on fire."]).replace("{n}", &scorer),
                (1, false) => pick(m, i, &["Get in! {s}", "Massive three points, lads.", "Good shift everyone. Recovery at ten, don't be late.", "{s} Clean work today."]).replace("{s}", &format!("{gf}-{ga}.")),
                (0, _) => pick(m, i, &["A point. We go again.", "Should have won that one.", "Not our best, but we didn't lose."]).to_string(),
                (_, true) => pick(m, i, &["Hurts to lose that one. We owe the fans.", "We'll remember this one. Back to work."]).to_string(),
                _ => pick(m, i, &["Heads up, boys. We go again.", "Not good enough today. Training at nine, let's respond.", "Tough one. Nobody sulks, we fix it this week."]).to_string(),
            };
            t(line)
        }
        Said::WellDone { .. } => t(pick(m, i, &["That was all you today.", "Take a bow, {n}.", "Best on the pitch for me."]).replace("{n}", &first(c, me))),
        Said::Congrats { about } => {
            let w = about_words(c, about);
            t(match about {
                About::Debut => pick(m, i, &["Welcome to the first team. First of many.", "Proper debut. Enjoy tonight."]).to_string(),
                About::FirstGoal => pick(m, i, &["First goal! Frame that ball.", "Finally off the mark! Many more."]).to_string(),
                About::Moved { club } => format!("Good luck at {}. Show them.", c.club_name(club)),
                _ => pick(m, i, &["Congrats on {w}. Deserved.", "Saw the news about {w}. Proud of you.", "Heard about {w}. You earned that."]).replace("{w}", &w),
            })
        }
        Said::Proud { about } => {
            let w = about_words(c, about);
            t(if family {
                pick(m, i, &["We heard about {w}. We are so proud of you.", "Everyone at home is talking about {w}. Call us tonight.", "We kept the paper. So proud of you."])
            } else {
                pick(m, i, &["So proud of you for {w}.", "Celebrating {w} tonight, no excuses."])
            }
            .replace("{w}", &w))
        }
        Said::GetWell { days } => {
            let weeks = (u32::from(days) + 6) / 7;
            t(if family {
                pick(m, i, &["Are you eating properly? Rest, and call us.", "Don't hide it from us. How bad is it?"]).to_string()
            } else {
                pick(m, i, &["Heal well. Don't rush it.", "Saw the news. Here if you need anything.", "About {w} weeks, they said? You'll come back stronger."]).replace("{w}", &weeks.to_string())
            })
        }
        Said::HeadUp => t(if family { pick(m, i, &["Come home for a few days. We are with you.", "Their loss. We believe in you."]) } else { pick(m, i, &["Their loss. Keep going.", "Head up. Something better is coming."]) }.to_string()),
        Said::CheckIn => t(pick(m, i, &["Haven't seen you in the squad lately. You okay? Keep working, your chance will come.", "Coffee after training? Want to hear how you're doing.", "Stay ready. The gaffer notices who keeps working."]).to_string()),
        Said::Welcome { who } if who == me => t(pick(m, i, &["Welcome, {n}! Good to have you.", "Welcome aboard, {n}. Ask if you need anything."]).replace("{n}", &first(c, me))),
        Said::Welcome { who } => t(pick(m, i, &["Welcome {n}! Good to have you.", "Big welcome to {n}."]).replace("{n}", &first(c, who))),
        Said::Farewell { who } if who == me => t(pick(m, i, &["Going to miss you. Good luck at the new place.", "All the best, brother. Keep in touch."]).to_string()),
        Said::Farewell { who } => t(pick(m, i, &["Good luck {n}, thanks for everything.", "All the best {n}!"]).replace("{n}", &first(c, who))),
        Said::Birthday { who } if who == me => t(if family { "Happy birthday! Call us tonight." } else { pick(m, i, &["Happy birthday! Cake's on you.", "Happy birthday, legend."]) }.to_string()),
        Said::Birthday { who } => t(pick(m, i, &["Happy birthday {n}!", "Happy birthday {n}, have a good one."]).replace("{n}", &first(c, who))),
        Said::Missing => t(pick(m, i, &["When are you coming home? Everyone keeps asking.", "We miss you. Call when you can.", "The house is quiet without you."]).to_string()),
        Said::AgentNews { club } => {
            let s = pick(m, i, &["Had a call from {c} about you. Early days, I'll keep you posted.", "{c} asked about you. Nothing concrete yet."]);
            let (a, b) = s.split_once("{c}").unwrap_or((s, ""));
            vec![Part::t(a), Part::l(Ref::club(club), c.club_name(club)), Part::t(b)]
        }
    }
}

fn sender(c: &Ctx, from: Sender) -> (String, Option<Named>, bool) {
    match from {
        Sender::Person(p) if c.w.people.get(p).is_some() => (c.person_name(p), Some(Named::new(Ref::person(p), c.person_name(p))), false),
        Sender::Person(_) => ("Someone".into(), None, false),
        Sender::Mother => ("Mum".into(), None, false),
        Sender::Father => ("Dad".into(), None, false),
        Sender::Sibling => ("Your sibling".into(), None, false),
        Sender::You => ("You".into(), None, true),
    }
}

fn line(c: &Ctx, me: PersonId, chat: &Chat, i: usize) -> ChatLine {
    let m = &chat.msgs[i];
    let (from, who, mine) = sender(c, m.from);
    // The message's own number in the room: stable as later messages arrive and older ones scroll away.
    let n = (chat.posted as usize).saturating_sub(chat.msgs.len()) + i;
    ChatLine { date: m.date.0, from, who, mine, text: words(c, me, m, n, chat.room == Room::Family) }
}

pub fn rooms(c: &Ctx) -> ApiResult<Value> {
    let me = need(c)?;
    let mut rooms: Vec<ChatRoomRow> = c
        .w
        .ext
        .chats
        .of
        .get(&me)
        .map(|inbox| {
            inbox
                .chats
                .iter()
                .enumerate()
                .filter(|(_, ch)| !ch.msgs.is_empty())
                .map(|(i, ch)| {
                    let (title, kind, with) = title(c, ch.room);
                    let last = ch.msgs.len() - 1;
                    let l = line(c, me, ch, last);
                    let text: String = l.text.iter().map(|p| p.t.as_str()).collect();
                    let preview = if matches!(ch.room, Room::Direct { .. }) { text } else { format!("{}: {text}", l.from) };
                    ChatRoomRow { id: i as u32, kind: kind.into(), title, with, last: Some(l.date), preview, unread: ch.unread() }
                })
                .collect()
        })
        .unwrap_or_default();
    rooms.sort_by_key(|r| (std::cmp::Reverse(r.last), r.id));
    serde_json::to_value(ChatsView { rooms }).map_err(|e| ApiError::Internal(e.to_string()))
}

pub fn room(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let me = need(c)?;
    let id = crate::contract::request::<IdReq>(args.clone())?.id;
    let ch = c.w.ext.chats.of.get(&me).and_then(|i| i.chats.get(id as usize)).ok_or_else(|| ApiError::NotFound("chat".into()))?;
    let (title, kind, _) = title(c, ch.room);
    let lines = (0..ch.msgs.len()).map(|i| line(c, me, ch, i)).collect();
    serde_json::to_value(ChatView { id, kind: kind.into(), title, lines }).map_err(|e| ApiError::Internal(e.to_string()))
}

pub fn read(s: &mut Session, args: &Value) -> ApiResult<Value> {
    let id = crate::contract::request::<IdReq>(args.clone())?.id;
    let me = s.my_person().ok_or_else(|| ApiError::Unauthorized("You are observing the world; there are no chats to read.".into()))?;
    if let Some(ch) = s.game.sim.world.ext.chats.of.get_mut(&me).and_then(|i| i.chats.get_mut(id as usize)) {
        ch.read = ch.posted;
        s.revision += 1;
    }
    Ok(crate::contract::wire(Done { ok: true }))
}
