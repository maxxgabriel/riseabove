//! The pathway and recognition pages: how a young player came to exist, why each step happened, who knows him and on what basis,
//! whether he may play for a state or nation and why, what a region has produced, how the world abroad regards the country's
//! football, and where the scenario's data came from (the India brief, items 11, 13, 17, 21, 22).
//!
//! What the world *is* (the route, the reasons, the rules and their evidence, the scenario's calendar, regard abroad, what a region
//! produced) is answered for anyone it is open to. What organisations *know* (their looks at a player, a coach's recommendation and
//! how far it is trusted) is what those organisations hold, not the player's own knowledge, so it is only in the omniscient view.
//! Nothing here carries a hidden ability, and every value that is an estimate, a seed or derived from an old save says so.

use pw_core::PlayerId;
use pw_sim::eligibility::judge_state;
use pw_world::eligibility::{Body, Judgement};
use pw_world::ecosystem::Tier;
use pw_world::recog::{Learned, Org, Segment, Source, VouchBasis};
use pw_world::scenario::DataOrigin;
use serde_json::Value;

use crate::contract::{AspectRow, DistrictView, LabelRow,
    CreationView, DerbyRow, EligibilityRow, EvidenceRow, ExportView, KnownBy, MarketRow, PathwayView, RecognitionView, ReferenceStatusRow, RegionOutputRow, RegionOutputView, ScenarioView, SegmentRegard, StepRow, TierRow, VouchView, WatchRow,
};
use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Ref};
use crate::pages::person::person_id;

fn to_json<T: serde::Serialize>(v: &T) -> ApiResult<Value> {
    serde_json::to_value(v).map_err(|e| ApiError::Bad(e.to_string()))
}

/// How far a name carries, in words (never the number).
fn standing_words(x: f32) -> &'static str {
    match x {
        x if x < 0.03 => "no name outside his own pitch",
        x if x < 0.12 => "a local name",
        x if x < 0.30 => "known across the district",
        x if x < 0.60 => "known beyond his district",
        _ => "a name nationally",
    }
}

fn level_words(x: f32) -> &'static str {
    match x {
        x if x < 0.15 => "thin",
        x if x < 0.4 => "some",
        x if x < 0.7 => "solid",
        _ => "strong",
    }
}

fn source_words(c: &Ctx, s: Source) -> String {
    match s {
        Source::Club(l) => format!("the coaches of {}", c.w.youth.local.get(l).map_or_else(|| "his local club".to_string(), |x| x.name.clone())),
        Source::Institution(i) => format!("the coaches of {}", c.w.minor.institutions.get(i as usize).map_or("his school", |x| x.name.as_str())),
        Source::District(r) => format!("the selectors of {}", c.w.ext.ecosystem.regions.get(r).map_or("his district", |x| x.name.as_str())),
        Source::Person(p) => c.person_name(p),
    }
}

fn org_words(c: &Ctx, o: Org) -> String {
    match o {
        Org::Club(k) => c.club_name(k),
        Org::Institution(i) => c.w.minor.institutions.get(i as usize).map_or_else(|| "an institution".into(), |x| x.name.clone()),
        Org::State(r) => format!("{} selectors", c.w.ext.ecosystem.regions.get(r).map_or("a state", |x| x.name.as_str())),
        Org::Federation(n) => format!("the {} federation", c.nation_name(n)),
        Org::Market(m) => format!("clubs in {}", pw_sim::export::market_name(c.w, m)),
    }
}

fn learned_words(l: Learned) -> &'static str {
    match l {
        Learned::Watched => "watched him play",
        Learned::Recommended => "was recommended him",
        Learned::Came => "he came to them",
        Learned::Buzz => "heard the talk and no more",
    }
}

fn judgement_row(c: &Ctx, j: &Judgement) -> EligibilityRow {
    let (kind, name) = match j.body {
        Body::State(r) => ("state", c.w.ext.ecosystem.regions.get(r).map_or_else(|| "a state".to_string(), |x| x.name.clone())),
        Body::National(n) => ("nation", c.nation_name(n)),
    };
    EligibilityRow {
        body_kind: kind.into(),
        body: name,
        eligible: j.eligible,
        reason: j.reason.text(),
        evidence: j.evidence.iter().map(|e| EvidenceRow { rule: e.rule.label().into(), holds: e.holds, detail: if e.detail == 0 { None } else { Some(e.detail) } }).collect(),
    }
}

