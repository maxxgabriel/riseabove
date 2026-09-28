//! The communication calendar (Z in the brief): things that are due later —
//! a follow-up on a running story in three days, analysis the morning after
//! a big match, a correction once the truth is known, a wave of supporter
//! reaction an hour after full time. The simulation ticks daily; each task
//! carries a minute of the day so that, within a day, first reactions come
//! before first reports, and reports before the analysis.

use pw_core::{Date, StoryId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Task {
    /// Check a running story for developments.
    FollowUp { thread: u32 },
    /// A considered piece after the first reports.
    Analysis { story: StoryId },
    /// Someone (agent, club) may deny a story.
    Denial { story: StoryId },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Pending {
    pub due: Date,
    /// Minute of the day, 0–1439.
    pub minute: u16,
    pub task: Task,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Agenda {
    pub items: Vec<Pending>,
}

impl Agenda {
    pub fn schedule(&mut self, due: Date, minute: u16, task: Task) {
        if self.items.iter().any(|p| p.task == task && p.due == due) {
            return;
        }
        self.items.push(Pending { due, minute: minute.min(1439), task });
    }

    /// Everything due on or before `today`, in time order.
    pub fn take_due(&mut self, today: Date) -> Vec<Pending> {
        let (mut due, rest): (Vec<Pending>, Vec<Pending>) = std::mem::take(&mut self.items).into_iter().partition(|p| p.due <= today);
        self.items = rest;
        due.sort_by(|a, b| a.due.cmp(&b.due).then(a.minute.cmp(&b.minute)));
        due
    }
}
