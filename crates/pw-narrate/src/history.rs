//! Names and lines for history below the professional game (and, with the
//! record and award engines, for history at every level).

use pw_world::World;
use pw_world::minor::{Entrant, InstKind, Level, MinorKind, MinorSeason};

use crate::fmt::{nation, player};

pub fn entrant(w: &World, e: Entrant) -> String {
    match e {
        Entrant::Inst(i) => w.minor.institutions.get(i as usize).map_or_else(|| "?".into(), |x| x.name.clone()),
        Entrant::Local(l) => w.youth.local.get(l).map_or_else(|| "?".into(), |x| x.name.clone()),
    }
}

pub fn institution(w: &World, i: u32) -> String {
    w.minor.institutions.get(i as usize).map_or_else(|| "?".into(), |x| x.name.clone())
}

pub fn level(l: Level) -> &'static str {
    match l {
        Level::Grassroots => "grassroots",
        Level::School => "school",
        Level::University => "university",
        Level::Amateur => "amateur",
        Level::Youth => "youth",
        Level::Professional => "professional",
        Level::International => "international",
        Level::State => "state",
    }
}

/// "the Riverton and district Schools League", "the Norland Amateur League (tier 2)".
pub fn comp_name(w: &World, kind: MinorKind, n: pw_core::NationId, region: &str) -> String {
    let where_ = if region.is_empty() { nation(w, n) } else { region.to_string() };
    match kind {
        MinorKind::SchoolLeague => format!("{where_} Schools League"),
        MinorKind::SchoolCup => format!("{where_} Schools Cup"),
        MinorKind::UniversityLeague => format!("{where_} Universities League"),
        MinorKind::UniversityCup => format!("{where_} Inter-University Championship"),
        MinorKind::AmateurLeague { tier: 1 } => format!("{where_} Amateur League"),
        MinorKind::AmateurLeague { tier } => format!("{where_} Amateur League, division {tier}"),
        MinorKind::GrassrootsCup => format!("{where_} Junior Cup"),
    }
}

pub fn season_line(w: &World, s: &MinorSeason) -> String {
    let name = comp_name(w, s.kind, s.nation, &s.region);
    let mut out = format!("{} won the {name} {}/{:02}", entrant(w, s.winner), s.season, (s.season + 1) % 100);
    if s.top_scorer.is_some() {
        out.push_str(&format!("; top scorer {} ({})", player(w, s.top_scorer), s.top_goals));
    }
    if s.best.is_some() && s.best != s.top_scorer {
        out.push_str(&format!("; best player {}", player(w, s.best)));
    }
    out.push('.');
    out
}

pub fn inst_kind(k: InstKind) -> &'static str {
    match k {
        InstKind::School => "school",
        InstKind::University => "university",
    }
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

use pw_world::records::{Broken, Holder, Mark, RecordKey, Scope, Stat};

pub fn holder(w: &World, h: Holder) -> String {
    match h {
        Holder::Person(p) => crate::fmt::person(w, p),
        Holder::Club(c) => crate::fmt::club(w, c),
        Holder::Entrant(e) => entrant(w, e),
        Holder::Nation(n) => nation(w, n),
        Holder::Region(r) => w.ext.ecosystem.regions.get(r).map_or_else(|| "?".into(), |x| x.name.clone()),
        Holder::Past(f) => w.backfill.figures.get(f as usize).map_or_else(|| "?".into(), |x| x.name.clone()),
    }
}

fn scope(w: &World, s: Scope) -> String {
    match s {
        Scope::World => "the world".into(),
        Scope::Nation(n) => nation(w, n),
        Scope::Comp(c) => w.comps.get(c).map_or_else(|| "?".into(), |x| x.name.clone()),
        Scope::Club(c) => crate::fmt::club(w, c),
        Scope::Institution(i) => institution(w, i),
        Scope::Local(l) => entrant(w, Entrant::Local(l)),
        Scope::Minor(n, code) => comp_name(w, MinorKind::from_code(code), n, ""),
        Scope::Region(r) => w.ext.ecosystem.regions.get(r).map_or_else(|| "?".into(), |x| x.name.clone()),
        Scope::Event(_, code) => match code {
            1 => "Santosh Trophy".into(),
            2 => "National School Games".into(),
            _ => "the tournament".into(),
        },
    }
}

/// A record's value in words.
pub fn value(stat: Stat, v: i64) -> String {
    crate::fmt::singulars(stat.render(v))
}

/// "the Riverton High School all-time scoring record".
pub fn record_name(w: &World, k: RecordKey) -> String {
    let sc = scope(w, k.scope);
    let article = if sc.starts_with("the ") { "" } else { "the " };
    format!("{article}{sc} {}", k.stat.title())
}

fn stood(days: i32) -> Option<String> {
    match days {
        d if d >= 730 => Some(format!("{} years", d / 365)),
        d if d >= 60 => Some(format!("{} months", d / 30)),
        _ => None,
    }
}

