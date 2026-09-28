//! The living world: the daily pipeline (01 §4) and every system it runs.

pub mod board;
pub mod contracts;
pub mod decisions;
pub mod development;
pub mod finance;
pub mod generate;
pub mod health;
pub mod hungarian;
pub mod market;
pub mod matchday;
pub mod morale;
pub mod people;
pub mod perception;
pub mod reputation;
pub mod save;
pub mod schedule;
pub mod season;
pub mod selection;

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

    /// Advance one day through the ordered pipeline.
    pub fn step(&mut self) -> DayStats {
        let t0 = std::time::Instant::now();
        let w = &mut self.world;
        let today = w.date;
        let monday = today.weekday() == Weekday::Mon;
        let first_of_month = today.day() == 1;

        // 1. Calendar: seasons, draws, contract expiries, loan ends.
        season::daily(w);
        contracts::daily(w);
        people::daily(w);

        // 3. Club management.
        if first_of_month {
            market::monthly(w);
            perception::monthly(w);
            morale::monthly_life(w);
            vacancies(w);
        }
        if monday {
            board::weekly(w);
        }

        // 4–5. Training and health.
        let days = health::team_days(w, today);
        health::daily(w, &days);

        // 6. Market.
        market::daily(w);
        if monday {
            contracts::weekly(w);
            market::weekly_loans(w);
        }
        if today.weekday() == Weekday::Thu {
            market::free_agent_sweep(w);
        }

        // 7. Decisions due today (answered or defaulted).
        decisions::resolve_due(w);

        // 8. Matches.
        let matches = w.fixtures.on(today).len();
        matchday::play_today(w);

        // 9. Aftermath (weekly systems run after the weekend's games).
        if monday {
            development::weekly(w);
            perception::weekly(w);
            morale::weekly(w);
            reputation::weekly(w);
            finance::weekly(w);
        }

        // 12. Archive.
        if first_of_month && today.month() == 8 {
            compact(w);
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
        matches!(e.kind, E::Transfer { .. } | E::Champion { .. } | E::Award { .. } | E::Debut { .. } | E::Retired { .. } | E::ManagerAppointed { .. })
    });
    let keep: std::collections::HashSet<u64> = w.fixtures.iter().map(|(_, f)| f.uid).collect();
    let external: Vec<_> = w.external_players().collect();
    w.reports.retain(|uid, r| keep.contains(uid) || external.iter().any(|&p| r.line(p).is_some()));
    w.decisions.all.iter_mut().for_each(|_| {});
}
