//! World building from the resolved import set. Every value the source lacked is filled by a stated rule and labelled
//! in `World::origins`; people, clubs, competitions and nations keep their source references. Nothing here reads a file.

use pw_core::rng::{Rng, stream};
use pw_core::{Attrs, ClubId, CompId, Date, Foot, N_ATTR, N_HIDDEN, NationId, Pos, PosGroup, StaffAttr, TeamId};
use pw_data::DataPack;
use pw_sim::generate::{attrs_for, ca_share_at, height_for, hidden_random, resolve_pa};
use pw_world::backfill::{PastFigure, PastSeason, Provenance};
use pw_world::contract::{ContractKind, Loan};
use pw_world::history::Spell;
use pw_world::nation::Confed;
use pw_world::origin::{Facet, Origin, PersonOrigin, Source, SourceRef, Unresolved, gap};
use pw_world::player::Reputation;
use pw_world::{CompKind, Contract, Format, Philosophy, PlayerCold, PlayerHot, PlayerStatus, SquadStatus, Team, TeamKind, World};
use rustc_hash::FxHashMap;

use crate::builder::{self, ClubSpec};
use crate::infer;
use crate::model::*;
use crate::{ImportReport, hash64};

fn parse_color(c: Option<u32>) -> u32 {
    c.unwrap_or(0x444444)
}

/// The weakest of two origins, in the order Imported < Inferred < Generated < Unknown.
fn weakest(a: Origin, b: Origin) -> Origin {
    let rank = |o: Origin| match o {
        Origin::Imported => 0,
        Origin::Inferred => 1,
        Origin::Generated => 2,
        Origin::Unknown => 3,
    };
    if rank(b) > rank(a) { b } else { a }
}

fn club_team(w: &mut World, club: ClubId, kind: TeamKind) -> TeamId {
    if let Some(t) = w.club_team(club, kind) {
        return t;
    }
    let t = w.teams.push(Team { club, kind, squad: Vec::new(), tactics: Default::default(), captain: Default::default(), familiarity_weeks: 0 });
    w.clubs[club].teams.push(t);
    t
}

fn default_position(group: PosGroup) -> Pos {
    match group {
        PosGroup::Gk => Pos::GK,
        PosGroup::Def => Pos::DC,
        PosGroup::Mid => Pos::MC,
        PosGroup::Att => Pos::ST,
    }
}

