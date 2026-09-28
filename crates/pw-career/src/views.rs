//! What the inhabited person can know (P3, S15). Every view reads the world
//! through their eyes: their own feelings (with the reasons), what they have
//! been told and by whom, what is public, and how their coaches' assessments
//! frame their ability. Nobody else's hidden numbers, no manager's private
//! plans, no club's secret shortlist.

use pw_core::{Attr, ClubId, Hidden, PersonId, PlayerId, Pos};
use pw_narrate::fmt::{club, feeling, level, money, nation, person, wage};
use pw_world::beliefs::{BeliefKind, Channel};
use pw_world::knowledge::{Observer, field, perceive, sigma};
use pw_world::life::Life;
use pw_world::{PlayerStatus, PromiseState, World};

fn words_condition(v: u8) -> &'static str {
    match v {
        0..=59 => "exhausted",
        60..=74 => "tired",
        75..=89 => "okay",
        _ => "fresh",
    }
}

fn words_sharpness(v: u8) -> &'static str {
    match v {
        0..=39 => "rusty",
        40..=59 => "lacking match sharpness",
        60..=79 => "sharp",
        _ => "razor sharp",
    }
}

fn words_morale(v: u8) -> &'static str {
    match v {
        0..=24 => "very low",
        25..=44 => "low",
        45..=59 => "okay",
        60..=79 => "good",
        _ => "superb",
    }
}

pub fn status(w: &World, me: PersonId) -> Vec<String> {
    let mut v = Vec::new();
    let pe = &w.people[me];
    v.push(format!("{} — {} years old, {}", person(w, me), pe.age(w.date), nation(w, pe.nation)));
    let p = pe.player;
    if p.is_some() {
        let h = &w.players.hot[p];
        let c = &w.players.cold[p];
        let naturals: Vec<&str> = c.natural_positions().map(|x| x.code()).collect();
        v.push(format!("Positions: {}", if naturals.is_empty() { c.best_pos.code().to_string() } else { naturals.join(", ") }));
        match h.status {
            PlayerStatus::Active => {
                v.push(format!("Club: {} ({}) — {}", club(w, h.club), w.team_name(h.team), c.status.label()));
                if let Some(l) = &c.loan {
                    v.push(format!("On loan at {} until {}", club(w, l.club), l.end));
                }
                v.push(format!("Contract: {} until {}", wage(c.contract.current_wage(w.date)), c.contract.end));
            }
            PlayerStatus::FreeAgent => v.push("Unattached — a free agent.".into()),
            PlayerStatus::Retired => v.push("Retired from playing.".into()),
            PlayerStatus::Amateur => {
                let local = w.youth.member_of.get(&p).map(|&l| w.youth.local[l].name.clone());
                v.push(format!("Playing amateur football{}.", local.map_or(String::new(), |n| format!(" for {n}"))));
            }
        }
        if h.status != PlayerStatus::Retired {
            v.push(format!(
                "Body: {}, {}. Morale {}. Confidence {}.",
                words_condition(h.condition),
                words_sharpness(h.sharpness),
                words_morale(h.morale),
                level(h.confidence)
            ));
            if h.injury != 0 {
                let name = w.data.injuries.get(usize::from(h.injury - 1)).map_or("injury", |d| d.name.as_str());
                v.push(format!("Injured: {name}, about {} days to go.", h.injury_days));
            }
            if h.ban > 0 {
                v.push(format!("Suspended for {} match(es).", h.ban));
            }
            let form: Vec<String> = h.form.iter().filter(|&&r| r > 0).map(|&r| format!("{:.1}", f32::from(r) / 10.0)).collect();
            if !form.is_empty() {
                v.push(format!("Last ratings: {}", form.join("  ")));
            }
            v.push(format!("Coaches rate your training as {}.", level(h.training)));
            v.push(format!("Senior career: {} appearances, {} goals.", c.senior_apps, c.senior_goals));
        }
        let life = &w.lives[me];
        if !life.morale_why.is_empty() {
            v.push("How you feel about things:".into());
            let mut why = life.morale_why.clone();
            why.sort_by_key(|(_, x)| std::cmp::Reverse(x.unsigned_abs()));
            for (f, x) in why.iter().take(6) {
                v.push(format!("  {} {}", feeling(*x), f.label()));
            }
        }
        if let Some(b) = w.beliefs.about(me, me).find(|b| matches!(b.kind, BeliefKind::SelectionOutlook { .. })) {
            if let BeliefKind::SelectionOutlook { start_pct } = b.kind {
                v.push(format!("Last time you spoke, {} put your chances of starting at about {}% ({}).", channel(w, &b.channel), start_pct, b.date));
            }
        }
    }
    if pe.staff.is_some() && w.staff[pe.staff].employed() {
        let s = &w.staff[pe.staff];
        v.push(format!("Works as {} at {}.", s.role.label(), club(w, s.club)));
    }
    v
}