/// `pathway.player {id}`: the player's route with the reason for each step, how he came to exist, whom he may play for, and (in the
/// omniscient view) who knows him.
pub fn player(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let id = person_id(args)?;
    let person = c.w.people.get(id).ok_or_else(|| ApiError::NotFound(format!("person {}", id.0)))?;
    let p: PlayerId = person.player.get().ok_or_else(|| ApiError::NotFound(format!("person {} is not a player", id.0)))?;
    let name = Named::new(Ref::person(id), c.person_name(id));
    let w = c.w;
    let eco = &w.ext.ecosystem;
    if !eco.is_configured() || !eco.story.contains_key(&p) {
        return to_json(&PathwayView { available: false, reason: Some("This world does not record a pathway for him.".into()), player: name, steps: Vec::new(), creation: None, eligibility: Vec::new(), recognition: None });
    }
    if !(c.observer() || c.is_me(p)) {
        return to_json(&PathwayView { available: false, reason: Some("His pathway is his own to tell.".into()), player: name, steps: Vec::new(), creation: None, eligibility: Vec::new(), recognition: None });
    }
    let route = pw_sim::ecosystem::route(w, p);
    let steps: Vec<StepRow> = route
        .iter()
        .map(|&(date, kind, target)| {
            let why = w.ext.pathway.for_step(p, kind, date);
            StepRow { date: date.0, kind: kind.label().into(), target: target_name(c, kind, target), why: why.map(|x| x.text().to_string()), recorded: why.is_some() }
        })
        .collect();
    let creation = w.ext.pathway.created.get(&p).map(|k| CreationView {
        date: if k.legacy { None } else { Some(k.date.0) },
        region: eco.regions.get(k.region).map_or_else(String::new, |r| r.name.clone()),
        provider: format!("{:?}", k.provider),
        institution: k.institution.and_then(|i| w.minor.institutions.get(i as usize)).map(|i| i.name.clone()),
        age: k.age,
        first_env: k.first_env.label().into(),
        first_finder: if k.first_finder.is_some() { Some(Named::new(Ref::person(k.first_finder), c.person_name(k.first_finder))) } else { None },
        why: format!("{:?}", k.why),
        provenance: if k.legacy { "Derived from an older save".into() } else if matches!(k.why, pw_world::pathway::Draw::WorldStart) { "Generated at world start".into() } else { "Recorded".into() },
    });
    let year = w.date.year();
    let mut eligibility: Vec<EligibilityRow> = Vec::new();
    let mut states: Vec<_> = eco.assoc.keys().copied().collect();
    states.sort();
    for s in states {
        let j = judge_state(w, p, s, year, false);
        // Only the states with a claim, or the one that refused for a stated reason, are worth a line.
        if j.eligible || j.evidence.iter().any(|e| e.holds && e.rule != pw_world::eligibility::Rule::Age) {
            eligibility.push(judgement_row(c, &j));
        }
    }
    for n in pw_sim::eligibility::eligible_nations(w, p) {
        eligibility.push(judgement_row(c, &pw_sim::eligibility::judge_national(w, p, n)));
    }
    let recognition = if c.sees_internal_state() { Some(recognition(c, p)) } else { None };
    to_json(&PathwayView { available: true, reason: None, player: name, steps, creation, eligibility, recognition })
}

fn target_name(c: &Ctx, kind: pw_world::ecosystem::StageKind, target: u32) -> Option<String> {
    use pw_world::ecosystem::StageKind as K;
    match kind {
        K::Academy | K::Released | K::Trial | K::SemiPro | K::Professional | K::StateLeague => Some(c.club_name(pw_core::ClubId(target))),
        K::University | K::School => c.w.minor.institutions.get(target as usize).map(|i| i.name.clone()),
        K::District | K::StateTeam | K::StateYouth => c.w.ext.ecosystem.regions.get(pw_core::RegionId(target)).map(|r| r.name.clone()),
        K::Grassroots | K::NationalCamp => None,
    }
}

