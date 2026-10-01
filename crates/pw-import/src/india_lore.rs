//! What the India world takes from its reference data beyond clubs, grounds and derbies (`india::build` does those): the real names
//! of the associations, leagues, schools, universities, academies and press; nicknames; and, as words for the views and the narration,
//! the programmes, partnerships, coaching ladder, representative sides, rules and football vocabulary.
//!
//! The rules of `india` hold here too. A record gives a name, a kind, a place and what it says it does; it never gives a strength, a
//! result or a history. Where the world needs a number the record does not have (an outlet's audience, a school's coaching), the
//! value is a neutral starting value chosen here by kind, the same for every record of that kind, and the simulation moves it from
//! there. Every thing taken is kept in `World::ext.lore` with the record's id and standing (`DataOrigin`).

use std::collections::HashMap;

use pw_core::rng::Rng;
use pw_core::{ClubId, CompId, NationId, RegionId};
use pw_world::World;
use pw_world::ecosystem::InstProfile;
use pw_world::lore::{
    LoreAcademy, LoreAlias, LoreAssociation, LoreBroadcaster, LoreGrade, LoreInstitution, LoreLanguage, LoreLicence, LoreOutlet, LorePartnership, LoreProgramme, LoreRule, LoreTeam, LoreTerm, Source,
};
use pw_world::media::{Outlet, OutletKind};
use pw_world::minor::{InstKind, Institution};
use smallvec::SmallVec;

use crate::india_ref::{OutletRow, Prov, Reference, season_covers};

/// A state of the world: its pack key (`WB`), its region and its districts by name.
pub struct StateOf {
    pub key: String,
    pub region: RegionId,
    pub districts: Vec<(String, RegionId)>,
}

/// What `india::build` hands over.
pub struct Built<'a> {
    pub nation: NationId,
    pub year: i32,
    pub states: &'a [StateOf],
    /// Reference club id -> the club of the world it is.
    pub placed: &'a HashMap<String, ClubId>,
    /// The national tiers, top first.
    pub pyramid: &'a [CompId],
    /// State key -> its premier league.
    pub state_premier: &'a [(String, CompId)],
}

/// Most real universities a state gets, counting the ones the pack names.
const UNIVERSITIES_PER_STATE: usize = 5;
/// Most real outlets of national and of state reach.
const NATIONAL_OUTLETS: usize = 12;
const OUTLETS_PER_STATE: usize = 3;

fn src(id: &str, prov: &Prov) -> Source {
    Source { id: id.to_string(), origin: prov.origin() }
}

fn lang_code(id: &str) -> String {
    id.strip_prefix("lang.").unwrap_or(id).to_string()
}

impl Built<'_> {
    fn state(&self, reference: &Reference, state_id: &str) -> Option<&StateOf> {
        self.states.iter().find(|s| reference.state_of_key(&s.key).is_some_and(|r| r.id == state_id))
    }

    /// The district of a state a city is in, by the district's name appearing in the city's (or the other way round); else the
    /// state's first district.
    fn district(&self, st: &StateOf, city: &str) -> RegionId {
        st.districts.iter().find(|(n, _)| city.contains(n.as_str()) || n.contains(city)).or(st.districts.first()).map_or(st.region, |x| x.1)
    }
}

pub fn apply(w: &mut World, reference: &Reference, b: &Built<'_>, rng: &mut Rng) {
    languages(w, reference);
    associations(w, reference, b);
    competitions(w, reference, b);
    institutions(w, reference, b, rng);
    academies_and_aliases(w, reference, b);
    press(w, reference, b);
    words(w, reference, b);
}

fn languages(w: &mut World, reference: &Reference) {
    let lore = &mut w.ext.lore;
    lore.languages =
        reference.languages.iter().filter(|l| l.prov.names_real_entity()).map(|l| LoreLanguage { code: l.code.clone(), name: l.name.clone(), script: l.script.clone().unwrap_or_default() }).collect();
    lore.terms = reference
        .terms
        .iter()
        .filter(|t| t.prov.names_real_entity())
        .map(|t| LoreTerm {
            concept: t.concept.clone(),
            lang: lang_code(&t.lang),
            text: t.canonical.clone(),
            romanised: t.synonyms.iter().find(|s| s.is_ascii()).cloned().unwrap_or_else(|| if t.canonical.is_ascii() { t.canonical.clone() } else { String::new() }),
            register: t.register.clone(),
        })
        .collect();
}