fn channel(w: &World, c: &Channel) -> String {
    match c {
        Channel::Witnessed => "you saw it yourself".into(),
        Channel::Told(p) => person(w, *p),
        Channel::Agent(p) => format!("your agent {}", person(w, *p)),
        Channel::Club(c) => club(w, *c),
        Channel::Media(s) => pw_narrate::press::outlet_name(w, &w.media.stories[*s]),
        Channel::Public => "everyone".into(),
        Channel::Inferred => "your own reading of things".into(),
    }
}

/// Your attributes as the people who work with you frame them: ranges that
/// tighten as they see more of you. Never the hidden truth.
pub fn self_view(w: &World, me: PersonId) -> Vec<String> {
    let p = w.people[me].player;
    if p.is_none() {
        return vec!["You are not a player.".into()];
    }
    let c = &w.players.cold[p];
    let club = w.players.hot[p].club;
    let t = &w.data.tuning.perception;
    let (judging, _) = if club.is_some() { w.club_manager_judging(club) } else { (6.0, 6.0) };
    let seen = if club.is_some() { w.knowledge.seen(club, p) } else { None };
    let s = sigma(t, seen, judging, w.date, false).max(0.6);
    let keeper = c.best_pos == Pos::GK;
    let mut v = vec![format!("As your coaches see you (± {:.0}):", s)];
    let groups: [(&str, &[Attr]); 4] = [
        (
            "Technical",
            &[Attr::Corners, Attr::Crossing, Attr::Dribbling, Attr::Finishing, Attr::FirstTouch, Attr::FreeKicks, Attr::Heading, Attr::LongShots, Attr::LongThrows, Attr::Marking, Attr::Passing, Attr::PenaltyTaking, Attr::Tackling, Attr::Technique],
        ),
        (
            "Mental",
            &[Attr::Aggression, Attr::Anticipation, Attr::Bravery, Attr::Composure, Attr::Concentration, Attr::Decisions, Attr::Determination, Attr::Flair, Attr::Leadership, Attr::OffTheBall, Attr::Positioning, Attr::Teamwork, Attr::Vision, Attr::WorkRate],
        ),
        ("Physical", &[Attr::Acceleration, Attr::Agility, Attr::Balance, Attr::JumpingReach, Attr::NaturalFitness, Attr::Pace, Attr::Stamina, Attr::Strength]),
        (
            "Goalkeeping",
            &[Attr::AerialReach, Attr::CommandOfArea, Attr::Communication, Attr::Eccentricity, Attr::Handling, Attr::Kicking, Attr::OneOnOnes, Attr::Reflexes, Attr::RushingOut, Attr::Throwing],
        ),
    ];
    for (name, attrs) in groups {
        if name == "Goalkeeping" && !keeper {
            continue;
        }
        v.push(format!("{name}:"));
        let mut line = String::new();
        for (i, &a) in attrs.iter().enumerate() {
            let est = perceive(c.attrs.get(a), s, Observer::Person(me.0), p, field::ATTR + a.idx() as u64);
            let lo = (est - s).round().clamp(1.0, 20.0) as u8;
            let hi = (est + s).round().clamp(1.0, 20.0) as u8;
            let cell = if lo == hi { format!("{lo}") } else { format!("{lo}-{hi}") };
            line += &format!("  {:<16}{:>6}", a.label(), cell);
            if i % 3 == 2 {
                v.push(std::mem::take(&mut line));
            }
        }
        if !line.is_empty() {
            v.push(line);
        }
    }
    for b in w.beliefs.about(me, me) {
        match b.kind {
            BeliefKind::Assessment { ca_lo, ca_hi, ceiling } => {
                let stars = "*".repeat(usize::from(ceiling));
                v.push(format!("{} rates your level at {}-{} and your ceiling {stars} ({}).", channel(w, &b.channel), ca_lo / 10, ca_hi / 10, b.date));
            }
            BeliefKind::ManagerRating { manager, stars } => {
                v.push(format!("{} gives you {} out of 5 ({}).", person(w, manager), stars, b.date));
            }
            _ => {}
        }
    }
    v.push(format!("Personality as others describe you: {}.", w.people[me].hidden.personality_label()));
    v
}

