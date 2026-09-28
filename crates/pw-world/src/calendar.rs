use pw_core::Date;
use pw_data::{CalendarDef, DateSpan, MonthDay};
use smallvec::SmallVec;

use crate::nation::NationSeason;

#[inline]
pub fn md(year: i32, m: MonthDay) -> Date {
    Date::from_ymd(year, u32::from(m.0), u32::from(m.1))
}

/// Resolve a span that may wrap the new year, anchored at `year` for its start.
pub fn span(year: i32, s: DateSpan) -> (Date, Date) {
    let end_year = if s.end < s.start { year + 1 } else { year };
    (md(year, s.start), md(end_year, s.end))
}

/// Concrete dates for the season starting in `year`.
pub fn season(cal: &CalendarDef, year: i32) -> NationSeason {
    let start = md(year, cal.season_start);
    let end = md(if cal.crosses_year() { year + 1 } else { year }, cal.season_end);
    let mut windows: SmallVec<[(Date, Date); 2]> = SmallVec::new();
    for &w in &cal.windows {
        // Anchor each window to the occurrence that overlaps this season: the
        // pre-season window opens before `start`; midseason ones fall inside.
        let (mut a, mut b) = span(year, w);
        if b < start.add_days(-120) {
            (a, b) = span(year + 1, w);
        }
        if a > end {
            (a, b) = span(year - 1, w);
        }
        windows.push((a, b));
    }
    windows.sort();
    let winter_break = cal.winter_break.map(|w| span(year, w));
    NationSeason { year, start, end, windows, winter_break }
}

/// The season a nation is in (or about to start) on `today`.
pub fn current_season(cal: &CalendarDef, today: Date) -> NationSeason {
    let y = today.year();
    let this = season(cal, y);
    if today < this.start.add_days(-60) {
        let prev = season(cal, y - 1);
        if today <= prev.end.add_days(30) {
            return prev;
        }
    }
    this
}

pub fn in_international_window(windows: &[DateSpan], d: Date) -> bool {
    let y = d.year();
    windows.iter().any(|&w| {
        let (a, b) = span(y, w);
        d >= a && d <= b
    })
}
