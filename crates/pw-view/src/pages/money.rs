//! `me.money`: what a wage becomes in a life. Payslips month by month (wage and other income, tax, living, what went home, what was
//! left), bonuses as they were paid, and a few plain sentences that turn the numbers into rent, family and months of savings. Read
//! from the personal ledger (`pw_world::ledger`), which is kept from the day a life is inhabited.

use pw_core::Money;
use pw_world::ledger::{Entry, Paid};
use serde_json::Value;

use crate::contract::{BonusRow, MoneyView, PayslipRow, TrainingLogView, TrainingWeekRow};
use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, Named, Part, Ref};

fn paid_words(p: Paid) -> String {
    match p {
        Paid::Match { goals, assists, clean } => {
            let mut bits = vec!["Appearance".to_string()];
            if goals > 0 {
                bits.push(format!("{goals} goal{}", if goals > 1 { "s" } else { "" }));
            }
            if assists > 0 {
                bits.push(format!("{assists} assist{}", if assists > 1 { "s" } else { "" }));
            }
            if clean {
                bits.push("clean sheet".into());
            }
            bits.join(", ")
        }
        Paid::Cap => "International cap".into(),
        Paid::Continental => "Qualifying for continental football".into(),
        Paid::Title => "Winning the title".into(),
        Paid::Promotion => "Promotion".into(),
        Paid::Loyalty => "Loyalty, a year on from signing".into(),
    }
}

pub fn money(c: &Ctx) -> ApiResult<Value> {
    let me = c.me().ok_or_else(|| ApiError::Unauthorized("You are observing the world; there is no payslip to read.".into()))?;
    let w = c.w;
    let p = w.people[me].player;
    let per_week = if p.is_some() && w.players.cold[p].contract.club.is_some() { w.players.cold[p].contract.current_wage(w.date) } else { 0 };
    let lines = w.ext.ledger.of.get(&me).map_or(&[][..], |v| v.as_slice());
    let mut months: Vec<PayslipRow> = Vec::new();
    let mut bonuses: Vec<BonusRow> = Vec::new();
    for l in lines {
        match l.entry {
            Entry::Payslip { wage, other, tax, net, living, family } => {
                // Bonuses paid since the previous payslip belong to this month.
                let since = months.last().map_or(pw_core::Date(i32::MIN), |m| pw_core::Date(m.date));
                let extra: Money = lines.iter().filter(|b| b.date > since && b.date <= l.date).filter_map(|b| if let Entry::Bonus { kept, .. } = b.entry { Some(kept) } else { None }).sum();
                months.push(PayslipRow { date: l.date.0, wage, other, tax, net, living, family, left: net - living - family, bonuses: extra });
            }
            Entry::Bonus { paid, gross, kept } => bonuses.push(BonusRow { date: l.date.0, what: paid_words(paid), gross, kept }),
        }
    }
    months.reverse();
    bonuses.reverse();
    bonuses.truncate(60);

    // What it means.
    let mut meaning: Vec<String> = Vec::new();
    if let Some(m) = months.first() {
        if m.net > 0 {
            let pct = |x: Money| ((x as f64 / m.net as f64) * 100.0).round() as i64;
            meaning.push(format!("Of what reaches your account each month, living takes about {}%.", pct(m.living)));
            if m.family > 0 {
                meaning.push(format!("You send about {}% home to your family.", pct(m.family)));
            }
            if m.left < 0 {
                meaning.push("You are spending more than you take home; the difference comes out of your savings.".into());
            }
        }
        if m.tax > 0 && m.wage + m.other > 0 {
            meaning.push(format!("Tax takes about {}% of your pay.", ((m.tax as f64 / (m.wage + m.other) as f64) * 100.0).round() as i64));
        }
    }
    if let Some(l) = w.lives.get(me) {
        let f = &l.finances;
        if f.spending > 0 && f.savings > 0 {
            let months_left = f.savings / f.spending.max(1);
            meaning.push(match months_left {
                0 => "Your savings would not last a month without pay.".to_string(),
                1..=11 => format!("Your savings would carry you about {months_left} month{} without pay.", if months_left == 1 { "" } else { "s" }),
                _ => format!("Your savings would carry you about {} years without pay.", months_left / 12),
            });
        }
        if f.debt > 0 {
            meaning.push("You are carrying debt.".into());
        }
    }
    if !bonuses.is_empty() {
        let kept: Money = bonuses.iter().map(|b| b.kept).sum();
        let gross: Money = bonuses.iter().map(|b| b.gross).sum();
        if gross > 0 {
            meaning.push(format!("Of the bonuses the club paid, about {}% reached you after tax and your agent's share.", ((kept as f64 / gross as f64) * 100.0).round() as i64));
        }
    }
    serde_json::to_value(MoneyView { per_week, months, bonuses, meaning }).map_err(|e| ApiError::Internal(e.to_string()))
}

/// `me.training`: the training ground, week by week (`pw_world::trainlog`), newest first.
pub fn training(c: &Ctx) -> ApiResult<Value> {
    use pw_world::trainlog::{Group, Trace, Week};
    let me = c.me().ok_or_else(|| ApiError::Unauthorized("You are observing the world; there is no training week to read.".into()))?;
    let w = c.w;
    let who = |p: pw_core::PersonId| if p.is_some() && w.people.get(p).is_some() { Part::l(Ref::person(p), c.person_name(p)) } else { Part::t("Someone") };
    let weeks: Vec<TrainingWeekRow> = w
        .ext
        .trainlog
        .of
        .get(&me)
        .map(|v| v.as_slice())
        .unwrap_or(&[])
        .iter()
        .rev()
        .take(26)
        .map(|wk| TrainingWeekRow {
            date: wk.date.0,
            group: match wk.group {
                Group::FirstTeam => "With the first-team group",
                Group::Partial => "Back for parts of sessions with the group",
                Group::Rehab => "Inside with the physio",
            }
            .into(),
            week: match wk.week {
                Week::Sharp => "A sharp week: you stood out",
                Week::Ordinary => "An ordinary week",
                Week::Flat => "A flat week: it did not come off",
            }
            .into(),
            coach: (wk.coach.is_some() && w.people.get(wk.coach).is_some()).then(|| Named::new(Ref::person(wk.coach), c.person_name(wk.coach))),
            traces: wk
                .traces
                .iter()
                .map(|t| match *t {
                    Trace::PulledAside { by, good: true } => vec![who(by), Part::t(" took you aside to say your work had been noticed")],
                    Trace::PulledAside { by, good: false } => vec![who(by), Part::t(" took you aside about your training")],
                    Trace::Flying { who: q } => vec![who(q), Part::t(" was flying all week")],
                    Trace::OffThePace { who: q } => vec![who(q), Part::t(" looked off the pace")],
                    Trace::StayedBehind { who: q } => vec![who(q), Part::t(" stayed behind for extra work")],
                    Trace::Fined { who: q } => vec![who(q), Part::t(" was fined by the club")],
                    Trace::BackInTraining { who: q } => vec![who(q), Part::t(" trained with the group again after injury")],
                    Trace::Arrived { who: q } => vec![who(q), Part::t(" joined the squad")],
                })
                .collect(),
        })
        .collect();
    serde_json::to_value(TrainingLogView { weeks }).map_err(|e| ApiError::Internal(e.to_string()))
}
