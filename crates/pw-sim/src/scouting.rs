//! The scouting network at work (03 §11, 07 §5, 08 §1).
//!
//! Each month a club's recruitment leadership (director of football, else
//! the manager) gives its scouts briefs driven by squad needs and budget: a
//! country, a competition, youth football, a named target. Each week a scout
//! can attend only so many matches, loses time travelling to a new country,
//! and files reports coloured by their judgement, their familiarity with the
//! football they're watching, and their biases. Clubs decide from those
//! reports, weighted by how recent and how well-founded they are.
//!
//! Other channels feed the same knowledge: analysts reading numbers from
//! leagues they track, and staff recommending former teammates they rated.

use pw_core::rng::{Rng, stream};
use pw_core::{Attr, ClubId, CompId, Date, Hidden, NationId, PersonId, PlayerId, StaffAttr, StaffId, TeamId};
use pw_world::knowledge::{Observer, field, perceive};
use pw_world::scouting::{Assignment, Brief, Note, Report, ScoutBias, ScoutProfile, Verdict};
use pw_world::{PlayerStatus, StaffRole, TeamKind, World};
use smallvec::SmallVec;

use crate::consider;

pub fn ensure(w: &mut World) {
    let ids: Vec<StaffId> =
        w.staff.ids().filter(|&s| matches!(w.staff[s].role, StaffRole::Scout | StaffRole::HeadOfYouth | StaffRole::DirectorOfFootball) && !w.scouting.profiles.contains_key(&s)).collect();
    for s in ids {
        let st = &w.staff[s];
        let person = &w.people[st.person];
        let mut rng = Rng::keyed(&[w.seed, stream::PERCEPTION, u64::from(s.0), 0x5c]);
        let home = person.nation;
        let club_nation = if st.club.is_some() { w.clubs[st.club].nation } else { home };
        let mut knows: SmallVec<[(NationId, u8); 4]> = SmallVec::new();
        if home.is_some() {
            knows.push((home, 90));
        }
        if club_nation.is_some() && club_nation != home {
            knows.push((club_nation, 60));
        }
        let b = |rng: &mut Rng| rng.normal_ms(0.0, 3.5).round().clamp(-10.0, 10.0) as i8;
        let bias = ScoutBias {
            physical: b(&mut rng),
            flair: b(&mut rng) + ((person.hidden.f(Hidden::Controversy) - 10.0) / 3.0) as i8,
            youth: b(&mut rng),
            home: b(&mut rng).abs(),
            context_blind: ((20.0 - st.attrs.f(StaffAttr::JudgingAbility)) / 2.0) as i8,
        };
        let capacity = if st.role == StaffRole::Scout { 3 } else { 1 };
        w.scouting.profiles.insert(s, ScoutProfile { staff: s, based: club_nation, knows, bias, capacity });
    }
}

fn recruiter(w: &World, club: ClubId) -> Option<PersonId> {
    let c = &w.clubs[club];
    let s = c.staff.iter().copied().find(|&s| w.staff[s].role == StaffRole::DirectorOfFootball).or(c.manager.get())?;
    Some(w.staff[s].person)
}

/// A recruitment department's working report book. Keep explicit targets and its own players regardless of the general limit;
/// older broad coverage falls back to the club's compact exposure record rather than retaining a full report on every opponent.
pub const GENERAL_REPORT_LIMIT: usize = 1000;

