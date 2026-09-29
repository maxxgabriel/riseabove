//! The living world: the daily pipeline (01 §4) and every system it runs.

/// Time each system of the daily pipeline, when `PW_PROFILE` is set.
macro_rules! prof {
    ($name:expr, $e:expr) => {{
        if crate::profile::enabled() {
            let t = std::time::Instant::now();
            let r = $e;
            crate::profile::record($name, t.elapsed().as_micros());
            r
        } else {
            $e
        }
    }};
}

pub mod affairs;
pub mod agents;
pub mod almanac;
pub mod audit;
pub mod awards;
pub mod backfill;
pub mod board;
pub mod boardroom;
pub mod commerce;
pub mod consider;
pub mod contracts;
pub mod culture;
pub mod deals;
pub mod decisions;
pub mod development;
pub mod dressing;
pub mod economy;
pub mod ecosystem;
pub mod evolution;
pub mod facts;
pub mod finance;
pub mod generate;
pub mod governance;
pub mod grapevine;
pub mod growth;
pub mod health;
pub mod honours;
pub mod hungarian;
pub mod inbox;
pub mod incidents;
pub mod intents;
pub mod interpret;
pub mod intl;
pub mod invariants;
pub mod life;
pub mod managers;
pub mod market;
pub mod matchday;
pub mod media;
pub mod medical;
pub mod mind;
pub mod minor;
pub mod morale;
pub mod negotiation;
pub mod newsroom;
pub mod officials;
pub mod pathway;
pub mod people;
pub mod perception;
pub mod planning;
pub mod press;
pub mod pressroom;
pub mod records;
pub mod renown;
pub mod reputation;
pub mod returns;
pub mod responses;
pub mod save;
pub mod schedule;
pub mod scouting;
pub mod season;
pub mod selection;
pub mod social;
pub mod socialnet;
pub mod stafflife;
pub mod staffing;
pub mod talk;
pub mod training;
pub mod university;
pub mod youth;

use pw_core::{DecisionId, Weekday};
use pw_world::World;
use rayon::prelude::*;

/// Timing of one simulated day, for profiling and the CLI.
#[derive(Clone, Copy, Debug, Default)]
pub struct DayStats {
    pub matches: usize,
    pub micros: u128,
}

/// Where simulated time goes, system by system (enabled by the `PW_PROFILE`
/// environment variable; off by default and free when off).
pub mod profile {
    use std::cell::RefCell;
    use std::sync::OnceLock;

    thread_local! {
        static TIMES: RefCell<Vec<(&'static str, u128, u32)>> = const { RefCell::new(Vec::new()) };
    }

    pub fn enabled() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var_os("PW_PROFILE").is_some())
    }

    pub fn record(name: &'static str, micros: u128) {
        TIMES.with(|t| {
            let mut t = t.borrow_mut();
            if let Some(x) = t.iter_mut().find(|x| x.0 == name) {
                x.1 += micros;
                x.2 += 1;
            } else {
                t.push((name, micros, 1));
            }
        });
    }

    /// (system, total microseconds, calls), slowest first; clears the tally.
    pub fn take() -> Vec<(&'static str, u128, u32)> {
        let mut v = TIMES.with(|t| std::mem::take(&mut *t.borrow_mut()));
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }
}

pub struct Sim {
    pub world: World,
}

impl Sim {
    /// Start simulating a world: a new world is prepared once; a loaded
    /// (already prepared) world continues exactly where it stopped.
    pub fn new(mut world: World) -> Self {
        if !world.prepared {
            prepare(&mut world);
            world.prepared = true;
        }
        Self { world }
    }

