//! `pathway` — live inside the world as one person, in a terminal.
//!
//!   pathway new synth [tiny|small|huge] [--seed S] [--warmup DAYS]
//!   pathway new import DIR [--seed S] [--warmup DAYS]
//!   pathway load FILE
//!
//! Every new world gets a fresh random seed unless `--seed` is given; the
//! seed is printed so the same world can be rebuilt for debugging.
//!
//! The world is built (or loaded) and can run on its own for as long as you
//! like before you choose anyone. Then you `find` a person and `become` them —
//! any player, any age, any situation — or `create` someone new through the
//! world's own generator. From then on the world asks you for their decisions.
//! Type `help` for commands.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use pw_career::{Advance, Game, Goal, GoalKind, NewPerson};
use pw_core::{Attr, AttrGroup, ClubId, DecisionId, NationId, PersonId, Pos};
use pw_data::DataPack;
use pw_world::interaction::{Tone, Topic};
use pw_world::{Focus, Intensity, Intent, Lifestyle, PartnerAsk, StaffRole, TrainingPlan, World};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<u64>().ok());
    let seed = args.iter().position(|a| a == "--seed").and_then(|i| args.get(i + 1)).map(|v| pw_core::rng::parse_seed(v));
    let world: World = match args.first().map(String::as_str) {
        Some("new") => match args.get(1).map(String::as_str) {
            Some("import") => {
                let dir = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| die("new import needs a folder")));
                let (w, rep) = pw_import::load_dir_seeded(&dir, DataPack::builtin(), seed).unwrap_or_else(|e| die(&e.to_string()));
                println!("Imported {} players, {} clubs ({} rows could not be placed). World seed {}.", rep.players, rep.clubs, rep.unresolved, pw_core::rng::seed_label(w.seed));
                w
            }
            _ => {
                let scale = match args.get(2).map(String::as_str) {
                    Some("tiny") => pw_import::synthetic::Scale::TINY,
                    Some("huge") => pw_import::synthetic::Scale::HUGE,
                    _ => pw_import::synthetic::Scale::SMALL,
                };
                let seed = seed.unwrap_or_else(pw_core::rng::fresh_seed);
                println!("World seed {}.", pw_core::rng::seed_label(seed));
                pw_import::synthetic::build(DataPack::builtin(), seed, scale)
            }
        },
        Some("load") => {
            let file = PathBuf::from(args.get(1).cloned().unwrap_or_else(|| die("load needs a file")));
            let g = Game::load(&file).unwrap_or_else(|e| die(&e.to_string()));
            repl(g);
            return;
        }
        _ => {
            println!("usage: pathway new synth [tiny|small|huge] [--seed S] [--warmup DAYS] | pathway new import DIR [--seed S] [--warmup DAYS] | pathway load FILE");
            return;
        }
    };
    let mut g = Game::new(world);
    if let Some(days) = flag("--warmup") {
        println!("Letting the world live for {days} days before you arrive…");
        for _ in 0..days {
            g.step();
        }
    }
    repl(g);
}

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1)
}

fn print(lines: impl IntoIterator<Item = String>) {
    for l in lines {
        println!("{l}");
    }
}

fn repl(mut g: Game) {
    println!("{} — the world is running. Type `help`.", g.world().date);
    let stdin = std::io::stdin();
    loop {
        let who = g.me().map_or("nobody".to_string(), |p| pw_narrate::fmt::person(g.world(), p));
        print!("\n[{} | {}] > ", g.world().date, who);
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        if !command(&mut g, &parts) {
            break;
        }
    }
}

fn me_or_warn(g: &Game) -> Option<PersonId> {
    let me = g.me();
    if me.is_none() {
        println!("You are not inhabiting anyone. Use `find` and `become`, or `create`.");
    }
    me
}

