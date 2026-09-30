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

use crate::contract::{
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
        note: "The weights and thresholds that decide who is noticed are initial tuning held in the scenario's data, not facts about football.".into(),
    })
}