pub fn rumours(w: &World, me: PersonId) -> Vec<String> {
    let mut v = Vec::new();
    let mut bs: Vec<_> = w.beliefs.of(me).collect();
    bs.sort_by_key(|b| std::cmp::Reverse(b.date));
    for b in bs {
        match b.kind {
            BeliefKind::ClubInterested { club: c } => v.push(format!("{} — {} are interested (via {}, {}% sure).", b.date, club(w, c), channel(w, &b.channel), b.confidence)),
            BeliefKind::Rumour { story } => {
                let s = &w.media.stories[story];
                v.push(format!("{} — {}: \"{}\"", b.date, pw_narrate::press::outlet_name(w, s), pw_narrate::press::headline(w, s)));
            }
            BeliefKind::BidMade { club: c, fee } => v.push(format!("{} — {} bid {} (via {}).", b.date, club(w, c), money(fee), channel(w, &b.channel))),
            _ => {}
        }
    }
    if v.is_empty() {
        v.push("You haven't heard of any interest.".into());
    }
    v
}

/// The people in your life, as you feel about them, and why.
pub fn relationships(w: &World, me: PersonId) -> Vec<String> {
    let today = w.date;
    let grudge = pw_world::social::grudge_factor(&w.people[me]);
    let mut rels: Vec<(PersonId, pw_world::Rel)> = w.social.relations_of(me).collect();
    rels.sort_by_key(|(p, r)| (std::cmp::Reverse(i32::from(r.affinity).abs() + (i32::from(r.trust) - 50).abs()), *p));
    let mut v = Vec::new();
    for (p, r) in rels.into_iter().take(20) {
        let role = role_of(w, p);
        let why = w.social.defining_memory(me, p, today, grudge).map_or(String::new(), |m| format!(" — they {} ({})", m.kind.text(), m.date));
        v.push(format!("{:<26} {:<22} {}{}", person(w, p), role, r.label(), why));
    }
    if v.is_empty() {
        v.push("No relationships to speak of yet.".into());
    }
    v
}

fn role_of(w: &World, p: PersonId) -> String {
    let x = &w.people[p];
    if x.staff.is_some() && w.staff[x.staff].employed() {
        return format!("{} ({})", w.staff[x.staff].role.label(), pw_narrate::fmt::club_short(w, w.staff[x.staff].club));
    }
    if x.player.is_some() {
        let h = &w.players.hot[x.player];
        if h.status == PlayerStatus::Active {
            return format!("player ({})", pw_narrate::fmt::club_short(w, h.club));
        }
    }
    if w.media.journalists.contains_key(&p) {
        return "journalist".into();
    }
    if w.agents.list.iter().any(|a| a.person == p) {
        return "agent".into();
    }
    "".into()
}

pub fn promises(w: &World, me: PersonId) -> Vec<String> {
    let mut v = Vec::new();
    for pr in w.social.promises.iter().filter(|pr| pr.to == me || pr.from == me) {
        let state = match pr.state {
            PromiseState::Open => format!("due {}", pr.due),
            PromiseState::Kept => "kept".into(),
            PromiseState::Broken => "BROKEN".into(),
            PromiseState::Void => "void".into(),
        };
        let (a, b) = if pr.from == me { ("You".to_string(), person(w, pr.to)) } else { (person(w, pr.from), "you".to_string()) };
        let mut line = format!("{} promised {b} {} — {state}", a, pr.kind.text());
        if let pw_world::PromiseKind::Minutes { share } = pr.kind {
            if pr.team_minutes > 0 {
                line += &format!(" (so far {:.0}% of {:.0}% promised)", pr.player_minutes as f32 / pr.team_minutes as f32 * 100.0, share * 100.0);
            }
        }
        v.push(line);
    }
    if v.is_empty() {
        v.push("No promises on record.".into());
    }
    v
}