fn command(g: &mut Game, p: &[&str]) -> bool {
    let arg = |i: usize| p.get(i).copied().unwrap_or("");
    match p[0] {
        "quit" | "exit" => return false,
        "help" => help(),
        "find" => {
            let q = p[1..].join(" ");
            for (id, l) in pw_career::views::find(g.world(), &q, 40) {
                println!("{:>7}  {l}", id.0);
            }
        }
        "clubs" => {
            let w = g.world();
            for (id, c) in w.clubs.iter_enumerated().take(200) {
                println!("{:>5}  {:<30} {}", id.0, c.name, w.nations[c.nation].code);
            }
        }
        "become" => {
            let Ok(n) = arg(1).parse::<u32>() else {
                println!("become <person id>  (ids come from `find`)");
                return true;
            };
            let salt = pw_core::rng::fresh_seed();
            if g.take_control(PersonId(n), salt) {
                println!("You are now {}.", pw_narrate::fmt::person(g.world(), PersonId(n)));
                print(pw_career::views::status(g.world(), PersonId(n)));
            } else {
                println!("No such person.");
            }
        }
        "create" => create(g, &p[1..]),
        "leave" => {
            g.release();
            println!("You stepped out. Their own mind carries on.");
        }
        "me" | "status" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::status(g.world(), me));
            }
        }
        "self" | "attributes" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::self_view(g.world(), me));
                println!("{}", pw_career::views::personality_hint(g.world(), me));
            }
        }
        "inbox" => {
            if let Some(me) = me_or_warn(g) {
                let w = g.world();
                let threads = w.inbox.threads_of(me);
                if threads.is_empty() {
                    println!("No messages.");
                }
                for t in threads.into_iter().take(arg(1).parse().unwrap_or(20)) {
                    let unread = t.messages.iter().filter(|&&m| !w.inbox.messages[m as usize].read).count();
                    println!("{} [{}] {}  ({} messages{})", t.last, t.id, pw_narrate::inbox::thread_title(w, t), t.messages.len(), if unread > 0 { format!(", {unread} new") } else { String::new() });
                }
            }
        }
        "thread" => {
            let Ok(n) = arg(1).parse::<u32>() else {
                println!("thread <thread #>  (see `inbox`)");
                return true;
            };
            let Some(me) = me_or_warn(g) else { return true };
            {
                let w = g.world();
                let Some(t) = w.inbox.threads.get(n as usize).filter(|t| t.owner == me) else {
                    println!("No such thread.");
                    return true;
                };
                println!("— {} —", pw_narrate::inbox::thread_title(w, t));
                for &mid in &t.messages {
                    let m = &w.inbox.messages[mid as usize];
                    println!("{} <{}> {}", m.date, mid, pw_narrate::inbox::message(w, m));
                    if let Some(r) = pw_narrate::inbox::replied(w, m, me) {
                        println!("    {r}");
                    } else {
                        for (i, o) in pw_sim::inbox::options(w, mid).into_iter().enumerate() {
                            println!("    reply {mid} {i}: {}", pw_narrate::inbox::reply(w, m, o));
                        }
                    }
                }
            }
            g.read_thread(n);
        }
        "reply" => {
            let (Ok(m), Ok(k)) = (arg(1).parse::<u32>(), arg(2).parse::<usize>()) else {
                println!("reply <message #> <option #>  (see `thread`)");
                return true;
            };
            println!("{}", if g.reply(m, k) { "Sent. What happens next is up to them." } else { "That isn't a valid reply." });
        }
        "events" => {
            if let Some(me) = me_or_warn(g) {
                let n: usize = arg(1).parse().unwrap_or(25);
                for it in pw_career::feed::recent(g.world(), me, n) {
                    println!("{}{} #{}  {}", if it.important { "! " } else { "  " }, it.date, it.event.0, it.text);
                }
                g.session.seen = g.world().events.last_id();
            }
        }
        "new" => {
            if let Some(me) = me_or_warn(g) {
                let items = pw_career::feed::since(g.world(), me, g.session.seen);
                if items.is_empty() {
                    println!("Nothing new.");
                }
                for it in items {
                    println!("{}{} #{}  {}", if it.important { "! " } else { "  " }, it.date, it.event.0, it.text);
                }
                g.session.seen = g.world().events.last_id();
            }
        }
        "why" => {
            let Ok(n) = arg(1).parse::<u32>() else {
                println!("why <event #>");
                return true;
            };
            let me = g.me().unwrap_or(PersonId::NONE);
            let w = g.world();
            match w.events.get(pw_core::EventId(n)) {
                Some(e) if me.is_none() || pw_career::feed::concerns(w, me, e) || e.vis == pw_world::Visibility::Public => {
                    println!("{}", pw_narrate::events::line(w, e, me).unwrap_or_default());
                    if e.causes.is_empty() {
                        println!("  (no recorded causes)");
                    }
                    for c in &e.causes {
                        println!("  because {}", pw_narrate::events::cause(w, c, me));
                    }
                    if let pw_world::EventKind::Meeting { meeting, .. } = e.kind {
                        print(pw_narrate::talk::meeting(w, &w.meetings.list[meeting], me));
                    }
                    if let pw_world::EventKind::Published { story } = e.kind {
                        println!("  {}", pw_narrate::press::body(w, &w.media.stories[story]));
                    }
                }
                _ => println!("You don't know anything about that."),
            }
        }
        "meetings" => {
            if let Some(me) = me_or_warn(g) {
                let w = g.world();
                let list: Vec<_> = w.meetings.involving(me).collect();
                for (_, m) in list.iter().rev().take(8).rev() {
                    print(pw_narrate::talk::meeting(w, m, me));
                }
            }
        }
        "news" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::news(g.world(), me, arg(1).parse().unwrap_or(15)));
            }
        }
        "social" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::social(g.world(), me, 15));
            }
        }
        "feed" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::feed(g.world(), me, arg(1).parse().unwrap_or(15)));
            }
        }
        "post" => post(g, &p[1..]),
        "records" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::records(g.world(), me));
            }
        }
        "history" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::history(g.world(), me));
            }
        }
        "rumours" | "interest" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::rumours(g.world(), me));
            }
        }
        "people" | "relationships" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::relationships(g.world(), me));
            }
        }
        "promises" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::promises(g.world(), me));
            }
        }
        "contract" | "agent" | "talks" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::contract(g.world(), me));
            }
        }
        "life" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::life(g.world(), me));
            }
        }
        "club" | "squad" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::club_view(g.world(), me));
            }
        }
        "table" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::table(g.world(), me));
            }
        }
        "fixtures" | "results" => {
            if let Some(me) = me_or_warn(g) {
                print(pw_career::views::fixtures(g.world(), me));
            }
        }
        "decisions" => decisions(g),
        "answer" => {
            let (Ok(d), Ok(k)) = (arg(1).parse::<u32>(), arg(2).parse::<u8>()) else {
                println!("answer <decision #> <option #>");
                return true;
            };
            println!("{}", if g.answer(DecisionId(d), k) { "Done. It takes effect as the day is simulated." } else { "That isn't a valid answer." });
        }
        "meet" => meet(g, &p[1..]),
        "train" => train(g, &p[1..]),
        "routine" => routine(g, &p[1..]),
        "lifestyle" => {
            let l = match arg(1) {
                "frugal" => Lifestyle::Frugal,
                "modest" => Lifestyle::Modest,
                "comfortable" => Lifestyle::Comfortable,
                "lavish" => Lifestyle::Lavish,
                _ => {
                    println!("lifestyle frugal|modest|comfortable|lavish");
                    return true;
                }
            };
            act(g, Intent::SetLifestyle(l));
        }
        "request-transfer" => act(g, Intent::TransferRequest),
        "withdraw-request" => act(g, Intent::WithdrawTransferRequest),
        "agents" => agents(g),
        "hire-agent" => {
            let Ok(n) = arg(1).parse::<u32>() else {
                println!("hire-agent <agent #>  (see `agents`)");
                return true;
            };
            act(g, Intent::HireAgent(pw_core::AgentId(n)));
        }
        "drop-agent" => act(g, Intent::DropAgent),
        "retire" => act(g, Intent::Retire),
        "unretire" => act(g, Intent::Unretire),
        "seek-job" => {
            let role = match arg(1) {
                "manager" => StaffRole::Manager,
                "assistant" => StaffRole::Assistant,
                "scout" => StaffRole::Scout,
                "gk-coach" => StaffRole::GkCoach,
                "fitness" => StaffRole::FitnessCoach,
                "youth" => StaffRole::HeadOfYouth,
                _ => StaffRole::Coach,
            };
            act(g, Intent::SeekStaffJob(role));
        }
        "dating" => act(g, Intent::OpenToDating(arg(1) != "off")),
        "partner" => {
            let ask = match arg(1) {
                "movein" => PartnerAsk::MoveIn,
                "marry" => PartnerAsk::Marry,
                "separate" => PartnerAsk::Separate,
                _ => {
                    println!("partner movein|marry|separate");
                    return true;
                }
            };
            act(g, Intent::AskPartner(ask));
        }
        "goal" => {
            let today = g.world().date;
            let kind = match arg(1) {
                "apps" => GoalKind::Appearances(arg(2).parse().unwrap_or(100)),
                "goals" => GoalKind::Goals(arg(2).parse().unwrap_or(50)),
                "topflight" => GoalKind::TopFlight,
                _ => GoalKind::Personal,
            };
            g.session.goals.push(Goal { text: p[1..].join(" "), pinned: today, kind, done: None });
            println!("Pinned.");
        }
        "goals" => {
            for gl in &g.session.goals {
                println!("{} {} (pinned {})", if gl.done.is_some() { "[x]" } else { "[ ]" }, gl.text, gl.pinned);
            }
        }
        "note" => {
            let today = g.world().date;
            g.session.notes.push((today, p[1..].join(" ")));
        }
        "next" | "n" => {
            let mode = match arg(1) {
                "day" | "" => Advance::Day,
                "week" => Advance::Week,
                "event" | "e" => Advance::UntilEvent,
                "month" => Advance::Days(30),
                "season" => Advance::Days(365),
                x => x.parse().map_or(Advance::Day, Advance::Days),
            };
            let rep = g.advance(mode);
            println!("{} day(s) passed.{}", rep.days, rep.stopped_for.map_or(String::new(), |s| format!(" Stopped: {s}")));
            if let Some(me) = g.me() {
                let items = pw_career::feed::since(g.world(), me, g.session.seen);
                for it in items.iter().filter(|i| i.important).take(12) {
                    println!("! {} #{}  {}", it.date, it.event.0, it.text);
                }
                let rest = items.iter().filter(|i| !i.important).count();
                if rest > 0 {
                    println!("  (+{rest} more in `new`)");
                }
                g.session.seen = g.world().events.last_id();
                if !g.pending().is_empty() {
                    decisions(g);
                }
            }
        }
        "save" => {
            let f = PathBuf::from(if arg(1).is_empty() { "world.pws" } else { arg(1) });
            match g.save(&f) {
                Ok(()) => println!("Saved to {}.", f.display()),
                Err(e) => println!("Save failed: {e}"),
            }
        }
        _ => println!("Unknown command. Type `help`."),
    }
    true
}