pub fn assemble(set: &ImportSet, pack: DataPack, seed: Option<u64>) -> (World, ImportReport) {
    let start = set.start.unwrap_or_else(crate::csv_pack::default_start);
    let seed = seed.or(set.seed).unwrap_or_else(pw_core::rng::fresh_seed);
    let mut w = World::new(pack, seed, start);
    let mut rep = ImportReport::default();
    w.origins.sources = set.sources.iter().map(|s| Source { name: s.name.clone(), snapshot: s.snapshot, note: s.note.clone() }).collect();

    // ---- nations ----------------------------------------------------------------------
    let mut nations: FxHashMap<&str, NationId> = FxHashMap::default();
    for n in &set.nations {
        let confed = n.confed.unwrap_or(Confed::Uefa);
        let id = builder::add_nation(
            &mut w,
            &n.code,
            &n.name,
            confed,
            n.reputation.unwrap_or(if n.minor { 1000 } else { 3000 }),
            n.calendar.as_deref().unwrap_or("autumn_spring"),
            n.economy.unwrap_or(if n.minor { 0.3 } else { 0.5 }),
            n.youth_rating.unwrap_or(if n.minor { 8 } else { 10 }),
        );
        w.origins.nations.insert(id, SourceRef { source: n.source, id: format!("nation:{}", n.key) });
        nations.insert(&n.key, id);
    }
    rep.nations = nations.len();

    // ---- competitions ------------------------------------------------------------------------
    let mut comps: FxHashMap<&str, CompId> = FxHashMap::default();
    for c in &set.comps {
        let nation = c.nation.as_deref().and_then(|n| nations.get(n)).copied().unwrap_or(NationId::NONE);
        let legs = c.legs.unwrap_or(if c.kind == CompKind::League { 2 } else { 1 });
        let format = match (c.format.as_deref().unwrap_or("").to_ascii_lowercase().as_str(), c.kind) {
            ("groups", _) | ("", CompKind::Continental) => Format::Groups { groups: c.groups.unwrap_or(8), size: c.group_size.unwrap_or(4), advance: c.advance.unwrap_or(2), legs: 2, ko_legs: c.legs.unwrap_or(2), final_legs: 1 },
            ("knockout", _) | ("", CompKind::Cup) | ("", CompKind::SuperCup) => Format::Knockout { legs, final_legs: 1 },
            _ => Format::League { rounds: legs },
        };
        let tier = c.tier.unwrap_or(if c.kind == CompKind::Cup { 4 } else { 1 });
        let size = c.size.unwrap_or(if c.kind == CompKind::Continental { 32 } else { 0 });
        let reputation = c.reputation.unwrap_or(if nation.is_some() { w.nations[nation].reputation / u16::from(tier).max(1) } else { 6000 });
        let confed = c.confed.or_else(|| nation.get().map(|n| w.nations[n].confed));
        let id = builder::add_comp(
            &mut w,
            &c.name,
            &c.short,
            nation,
            confed,
            c.kind,
            tier,
            c.team_kind,
            size,
            c.promote.unwrap_or(0),
            c.relegate.unwrap_or(0),
            reputation,
            format,
            c.prize_pool.unwrap_or(i64::from(reputation) * 4_000),
        );
        w.origins.comps.insert(id, SourceRef { source: c.source, id: format!("competition:{}", c.key) });
        comps.insert(&c.key, id);
    }
    rep.competitions = comps.len();

    // ---- clubs ----------------------------------------------------------------------------------
    let mut clubs: FxHashMap<&str, ClubId> = FxHashMap::default();
    for c in &set.clubs {
        let nation = c.nation.as_deref().and_then(|n| nations.get(n)).copied().unwrap_or(NationId::NONE);
        let league = c.league.as_deref().and_then(|l| comps.get(l)).copied().unwrap_or(CompId::NONE);
        let mut gaps = 0u16;
        let league_rep = if league.is_some() { w.comps[league].reputation } else { 1000 };
        let reputation = c.reputation.unwrap_or_else(|| {
            gaps |= gap::REPUTATION;
            league_rep
        });
        let mut fac = builder::default_facilities(reputation);
        if let Some(v) = c.training {
            fac.training = v;
            fac.medical = v;
        } else {
            gaps |= gap::FACILITIES;
        }
        if let Some(v) = c.youth {
            fac.youth = v;
        }
        if let Some(v) = c.academy {
            fac.academy = v;
        }
        let mut extra = c.extra_teams.clone();
        if extra.is_empty() {
            gaps |= gap::YOUTH_SIDES;
        }
        extra.retain(|k| *k != TeamKind::First);
        if c.balance.is_none() && c.transfer_budget.is_none() {
            gaps |= gap::FINANCES;
        }
        if c.city.is_empty() {
            gaps |= gap::CITY;
        }
        if c.colours.iter().any(Option::is_none) {
            gaps |= gap::COLOURS;
        }
        if c.founded.is_none() {
            gaps |= gap::FOUNDED;
        }
        if c.stadium.is_empty() {
            gaps |= gap::STADIUM;
        }
        if c.capacity.is_none() {
            gaps |= gap::CAPACITY;
        }
        let id = builder::add_club(
            &mut w,
            ClubSpec {
                name: &c.name,
                short: &c.short,
                nation,
                city: &c.city,
                league,
                reputation,
                balance: c.balance.unwrap_or(i64::from(reputation) * 2_000),
                stadium: &c.stadium,
                capacity: c.capacity.unwrap_or(u32::from(reputation) * 5 + 3_000),
                facilities: fac,
                colors: [parse_color(c.colours[0]), parse_color(c.colours[1])],
                founded: c.founded.unwrap_or(1900),
                extra_teams: &extra,
            },
        );
        if let Some(b) = c.transfer_budget {
            w.clubs[id].finance.transfer_budget = b;
        }
        if let Some(b) = c.wage_budget {
            w.clubs[id].finance.wage_budget = b;
        }
        w.origins.clubs.insert(id, SourceRef { source: c.source, id: format!("club:{}", c.key) });
        w.origins.club_gaps.insert(id, gaps);
        clubs.insert(&c.key, id);
    }
    rep.clubs = clubs.len();

    for (comp, club) in &set.entrants {
        if let (Some(&c), Some(&club)) = (comps.get(comp.as_str()), clubs.get(club.as_str())) {
            let kind = w.comps[c].team_kind;
            let team = club_team(&mut w, club, kind);
            if !w.comps[c].state.entrants.contains(&team) {
                w.comps[c].state.entrants.push(team);
            }
        }
    }

    // ---- staff --------------------------------------------------------------------------------------
    for s in &set.staff {
        let Some(role) = s.role else { continue };
        let club = s.club.as_deref().and_then(|c| clubs.get(c)).copied().unwrap_or(ClubId::NONE);
        let mut rng = Rng::keyed(&[seed, stream::STAFF, 0x51, hash64(&s.key)]);
        let mut mark = PersonOrigin::new(SourceRef { source: s.source, id: format!("staff:{}", s.key) });
        let nation_known = s.nationality.as_deref().and_then(|n| nations.get(n)).copied();
        let nation = nation_known.or_else(|| club.get().map(|_| w.clubs[club].nation)).unwrap_or(NationId::NONE);
        if nation_known.is_none() {
            mark.mark(Facet::Identity, Origin::Inferred);
        }
        let gen_nation = if nation.is_some() { nation } else { w.nations.ids().next().unwrap_or(NationId::NONE) };
        if gen_nation.is_none() {
            continue;
        }
        let rep_base = if club.is_some() { f32::from(w.clubs[club].reputation) } else { 3000.0 };
        let level = 5.0 + rep_base / 800.0;
        let id = builder::new_staff_at(&mut w, club, gen_nation, rep_base, role, level, &mut rng);
        let person = w.staff[id].person;
        w.staff[id].joined = start;
        let (first, last) = (w.names.intern(&s.first), w.names.intern(&s.last));
        let nation2 = s.nationality2.as_deref().and_then(|n| nations.get(n)).copied().unwrap_or(NationId::NONE);
        let dob = match (s.dob, s.age) {
            (Some(d), _) => d,
            (None, Some(age)) => {
                mark.mark(Facet::Identity, Origin::Inferred);
                start.add_days(-(365 * i32::from(age) + rng.range_i32(0, 364)))
            }
            (None, None) => {
                mark.mark(Facet::Identity, Origin::Generated);
                w.people[person].dob
            }
        };
        {
            let p = &mut w.people[person];
            p.first = first;
            p.last = last;
            p.common = pw_world::NameId::NONE;
            p.dob = dob;
            p.nation = nation;
            p.nation2 = nation2;
        }
        let mut known = 0;
        for (a, v) in &s.attrs {
            w.staff[id].attrs.set(*a, *v);
            known += 1;
        }
        mark.mark(Facet::Attributes, if known == 0 { Origin::Generated } else if known == StaffAttr::ALL.len() { Origin::Imported } else { Origin::Inferred });
        mark.mark(Facet::Hidden, Origin::Generated);
        mark.mark(Facet::Contract, Origin::Generated);
        mark.mark(Facet::Club, if s.club.is_none() { Origin::Unknown } else if s.club_link == Some(Link::Stated) { Origin::Imported } else { Origin::Inferred });
        if let Some(r) = s.reputation {
            w.staff[id].reputation = r;
        }
        if let Some(f) = s.formation.as_deref().and_then(|f| w.data.formation_index(f)) {
            w.staff[id].philosophy = Philosophy { formations: [f, f], ..w.staff[id].philosophy.clone() };
        }
        if let Some(d) = s.contract_end {
            w.staff[id].contract_end = d;
            mark.mark(Facet::Contract, Origin::Imported);
        }
        if club.is_some() {
            w.clubs[club].staff.push(id);
            if role == pw_world::StaffRole::Manager && w.clubs[club].manager.is_none() {
                w.clubs[club].manager = id;
            }
        }
        w.origins.add_person(person, mark);
        rep.staff += 1;
    }

    // ---- players ----------------------------------------------------------------------------------------
    // Typical ability per club from the values its imported players carry, for players whose own value is missing.
    let mut club_values: FxHashMap<&str, Vec<f64>> = FxHashMap::default();
    for p in &set.players {
        if let (Some(c), Some(v)) = (&p.club, p.value) {
            club_values.entry(c).or_default().push(v as f64);
        }
    }
    let club_level: FxHashMap<&str, f32> = club_values.iter_mut().filter_map(|(k, v)| infer::club_level(v).map(|l| (*k, l))).collect();

    let mut spells_by_player: FxHashMap<&str, Vec<&ImpSpell>> = FxHashMap::default();
    for s in &set.spells {
        spells_by_player.entry(&s.player).or_default().push(s);
    }
    for p in &set.players {
        let dob = p.dob.expect("resolved players have a birth date");
        let age = dob.age_years(start);
        let mut rng = Rng::keyed(&[seed, stream::WORLDGEN, hash64(&p.key)]);
        let mut mark = PersonOrigin::new(SourceRef { source: p.source, id: format!("player:{}", p.key) });

        let club = p.club.as_deref().and_then(|c| clubs.get(c)).copied().unwrap_or(ClubId::NONE);
        let loan_from = p.loan_from.as_deref().and_then(|c| clubs.get(c)).copied();
        let registered = loan_from.unwrap_or(club);
        let nation_known = p.nationality.as_deref().and_then(|n| nations.get(n)).copied();
        let nation = nation_known.or_else(|| club.get().map(|_| w.clubs[club].nation)).unwrap_or(NationId::NONE);
        if nation_known.is_none() {
            mark.mark(Facet::Identity, Origin::Inferred);
        }
        let nation2 = p.nationality2.as_deref().and_then(|n| nations.get(n)).copied().unwrap_or(NationId::NONE);

        // Positions.
        let (naturals, pos_origin): (Vec<Pos>, Origin) = if !p.positions.is_empty() {
            (p.positions.clone(), Origin::Imported)
        } else if let Some(g) = p.position_group {
            (vec![default_position(g)], Origin::Inferred)
        } else {
            (vec![Pos::MC], Origin::Generated)
        };
        mark.mark(Facet::Position, pos_origin);
        let natural = naturals[0];

        // Ability: stated, else from market value, else from the club's level.
        let (target, target_origin, value_pa) = if let Some(ca) = p.ca {
            (ca, Origin::Imported, None)
        } else if let Some(v) = p.value {
            let (ca, pa) = infer::ability_from_value(v as f64, age);
            (ca, Origin::Inferred, Some(pa))
        } else {
            let level = p.club.as_deref().and_then(|c| club_level.get(c)).copied().unwrap_or_else(|| {
                let league = if club.is_some() { w.clubs[club].league } else { CompId::NONE };
                infer::league_level(if league.is_some() { w.comps[league].reputation } else { 1000 })
            });
            let share = ca_share_at(age);
            let young = if age < 27.0 { share + (1.0 - share) * 0.35 } else { 1.0 };
            ((level * young + rng.normal() * 7.0).clamp(20.0, 190.0), Origin::Generated, None)
        };

        let known_attrs = p.attrs.len();
        let mut attrs = if known_attrs >= N_ATTR { Attrs::splat(800) } else { attrs_for(&w.data.weights, natural, target, &mut rng) };
        for (a, v) in &p.attrs {
            attrs.set(*a, v.clamp(1.0, 20.0));
        }
        mark.mark(Facet::Attributes, if known_attrs >= N_ATTR { Origin::Imported } else if known_attrs > 0 { Origin::Inferred } else if target_origin == Origin::Generated { Origin::Generated } else { Origin::Inferred });

        let mut hidden = hidden_random(&mut rng);
        for (h, v) in &p.hidden {
            hidden.set(*h, *v);
        }
        mark.mark(Facet::Hidden, if p.hidden.len() >= N_HIDDEN { Origin::Imported } else if p.hidden.is_empty() { Origin::Generated } else { Origin::Inferred });

        let (left, right) = match (p.feet, p.foot) {
            (Some((l, r)), _) => (l, r),
            (None, Some(Foot::Left)) => (20, rng.range_i32(4, 12) as u8),
            (None, Some(Foot::Either)) => (18, 19),
            (None, Some(Foot::Right)) => (rng.range_i32(4, 12) as u8, 20),
            (None, None) => (rng.range_i32(4, 12) as u8, 20),
        };
        let height = p.height.unwrap_or_else(|| height_for(natural, &mut rng));
        let weight = p.weight.unwrap_or((f32::from(height) * 0.42) as u8);
        mark.mark(
            Facet::Physique,
            if p.height.is_none() {
                Origin::Generated
            } else if p.weight.is_some() && (p.feet.is_some() || p.foot.is_some()) {
                Origin::Imported
            } else {
                Origin::Inferred
            },
        );

        let joined = p.joined.unwrap_or(start);
        let person = builder::add_person(&mut w, &p.first, &p.last, &p.common, dob, nation, nation2);
        w.people[person].hidden = hidden;

        let team = if club.is_some() { club_team(&mut w, club, p.team) } else { TeamId::NONE };
        let contract = if registered.is_some() {
            Contract {
                club: registered,
                kind: if dob.age_on(start) < 17 { ContractKind::Youth } else { ContractKind::Professional },
                wage: p.wage.unwrap_or(0),
                start: joined,
                end: p.contract_end.unwrap_or(Date(0)),
                yearly_rise: 3,
                ..Default::default()
            }
        } else {
            Contract::default()
        };
        let loan = loan_from.map(|parent| Loan { parent, club, start, end: p.loan_end.unwrap_or(start.add_months(11)), wage_share: 70, fee: 0, buy_option: 0, recall: true });
        mark.mark(Facet::Contract, if registered.is_none() { Origin::Unknown } else if p.contract_end.is_some() && p.wage.is_some() { Origin::Imported } else if p.contract_end.is_some() { Origin::Inferred } else { Origin::Generated });
        mark.mark(Facet::Value, if p.value.is_some() { Origin::Imported } else { Origin::Inferred });
        let has_rep = p.rep_current.is_some();
        mark.mark(Facet::Reputation, if has_rep { Origin::Imported } else { Origin::Inferred });
        mark.mark(Facet::Career, if p.apps.is_some() || p.caps.is_some() || p.goals.is_some() { Origin::Imported } else { Origin::Unknown });
        mark.mark(Facet::Club, if p.club.is_some() { Origin::Imported } else { Origin::Unknown });

        let mut cold = PlayerCold {
            person,
            attrs,
            pa: 0,
            ca: 0,
            familiarity: builder::familiarity_from(&naturals),
            best_pos: natural,
            left_foot: left,
            right_foot: right,
            height,
            weight,
            traits: Default::default(),
            bio_offset: (rng.normal() * 6.0).round() as i8,
            pa_rerolled: dob.age_on(start) >= 20,
            wear: [0; pw_data::N_BODY_REGIONS],
            contract,
            loan,
            value: p.value.unwrap_or(0),
            rep: Reputation { current: p.rep_current.unwrap_or(0), home: p.rep_home.unwrap_or(0), world: p.rep_world.unwrap_or(0) },
            status: p.status.unwrap_or(SquadStatus::Squad),
            shirt: p.shirt.unwrap_or(0),
            caps: p.caps.unwrap_or(0),
            intl_goals: p.intl_goals.unwrap_or(0),
            joined,
            youth_club: registered,
            injuries_career: 0,
            senior_apps: p.apps.unwrap_or(0).min(u32::from(u16::MAX)) as _,
            senior_goals: p.goals.unwrap_or(0).min(u32::from(u16::MAX)) as _,
            plan: Default::default(),
        };
        if let Some(s) = p.status {
            cold.contract.promised_status = Some(s);
        }
        cold.refresh_ca(&w.data.weights);
        // Potential.
        let ours = f32::from(cold.ca);
        let (pa, pa_origin) = match (p.pa, p.ca, value_pa) {
            (Some(pa), Some(ca), _) if ca > 0.0 => {
                let pa = f32::from(resolve_pa(pa, ca.min(200.0) as u8, &mut rng));
                (ours * pa / ca, Origin::Imported)
            }
            (_, _, Some(vpa)) => (vpa * rng.normal_ms(1.0, 0.03).clamp(0.92, 1.08).max(ours / vpa.max(1.0)), Origin::Inferred),
            _ => (ours / ca_share_at(age) * rng.normal_ms(1.0, 0.1), Origin::Generated),
        };
        cold.pa = pa.round().clamp(ours, 200.0) as u8;
        mark.mark(Facet::Potential, pa_origin);

        let hot = PlayerHot { club: registered, team, status: if registered.is_some() { PlayerStatus::Active } else { PlayerStatus::FreeAgent }, condition: 95, sharpness: 70, fitness: 85, ..PlayerHot::default() };
        let pid = w.players.push(hot, cold);
        w.people[person].player = pid;
        if team.is_some() {
            w.teams[team].squad.push(pid);
        }

        // History: earlier spells, then the current one.
        let mut open_fee = 0;
        if let Some(list) = spells_by_player.get(p.key.as_str()) {
            let mut closed: Vec<Spell> = Vec::new();
            for s in list {
                match s.to {
                    Some(to) => {
                        if let Some(&c) = clubs.get(s.club.as_str()) {
                            closed.push(Spell { club: c, from: s.from, to: Some(to), loan: s.loan, fee: s.fee.unwrap_or(0) });
                        }
                    }
                    None => open_fee = s.fee.unwrap_or(0),
                }
            }
            if !closed.is_empty() {
                w.history.spells.entry(pid).or_default().extend(closed);
            }
        }
        if registered.is_some() {
            w.history.start_spell(pid, registered, joined, loan_from.is_some(), open_fee);
        }
        w.origins.add_person(person, mark);
    }
    rep.players = w.players.len();
    rep.spells = w.history.spells.values().map(Vec::len).sum();

    // ---- past seasons ------------------------------------------------------------------------------------
    let mut figures: FxHashMap<&str, u32> = FxHashMap::default();
    for s in &set.seasons {
        let (Some(&comp), Some(&champion)) = (comps.get(s.comp.as_str()), clubs.get(s.champion.as_str())) else { continue };
        let runner_up = s.runner_up.as_deref().and_then(|c| clubs.get(c)).copied().unwrap_or(ClubId::NONE);
        let top_scorer = match &s.top_scorer {
            Some(name) => *figures.entry(name.as_str()).or_insert_with(|| {
                let id = w.backfill.figures.len() as u32;
                let nation = w.comps[comp].nation;
                // A past figure is a name, not a person in the world; the birth year is a placeholder and never shown as fact.
                w.backfill.figures.push(PastFigure { id, name: name.clone(), nation, club: ClubId::NONE, born: s.season - 27, apps: 0, goals: s.top_goals.unwrap_or(0), provenance: Provenance::Imported });
                id
            }),
            None => u32::MAX,
        };
        w.backfill.seasons.push(PastSeason { comp, season: s.season, champion, runner_up, top_scorer, top_goals: s.top_goals.unwrap_or(0), provenance: Provenance::Imported });
        rep.seasons += 1;
    }

    builder::finalize(&mut w);
    builder::ensure_staff(&mut w);
    fill_contracts(&mut w, start);

    // ---- bookkeeping ---------------------------------------------------------------------------------------------
    for u in &set.unresolved {
        w.origins.add_unresolved(Unresolved { source: u.source, id: u.id.clone(), what: u.what.to_string(), reason: u.reason.clone() });
    }
    w.origins.findings = set.issues.by_count().into_iter().map(|(k, n)| (k.to_string(), n)).collect();
    rep.unresolved = w.origins.unresolved_total as usize;
    rep.findings = w.origins.findings.clone();
    rep.warnings = set.issues.sample.iter().filter(|i| i.severity != Severity::Info).take(200).map(|i| format!("{} {}: {}", i.file, i.key, i.message)).collect();
    rep.possible_duplicates = set.possible_duplicates.len();
    for facet in Facet::ALL {
        let counts = [Origin::Imported, Origin::Inferred, Origin::Generated, Origin::Unknown].map(|o| w.origins.count_facet(facet, o));
        rep.facets.push((facet.label(), counts));
    }
    (w, rep)
}