/// A state's association: the one state association the reference has for it. Two (a merged territory's former bodies) is no answer.
fn associations(w: &mut World, reference: &Reference, b: &Built<'_>) {
    for st in b.states {
        let Some(sr) = reference.state_of_key(&st.key) else { continue };
        let of_state: Vec<_> = reference.associations.iter().filter(|a| a.state.as_deref() == Some(sr.id.as_str()) && a.kind.as_deref() == Some("state") && a.prov.names_real_entity()).collect();
        let [a] = of_state.as_slice() else { continue };
        w.ext.ecosystem.assoc_name.insert(st.region, a.name.clone());
        w.ext
            .lore
            .associations
            .insert(st.region, LoreAssociation { source: src(&a.id, &a.prov), name: a.name.clone(), abbr: a.abbr.clone().unwrap_or_default(), hq: a.hq_city.clone().unwrap_or_default() });
    }
}

/// The national tiers and each state's premier league carry the name of the real competition they are.
fn competitions(w: &mut World, reference: &Reference, b: &Built<'_>) {
    let name = |w: &mut World, comp: CompId, r: &crate::india_ref::CompRow| {
        w.comps[comp].name = r.name.clone();
        if let Some(short) = r.short.as_deref().filter(|s| !s.is_empty()) {
            w.comps[comp].short_name = short.to_string();
        }
        w.ext.lore.competitions.insert(comp, src(&r.id, &r.prov));
    };
    for (t, &comp) in b.pyramid.iter().enumerate() {
        if let Some(r) = reference.national_league(t as u8 + 1).filter(|r| r.prov.names_real_entity()) {
            name(w, comp, r);
        }
    }
    for (key, comp) in b.state_premier {
        let Some(sr) = reference.state_of_key(key) else { continue };
        if let Some(r) = reference.state_top_league(&sr.id, b.year).filter(|r| r.prov.names_real_entity()) {
            name(w, *comp, r);
        }
    }
}

fn add_institution(w: &mut World, nation: NationId, kind: InstKind, name: &str, city: &str, profile: InstProfile, prestige: u16, coaching: u8, lore: LoreInstitution) -> u32 {
    let id = w.minor.institutions.len() as u32;
    // The founding year is not in the record: unknown, written as the year the world begins, never an invented date.
    let founded = w.date.year();
    w.minor.institutions.push(Institution { id, kind, name: name.to_string(), nation, city: city.to_string(), founded, prestige, coaching, members: Vec::new(), alumni_pros: SmallVec::new() });
    w.ext.ecosystem.inst.insert(id, profile);
    w.ext.lore.institutions.insert(id, lore);
    id
}