    /// Advance one day through the ordered pipeline. Nothing here knows or
    /// asks whether anyone in the world is controlled by a human (S1–S2).
    pub fn step(&mut self) -> DayStats {
        let t0 = std::time::Instant::now();
        let w = &mut self.world;
        let today = w.date;
        let monday = today.weekday() == Weekday::Mon;
        let first_of_month = today.day() == 1;

        // New people (regens, partners, staff) get a life the day they appear.
        prof!("life::sync", life::sync(w));

        // 1. Calendar: seasons, draws, contract expiries, loan ends, intakes.
        if today.month() == 12 && today.day() == 20 {
            prof!("honours::yearly_votes", honours::yearly_votes(w));
        }
        if today.month() == 7 && today.day() == 1 {
            prof!("officials::season_review", officials::season_review(w));
            prof!("ecosystem::yearly", ecosystem::yearly(w));
            prof!("university::yearly", university::yearly(w));
            prof!("evolution::yearly", evolution::yearly(w));
            prof!("economy::yearly", economy::yearly(w));
            prof!("governance::yearly", governance::yearly(w));
            prof!("managers::yearly", managers::yearly(w));
            prof!("commerce::ensure", commerce::ensure(w));
            prof!("commerce::yearly", commerce::yearly(w));
            prof!("culture::ensure", culture::ensure(w));
            prof!("culture::yearly", culture::yearly(w));
            prof!("newsroom::yearly", newsroom::yearly(w));
        }
        // People created by the yearly systems (referees, journalists) get lives.
        life::sync(w);
        prof!("season::daily", season::daily(w));
        prof!("contracts::daily", contracts::daily(w));
        prof!("people::daily", people::daily(w));

        // 2. What people decided to do (AI minds last week, humans today).
        prof!("intents::process", intents::process(w));

        // 3. Club management and the slow rhythms of life.
        if first_of_month {
            prof!("market::monthly", market::monthly(w));
            prof!("deals::shortlists", deals::shortlists(w));
            prof!("deals::monthly", deals::monthly(w));
            prof!("perception::monthly", perception::monthly(w));
            prof!("life::monthly", life::monthly(w));
            prof!("mind::monthly", mind::monthly(w));
            prof!("social::monthly", social::monthly(w));
            prof!("stafflife::monthly", stafflife::monthly(w));
            prof!("staffing::monthly", staffing::monthly(w));
            prof!("governance::monthly", governance::monthly(w));
            prof!("managers::monthly", managers::monthly(w));
            prof!("scouting::ensure", scouting::ensure(w));
            prof!("scouting::assign", scouting::assign(w));
            prof!("youth::school", youth::school(w));
            prof!("intl::ensure", intl::ensure(w));
            prof!("medical::monthly", medical::monthly(w));
            prof!("growth::monthly", growth::monthly(w));
            prof!("dressing::monthly", dressing::monthly(w));
            prof!("interpret::monthly", interpret::monthly(w));
            prof!("honours::monthly", honours::monthly(w));
            prof!("renown::monthly", renown::monthly(w));
            prof!("grapevine::compact", grapevine::compact(w));
            prof!("incidents::monthly", incidents::monthly(w));
            prof!("affairs::monthly", affairs::monthly(w));
            prof!("commerce::monthly", commerce::monthly(w));
            prof!("socialnet::monthly", socialnet::monthly(w));
            prof!("awards::scan", awards::scan(w));
            if today.month() == 1 {
                prof!("awards::inductions", awards::inductions(w));
            }
            if today.month() == 6 {
                prof!("minor::season_end", minor::season_end(w));
                prof!("awards::minor_players", awards::minor_players(w));
                prof!("youth::reviews", youth::reviews(w));
            }
            if today.month() == 9 {
                prof!("pathway::yearly", pathway::yearly(w));
                prof!("youth::yearly", youth::yearly(w));
            }
            vacancies(w);
            w.beliefs.forget(today.add_days(-240));
        }
        // Anyone created by the monthly systems (cohorts, staff) has a life.
        life::sync(w);
        if monday {
            prof!("training::weekly", training::weekly(w));
            prof!("board::weekly", board::weekly(w));
        }

        // 4–5. Training and health.
        let days = prof!("health::team_days", health::team_days(w, today));
        prof!("health::daily", health::daily(w, &days));

        // 6. Market and contract talks.
        prof!("market::daily", market::daily(w));
        if monday {
            prof!("contracts::weekly", contracts::weekly(w));
            prof!("market::weekly_loans", market::weekly_loans(w));
        }
        if today.weekday() == Weekday::Thu {
            prof!("market::free_agent_sweep", market::free_agent_sweep(w));
        }
        prof!("deals::daily", deals::daily(w));
        prof!("negotiation::daily", negotiation::daily(w));
        if monday {
            prof!("deals::recalls", deals::recalls(w));
            prof!("deals::pre_contracts", deals::pre_contracts(w));
            prof!("deals::trials", deals::trials(w));
        }

        // 7. Decisions due today (answered or defaulted), then conversations.
        prof!("decisions::resolve_due", decisions::resolve_due(w));
        prof!("talk::daily", talk::daily(w));

        // 7b. Incidents: postponements, travel, births, deferred decisions.
        prof!("incidents::daily", incidents::daily(w));

        // 8. Matches.
        let matches = w.fixtures.on(today).len();
        prof!("returns::pre_match", returns::pre_match(w));
        prof!("officials::pre_match", officials::pre_match(w));
        prof!("matchday::play_today", matchday::play_today(w));
        prof!("officials::daily", officials::daily(w));
        // National teams: windows, qualifiers, tournaments.
        prof!("intl::daily", intl::daily(w));

        // 8b. What people heard today, and whom they told; what got printed.
        prof!("grapevine::daily", grapevine::daily(w));
        prof!("pressroom::daily", pressroom::daily(w));
        prof!("newsroom::daily", newsroom::daily(w));
        prof!("socialnet::persons_post", socialnet::persons_post(w));
        prof!("socialnet::daily", socialnet::daily(w));
        prof!("inbox::daily", inbox::daily(w));

        // 9. Aftermath (weekly systems run after the weekend's games).
        life::sync(w);
        if monday {
            prof!("development::weekly", development::weekly(w));
            prof!("medical::weekly", medical::weekly(w));
            prof!("grapevine::feelings", grapevine::feelings(w));
            prof!("incidents::weekly", incidents::weekly(w));
            prof!("newsroom::weekly", newsroom::weekly(w));
            prof!("socialnet::weekly", socialnet::weekly(w));
            prof!("dressing::weekly", dressing::weekly(w));
            prof!("perception::weekly", perception::weekly(w));
            prof!("social::weekly", social::weekly(w));
            prof!("morale::weekly", morale::weekly(w));
            prof!("talk::manager_summons", talk::manager_summons(w));
            prof!("mind::weekly", mind::weekly(w));
            prof!("youth::weekly", youth::weekly(w));
            prof!("minor::weekly", minor::weekly(w));
            prof!("agents::weekly", agents::weekly(w));
            prof!("media::weekly", media::weekly(w));
            prof!("reputation::weekly", reputation::weekly(w));
            prof!("finance::weekly", finance::weekly(w));
        }

        // 12. Archive.
        if first_of_month && today.month() == 8 {
            compact(w);
            prof!("staffing::yearly_growth", staffing::yearly_growth(w));
        }
        w.date = today.add_days(1);
        w.days_simulated += 1;
        DayStats { matches, micros: t0.elapsed().as_micros() }
    }