/// post <praise|criticise|celebrate|lament|defend|mock|agree|disagree|statement> [about <person #>] [reply <post #>|quote <post #>]
fn post(g: &mut Game, a: &[&str]) {
    use pw_world::socialnet::{Concept, NO_POST};
    let Some(me) = me_or_warn(g) else { return };
    let concept = match a.first().copied().unwrap_or("") {
        "praise" => Concept::Praise,
        "criticise" | "criticize" => Concept::Criticise,
        "celebrate" => Concept::Celebrate,
        "lament" => Concept::Lament,
        "defend" => Concept::Defend,
        "mock" => Concept::Mock,
        "agree" => Concept::Agree,
        "disagree" => Concept::Disagree,
        "statement" => Concept::Statement,
        _ => {
            println!("post <praise|criticise|celebrate|lament|defend|mock|agree|disagree|statement> [about <person #>] [reply <post #>|quote <post #>]");
            return;
        }
    };
    let (mut about, mut reply_to, mut quote_of) = (me, NO_POST, NO_POST);
    for pair in a[1..].chunks(2) {
        let n: u32 = pair.get(1).and_then(|x| x.parse().ok()).unwrap_or(u32::MAX);
        match pair[0] {
            "about" if n != u32::MAX && (n as usize) < g.world().people.len() => about = PersonId(n),
            "reply" => reply_to = n,
            "quote" => quote_of = n,
            _ => {}
        }
    }
    // Replies and quotes must point at posts that exist; the subject follows.
    for id in [reply_to, quote_of] {
        if id != NO_POST {
            match g.world().net.post(id) {
                Some(p) if about == me && p.about.is_some() => about = p.about,
                Some(_) => {}
                None => {
                    println!("There is no post #{id}.");
                    return;
                }
            }
        }
    }
    act(g, Intent::Post { about, concept, reply_to, quote_of });
}