/// Universities: the pack's are labelled by their record; states with fewer than `UNIVERSITIES_PER_STATE` get more of the real ones.
/// Schools, sports schools, sports hostels and SAI centres of the reference join the generated schools of their district.
fn institutions(w: &mut World, reference: &Reference, b: &Built<'_>, rng: &mut Rng) {
    let known: Vec<(u32, String)> = w.minor.institutions.iter().filter(|i| i.kind == InstKind::University).map(|i| (i.id, i.name.clone())).collect();
    let mut per_state: HashMap<String, usize> = HashMap::new();
    for u in reference.universities.iter().filter(|u| u.prov.names_real_entity()) {
        let Some(st) = b.state(reference, &u.state) else { continue };
        let lore = LoreInstitution { source: src(&u.id, &u.prov), kind: u.kind.clone().unwrap_or_default(), operator: String::new(), note: String::new() };
        if let Some((id, _)) = known.iter().find(|(_, n)| *n == u.name) {
            w.ext.lore.institutions.insert(*id, lore);
            *per_state.entry(st.key.clone()).or_default() += 1;
        }
    }
    for u in reference.universities.iter().filter(|u| u.prov.names_real_entity()) {
        let Some(st) = b.state(reference, &u.state) else { continue };
        if known.iter().any(|(_, n)| *n == u.name) {
            continue;
        }
        let n = per_state.entry(st.key.clone()).or_default();
        if *n >= UNIVERSITIES_PER_STATE {
            continue;
        }
        *n += 1;
        // A university the pack does not seed starts at the low end of what the pack gives its own: one scholarship, modest means.
        let region = b.district(st, &u.city);
        let profile = InstProfile { region, resources: 28.0, facilities: 25.0, scholarships: 1, residential: u.residential.unwrap_or(true), success: 30.0, real: true };
        let lore = LoreInstitution { source: src(&u.id, &u.prov), kind: u.kind.clone().unwrap_or_default(), operator: String::new(), note: String::new() };
        let coaching = rng.range_i32(5, 9) as u8;
        add_institution(w, b.nation, InstKind::University, &u.name, &u.city, profile, rng.range_i32(250, 550) as u16, coaching, lore);
    }
    for s in reference.schools.iter().filter(|s| s.prov.names_real_entity()) {
        let Some(st) = b.state(reference, &s.state) else { continue };
        let sporting = matches!(s.kind.as_str(), "sports_school" | "sports_hostel" | "sai_centre" | "academy_school");
        let region = b.district(st, &s.city);
        let residential = s.residential.unwrap_or(sporting);
        // A sporting school starts where the generated state sports hostel does; any other school where a good generated one does.
        let (resources, scholarships, prestige, coaching) = if sporting { (45.0, 6, 400, 7) } else { (35.0, 0, 450, 5) };
        let profile = InstProfile { region, resources, facilities: resources * 0.8, scholarships, residential, success: 30.0, real: true };
        let lore = LoreInstitution { source: src(&s.id, &s.prov), kind: s.kind.clone(), operator: s.operator.clone().unwrap_or_default(), note: prose(s.programme.as_deref().unwrap_or_default()) };
        add_institution(w, b.nation, InstKind::School, &s.name, &s.city, profile, prestige, coaching, lore);
    }
}

/// The academy a club runs, when the reference names it for a club of this world; and the other names clubs go by.
fn academies_and_aliases(w: &mut World, reference: &Reference, b: &Built<'_>) {
    for a in reference.academies.iter().filter(|a| a.prov.names_real_entity() && a.status.as_deref() != Some("inactive")) {
        let Some(club) = a.parent.as_deref().and_then(|p| b.placed.get(p)) else { continue };
        w.ext.lore.academies.insert(*club, LoreAcademy { source: src(&a.id, &a.prov), name: a.name.clone(), kind: a.kind.clone(), residential: a.residential, age_groups: a.age_groups.clone() });
    }
    for a in reference.aliases.iter().filter(|a| a.prov.names_real_entity() && a.kind != "derby_name") {
        let Some(club) = a.entity.as_deref().and_then(|e| b.placed.get(e)) else { continue };
        let v = w.ext.lore.aliases.entry(*club).or_default();
        if !v.iter().any(|x| x.text == a.alias) {
            v.push(LoreAlias { text: a.alias.clone(), kind: a.kind.clone(), lang: a.lang.as_deref().map(lang_code).unwrap_or_default(), origin: a.prov.origin() });
        }
    }
}

/// How the world's press is seeded from a real outlet's kind: (kind in the simulation, accuracy, sensationalism), both 1-20. The
/// reference deliberately has no quality scores; these are the same starting values for every outlet of a kind, and credibility is
/// earned in the simulation from what they print.
fn outlet_seed(kind: &str) -> (OutletKind, u8, u8) {
    match kind {
        "national_sports" => (OutletKind::National, 14, 7),
        "football_specialist" => (OutletKind::National, 15, 6),
        "general_news" | "news_agency" => (OutletKind::National, 14, 6),
        "tv_sports" | "radio" => (OutletKind::Broadcaster, 12, 9),
        "digital_sports" => (OutletKind::DataSite, 13, 9),
        "official_club" | "official_federation" | "official_league" => (OutletKind::FanChannel, 9, 4),
        "campus" | "local_newspaper" => (OutletKind::Local, 10, 9),
        _ => (OutletKind::Local, 11, 9),
    }
}

/// An outlet's audience (1-20) from where it reaches and, where the record says, how large its audience is.
fn reach_of(reach: &str, audience: Option<&str>) -> u8 {
    let base: i32 = match reach {
        "national" => 12,
        "multi_state" => 9,
        "state" => 6,
        _ => 3,
    };
    let size = match audience {
        Some("large") => 3,
        Some("small") => -3,
        Some("niche") => -5,
        _ => 0,
    };
    (base + size).clamp(1, 20) as u8
}

