//! Tactical schools, the meta, and rule changes. See
//! `pw_world::evolution`.

use pw_core::rng::stream;
use pw_core::{ClubId, CompId, NationId, PersonId, StaffId};
use pw_world::evolution::{Federation, RuleCause, RuleChange, RuleKey, School};
use pw_world::event::{EventKind, Visibility};
use pw_world::{CompKind, StaffRole, TeamKind, World};
use smallvec::SmallVec;

fn style_distance(a: (u8, u8, u8), b: (u8, u8, u8)) -> u32 {
    u32::from(a.0.abs_diff(b.0)) + u32::from(a.1.abs_diff(b.1)) + u32::from(a.2.abs_diff(b.2))
}

/// Last season's top-flight champions: (nation, club, manager).
fn champions(w: &World, season: i32) -> Vec<(NationId, ClubId, StaffId)> {
    let mut v: Vec<(NationId, ClubId, StaffId)> = w
        .history
        .honours
        .iter()
        .filter(|h| h.season == season)
        .filter(|h| {
            let c = &w.comps[h.comp];
            c.kind == CompKind::League && c.tier == 1 && c.team_kind == TeamKind::First
        })
        .filter_map(|h| w.clubs[h.club].manager.get().map(|m| (w.comps[h.comp].nation, h.club, m)))
        .collect();
    v.sort();
    v
}

/// July: schools are founded and fade, the meta follows the champions,
/// federations consider their rules.
pub fn yearly(w: &mut World) {
    let today = w.date;
    let season = today.year() - 1;
    let champs = champions(w, season);
    for &(nation, club, m) in &champs {
        let phil = w.staff[m].philosophy;
        let style = (phil.press, phil.tempo, phil.directness);
        // Success is imitated: the nation's fashion moves toward the champion.
        if let Some(c) = w.culture.nations.get_mut(&nation) {
            let pull = |t: u8, x: u8| (f32::from(t) * 0.8 + f32::from(x) * 0.2).round() as u8;
            c.trend_press = pull(c.trend_press, style.0);
            c.trend_tempo = pull(c.trend_tempo, style.1);
            c.trend_direct = pull(c.trend_direct, style.2);
        }
        // Schools: a school's manager adds to its record; a distinctive
        // repeat champion founds one.
        if let Some(&s) = w.evolution.school_of.get(&m) {
            let sc = &mut w.evolution.schools[s as usize];
            sc.titles += 1;
            sc.prestige = (sc.prestige + 80).min(1000);
            continue;
        }
        let person = w.staff[m].person;
        let wins = w.history.honours.iter().filter(|h| h.club == club && h.season >= season - 5).count();
        let trend = w.culture.nations.get(&nation).map_or((50, 50, 50), |c| (c.trend_press, c.trend_tempo, c.trend_direct));
        if wins >= 2 && style_distance(style, trend) >= 30 {
            let id = w.evolution.schools.len() as u32;
            w.evolution.schools.push(School {
                id,
                founder: person,
                origin: nation,
                born: today,
                press: phil.press,
                tempo: phil.tempo,
                direct: phil.directness,
                youth: phil.youth_trust,
                adherents: [m].into_iter().collect(),
                prestige: 400,
                titles: wins as u16,
            });
            w.evolution.school_of.insert(m, id);
            w.evolution.formed_by.insert(person, id);
            w.events.push(today, Visibility::Public, EventKind::SchoolFounded { school: id, founder: person });
        }
    }
    // People formed under a school: players and coaches at its managers' clubs.
    let adherents: Vec<(StaffId, u32)> = w.evolution.school_of.iter().map(|(&s, &id)| (s, id)).collect();
    for (m, id) in adherents {
        let club = w.staff[m].club;
        if club.is_none() || w.staff[m].retired {
            continue;
        }
        let first = w.clubs[club].first_team();
        let mut formed: SmallVec<[PersonId; 32]> = w.teams[first].squad.iter().filter(|&&p| w.age(p) >= 24).map(|&p| w.players.cold[p].person).collect();
        formed.extend(w.clubs[club].staff.iter().filter(|&&s| matches!(w.staff[s].role, StaffRole::Assistant | StaffRole::Coach)).map(|&s| w.staff[s].person));
        for p in formed {
            w.evolution.formed_by.entry(p).or_insert(id);
        }
    }
    // Prestige fades without success; adherents who left management drop out.
    for s in w.evolution.schools.iter_mut() {
        s.prestige = s.prestige.saturating_sub(25);
    }
    let retired: Vec<StaffId> = w.evolution.school_of.keys().copied().filter(|&s| w.staff[s].retired).collect();
    for s in retired {
        if let Some(id) = w.evolution.school_of.remove(&s) {
            w.evolution.schools[id as usize].adherents.retain(|x| *x != s);
        }
    }
    rules(w);
}