/// Missing wages, contract ends and reputations get plausible values, and the origin book says so.
fn fill_contracts(w: &mut World, start: Date) {
    for p in w.players.ids() {
        let club = w.players.hot[p].club;
        if club.is_none() {
            continue;
        }
        let person = w.players.cold[p].person;
        let mut rng = Rng::keyed(&[w.seed, stream::CONTRACTS, u64::from(p.0)]);
        if w.players.cold[p].contract.wage == 0 {
            w.players.cold[p].contract.wage = pw_sim::market::wage_demand(w, p, club);
            if let Some(o) = w.origins.people.get_mut(&person) {
                let cur = o.get(Facet::Contract);
                o.mark(Facet::Contract, weakest(cur, Origin::Inferred));
            }
        }
        if w.players.cold[p].contract.end.0 == 0 {
            let years = rng.range_i32(1, 4);
            w.players.cold[p].contract.end = Date::from_ymd(start.year() + years, 6, 30);
            if let Some(o) = w.origins.people.get_mut(&person) {
                o.mark(Facet::Contract, Origin::Generated);
            }
        }
        if w.players.cold[p].rep.current == 0 {
            let c = &w.players.cold[p];
            let league = w.clubs[club].league;
            let stage = if league.is_some() { f32::from(w.comps[league].reputation) / 10_000.0 } else { 0.1 };
            let r = (10_000.0 * (f32::from(c.ca) / 200.0).powf(1.6) * (0.35 + 0.65 * stage)) as u16;
            w.players.cold[p].rep = Reputation { current: r, home: r, world: (f32::from(r) * stage.sqrt()) as u16 };
        }
    }
}