/// Most outlets of one kind in the national press (data and results sites crowd out journalism otherwise).
const NATIONAL_PER_KIND: usize = 2;

/// The press of the India world: real outlets that care about football (specialists and strong football coverage, and the national
/// sports and specialist titles whatever their emphasis), at most `NATIONAL_OUTLETS` with national reach and `OUTLETS_PER_STATE` based
/// in each state of the world; official channels of clubs in the world; and the broadcasters holding rights to its competitions this
/// season. Journalists are hired by `pw_sim::media::ensure_media` like any outlet's.
fn press(w: &mut World, reference: &Reference, b: &Built<'_>) {
    let cares = |o: &OutletRow| matches!(o.football_emphasis.as_deref(), Some("specialist" | "strong")) || matches!(o.kind.as_str(), "national_sports" | "football_specialist" | "tv_sports");
    let usable = |o: &&OutletRow| o.prov.names_real_entity() && o.active != Some(false);
    let mut chosen: Vec<(&OutletRow, ClubId)> = Vec::new();
    // Specialists first, then strong coverage; within each, the order the files give.
    let ranked = |pred: &dyn Fn(&OutletRow) -> bool| -> Vec<&OutletRow> {
        let mut v: Vec<&OutletRow> = reference.outlets.iter().filter(usable).filter(|o| pred(o)).collect();
        v.sort_by_key(|o| if o.football_emphasis.as_deref() == Some("specialist") { 0 } else { 1 });
        v
    };
    let national = ranked(&|o| cares(o) && o.club.is_none() && matches!(o.reach.as_str(), "national" | "multi_state") && !o.kind.starts_with("official"));
    let mut per_kind: HashMap<&str, usize> = HashMap::new();
    for o in national {
        if chosen.len() >= NATIONAL_OUTLETS {
            break;
        }
        // Specialist football journalism is not capped; other kinds (data sites, aggregators) at `NATIONAL_PER_KIND` each.
        let n = per_kind.entry(o.kind.as_str()).or_default();
        if o.kind != "football_specialist" && *n >= NATIONAL_PER_KIND {
            continue;
        }
        *n += 1;
        chosen.push((o, ClubId::NONE));
    }
    for st in b.states {
        let Some(sr) = reference.state_of_key(&st.key) else { continue };
        let local = ranked(&|o| cares(o) && o.club.is_none() && o.home_state.as_deref() == Some(sr.id.as_str()) && matches!(o.reach.as_str(), "state" | "local") && !o.kind.starts_with("official"));
        chosen.extend(local.into_iter().take(OUTLETS_PER_STATE).map(|o| (o, ClubId::NONE)));
    }
    for o in reference.outlets.iter().filter(usable) {
        if let Some(club) = o.club.as_deref().and_then(|c| b.placed.get(c)) {
            chosen.push((o, *club));
        }
    }
    for (o, club) in chosen {
        // Anything that belongs to one club (its official channels, its supporters' groups) speaks as a fan channel of that club.
        let (kind, accuracy, sensationalism) = match (club.is_some(), o.kind.starts_with("official")) {
            (true, true) => (OutletKind::FanChannel, 9, 4),
            (true, false) => (OutletKind::FanChannel, 7, 13),
            _ => outlet_seed(&o.kind),
        };
        let home = o.home_state.as_deref().and_then(|s| b.state(reference, s)).map_or(RegionId::NONE, |s| s.region);
        let oid =
            w.media.outlets.push(Outlet { name: o.name.clone(), nation: b.nation, kind, reach: reach_of(&o.reach, o.audience.as_deref()), accuracy, sensationalism, leaning: club, credibility: 55 });
        w.ext.lore.outlets.insert(
            oid,
            LoreOutlet {
                source: src(&o.id, &o.prov),
                kind: o.kind.clone(),
                languages: o.languages.iter().map(|l| lang_code(l)).collect(),
                reach: o.reach.clone(),
                home,
                city: o.home_city.clone().unwrap_or_default(),
            },
        );
    }
    // Broadcasters with rights to a competition of this world in the season it begins.
    let comp_of = |id: &str| w.ext.lore.competitions.iter().find(|(_, s)| s.id == id).map(|(c, _)| *c);
    let mut casters: Vec<LoreBroadcaster> = Vec::new();
    for r in reference.rights.iter().filter(|r| r.prov.names_real_entity() && season_covers(&r.season, b.year)) {
        let Some(comp) = comp_of(&r.competition) else { continue };
        let Some(bc) = reference.broadcasters.iter().find(|x| x.id == r.broadcaster && x.prov.names_real_entity()) else { continue };
        match casters.iter_mut().find(|c| c.source.id == bc.id) {
            Some(c) if !c.comps.contains(&comp) => c.comps.push(comp),
            Some(_) => {}
            None => casters.push(LoreBroadcaster { source: src(&bc.id, &r.prov), name: bc.name.clone(), languages: bc.languages.iter().map(|l| lang_code(l)).collect(), comps: vec![comp] }),
        }
    }
    for c in &casters {
        let oid =
            w.media.outlets.push(Outlet { name: c.name.clone(), nation: b.nation, kind: OutletKind::Broadcaster, reach: 14, accuracy: 12, sensationalism: 9, leaning: ClubId::NONE, credibility: 55 });
        w.ext
            .lore
            .outlets
            .insert(oid, LoreOutlet { source: c.source.clone(), kind: "broadcaster".into(), languages: c.languages.clone(), reach: "national".into(), home: RegionId::NONE, city: String::new() });
    }
    w.ext.lore.broadcasters = casters;
}