fn act(g: &mut Game, i: Intent) {
    if me_or_warn(g).is_some() && g.act(i) {
        println!("Noted — it happens as the next day is simulated.");
    }
}

fn decisions(g: &Game) {
    let w = g.world();
    let pending = g.pending();
    if pending.is_empty() {
        println!("Nothing needs your answer.");
    }
    for id in pending {
        let d = &w.decisions.all[id];
        println!("Decision #{} (answer by {}): {}", id.0, d.deadline, pw_narrate::choices::title(w, d));
        if let pw_world::DecisionKind::Meeting { meeting } = d.kind {
            print(pw_narrate::talk::meeting(w, &w.meetings.list[meeting], g.me().unwrap_or(PersonId::NONE)));
        }
        if let pw_world::DecisionKind::Negotiation { talk } = d.kind {
            print(pw_narrate::talk::negotiation(w, &w.talks[talk]));
        }
        for (i, c) in d.options.iter().enumerate() {
            let mark = if i == usize::from(d.default) { " (what you'd do by instinct)" } else { "" };
            println!("   {i}: {}{mark}", pw_narrate::choices::option(c));
        }
    }
}

fn meet(g: &mut Game, p: &[&str]) {
    let Some(me) = me_or_warn(g) else { return };
    let w = g.world();
    let pl = w.people[me].player;
    let with = match p.first().copied().unwrap_or("") {
        "manager" => (pl.is_some()).then(|| w.manager_of_player(pl)).flatten(),
        "agent" => w.agents.agent_of(pl).map(|a| w.agents.list[a].person),
        x => x.parse::<u32>().ok().map(PersonId),
    };
    let Some(with) = with else {
        println!("meet manager|agent|<person id> <topic> [tone]");
        println!("topics: minutes feedback position contract loan leave follow-up teammate apology market");
        println!("tones: calm assertive aggressive humble joking");
        return;
    };
    let topic = match p.get(1).copied().unwrap_or("") {
        "minutes" => Topic::PlayingTime,
        "feedback" => Topic::Feedback,
        "position" => Topic::Position,
        "contract" => Topic::NewContract,
        "loan" => Topic::LoanRequest,
        "leave" => Topic::WantAway,
        "follow-up" => Topic::PromiseFollowUp,
        "teammate" => Topic::TeammateIssue,
        "apology" => Topic::Apology,
        "market" => Topic::AgentReview,
        _ => {
            println!("Which topic? minutes feedback position contract loan leave follow-up teammate apology market");
            return;
        }
    };
    let tone = match p.get(2).copied().unwrap_or("calm") {
        "assertive" => Tone::Assertive,
        "aggressive" => Tone::Aggressive,
        "humble" => Tone::Humble,
        "joking" => Tone::Joking,
        _ => Tone::Calm,
    };
    act(g, Intent::RequestMeeting { with, topic, tone });
}