/// A manager takes a job: one formed in a school carries its principles
/// (with their own drift); others may come up through their nation's most
/// prestigious tradition.
pub fn on_appointed(w: &mut World, m: StaffId) {
    if w.evolution.school_of.contains_key(&m) {
        return;
    }
    let person = w.staff[m].person;
    let school = w.evolution.formed_by.get(&person).copied().or_else(|| {
        let nation = w.people[person].nation;
        let best = w.evolution.schools.iter().filter(|s| s.origin == nation && s.prestige >= 300).max_by_key(|s| (s.prestige, std::cmp::Reverse(s.id)))?;
        let roll = w.roll(stream::CULTURE, &[u64::from(m.0), u64::from(best.id), 0x5c1]);
        (roll < f32::from(best.prestige) / 2000.0).then_some(best.id)
    });
    let Some(id) = school else { return };
    let s = w.evolution.schools[id as usize].clone();
    let p = &mut w.staff[m].philosophy;
    let blend = |own: u8, school: u8| (f32::from(own) * 0.4 + f32::from(school) * 0.6).round() as u8;
    p.press = blend(p.press, s.press);
    p.tempo = blend(p.tempo, s.tempo);
    p.directness = blend(p.directness, s.direct);
    p.youth_trust = blend(p.youth_trust, s.youth);
    w.evolution.school_of.insert(m, id);
    let sc = &mut w.evolution.schools[id as usize];
    if !sc.adherents.contains(&m) {
        sc.adherents.push(m);
    }
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

fn federation(w: &mut World, n: NationId) -> Federation {
    let seed = w.seed;
    *w.evolution.federations.entry(n).or_insert_with(|| {
        let k = pw_core::rng::hash_key(&[seed, stream::CULTURE, u64::from(n.0), 0xfed]);
        Federation { conservatism: (40 + k % 50) as u8, last_change: pw_core::Date(0) }
    })
}

fn nation_comps(w: &World, n: NationId) -> Vec<CompId> {
    w.comps.iter_enumerated().filter(|(_, c)| c.nation == n && c.team_kind == TeamKind::First).map(|(id, _)| id).collect()
}

fn rules(w: &mut World) {
    let today = w.date;
    let nations: Vec<NationId> = w.nations.ids().filter(|&n| !w.nations[n].leagues.is_empty()).collect();
    let since = today.add_days(-365);
    // Last season's injuries and reds per nation.
    let mut injuries: pw_world::FxHashMap<NationId, u32> = Default::default();
    let mut reds: pw_world::FxHashMap<NationId, u32> = Default::default();
    for e in w.events.since(since) {
        match e.kind {
            EventKind::Injured { player, .. } => {
                let c = w.players.hot[player].club;
                if c.is_some() {
                    *injuries.entry(w.clubs[c].nation).or_default() += 1;
                }
            }
            EventKind::Suspended { player, .. } => {
                let c = w.players.hot[player].club;
                if c.is_some() {
                    *reds.entry(w.clubs[c].nation).or_default() += 1;
                }
            }
            _ => {}
        }
    }
    for n in nations {
        let fed = federation(w, n);
        if fed.last_change.0 != 0 && fed.last_change.days_until(today) < 6 * 365 {
            continue;
        }
        let comps = nation_comps(w, n);
        let Some(&top) = w.nations[n].leagues.first() else { continue };
        let clubs = w.comps[top].state.table.len().max(1) as f32 * w.nations[n].leagues.len() as f32;
        let prof = pw_world::rules::profile(w, n);
        let season = w.comps[top].state.season + 1;
        // Candidate changes with their pressure (≥ 1 = over the threshold).
        let mut options: Vec<(f32, RuleKey, i32, i32, RuleCause)> = Vec::new();
        let subs = i32::from(w.comps[top].rules.subs);
        let inj = injuries.get(&n).copied().unwrap_or(0) as f32 / clubs;
        if subs < 5 {
            options.push((inj / 30.0, RuleKey::Subs, subs, (subs + 2).min(5), RuleCause::InjuryCrisis { per_club: inj }));
        }
        let rd = reds.get(&n).copied().unwrap_or(0) as f32 / clubs;
        if prof.red_ban_straight < 5 {
            options.push((rd / 5.0, RuleKey::RedBan, i32::from(prof.red_ban_straight), i32::from(prof.red_ban_straight) + 1, RuleCause::CardEpidemic { per_club: rd }));
        }
        if let Some(side) = w.intl.sides.get(&(n, pw_world::intl::Level::Senior)) {
            let (wn, d, l) = side.record;
            let games = (wn + d + l).max(1) as f32;
            let win_rate = f32::from(wn) / games;
            let squad: Vec<pw_core::PlayerId> = w.comps[top].state.table.iter().flat_map(|r| w.teams[r.team].squad.iter().copied()).collect();
            let home = squad.iter().filter(|&&p| w.people[w.players.cold[p].person].nation == n).count() as f32 / squad.len().max(1) as f32;
            if games >= 8.0 && prof.homegrown_min < 8 {
                let pressure = (0.4 - win_rate).max(0.0) * 3.0 + (0.45 - home).max(0.0) * 3.0;
                options.push((pressure, RuleKey::HomegrownMin, i32::from(prof.homegrown_min), i32::from(prof.homegrown_min) + 2, RuleCause::NationalDecline { win_rate, homegrown: home }));
            }
        }
        let away_ties: u16 = comps
            .iter()
            .filter(|&&c| w.comps[c].rules.away_goals)
            .flat_map(|&c| w.comps[c].state.ties.iter())
            .filter(|t| t.legs >= 2 && t.goals_a == t.goals_b && t.away_a != t.away_b && t.winner.is_some())
            .count() as u16;
        if comps.iter().any(|&c| w.comps[c].rules.away_goals) {
            options.push((f32::from(away_ties) / 3.0, RuleKey::AwayGoals, 1, 0, RuleCause::AwayGoalsDebate { ties: away_ties }));
        }
        options.retain(|o| o.0 >= 1.0);
        options.sort_by(|a, b| b.0.total_cmp(&a.0));
        let Some(&(pressure, key, old, new, cause)) = options.first() else { continue };
        let chance = 0.3 * (1.0 - f32::from(fed.conservatism) / 100.0) * pressure.min(2.0);
        if w.roll(stream::CULTURE, &[u64::from(n.0), today.year() as u64, 0x401e]) >= chance {
            continue;
        }
        // Enact: competition rules change for the coming season; profile
        // rules are read through the change log (`rules::profile`).
        match key {
            RuleKey::Subs => {
                for &c in &comps {
                    w.comps[c].rules.subs = new as u8;
                }
            }
            RuleKey::AwayGoals => {
                for &c in &comps {
                    w.comps[c].rules.away_goals = false;
                }
            }
            RuleKey::RedBan | RuleKey::HomegrownMin => {}
        }
        let id = w.evolution.changes.len() as u32;
        w.evolution.changes.push(RuleChange { id, nation: n, key, old, new, from_season: season, date: today, cause });
        if let Some(f) = w.evolution.federations.get_mut(&n) {
            f.last_change = today;
        }
        w.events.push(today, Visibility::Public, EventKind::RuleChanged { change: id });
    }
}