/// Programmes, partnerships, representative sides, the coaching and refereeing ladders and the rules: words for the views and the
/// narration. The rules' numbers are not applied from here.
fn words(w: &mut World, reference: &Reference, b: &Built<'_>) {
    let comp_of: HashMap<String, CompId> = w.ext.lore.competitions.iter().map(|(c, s)| (s.id.clone(), *c)).collect();
    let lore = &mut w.ext.lore;
    lore.programmes = reference
        .programmes
        .iter()
        .filter(|p| p.prov.names_real_entity() && p.kind != "women")
        .filter_map(|p| {
            let regions: Vec<RegionId> = match &p.region {
                Some(toml::Value::String(s)) if s == "national" => Vec::new(),
                Some(toml::Value::String(s)) => vec![b.state(reference, s)?.region],
                Some(toml::Value::Array(a)) => a.iter().filter_map(|v| v.as_str()).filter_map(|s| b.state(reference, s)).map(|s| s.region).collect(),
                _ => Vec::new(),
            };
            Some(LoreProgramme {
                source: src(&p.id, &p.prov),
                name: p.name.clone(),
                operator: p.operator.clone().map(|o| reference.associations.iter().find(|a| a.id == o).map_or(o, |a| a.name.clone())).unwrap_or_default(),
                kind: p.kind.clone(),
                ages: p.ages.clone().unwrap_or_default(),
                regions,
                description: prose(p.description.as_deref().unwrap_or_default()),
            })
        })
        .collect();
    lore.partnerships = reference
        .partnerships
        .iter()
        .filter(|p| p.prov.names_real_entity() && !p.components.iter().all(|c| c == "women"))
        .filter_map(|p| {
            let clubs: Vec<ClubId> = p.indian.iter().filter_map(|i| b.placed.get(i)).copied().collect();
            // A partnership of a club not in this world is not this world's; the federation's are.
            if clubs.is_empty() && !p.indian.iter().any(|i| i.starts_with("assoc.aiff")) {
                return None;
            }
            Some(LorePartnership {
                source: src(&p.id, &p.prov),
                clubs,
                foreign: p.foreign.iter().map(|f| (f.name.clone(), f.nation.clone())).collect(),
                components: p.components.clone(),
                purpose: prose(p.purpose.as_deref().unwrap_or_default()),
                active: p.status.as_deref() != Some("expired"),
            })
        })
        .collect();
    // A state's side only for a state of this world; one record of each national side (two files describe the same teams).
    let in_world = |assoc: &str| {
        reference.associations.iter().find(|a| a.id == assoc).and_then(|a| a.state.as_deref()).is_some_and(|st| b.states.iter().any(|s| reference.state_of_key(&s.key).is_some_and(|r| r.id == st)))
    };
    let mut seen: Vec<(String, String, String)> = Vec::new();
    lore.teams = reference
        .teams
        .iter()
        .filter(|t| t.prov.names_real_entity() && t.gender != "women")
        .filter(|t| t.kind != "state_representative" || t.association.as_deref().is_none_or(in_world))
        .filter(|t| {
            if t.kind == "state_representative" {
                return true;
            }
            let key = (t.kind.clone(), t.gender.clone(), t.age.clone());
            if seen.contains(&key) {
                return false;
            }
            seen.push(key);
            true
        })
        .map(|t| LoreTeam {
            source: src(&t.id, &t.prov),
            name: t.name.clone(),
            kind: t.kind.clone(),
            gender: t.gender.clone(),
            age: t.age.clone(),
            eligibility: prose(t.eligibility.as_deref().unwrap_or_default()),
        })
        .collect();
    let mut licences: Vec<LoreLicence> = reference
        .licences
        .iter()
        .filter(|l| l.prov.names_real_entity())
        .map(|l| LoreLicence { source: src(&l.id, &l.prov), name: l.name.clone(), body: l.body.clone(), order: l.order, requirement: prose(l.requirement.as_deref().unwrap_or_default()) })
        .collect();
    licences.sort_by_key(|l| l.order);
    lore.licences = licences;
    let mut grades: Vec<LoreGrade> = reference
        .grades
        .iter()
        .filter(|g| g.prov.names_real_entity())
        .map(|g| LoreGrade { source: src(&g.id, &g.prov), name: g.name.clone(), body: g.body.clone(), order: g.order, scope: g.scope.clone() })
        .collect();
    grades.sort_by_key(|g| g.order);
    lore.grades = grades;
    lore.rules = reference
        .rules
        .iter()
        .filter(|r| r.prov.names_real_entity())
        .map(|r| {
            let mut comps: Vec<CompId> = r.applies_to.iter().chain(r.competition.iter()).filter_map(|c| comp_of.get(c)).copied().collect();
            comps.sort();
            comps.dedup();
            LoreRule { source: src(&r.id, &r.prov), topic: r.topic.clone(), statement: prose(&r.statement), comps }
        })
        .collect();
}