pub fn compact_reports(w: &mut World) {
    let mut pinned = pw_world::FxHashSet::default();
    for (&(club, _), list) in &w.deals.shortlists {
        pinned.extend(list.targets.iter().map(|&(player, _)| (club, player)));
    }
    pinned.extend(w.deals.deals.iter().filter(|d| d.is_open()).map(|d| (d.buyer, d.player)));
    pinned.extend(w.deals.pre_contracts.iter().map(|p| (p.club, p.player)));
    pinned.extend(w.deals.trials.iter().filter(|t| t.until > w.date).map(|t| (t.club, t.player)));
    pinned.extend(w.youth.trials.iter().filter(|t| t.until > w.date).map(|t| (t.club, t.player)));
    for assignment in &w.scouting.assignments {
        if assignment.until > w.date && let Brief::Player(player) = assignment.brief {
            pinned.insert((assignment.club, player));
        }
    }
    let mut books: pw_world::FxHashMap<ClubId, Vec<(Date, PlayerId)>> = pw_world::FxHashMap::default();
    for (&(club, player), reports) in &w.scouting.reports {
        let h = &w.players.hot[player];
        if h.status == PlayerStatus::Retired {
            continue;
        }
        if h.club == club || w.players.cold[player].contract.club == club || w.people[w.players.cold[player].person].mind == pw_world::MindKind::External {
            pinned.insert((club, player));
        } else if let Some(date) = reports.iter().map(|r| r.date).max() {
            books.entry(club).or_default().push((date, player));
        }
    }
    for (club, book) in books.iter_mut() {
        book.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        pinned.extend(book.iter().filter(|(_, p)| !pinned.contains(&(*club, *p))).take(GENERAL_REPORT_LIMIT).map(|&(_, p)| (*club, p)).collect::<Vec<_>>());
    }
    w.scouting.reports.retain(|key, _| pinned.contains(key) && w.players.hot[key.1].status != PlayerStatus::Retired);
    w.scouting.reports.shrink_to_fit();
}

/// Monthly briefs from each club's recruitment leadership.
pub fn assign(w: &mut World) {
    let today = w.date;
    let month = (today.year() as u64) * 12 + u64::from(today.month());
    w.scouting.assignments.retain(|a| a.until > today);
    let clubs: Vec<ClubId> = w.clubs.ids().collect();
    // Nations a club could realistically buy from, weighted by league strength.
    let nations: Vec<(NationId, f32)> = w.nations.iter_enumerated().filter(|(_, n)| !n.leagues.is_empty()).map(|(id, n)| (id, (f32::from(n.reputation) / 1000.0).powi(2) + 0.2)).collect();
    for club in clubs {
        let scouts: Vec<StaffId> = w.clubs[club].staff.iter().copied().filter(|&s| w.scouting.profiles.contains_key(&s)).collect();
        if scouts.is_empty() {
            continue;
        }
        if recruiter(w, club).is_none() {
            continue;
        }
        let home = w.clubs[club].nation;
        let rep = f32::from(w.clubs[club].reputation) / 10_000.0;
        let needs = w.clubs[club].market.needs.clone();
        let youth_focus = w.governance.get(&club).map_or(30, |g| g.policy.youth_investment);
        let mut rng = Rng::keyed(&[w.seed, stream::PERCEPTION, u64::from(club.0), month]);
        for (i, s) in scouts.into_iter().enumerate() {
            if w.scouting.assignments.iter().any(|a| a.scout == s) {
                continue;
            }
            let role = w.staff[s].role;
            let brief = if role == StaffRole::HeadOfYouth || (i == 1 && rng.chance(f32::from(youth_focus) / 100.0)) {
                Brief::Youth(home)
            } else if let Some(n) = needs.get(i % needs.len().max(1)).filter(|_| !needs.is_empty() && rng.chance(0.4)) {
                Brief::Need(n.group)
            } else if rng.chance(0.55 - rep * 0.4) || nations.len() <= 1 {
                // Smaller clubs mostly look at home.
                Brief::Nation(home)
            } else {
                let weights: Vec<f32> = nations.iter().map(|(n, x)| if *n == home { 0.0 } else { *x * (0.3 + rep) }).collect();
                Brief::Nation(nations[rng.weighted(&weights)].0)
            };
            let until = today.add_days(rng.range_i32(28, 84));
            w.scouting.assignments.push(Assignment { scout: s, club, brief, since: today, until });
        }
    }
}

/// Fixtures played in the last week that a brief covers.
fn covered(w: &World, brief: &Brief, club: ClubId) -> Vec<(TeamId, Date, CompId)> {
    let from = w.date.add_days(-7);
    let mut v = Vec::new();
    for f in w.fixtures.between(from, w.date) {
        let fx = w.fixtures.get(f);
        if fx.score.is_none() {
            continue;
        }
        let comp = &w.comps[fx.comp];
        let nation_of = |t: TeamId| w.clubs[w.teams[t].club].nation;
        let ok = |t: TeamId| match *brief {
            Brief::Nation(n) => nation_of(t) == n && w.teams[t].kind == TeamKind::First && w.teams[t].club != club,
            Brief::Competition(c) => fx.comp == c && w.teams[t].club != club,
            Brief::Youth(n) => nation_of(t) == n && w.teams[t].kind.is_youth() && w.teams[t].club != club,
            Brief::Player(p) => w.players.hot[p].team == t,
            Brief::Need(_) => nation_of(t) == w.clubs[club].nation && w.teams[t].club != club,
        };
        let _ = comp;
        for t in [fx.home, fx.away] {
            if ok(t) {
                v.push((t, fx.date, fx.comp));
            }
        }
    }
    v
}