pub fn contract(w: &World, me: PersonId) -> Vec<String> {
    let p = w.people[me].player;
    if p.is_none() {
        return vec!["No playing contract.".into()];
    }
    let c = &w.players.cold[p].contract;
    let mut v = Vec::new();
    if c.club.is_some() {
        v.push(format!("{} — {} until {}", club(w, c.club), wage(c.current_wage(w.date)), c.end));
        v.push(format!("Yearly rise {}%, relegation cut {}%, appearance bonus {}, goal bonus {}", c.yearly_rise, c.relegation_cut, money(c.appearance_bonus), money(c.goal_bonus)));
        if c.release_clause > 0 {
            v.push(format!("Release clause: {}", money(c.release_clause)));
        }
        if let Some(s) = c.promised_status {
            v.push(format!("Status written into the contract: {}", s.label()));
        }
    } else {
        v.push("No contract.".into());
    }
    match w.agents.of_player.get(&p) {
        Some(r) => {
            let a = &w.agents.list[r.agent];
            v.push(format!("Agent: {} ({}% fee, until {}). You are {} with them.", person(w, a.person), r.fee_pct, r.until, level(r.satisfaction)));
        }
        None => v.push("No agent.".into()),
    }
    if w.market.has_requested(p) {
        v.push("You have handed in a transfer request.".into());
    }
    if w.market.listed.contains_key(&p) {
        v.push("Your club has made you available for transfer.".into());
    }
    if let Some(&t) = w.market.talking.get(&p) {
        v.extend(pw_narrate::talk::negotiation(w, &w.talks[t]));
    }
    v
}

pub fn life(w: &World, me: PersonId) -> Vec<String> {
    let l: &Life = &w.lives[me];
    let mut v = vec![format!("Living in {} since {}.", nation(w, l.home), l.home_since)];
    let langs: Vec<String> = l.languages.iter().map(|(n, f)| format!("{} ({})", w.nations.get(*n).map_or("?".to_string(), |x| x.code.clone()), level(*f))).collect();
    v.push(format!("Languages: {}", langs.join(", ")));
    match l.partner() {
        Some(pt) => v.push(format!(
            "Partner: {} — {}, {}; your bond is {}{}.",
            person(w, pt.person),
            pt.status.label(),
            w.lives[pt.person].occupation.label(),
            level(pt.bond),
            if pt.lives != l.home { format!(", living in {}", nation(w, pt.lives)) } else { String::new() }
        )),
        None => v.push("Single.".into()),
    }
    if l.household.children > 0 {
        v.push(format!("Children: {}", l.household.children));
    }
    let par = &l.household.parents;
    v.push(match par.alive {
        0 => "Your parents have passed away.".into(),
        n => format!("{} parent(s) in {}, health {}, you are {} close.", n, nation(w, par.nation), level(par.health), level(par.closeness)),
    });
    let f = &l.finances;
    v.push(format!("Money: savings {}, last month in {} / out {}, lifestyle {}{}.", money(f.savings), money(f.income), money(f.spending), f.lifestyle.label(), if f.debt > 0 { format!(", debt {}", money(f.debt)) } else { String::new() }));
    let r = l.routine;
    v.push(format!(
        "Your week (hours): rest {} · recovery {} · family {} · partner {} · social {} · study {} · hobbies {} · media {} · nights out {} · language {}",
        r.rest, r.recovery, r.family, r.partner, r.social, r.study, r.hobbies, r.media, r.nightlife, r.language
    ));
    v.push(format!("Stress {}, sleep {}, sense of purpose {}.", level(100 - l.stress), level(l.sleep), level(l.fulfilment)));
    if !l.wellbeing_why.is_empty() {
        let mut why = l.wellbeing_why.clone();
        why.sort_by_key(|(_, x)| std::cmp::Reverse(x.unsigned_abs()));
        for (fct, x) in why.iter().take(5) {
            v.push(format!("  {} {}", feeling(*x), fct.label()));
        }
    }
    v
}