/// Reference prose as a reader of the world sees it. The records are written for the people who keep them, and some clauses talk to
/// them (a file to look in, a setting of the simulation, a warning that a fact is unchecked); those clauses are dropped, and what the
/// record says about football is kept as written. Standing is shown by the record's origin, not by its wording.
pub fn prose(text: &str) -> String {
    let for_keepers = |c: &str| {
        let l = c.to_ascii_lowercase();
        l.contains(".toml") || l.contains("simulation") || l.contains("not verified") || l.contains("unverified") || l.contains("re-checked") || l.contains("model knowledge") || l.contains("scenario setting")
            || l.contains("placeholder") || l.contains("not confirmed")
    };
    let mut out: Vec<String> = Vec::new();
    for sentence in text.split(". ") {
        let kept: Vec<&str> = sentence.split("; ").map(str::trim).filter(|c| !c.is_empty() && !for_keepers(c)).collect();
        if !kept.is_empty() {
            out.push(kept.join("; ").trim_end_matches('.').to_string());
        }
    }
    if out.is_empty() { String::new() } else { format!("{}.", out.join(". ")) }
}

#[cfg(test)]
mod tests {
    use super::prose;

    #[test]
    fn prose_keeps_what_a_record_says_about_football_and_drops_what_it_says_to_its_keepers() {
        assert_eq!(
            prose("Entry-level short courses for coaches of young children, run by AIFF and state associations to a common syllabus; feeds the licence ladder in development/coach_education.toml."),
            "Entry-level short courses for coaches of young children, run by AIFF and state associations to a common syllabus."
        );
        assert_eq!(
            prose("Each state or UT association fields a team. Which registration or residence basis qualifies a player is NOT verified; the simulation's current default is in pack.toml [eligibility] and is a scenario setting."),
            "Each state or UT association fields a team."
        );
        assert_eq!(
            prose("Football-for-development NGOs coach children in many states. Placeholder for NGOs of the kind; specific organisations are not confirmed here."),
            "Football-for-development NGOs coach children in many states."
        );
        assert_eq!(prose("A plain sentence."), "A plain sentence.");
        assert_eq!(prose(""), "");
    }
}