/// A record falling, with its history — only what the book holds.
pub fn broken(w: &World, b: &Broken) -> String {
    let mut s = format!("{} set {} ({})", holder(w, b.new.holder), record_name(w, b.key), value(b.key.stat, b.new.value));
    if let Some(Mark { holder: h, value: v, .. }) = b.old {
        s.push_str(&format!(", surpassing {} ({})", holder(w, h), value(b.key.stat, v)));
        if let Some(t) = stood(b.stood_days) {
            s.push_str(&format!(", which had stood for {t}"));
        }
        if let Holder::Past(f) = h
            && w.backfill.figures.get(f as usize).is_some_and(|x| x.provenance == pw_world::backfill::Provenance::Generated)
        {
            s.push_str(" (a mark from this world's generated history)");
        }
    }
    if let Some(a) = b.new.against {
        s.push_str(&format!(" against {}", holder(w, a)));
    }
    s.push('.');
    s
}

/// Context for a professional record announced by `honours`: who held it
/// before and for how long, from the record book's silent mirror.
pub fn pro_context(w: &World, date: pw_core::Date, kind: pw_world::event::RecordKind, club: pw_core::ClubId) -> String {
    use pw_world::event::RecordKind as R;
    let (stat, scope) = match kind {
        R::ClubTopScorer => (Stat::Goals, Some(Scope::Club(club))),
        R::ClubMostApps => (Stat::Apps, Some(Scope::Club(club))),
        R::ClubRecordSigning => (Stat::FeePaid, Some(Scope::Club(club))),
        R::ClubRecordSale => (Stat::FeeReceived, Some(Scope::Club(club))),
        R::ClubBiggestWin => (Stat::BiggestWin, Some(Scope::Club(club))),
        R::WorldRecordFee => (Stat::FeePaid, Some(Scope::World)),
        R::NationMostCaps => (Stat::Caps, None),
        R::NationTopScorer => (Stat::IntlGoals, None),
        R::LeagueGoalsInSeason => return String::new(),
    };
    let Some(b) = w.records.broken.iter().rev().take(500).find(|b| !b.announced && b.new.date == date && b.key.stat == stat && scope.is_none_or(|s| s == b.key.scope)) else { return String::new() };
    let Some(old) = b.old else { return String::new() };
    let mut s = format!(" The previous mark was {} ({})", holder(w, old.holder), value(stat, old.value));
    if let Some(t) = stood(b.stood_days) {
        s.push_str(&format!(", set {t} earlier"));
    }
    s.push('.');
    s
}

// ---------------------------------------------------------------------------
// Votes, halls, the chronicle
// ---------------------------------------------------------------------------

use pw_world::awards::{Ballot, Feat, FirstTo, HallScope, Vote, Why};

pub fn why(w: Why) -> &'static str {
    match w {
        Why::Performances => "performances",
        Why::Trophies => "trophies",
        Why::Fame => "profile",
        Why::Goals => "goals",
        Why::Familiarity => "what they saw week in, week out",
        Why::Longevity => "longevity",
        Why::Loyalty => "loyalty",
        Why::International => "international career",
    }
}

pub fn ballot(w: &World, b: Ballot) -> String {
    match b {
        Ballot::WorldPlayer { young: false } => "World Player of the Year".into(),
        Ballot::WorldPlayer { young: true } => "World Young Player of the Year".into(),
        Ballot::PlayersPlayer { comp } => format!("{} Players' Player of the Season", w.comps.get(comp).map_or("?", |c| c.name.as_str())),
        Ballot::MinorPlayer { nation: n, university } => format!("{} {} Player of the Year", nation(w, n), if university { "University" } else { "Schools" }),
        Ballot::Hall { hall } => w.acclaim.halls.get(hall as usize).map_or_else(|| "a hall of fame".into(), |h| hall_name(w, h.scope)),
    }
}

pub fn hall_name(w: &World, s: HallScope) -> String {
    match s {
        HallScope::World => "the Hall of Fame".into(),
        HallScope::Nation(n) => format!("the {} Football Hall of Fame", nation(w, n)),
        HallScope::Club(c) => format!("the {} Hall of Fame", crate::fmt::club(w, c)),
        HallScope::Institution(i) => format!("the {} sports hall of fame", institution(w, i)),
    }
}

/// "X won the vote, named on 61 of 180 ballots, mostly for their goals."
pub fn vote(w: &World, v: &Vote, person_: pw_core::PersonId) -> String {
    let rank = v.result.iter().position(|x| x.0 == person_);
    let name = crate::fmt::person(w, person_);
    let what = ballot(w, v.ballot);
    let mut s = match rank {
        Some(0) => format!("{name} won {what}"),
        Some(r) => format!("{name} finished {} in the {what} vote", pw_world::event::ordinal(r as u8 + 1)),
        None => return format!("{name} was not named in the {what} vote."),
    };
    if !v.casts.is_empty() {
        s.push_str(&format!(", named on {} of {} ballots", v.named_by(person_), v.voters));
        if let Some(r) = v.main_reason(person_) {
            s.push_str(&format!(", mostly for {}", why(r)));
        }
    }
    s.push('.');
    s
}