/// A scout's estimate of one player after watching them, with their biases.
pub(crate) fn judge(w: &World, s: StaffId, club: ClubId, p: PlayerId, context: u16) -> Report {
    let st = &w.staff[s];
    let prof = &w.scouting.profiles[&s];
    let c = &w.players.cold[p];
    let person = &w.people[c.person];
    let prior: u16 = w.scouting.of(club, p).iter().filter(|r| r.scout == s).map(|r| r.minutes).max().unwrap_or(0);
    let minutes = prior.saturating_add(90);
    let nation = if w.players.hot[p].club.is_some() { w.clubs[w.players.hot[p].club].nation } else { person.nation };
    let fam = f32::from(prof.familiarity(nation)) / 100.0;
    let judging = st.attrs.f(StaffAttr::JudgingAbility);
    let judging_pa = st.attrs.f(StaffAttr::JudgingPotential);
    // Uncertainty shrinks with minutes watched, judgement and familiarity.
    let sigma = (4.0 * (1.0 - (f32::from(minutes) / 1500.0).min(0.85)) * (1.3 - 0.03 * judging) * (1.4 - 0.4 * fam)).max(0.4);
    let a = |x: Attr| c.attrs.get(x);
    let b = prof.bias;
    let physical = (a(Attr::Pace) + a(Attr::Strength) + a(Attr::Stamina) - 30.0) / 10.0 * f32::from(b.physical) * 0.8;
    let flair = (a(Attr::Flair) + a(Attr::Dribbling) - 20.0) / 10.0 * f32::from(b.flair) * 0.8;
    let home = if person.nation == prof.based { f32::from(b.home) * 0.6 } else { 0.0 };
    let weak_league = if context < 3000 { f32::from(b.context_blind) * 0.8 } else { 0.0 };
    // A young player who has already grown looks better than he is; one who has not looks worse. Scouts who lean on athletic
    // readings (the same bias as above) lean on this most, and it fades as the others catch up (by nineteen there is no gap).
    let maturity = {
        let age = person.dob.age_years(w.date);
        let gap = ((19.0 - age) / 4.0).clamp(0.0, 1.0);
        -f32::from(c.bio_offset) * 0.5 * gap * (1.0 + 0.15 * f32::from(b.physical)).max(0.25) // truth-ok: what a scout sees of a boy's build, not his ability
    };
    let skew = physical + flair + home + weak_league + maturity;
    let ca = (perceive(f32::from(c.ca), sigma * 6.0, Observer::Person(st.person.0), p, field::CA) + skew).clamp(1.0, 200.0);
    let age = person.dob.age_years(w.date);
    let youth = if age < 21.0 { f32::from(b.youth) * (21.0 - age) * 0.6 } else { 0.0 };
    let pa_sigma = sigma * 6.0 + (20.0 - judging_pa) * 1.3;
    let pa = (perceive(f32::from(c.pa), pa_sigma, Observer::Person(st.person.0), p, field::PA) + skew + youth).clamp(ca, 200.0);
    // Grade for this club: how the player would fit its level and needs.
    let ideal = crate::market::ideal_ca(w.clubs[club].reputation);
    let need = consider::club_need_for(w, club, p);
    let score = (ca - ideal) / 10.0 + (pa - ca).max(0.0) / 25.0 * if age < 23.0 { 1.0 } else { 0.2 } + need;
    let grade = (3.0 + score).round().clamp(1.0, 5.0) as u8;
    let verdict = if grade >= 4 {
        Verdict::Sign
    } else if grade == 3 {
        Verdict::Monitor
    } else {
        Verdict::Pass
    };
    let mut notes: SmallVec<[Note; 3]> = SmallVec::new();
    let key = pw_core::Role::default_for(c.best_pos).key_attrs();
    if let Some(&(best, _)) = key.iter().max_by(|x, y| a(x.0).total_cmp(&a(y.0))) {
        notes.push(Note::Strength(best));
    }
    if let Some(&(worst, _)) = key.iter().min_by(|x, y| a(x.0).total_cmp(&a(y.0))) {
        notes.push(Note::Weakness(worst));
    }
    // What a scout can see of attitude depends on how long they've watched.
    let prof_h = person.hidden.f(Hidden::Professionalism);
    if minutes >= 270 {
        if prof_h >= 15.0 {
            notes.push(Note::GoodAttitude);
        } else if prof_h <= 7.0 {
            notes.push(Note::PoorAttitude);
        }
    }
    if c.injuries_career >= 6 && notes.len() < 3 {
        notes.push(Note::InjuryConcern);
    }
    if age < 19.0 && notes.len() < 3 {
        if c.bio_offset >= 8 {
            notes.push(Note::LateDeveloper);
        } else if c.bio_offset <= -8 {
            notes.push(Note::PhysicallyAhead);
        }
    }
    Report { scout: s, date: w.date, minutes, ca: ca as u8, pa: pa as u8, band: (sigma * 6.0) as u8, grade, verdict, notes, context }
}