    pub fn run(&mut self, days: u32) {
        for _ in 0..days {
            self.step();
        }
    }

    /// Advance until a pending decision needs an external mind, or `max_days` pass.
    pub fn run_until_decision(&mut self, max_days: u32) -> Option<DecisionId> {
        for _ in 0..max_days {
            self.step();
            let pending = self.world.decisions.all.iter_enumerated().find(|(_, d)| !d.resolved && d.answer.is_none()).map(|(id, _)| id);
            if pending.is_some() {
                return pending;
            }
        }
        None
    }

    pub fn answer(&mut self, id: DecisionId, choice: u8) -> bool {
        self.world.decisions.answer(id, choice)
    }
}

/// Bring freshly imported or loaded worlds into a consistent state.
pub fn prepare(w: &mut World) {
    w.knowledge.resize(w.clubs.len());
    life::sync(w);
    economy::ensure(w);
    governance::ensure(w);
    managers::ensure(w);
    scouting::ensure(w);
    youth::ensure(w);
    ecosystem::yearly(w);
    minor::ensure(w);
    officials::ensure(w);
    intl::ensure(w);
    commerce::ensure(w);
    culture::ensure(w);
    backfill::generate(w);
    newsroom::ensure_profiles(w);
    socialnet::ensure(w);
    let weights = w.data.weights.clone();
    w.players.cold.par_iter_mut().for_each(|c| c.refresh_ca(&weights));
    for t in w.teams.ids() {
        let club = w.teams[t].club;
        for i in 0..w.teams[t].squad.len() {
            let p = w.teams[t].squad[i];
            w.knowledge.observe(club, p, 900, w.date);
        }
    }
    vacancies(w);
    market::monthly(w);
    agents::ensure_market(w);
    media::ensure_media(w);
    life::sync(w);
    // Everyone starts the world with a plan for their training and their week.
    mind::monthly(w);
}

fn vacancies(w: &mut World) {
    let empty: Vec<_> = w.clubs.iter_enumerated().filter(|(_, c)| c.manager.is_none()).map(|(id, _)| id).collect();
    for c in empty {
        board::appoint(w, c);
    }
}

/// Yearly: drop old fixtures and events, keep history (01 §8 compaction).
fn compact(w: &mut World) {
    let cutoff = w.date.add_days(-400);
    w.fixtures.compact(cutoff);
    use pw_world::event::EventKind as E;
    w.events.compact(cutoff, |e| {
        // History people and the press will keep quoting.
        matches!(
            e.kind,
            E::Transfer { .. }
                | E::Champion { .. }
                | E::Award { .. }
                | E::Debut { .. }
                | E::Retired { .. }
                | E::ManagerAppointed { .. }
                | E::Meeting { .. }
                | E::PromiseMade { .. }
                | E::PromiseKept { .. }
                | E::PromiseBroken { .. }
                | E::TransferRequested { .. }
                | E::Published { .. }
                | E::Life { .. }
                | E::JoinedStaff { .. }
        )
    });
    let keep: std::collections::HashSet<u64> = w.fixtures.iter().map(|(_, f)| f.uid).collect();
    let external: Vec<_> = w.external_players().collect();
    // Recording level of detail only (S2): full reports are kept for matches
    // involving someone a human inhabits; outcomes never depended on it.
    w.reports.retain(|uid, r| keep.contains(uid) || external.iter().any(|&p| r.line(p).is_some()));
    let today = w.date;
    w.social.prune(today, today.add_days(-3 * 365));
    w.ext.decisions.compact(today.add_days(-2 * 365));
}
