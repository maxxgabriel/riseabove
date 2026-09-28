//! The living world: the daily pipeline (01 §4) and every system it runs.

pub mod agents;
pub mod board;
pub mod consider;
pub mod contracts;
pub mod deals;
pub mod decisions;
pub mod development;
pub mod economy;
pub mod finance;
pub mod generate;
pub mod governance;
pub mod health;
pub mod hungarian;
pub mod intents;
pub mod life;
pub mod managers;
pub mod market;
pub mod matchday;
pub mod media;
pub mod mind;
pub mod morale;
pub mod negotiation;
pub mod people;
pub mod perception;
pub mod planning;
pub mod reputation;
pub mod save;
pub mod schedule;
pub mod scouting;
pub mod season;
pub mod selection;
pub mod social;
pub mod staffing;
pub mod talk;
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

pub struct Sim {
    pub world: World,
}

impl Sim {
    pub fn new(mut world: World) -> Self {
        prepare(&mut world);
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
        life::sync(w);

        // 1. Calendar: seasons, draws, contract expiries, loan ends, intakes.
        if today.month() == 7 && today.day() == 1 {
            economy::yearly(w);
            governance::yearly(w);
            managers::yearly(w);
        }
        season::daily(w);
        contracts::daily(w);
        people::daily(w);

        // 2. What people decided to do (AI minds last week, humans today).
        intents::process(w);

        // 3. Club management and the slow rhythms of life.
        if first_of_month {
            market::monthly(w);
            deals::shortlists(w);
            deals::monthly(w);
            perception::monthly(w);
            life::monthly(w);
            mind::monthly(w);
            social::monthly(w);
            staffing::monthly(w);
            governance::monthly(w);
            managers::monthly(w);
            scouting::ensure(w);
            scouting::assign(w);
            youth::school(w);
            if today.month() == 6 {
                youth::reviews(w);
            }
            if today.month() == 9 {
                youth::yearly(w);
            }
            vacancies(w);
            w.beliefs.forget(today.add_days(-240));
        }
        if monday {
            board::weekly(w);
        }

        // 4–5. Training and health.
        let days = health::team_days(w, today);
        health::daily(w, &days);

        // 6. Market and contract talks.
        market::daily(w);
        if monday {
            contracts::weekly(w);
            market::weekly_loans(w);
        }
        if today.weekday() == Weekday::Thu {
            market::free_agent_sweep(w);
        }
        deals::daily(w);
        negotiation::daily(w);
        if monday {
            deals::recalls(w);
            deals::pre_contracts(w);
            deals::trials(w);
        }

        // 7. Decisions due today (answered or defaulted), then conversations.
        decisions::resolve_due(w);
        talk::daily(w);

        // 8. Matches.
        let matches = w.fixtures.on(today).len();
        matchday::play_today(w);

        // 9. Aftermath (weekly systems run after the weekend's games).
        if monday {
            development::weekly(w);
            perception::weekly(w);
            social::weekly(w);
            morale::weekly(w);
            talk::manager_summons(w);
            mind::weekly(w);
            youth::weekly(w);
            agents::weekly(w);
            media::weekly(w);
            reputation::weekly(w);
            finance::weekly(w);
        }

        // 12. Archive.
        if first_of_month && today.month() == 8 {
            compact(w);
            staffing::yearly_growth(w);
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
}