/// Weekly: scouts attend matches within their briefs and file reports.
pub fn weekly(w: &mut World) {
    let today = w.date;
    let week = (today.0 / 7) as u64;
    let assignments = w.scouting.assignments.clone();
    for a in assignments {
        let Some(prof) = w.scouting.profiles.get(&a.scout).cloned() else { continue };
        if !w.staff[a.scout].employed() || w.staff[a.scout].club != a.club {
            continue;
        }
        let mut rng = Rng::keyed(&[w.seed, stream::PERCEPTION, u64::from(a.scout.0), week]);
        // Travel: moving to a new country's football costs the first week.
        let target_nation = match a.brief {
            Brief::Nation(n) | Brief::Youth(n) => n,
            Brief::Competition(c) => w.comps[c].nation,
            Brief::Player(p) => w.clubs.get(w.players.hot[p].club).map_or(NationId::NONE, |c| c.nation),
            Brief::Need(_) => w.clubs[a.club].nation,
        };
        let mut capacity = usize::from(prof.capacity);
        if target_nation.is_some() && prof.based != target_nation {
            capacity = capacity.saturating_sub(2);
            if let Some(pp) = w.scouting.profiles.get_mut(&a.scout) {
                pp.based = target_nation;
            }
        }
        let mut matches = covered(w, &a.brief, a.club);
        rng.shuffle(&mut matches);
        for (team, date, comp) in matches.into_iter().take(capacity) {
            let context = w.comps[comp].reputation;
            let seen: Vec<PlayerId> = w.teams[team].squad.iter().copied().filter(|&p| w.players.hot[p].last_match == date).collect();
            for p in seen {
                // A scout looks harder at players who might fit the brief.
                let interesting = match a.brief {
                    Brief::Need(g) => w.players.cold[p].best_pos.group() == g,
                    Brief::Player(x) => x == p,
                    Brief::Youth(_) => true,
                    _ => rng.chance(0.4),
                };
                w.knowledge.observe(a.club, p, if interesting { 90 } else { 45 }, today);
                if interesting {
                    let r = judge(w, a.scout, a.club, p, context);
                    w.scouting.file(a.club, p, r);
                }
            }
        }
        // Familiarity with the country grows with time spent there.
        if target_nation.is_some()
            && let Some(pp) = w.scouting.profiles.get_mut(&a.scout)
        {
            match pp.knows.iter_mut().find(|(n, _)| *n == target_nation) {
                Some((_, f)) => *f = (*f + 2).min(100),
                None => pp.knows.push((target_nation, 10)),
            }
        }
    }
    analysts(w);
    if today.day() <= 7 {
        recommendations(w);
    }
    w.scouting.forget(today.add_days(-730));
}