/// Your club as a squad member sees it: names, ages, positions, public stats.
pub fn club_view(w: &World, me: PersonId) -> Vec<String> {
    let c = w.club_of_person(me);
    if c.is_none() {
        return vec!["You are not at a club.".into()];
    }
    let cl = &w.clubs[c];
    let mut v = vec![format!("{} — {}, founded {}. Stadium {} ({}).", cl.name, nation(w, cl.nation), cl.founded, cl.stadium, cl.capacity)];
    if let Some(m) = cl.manager.get() {
        v.push(format!("Manager: {}", w.staff_name(m)));
    }
    if cl.league.is_some() {
        let comp = &w.comps[cl.league];
        let pos = comp.position_of(cl.first_team()).map_or("-".to_string(), |x| x.to_string());
        v.push(format!("{}: currently {pos}.", comp.name));
    }
    let p = w.people[me].player;
    let team = if p.is_some() { w.players.hot[p].team } else { cl.first_team() };
    let t = &w.teams[team];
    v.push(format!("{} squad (captain {}):", w.team_name(team), if t.captain.is_some() { w.player_name(t.captain) } else { "-".into() }));
    let mut squad = t.squad.clone();
    squad.sort_by_key(|&x| (w.players.cold[x].best_pos.idx(), x));
    for x in squad {
        let cx = &w.players.cold[x];
        let apps: u16 = w.stats.for_player(x).map(|l| l.apps).sum();
        let goals: u16 = w.stats.for_player(x).map(|l| l.goals).sum();
        let mark = if x == p { "»" } else { " " };
        v.push(format!("{mark} {:<4} {:<26} {:>2}  {:>3} apps {:>3} gls", cx.best_pos.code(), w.player_name(x), w.age(x), apps, goals));
    }
    v
}

pub fn table(w: &World, me: PersonId) -> Vec<String> {
    let c = w.club_of_person(me);
    let league = if c.is_some() { w.clubs[c].league } else { pw_core::CompId::NONE };
    if league.is_none() {
        return vec!["No league.".into()];
    }
    let comp = &w.comps[league];
    let mut v = vec![format!("{} {}", comp.name, comp.state.season)];
    for (i, r) in comp.sorted_table().iter().enumerate() {
        let mine = w.teams[r.team].club == c;
        v.push(format!("{}{:>2}. {:<28} {:>2} {:>2} {:>2} {:>2} {:>3}:{:<3} {:>3}", if mine { "»" } else { " " }, i + 1, w.team_name(r.team), r.played, r.won, r.drawn, r.lost, r.gf, r.ga, r.points));
    }
    v
}

pub fn fixtures(w: &World, me: PersonId) -> Vec<String> {
    let p = w.people[me].player;
    if p.is_none() {
        return vec![];
    }
    let team = w.players.hot[p].team;
    if team.is_none() {
        return vec!["No team.".into()];
    }
    let mut v = Vec::new();
    let played: Vec<_> = w.fixtures.between(w.date.add_days(-35), w.date.add_days(-1)).filter(|&f| w.fixtures.get(f).involves(team)).collect();
    for f in played {
        let fx = w.fixtures.get(f);
        if let Some(s) = fx.score {
            let mine = w.reports.get(&fx.uid).and_then(|r| r.line(p)).map_or(String::new(), |l| if l.minutes > 0 { format!("  you: {}' {:.1}", l.minutes, l.rating) } else { "  you: unused".into() });
            v.push(format!("{}  {} {}-{} {}  ({}){mine}", fx.date, w.team_short(fx.home), s.home, s.away, w.team_short(fx.away), w.comps[fx.comp].short_name));
        }
    }
    let next: Vec<_> = w.fixtures.between(w.date, w.date.add_days(35)).filter(|&f| w.fixtures.get(f).involves(team)).collect();
    for f in next {
        let fx = w.fixtures.get(f);
        v.push(format!("{}  {} v {}  ({})", fx.date, w.team_short(fx.home), w.team_short(fx.away), w.comps[fx.comp].short_name));
    }
    v
}

