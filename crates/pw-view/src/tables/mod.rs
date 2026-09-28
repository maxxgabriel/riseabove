//! Table sources, one per kind of list the client shows.

mod clubs;
mod comps;
mod events;
mod fixtures;
mod players;
mod stats;
mod staff;

use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, TableReq};
use crate::table::run;

pub use comps::{format_text, kind_text, visible_table};
pub use fixtures::{round_text, score_text};

pub fn comp_summary_stage(c: &Ctx, comp: pw_core::CompId) -> String {
    comps::stage_text(c, comp)
}

pub fn query(c: &Ctx, req: &TableReq) -> ApiResult<Value> {
    let resp = match req.table.as_str() {
        "players" => run(&players::Players, c, req),
        "staff" => run(&staff::StaffTable, c, req),
        "clubs" => run(&clubs::Clubs, c, req),
        "nations" => run(&clubs::Nations, c, req),
        "comps" => run(&comps::Comps, c, req),
        "standings" => run(&comps::Standings, c, req),
        "fixtures" => run(&fixtures::Fixtures, c, req),
        "player_stats" => run(&stats::PlayerSeasons, c, req),
        "comp_stats" => run(&stats::CompLeaders, c, req),
        "events" => run(&events::Events, c, req),
        "transfers" => run(&events::Transfers, c, req),
        "honours" => run(&events::Honours, c, req),
        "awards" => run(&events::Awards, c, req),
        "spells" => run(&events::Spells, c, req),
        other => return Err(ApiError::NotFound(format!("table {other}"))),
    };
    serde_json::to_value(resp).map_err(|e| ApiError::State(e.to_string()))
}