pub fn chronicle(w: &World, e: &pw_world::awards::Entry) -> String {
    let club = |c| crate::fmt::club(w, c);
    let nth = |n: u16| if n == 0 { "the first".to_string() } else { format!("the {}", pw_world::event::ordinal((n + 1).min(255) as u8)) };
    match e.feat {
        Feat::Double { club: c, season } => {
            format!("{} won the league and cup double in {season}/{:02} — {} in {}'s history.", club(c), (season + 1) % 100, nth(e.before), nation(w, w.clubs[c].nation))
        }
        Feat::Unbeaten { club: c, comp, season } => {
            format!("{} went through the {} {season}/{:02} season unbeaten — {} to do it there.", club(c), w.comps.get(comp).map_or("?", |x| x.name.as_str()), (season + 1) % 100, nth(e.before))
        }
        Feat::FirstTournament { nation: n, tournament } => {
            let t = w.intl.tournaments.iter().find(|t| t.id == tournament).map_or(String::new(), |t| format!(" ({})", t.year));
            format!("{} won an international tournament for the first time{t}.", nation(w, n))
        }
        Feat::FirstWorldPlayerFrom { nation: n, person: p, year } => {
            format!("{} became the first player from {} to be named World Player of the Year ({year}).", crate::fmt::person(w, p), nation(w, n))
        }
        Feat::WorldPlayerAgain { person: p, times, year } => format!("{} was named World Player of the Year for the {} time ({year}).", crate::fmt::person(w, p), pw_world::event::ordinal(times)),
        Feat::FirstTo { person: p, what } => {
            let what = match what {
                FirstTo::SeniorApps(n) => format!("{n} senior appearances"),
                FirstTo::CareerGoals(n) => format!("{n} career goals"),
                FirstTo::Caps(n) => format!("{n} international caps"),
            };
            format!("{} became the first player in the world to reach {what}.", crate::fmt::person(w, p))
        }
        Feat::FirstTitle { club: c, comp, season } => {
            format!("{} won the {} for the first time in their history ({season}/{:02}).", club(c), w.comps.get(comp).map_or("?", |x| x.name.as_str()), (season + 1) % 100)
        }
    }
}

// ---------------------------------------------------------------------------
// Schools of thought and rule changes
// ---------------------------------------------------------------------------

use pw_world::evolution::{RuleCause, RuleChange, RuleKey, School};

/// "the Okafor school" — named after its founder.
pub fn school_name(w: &World, s: &School) -> String {
    let full = crate::fmt::person(w, s.founder);
    let surname = full.rsplit(' ').next().unwrap_or(&full).to_string();
    format!("the {surname} school")
}

/// Its principles in the football words of the year it was born.
pub fn school_style(s: &School) -> String {
    let v = crate::lexicon::Voice { era: crate::lexicon::Era::of_year(s.born.year()), ..crate::lexicon::Voice::neutral() };
    crate::grammar::style(&v, s.press, s.tempo, s.direct, u64::from(s.id))
}

pub fn school_founded(w: &World, s: &School) -> String {
    format!("{} has become a school of thought: {}, built on {}.", crate::fmt::person(w, s.founder), school_name(w, s), school_style(s))
}

pub fn rule_change(w: &World, c: &RuleChange) -> String {
    let n = nation(w, c.nation);
    let what = match c.key {
        RuleKey::Subs => format!("teams may now make {} substitutions (was {})", c.new, c.old),
        RuleKey::RedBan => format!("a straight red card now brings a {}-match ban (was {})", c.new, c.old),
        RuleKey::HomegrownMin => format!("squads must now include at least {} homegrown players (was {})", c.new, c.old),
        RuleKey::AwayGoals => "the away goals rule is abolished".to_string(),
    };
    let why = match c.cause {
        RuleCause::InjuryCrisis { per_club } => format!("after a season of {per_club:.0} injuries per club"),
        RuleCause::CardEpidemic { per_club } => format!("after {per_club:.1} suspensions per club last season"),
        RuleCause::NationalDecline { win_rate, homegrown } => {
            format!("with the national side winning {:.0}% of its games and {:.0}% of top-flight players homegrown", win_rate * 100.0, homegrown * 100.0)
        }
        RuleCause::AwayGoalsDebate { ties } => format!("after {ties} ties were decided on away goals"),
    };
    format!("The {n} federation announced that from {}/{:02} {what}, {why}.", c.from_season, (c.from_season + 1) % 100)
}