pub fn news(w: &World, me: PersonId, n: usize) -> Vec<String> {
    let home = w.lives[me].home;
    let mut v = Vec::new();
    for s in w.media.stories.iter().rev().filter(|s| s.outlet.is_some() && w.media.outlets[s.outlet].nation == home).take(n) {
        v.push(format!("{}  [{}] {}", s.date, pw_narrate::press::outlet_name(w, s), pw_narrate::press::headline(w, s)));
    }
    if v.is_empty() {
        v.push("No news.".into());
    }
    v
}

pub fn social(w: &World, me: PersonId, n: usize) -> Vec<String> {
    let club = w.club_of_person(me);
    let mut v: Vec<String> = w
        .media
        .reactions
        .iter()
        .rev()
        .filter(|r| r.about == me || (club.is_some() && r.club == club))
        .take(n)
        .map(|r| format!("{}  {}", r.date, pw_narrate::press::reaction(w, r)))
        .collect();
    let mine: Vec<(ClubId, &pw_world::media::FanStanding)> = w.media.fans.iter().filter(|((_, p), _)| *p == me).map(|((c, _), f)| (*c, f)).collect();
    for (c, f) in mine {
        let why: Vec<&str> = f.reasons.iter().map(|r| r.0.label()).collect();
        v.push(format!("{} fans see you as: {} ({})", club(w, c), f.label(), why.join(", ")));
    }
    if let Some(&img) = w.media.image.get(&me) {
        v.push(format!("Public image: {}", if img > 100 { "positive" } else if img < -100 { "negative" } else { "neutral" }));
    }
    if v.is_empty() {
        v.push("Nobody is talking about you. Yet.".into());
    }
    v
}

/// A person's social feed: posts from the persistent population, rendered
/// in their authors' voices, with the ids to reply to or quote.
pub fn feed(w: &World, me: PersonId, n: usize) -> Vec<String> {
    let ids = pw_sim::socialnet::feed(w, me, n);
    if ids.is_empty() {
        return vec!["Your feed is quiet.".into()];
    }
    ids.into_iter()
        .filter_map(|id| w.net.post(id))
        .map(|p| {
            let a = &w.net.accounts[p.author as usize];
            let lead = if p.reply_to != pw_world::socialnet::NO_POST { format!("↳ #{} ", p.reply_to) } else { String::new() };
            format!("#{} {} @{} ({}): {lead}{}  [{} likes, {} reposts]", p.id, p.date, a.handle, a.display, pw_narrate::social::post(w, p), p.likes, p.reposts)
        })
        .collect()
}

/// People a human could step into: players matching a name fragment.
pub fn find(w: &World, text: &str, limit: usize) -> Vec<(PersonId, String)> {
    let q = text.to_lowercase();
    let mut v = Vec::new();
    for (id, pe) in w.people.iter_enumerated() {
        if pe.player.is_none() {
            continue;
        }
        let name = pe.display_name(&w.names);
        if !q.is_empty() && !name.to_lowercase().contains(&q) {
            continue;
        }
        let p: PlayerId = pe.player;
        let h = &w.players.hot[p];
        let where_ = match h.status {
            PlayerStatus::Active => pw_narrate::fmt::club_short(w, h.club),
            PlayerStatus::FreeAgent => "free agent".into(),
            PlayerStatus::Retired => "retired".into(),
            PlayerStatus::Amateur => w.youth.member_of.get(&p).map_or("amateur".to_string(), |&l| w.youth.local[l].name.clone()),
        };
        v.push((id, format!("{:<26} {:>2}  {:<4} {}", name, pe.age(w.date), w.players.cold[p].best_pos.code(), where_)));
        if v.len() >= limit {
            break;
        }
    }
    v
}

pub fn personality_hint(w: &World, me: PersonId) -> String {
    let h = &w.people[me].hidden;
    let adapt = if h.get(Hidden::Adaptability) >= 14 { "settle quickly in new places" } else if h.get(Hidden::Adaptability) <= 7 { "find it hard to settle away from home" } else { "adapt reasonably well" };
    format!("People say you {adapt}.")
}