fn train(g: &mut Game, p: &[&str]) {
    let Some(me) = me_or_warn(g) else { return };
    let pl = g.world().people[me].player;
    if pl.is_none() {
        return;
    }
    let cur = g.world().players.cold[pl].plan;
    if p.is_empty() {
        println!("Current plan: focus {:?}, intensity {:?}, extra sessions {}, recovery sessions {}", cur.focus, cur.intensity, cur.extra, cur.recovery);
        println!("train <general|technical|mental|physical|goalkeeping|<attribute key>|pos:<CODE>> [light|normal|high] [extra 0-3] [recovery 0-3]");
        return;
    }
    let focus = match p[0] {
        "general" => Focus::General,
        "technical" => Focus::Group(AttrGroup::Technical),
        "mental" => Focus::Group(AttrGroup::Mental),
        "physical" => Focus::Group(AttrGroup::Physical),
        "goalkeeping" => Focus::Group(AttrGroup::Goalkeeping),
        x if x.starts_with("pos:") => match Pos::from_code(&x[4..].to_uppercase()) {
            Some(pos) => Focus::Position(pos),
            None => cur.focus,
        },
        x => Attr::from_key(x).map_or(cur.focus, Focus::Attribute),
    };
    let intensity = match p.get(1).copied() {
        Some("light") => Intensity::Light,
        Some("high") => Intensity::High,
        Some("normal") => Intensity::Normal,
        _ => cur.intensity,
    };
    let extra = p.get(2).and_then(|x| x.parse().ok()).unwrap_or(cur.extra);
    let recovery = p.get(3).and_then(|x| x.parse().ok()).unwrap_or(cur.recovery);
    act(g, Intent::SetTraining(TrainingPlan { focus, intensity, extra, recovery }));
}