fn recognition(c: &Ctx, p: PlayerId) -> RecognitionView {
    let w = c.w;
    let tiers = [Tier::Grassroots, Tier::School, Tier::District, Tier::Adult, Tier::Academy, Tier::State];
    let names = ["grassroots", "school", "district", "adult", "academy", "state"];
    let evidence: Vec<TierRow> = pw_sim::recognition::summary(w, p)
        .into_iter()
        .map(|(t, games, proof)| TierRow { level: names[tiers.iter().position(|x| *x == t).unwrap_or(0)].into(), games: games.round() as u32, proof: level_words(proof).into() })
        .collect();
    let mut known: Vec<(Org, pw_world::recog::Acquaintance)> = w.ext.recog.acquaint.iter().filter(|((_, q), _)| *q == p).map(|((o, _), a)| (*o, *a)).collect();
    known.sort_by_key(|(o, _)| *o);
    let known_by = known
        .into_iter()
        .map(|(o, a)| KnownBy { org: org_words(c, o), looks: a.sightings, years: a.years, how: learned_words(a.how).into(), first: a.first.0, last: a.last.0, first_by: if a.first_by.is_some() { Some(c.person_name(a.first_by)) } else { None } })
        .collect();
    let vouch = w.ext.recog.vouch.get(&p).map(|v| VouchView {
        from: source_words(c, v.from),
        basis: match v.basis {
            VouchBasis::Trained { months } => format!("coached him for {months} months"),
            VouchBasis::Watched { games } => format!("watched him in {games} games"),
            VouchBasis::Legacy => "an older save knew only that someone vouched".into(),
        },
        strength: level_words(v.strength).into(),
        credibility: level_words(v.credibility).into(),
        date: v.date.0,
    });
    let watching = w.ext.recog.watching.iter().filter(|x| x.player == p).map(|x| WatchRow { org: org_words(c, x.org), by: c.person_name(x.by), games_left: x.left, since: x.since.0 }).collect();
    RecognitionView { standing: standing_words(pw_sim::recognition::standing(w, p)).into(), evidence, known_by, vouch, watching, buzz: w.ext.ecosystem.repute.get(&p).is_some_and(|r| r.buzz > 10) }
}

/// `ecosystem.regions`: what each region has produced, by quality as well as number.
pub fn regions(c: &Ctx, _args: &Value) -> ApiResult<Value> {
    let eco = &c.w.ext.ecosystem;
    if !eco.is_configured() {
        return to_json(&RegionOutputView { available: false, rows: Vec::new(), note: "This world has no regions.".into() });
    }
    let mut rows: Vec<RegionOutputRow> = pw_sim::ecosystem::region_output(c.w)
        .into_iter()
        .map(|o| {
            let r = &eco.regions[o.region];
            RegionOutputRow {
                region: r.name.clone(),
                kind: if r.kind == pw_world::ecosystem::RegionKind::State { "state".into() } else { "district".into() },
                professionals: o.professionals,
                top_tier: o.top_tier,
                internationals: o.internationals,
                senior_apps: o.senior_apps,
                value: o.value,
            }
        })
        .collect();
    rows.sort_by(|a, b| (b.top_tier, b.internationals, b.professionals).cmp(&(a.top_tier, a.internationals, a.professionals)).then(a.region.cmp(&b.region)));
    to_json(&RegionOutputView { available: true, rows, note: "Players who have made ten or more senior appearances. Value is the market's, summed.".into() })
}

/// `ecosystem.export`: how each market abroad regards each kind of the country's football.
pub fn export(c: &Ctx, _args: &Value) -> ApiResult<Value> {
    let w = c.w;
    if !w.ext.ecosystem.is_configured() {
        return to_json(&ExportView { available: false, markets: Vec::new(), note: "This world has no export markets.".into() });
    }
    let n = w.ext.scenario.markets.len().max(1) as u8;
    let markets = (0..n)
        .map(|m| MarketRow {
            name: pw_sim::export::market_name(w, m),
            nations: w.ext.scenario.markets.get(usize::from(m)).map(|d| d.nations.clone()).unwrap_or_default(),
            segments: Segment::ALL
                .iter()
                .map(|&seg| {
                    let r = w.ext.recog.export.get(&(m, seg)).copied().unwrap_or_default();
                    SegmentRegard {
                        segment: seg.label().into(),
                        level: r.level.round() as u32,
                        exports: r.exports,
                        successes: r.successes,
                        visits: r.visits,
                        provenance: if r.legacy { "Derived from an older save" } else if r.exports == 0 && r.visits == 0 { "Scenario seed, nothing earned yet" } else { "Earned" }.into(),
                    }
                })
                .collect(),
        })
        .collect();
    to_json(&ExportView { available: true, markets, note: "Regard is per market and per kind of football. Nations in no market do not look at this country at all.".into() })
}

