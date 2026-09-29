//! Small text helpers for server-side messages (the client formats data cells itself).

use pw_core::Date;

pub fn date(d: Date) -> String {
    let (y, m, day) = d.ymd();
    format!("{y}-{m:02}-{day:02}")
}