fn routine(g: &mut Game, p: &[&str]) {
    let Some(me) = me_or_warn(g) else { return };
    let mut r = g.world().lives[me].routine;
    if p.is_empty() {
        println!("routine rest=14 recovery=3 family=6 partner=6 social=6 study=0 hobbies=6 media=1 nightlife=2 language=0  (budget 60 hours)");
        return;
    }
    for kv in p {
        let Some((k, v)) = kv.split_once('=') else { continue };
        let Ok(h) = v.parse::<u8>() else { continue };
        match k {
            "rest" => r.rest = h,
            "recovery" => r.recovery = h,
            "family" => r.family = h,
            "partner" => r.partner = h,
            "social" => r.social = h,
            "study" => r.study = h,
            "hobbies" => r.hobbies = h,
            "media" => r.media = h,
            "nightlife" => r.nightlife = h,
            "language" => r.language = h,
            _ => {}
        }
    }
    act(g, Intent::SetRoutine(r));
}

fn agents(g: &Game) {
    let Some(me) = me_or_warn(g) else { return };
    let w = g.world();
    let home = w.lives[me].home;
    let mut list: Vec<_> = w.agents.list.iter_enumerated().filter(|(_, a)| a.active && a.covers(home)).collect();
    list.sort_by_key(|(_, a)| std::cmp::Reverse(a.reputation));
    for (id, a) in list.into_iter().take(20) {
        // What anyone can find out about an agent: who they represent and their standing.
        println!("{:>5}  {:<26} reputation {:>5}  clients {:>3}", id.0, pw_narrate::fmt::person(w, a.person), a.reputation, a.clients.len());
    }
}

fn create(g: &mut Game, p: &[&str]) {
    // create <first> <last> <age> <position> [club id]
    if p.len() < 4 {
        println!("create <first> <last> <age> <position code> [club id]   — talent is drawn by the world, and hidden.");
        return;
    }
    let age: u8 = p[2].parse().unwrap_or(16);
    let pos = Pos::from_code(&p[3].to_uppercase()).unwrap_or(Pos::MC);
    let club = p.get(4).and_then(|x| x.parse::<u32>().ok()).map_or(ClubId::NONE, ClubId);
    let salt = pw_core::rng::fresh_seed();
    let nation = if club.is_some() { g.world().clubs[club].nation } else { NationId(0) };
    let (person, _) = pw_career::create_person(&mut g.sim.world, NewPerson { first: p[0].into(), last: p[1].into(), nation, club, age, pos, salt });
    if g.take_control(person, salt) {
        println!("{} enters the world.", pw_narrate::fmt::person(g.world(), person));
        print(pw_career::views::status(g.world(), person));
    }
}

fn help() {
    println!(
        "\
World:     find <name> · clubs · become <id> · create <first> <last> <age> <pos> [club] · leave
You:       me · self · life · people · promises · contract · rumours · records · history · goals · goal <apps N|goals N|topflight|text> · note <text>
Club:      club · table · fixtures · news · social · feed [n]
Post:      post <praise|criticise|celebrate|lament|defend|mock|agree|disagree|statement> [about <#>] [reply <post #>|quote <post #>]
Feed:      new · events [n] · inbox [n] · thread <#> · reply <msg #> <option #> · why <event #> · meetings
Decide:    decisions · answer <decision #> <option #>
Act:       meet manager|agent|<id> <topic> [tone] · train … · routine k=v … · lifestyle …
           request-transfer · withdraw-request · agents · hire-agent <#> · drop-agent
           retire · unretire · seek-job <manager|coach|scout|…> · dating on|off · partner movein|marry|separate
Time:      next [day|week|event|month|season|N]
Other:     save [file] · quit"
    );
}
