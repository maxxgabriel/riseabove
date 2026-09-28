//! Table sources, one per kind of list the client shows.

mod clubs;
mod comps;
mod events;
mod fixtures;
mod players;
mod society;
mod stats;
mod staff;
mod systems;

use serde_json::Value;

use crate::ctx::Ctx;
use crate::model::{ApiError, ApiResult, TableReq};
use crate::table::run;

pub use comps::{format_text, kind_text, visible_table};
pub use fixtures::{round_text, score_text};
pub use systems::ownership_label;

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
        "stories" => run(&systems::Stories, c, req),
        "agents" => run(&systems::AgentsTable, c, req),
        "talks" => run(&systems::Talks, c, req),
        "bids" => run(&systems::Bids, c, req),
        "intl_matches" => run(&systems::IntlMatches, c, req),
        "tournaments" => run(&systems::Tournaments, c, req),
        "boards" => run(&systems::Boards, c, req),
        "sponsors" => run(&systems::Sponsors, c, req),
        "posts" => run(&society::Posts, c, req),
        "chants" => run(&society::chants(), c, req),
        "memes" => run(&society::memes(), c, req),
        "groups" => run(&society::groups(), c, req),
        "rivalries" => run(&society::rivalries(), c, req),
        "incidents" => run(&society::Incidents, c, req),
        "conferences" => run(&society::Conferences, c, req),
        "quotes" => run(&society::Quotes, c, req),
        "referees" => run(&society::referees(), c, req),
        "controversies" => run(&society::controversies(), c, req),
        "charges" => run(&society::charges(), c, req),
        "record_book" => run(&society::record_book(), c, req),
        "records_broken" => run(&society::records_broken(), c, req),
        "votes" => run(&society::votes(), c, req),
        "hall_members" => run(&society::hall_members(), c, req),
        "chronicle" => run(&society::chronicle(), c, req),
        "schools" => run(&society::schools(), c, req),
        "rule_changes" => run(&society::rule_changes(), c, req),
        "institutions" => run(&society::institutions(), c, req),
        "minor_seasons" => run(&society::minor_seasons(), c, req),
        "outlets" => run(&society::outlets(), c, req),
        "journalists" => run(&society::journalists(), c, req),
        "grapevine" => run(&society::grapevine(), c, req),
        other => return Err(ApiError::NotFound(format!("table {other}"))),
    };
    serde_json::to_value(resp).map_err(|e| ApiError::State(e.to_string()))
}