/// Analysts read the numbers from leagues the club follows: broad, shallow evidence.
fn analysts(w: &mut World) {
    let today = w.date;
    let clubs: Vec<(ClubId, usize)> = w.clubs.iter_enumerated().map(|(id, c)| (id, c.staff.iter().filter(|&&s| w.staff[s].role == StaffRole::Analyst).count())).filter(|&(_, n)| n > 0).collect();
    for (club, n) in clubs {
        let home = w.clubs[club].nation;
        let rep = w.clubs[club].reputation;
        // Leagues followed: home top flights plus the strongest leagues abroad for big clubs.
        let comps: Vec<CompId> = w
            .comps
            .iter_enumerated()
            .filter(|(_, c)| c.is_league() && c.team_kind == TeamKind::First && (c.nation == home || (rep >= 6000 && c.tier == 1 && c.reputation >= 6000)))
            .map(|(id, _)| id)
            .collect();
        let mut found: Vec<(PlayerId, f32)> = Vec::new();
        for comp in comps {
            for l in w.stats.for_comp(comp) {
                if l.apps >= 5 && l.club != club {
                    found.push((l.player, l.avg_rating() + l.goals as f32 * 0.05 + l.key_passes as f32 * 0.02));
                }
            }
        }
        found.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        for (p, _) in found.into_iter().take(10 * n) {
            w.knowledge.observe(club, p, 30, today);
        }
    }
}

/// Staff who played the game recommend former teammates they rated — the
/// people they remember sharing a pitch with, being mentored by, or backing.
fn recommendations(w: &mut World) {
    use pw_world::MemoryKind as M;
    let today = w.date;
    let staff: Vec<StaffId> = w.staff.ids().filter(|&s| w.staff[s].employed() && w.people[w.staff[s].person].player.is_some()).collect();
    for s in staff {
        let club = w.staff[s].club;
        let me = w.staff[s].person;
        let mut former: Vec<(PersonId, f32)> = w
            .social
            .memories_of(me)
            .filter(|m| matches!(m.kind, M::SharedPitch | M::Mentored | M::Celebrated | M::Backed | M::ExtraWork | M::DefendedMe))
            .map(|m| (m.about, w.social.get(me, m.about).map_or(50.0, |r| f32::from(r.respect))))
            .filter(|&(_, r)| r >= 55.0)
            .collect();
        former.dedup_by_key(|x| x.0);
        former.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        for (other, _) in former.into_iter().take(3) {
            let pl = w.people[other].player;
            if pl.is_none() || w.players.hot[pl].status == PlayerStatus::Retired || w.players.hot[pl].club == club {
                continue;
            }
            w.knowledge.observe(club, pl, 180, today);
        }
    }
}

/// What a club believes about a player: its reports if it has any (recent,
/// well-founded reports weigh more; stale ones fade), else its general view.
pub fn view(w: &World, club: ClubId, p: PlayerId) -> (f32, f32, f32, f32) {
    // The club's people have formed a judgement (section 1.5); that is what the club believes.
    if let Some(r) = crate::dossier::reading(w, club, p) {
        return r;
    }
    let reports = w.scouting.of(club, p);
    if reports.is_empty() {
        return crate::perception::club_view(w, club, p);
    }
    let (mut ca, mut pa, mut band, mut total) = (0.0, 0.0, 0.0, 0.0);
    for r in reports {
        let age_days = r.date.days_until(w.date).max(0) as f32;
        let fresh = pw_core::math::exp(-age_days / 180.0);
        let judge = w.staff.get(r.scout).map_or(10.0, |s| s.attrs.f(StaffAttr::JudgingAbility));
        let weight = fresh * (0.5 + f32::from(r.minutes) / 900.0) * (0.5 + judge / 20.0);
        ca += f32::from(r.ca) * weight;
        pa += f32::from(r.pa) * weight;
        band += f32::from(r.band) * weight;
        total += weight;
    }
    if total <= 1e-3 {
        return crate::perception::club_view(w, club, p);
    }
    // Staleness widens the band.
    let newest = reports.iter().map(|r| r.date).max().unwrap_or(w.date);
    let stale = newest.days_until(w.date).max(0) as f32 / 30.0;
    (ca / total, band / total + stale, pa / total, band / total * 1.5 + stale)
}

/// Do the club's scouts disagree about this player? (spread of CA estimates)
pub fn disagreement(w: &World, club: ClubId, p: PlayerId) -> f32 {
    if let Some(d) = w.dossiers.get(club, p) {
        // Between everyone who has an opinion, not only the scouts.
        return d.opinions.iter().map(|o| o.ca.mid).fold(f32::MIN, f32::max) - d.opinions.iter().map(|o| o.ca.mid).fold(f32::MAX, f32::min);
    }
    let r = w.scouting.of(club, p);
    if r.len() < 2 {
        return 0.0;
    }
    let max = r.iter().map(|x| x.ca).max().unwrap_or(0);
    let min = r.iter().map(|x| x.ca).min().unwrap_or(0);
    f32::from(max - min)
}