/// `ecosystem.scenario`: the calendar, where the tuning came from, where each club's starting data came from, what reading the
/// reference data found, and the derbies it names (labels only).
pub fn scenario(c: &Ctx, _args: &Value) -> ApiResult<Value> {
    let sc = &c.w.ext.scenario;
    let rep = &sc.reference;
    let mut origin = [0u32; 3];
    for o in sc.club_origin.values() {
        origin[match o {
            DataOrigin::Imported => 0,
            DataOrigin::ScenarioSeed => 1,
            DataOrigin::Generated => 2,
        }] += 1;
    }
    let unknown = c.w.clubs.iter_enumerated().filter(|(id, _)| !sc.club_origin.contains_key(id)).count() as u32;
    const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    let calendar = sc
        .calendar
        .iter()
        .map(|r| {
            let months: Vec<&str> = r.months.iter().filter_map(|&m| MONTHS.get(usize::from(m).wrapping_sub(1)).copied()).collect();
            let when = if r.day == 0 { months.join(", ") } else { format!("{} {}", r.day, months.join(", ")) };
            crate::contract::CalendarRow { event: format!("{:?}", r.event), when }
        })
        .collect();
    let lr = lore_rows(c);
    to_json(&ScenarioView {
        available: c.w.ext.ecosystem.is_configured(),
        source: if sc.source.is_empty() { "built-in defaults".into() } else { sc.source.clone() },
        calendar,
        clubs_imported: origin[0],
        clubs_seeded: origin[1],
        clubs_generated: origin[2],
        clubs_unknown: unknown,
        reference_loaded: rep.records > 0,
        reference_files: rep.files,
        reference_records: rep.records,
        reference_by_status: if rep.records > 0 { pw_world::scenario::REFERENCE_STATUS_LABELS.iter().zip(rep.by_status).map(|(l, n)| ReferenceStatusRow { label: (*l).into(), records: n }).collect() } else { Vec::new() },
        reference_findings: rep.findings,
        finding_samples: rep.finding_samples.clone(),
        clubs_matched: rep.clubs_matched,
        clubs_from_reference: rep.clubs_from_reference,
        derbies: sc
            .known_derbies
            .iter()
            .map(|d| DerbyRow { name: d.name.clone(), a: c.w.clubs[d.a].name.clone(), b: c.w.clubs[d.b].name.clone(), kind: if d.derby { "Derby" } else { "Rivalry" }.into(), origin: d.origin.label().into() })
            .collect(),
        associations: lr.associations,
        press: lr.press,
        broadcasters: lr.broadcasters,
        institutions_real: c.w.ext.lore.institutions.len() as u32,
        programmes: lr.programmes,
        partnerships: lr.partnerships,
        coaching_ladder: lr.licences,
        referee_ladder: lr.grades,
        representative_sides: lr.teams,
        rules: lr.rules,
        languages: lr.languages,
        note: "The weights and thresholds that decide who is noticed are initial tuning held in the scenario's data, not facts about football. Names, programmes and rules come from reference records of the standing shown; none of them sets a strength or a result.".into(),
    })
}

#[derive(Default)]
struct LoreRows {
    associations: Vec<LabelRow>,
    press: Vec<LabelRow>,
    broadcasters: Vec<LabelRow>,
    programmes: Vec<LabelRow>,
    partnerships: Vec<LabelRow>,
    licences: Vec<LabelRow>,
    grades: Vec<LabelRow>,
    teams: Vec<LabelRow>,
    rules: Vec<LabelRow>,
    languages: Vec<LabelRow>,
}

fn row(name: impl Into<String>, detail: impl Into<String>, note: impl Into<String>, origin: DataOrigin) -> LabelRow {
    LabelRow { name: name.into(), detail: detail.into(), note: note.into(), origin: origin.label().into() }
}

/// What the world took from reference data, as rows for the scenario page (see `pw_world::lore`).
fn lore_rows(c: &Ctx) -> LoreRows {
    let w = c.w;
    let lore = &w.ext.lore;
    let lang_name = |code: &str| lore.languages.iter().find(|l| l.code == code).map_or(code.to_string(), |l| l.name.clone());
    let langs = |v: &[String]| v.iter().map(|l| lang_name(l)).collect::<Vec<_>>().join(", ");
    let words = |s: &str| s.replace('_', " ");
    let mut out = LoreRows::default();
    let mut assoc: Vec<_> = lore.associations.iter().collect();
    assoc.sort_by_key(|(r, _)| w.ext.ecosystem.regions[**r].name.clone());
    out.associations = assoc.into_iter().map(|(r, a)| row(a.name.clone(), w.ext.ecosystem.regions[*r].name.clone(), a.hq.clone(), a.source.origin)).collect();
    let mut press: Vec<_> = lore.outlets.iter().filter(|(_, o)| o.kind != "broadcaster").collect();
    press.sort_by_key(|(id, _)| **id);
    out.press = press
        .into_iter()
        .map(|(id, o)| {
            let home = if o.home.is_some() { w.ext.ecosystem.regions[o.home].name.clone() } else { words(&o.reach) };
            row(w.media.outlets[*id].name.clone(), format!("{} · {}", words(&o.kind), home), langs(&o.languages), o.source.origin)
        })
        .collect();
    out.broadcasters =
        lore.broadcasters.iter().map(|b| row(b.name.clone(), b.comps.iter().map(|&x| w.comps[x].name.clone()).collect::<Vec<_>>().join(", "), langs(&b.languages), b.source.origin)).collect();
    out.programmes = lore
        .programmes
        .iter()
        .map(|p| {
            let wh = if p.regions.is_empty() { "national".to_string() } else { p.regions.iter().map(|&r| w.ext.ecosystem.regions[r].name.clone()).collect::<Vec<_>>().join(", ") };
            let ages = if p.ages.is_empty() { String::new() } else { format!(" · ages {}", p.ages) };
            row(p.name.clone(), format!("{} · {}{ages} · {wh}", p.operator, words(&p.kind)), p.description.clone(), p.source.origin)
        })
        .collect();
    out.partnerships = lore
        .partnerships
        .iter()
        .map(|p| {
            let ours: Vec<String> = p.clubs.iter().map(|&x| w.clubs[x].name.clone()).collect();
            let theirs: Vec<String> = p.foreign.iter().map(|(n, nat)| format!("{n} ({nat})")).collect();
            let parties = if ours.is_empty() { "All India Football Federation".to_string() } else { ours.join(", ") };
            let status = if p.active { "" } else { " (ended)" };
            row(format!("{parties} with {}{status}", theirs.join(", ")), p.components.iter().map(|x| words(x)).collect::<Vec<_>>().join(", "), p.purpose.clone(), p.source.origin)
        })
        .collect();
    out.licences = lore.licences.iter().map(|l| row(l.name.clone(), format!("step {} · {}", l.order, l.body), l.requirement.clone(), l.source.origin)).collect();
    out.grades = lore.grades.iter().map(|g| row(g.name.clone(), format!("step {} · {}", g.order, g.body), words(&g.scope), g.source.origin)).collect();
    out.teams = lore.teams.iter().map(|t| row(t.name.clone(), format!("{} · {}", words(&t.kind), t.age), t.eligibility.clone(), t.source.origin)).collect();
    out.rules = lore.rules.iter().map(|r| row(words(&r.topic), r.comps.iter().map(|&x| w.comps[x].short_name.clone()).collect::<Vec<_>>().join(", "), r.statement.clone(), r.source.origin)).collect();
    let mut codes: Vec<&str> = lore.terms.iter().map(|t| t.lang.as_str()).collect();
    codes.sort_unstable();
    codes.dedup();
    out.languages = codes
        .into_iter()
        .map(|code| {
            let goal = lore.terms("match.goal", code).next();
            let example =
                goal.map_or(String::new(), |t| if t.romanised.is_empty() || t.romanised == t.text { format!("\"goal\": {}", t.text) } else { format!("\"goal\": {} ({})", t.text, t.romanised) });
            row(lang_name(code), format!("{} words", lore.terms.iter().filter(|t| t.lang == code).count()), example, DataOrigin::ScenarioSeed)
        })
        .collect();
    out
}

fn word(x: f32) -> &'static str {
    match x {
        x if x < 20.0 => "very low",
        x if x < 40.0 => "low",
        x if x < 60.0 => "middling",
        x if x < 80.0 => "high",
        _ => "very high",
    }
}

/// `ecosystem.district {id?}`: what a place is like for a child growing up in it, and who is near. With no id, the district of the person
/// being lived as. These are facts about the place (its coaching, its scouting, its money), never about any child in it.
pub fn district(c: &Ctx, args: &Value) -> ApiResult<Value> {
    let eco = &c.w.ext.ecosystem;
    let none = |why: &str| to_json(&DistrictView { available: false, reason: Some(why.into()), name: String::new(), state: String::new(), association: None, population_k: 0, aspects: Vec::new(), academies: Vec::new(), universities: Vec::new(), schools: 0 });
    if !eco.is_configured() {
        return none("This world has no districts.");
    }
    let id = match args.get("id").and_then(Value::as_u64) {
        Some(n) => pw_core::RegionId(n as u32),
        None => match c.my_player().and_then(|p| eco.story.get(&p)) {
            Some(s) => s.dev,
            None => return none("Choose a district to look at."),
        },
    };
    if id.is_none() || (id.0 as usize) >= eco.regions.len() {
        return none("There is no such place.");
    }
    let r = &eco.regions[id];
    let state = eco.state_of(id);
    let state_name = if state.is_some() { eco.regions[state].name.clone() } else { r.name.clone() };
    let a = eco.assoc.get(&state);
    let mut aspects = vec![
        AspectRow { label: "Children playing".into(), level: word(r.participation).into(), note: "How much of the district plays organised football at all. More players means more competition for every place, and more talent to find.".into() },
        AspectRow { label: "Coaching".into(), level: word(r.coach_density).into(), note: "Licensed coaches per child. Good coaches develop players and speak up for the best of them; without them, talent goes unrecognised.".into() },
        AspectRow { label: "Facilities".into(), level: word(r.facilities).into(), note: "Pitches, floodlights and equipment. Poor facilities limit training and the level of the games scouts can watch.".into() },
        AspectRow { label: "Competitive football".into(), level: word(r.competition_density).into(), note: "How many real matches a child gets. Evidence from more games, against real opposition, is believed sooner.".into() },
        AspectRow { label: "Scouting coverage".into(), level: word(r.scouting_coverage).into(), note: "How much of the district's football anyone with influence watches. Low coverage means a great player can go unseen for years.".into() },
        AspectRow { label: "Academy access".into(), level: word(r.academy_access).into(), note: "How reachable professional academies are from here.".into() },
        AspectRow { label: "Household means".into(), level: word(r.economic_access).into(), note: "How easily families can afford travel, kit and trials. It decides who can turn up when a chance comes.".into() },
        AspectRow { label: "Football culture".into(), level: word(r.culture).into(), note: "How much the place cares. It grows from what the district has produced and fades without it.".into() },
    ];
    if let Some(a) = a {
        aspects.push(AspectRow { label: "State association".into(), level: word(a.governance).into(), note: "How well the state association is run. It shapes funding, competitions and how state sides are picked.".into() });
    }
    let mut academies: Vec<(f32, pw_core::ClubId)> = c.w.youth.academies.keys().map(|&k| (eco.travel_burden(eco.region_of_club(k), id), k)).filter(|x| x.0 < 0.3).collect();
    academies.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let academies = academies.into_iter().take(12).map(|(_, k)| Named::new(Ref::club(k), c.club_name(k))).collect();
    let mut unis: Vec<(f32, String)> = eco
        .inst
        .iter()
        .filter(|(i, p)| c.w.minor.institutions.get(**i as usize).is_some_and(|x| x.kind == pw_world::minor::InstKind::University) && p.region.is_some())
        .map(|(i, p)| (eco.travel_burden(p.region, id), c.w.minor.institutions[*i as usize].name.clone()))
        .filter(|x| x.0 < 0.3)
        .collect();
    unis.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let schools = eco.inst.iter().filter(|(i, p)| p.region == id && c.w.minor.institutions.get(**i as usize).is_some_and(|x| x.kind == pw_world::minor::InstKind::School)).count() as u32;
    to_json(&DistrictView {
        available: true,
        reason: None,
        name: r.name.clone(),
        state: state_name,
        association: eco.assoc_name.get(&state).cloned(),
        population_k: r.population_k,
        aspects,
        academies,
        universities: unis.into_iter().take(8).map(|x| x.1).collect(),
        schools,
    })
}
