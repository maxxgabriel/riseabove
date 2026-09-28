use serde::{Deserialize, Serialize};

/// A calendar day, stored as days since 1970-01-01 (proleptic Gregorian).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Date(pub i32);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    pub const fn index(self) -> u32 {
        self as u32
    }
}

impl Date {
    pub fn from_ymd(y: i32, m: u32, d: u32) -> Self {
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy as i32;
        Date(era * 146_097 + doe - 719_468)
    }

    pub fn ymd(self) -> (i32, u32, u32) {
        let z = self.0 + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = yoe + era * 400 + i32::from(m <= 2);
        (y, m, d)
    }

    #[inline]
    pub fn year(self) -> i32 {
        self.ymd().0
    }

    #[inline]
    pub fn month(self) -> u32 {
        self.ymd().1
    }

    #[inline]
    pub fn day(self) -> u32 {
        self.ymd().2
    }

    #[inline]
    pub fn weekday(self) -> Weekday {
        const DAYS: [Weekday; 7] = [Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri, Weekday::Sat, Weekday::Sun];
        DAYS[(self.0 + 3).rem_euclid(7) as usize]
    }

    #[inline]
    pub const fn add_days(self, n: i32) -> Self {
        Date(self.0 + n)
    }

    #[inline]
    pub const fn days_until(self, later: Date) -> i32 {
        later.0 - self.0
    }

    /// Same month/day in `year`, clamping 29 Feb to 28 Feb in common years.
    pub fn with_year(self, year: i32) -> Self {
        let (_, m, d) = self.ymd();
        let d = if m == 2 && d == 29 && !is_leap(year) { 28 } else { d };
        Date::from_ymd(year, m, d)
    }

    pub fn add_months(self, n: i32) -> Self {
        let (y, m, d) = self.ymd();
        let total = y * 12 + (m as i32 - 1) + n;
        let (ny, nm) = (total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
        Date::from_ymd(ny, nm, d.min(days_in_month(ny, nm)))
    }

    /// Whole years of age on `today`. A 29 Feb birthday ages on 1 March in common years.
    pub fn age_on(self, today: Date) -> u32 {
        let (by, bm, bd) = self.ymd();
        let (ty, tm, td) = today.ymd();
        let mut age = ty - by;
        if (tm, td) < (bm, bd) {
            age -= 1;
        }
        age.max(0) as u32
    }

    /// Fractional age in years, for growth curves.
    #[inline]
    pub fn age_years(self, today: Date) -> f32 {
        (today.0 - self.0) as f32 / 365.2425
    }

    pub fn next_weekday(self, wd: Weekday) -> Self {
        let delta = (wd.index() as i32 - self.weekday().index() as i32).rem_euclid(7);
        self.add_days(delta)
    }
}

pub fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(y) => 29,
        _ => 28,
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (y, m, d) = self.ymd();
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

impl std::fmt::Debug for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_weekday() {
        for days in -800_000..800_000 {
            let d = Date(days);
            let (y, m, dd) = d.ymd();
            assert_eq!(Date::from_ymd(y, m, dd), d);
        }
        assert_eq!(Date::from_ymd(1970, 1, 1).weekday(), Weekday::Thu);
        assert_eq!(Date::from_ymd(2026, 9, 28).weekday(), Weekday::Mon);
    }

    #[test]
    fn leap_birthday_ages_on_first_march() {
        let dob = Date::from_ymd(2004, 2, 29);
        assert_eq!(dob.age_on(Date::from_ymd(2023, 2, 28)), 18);
        assert_eq!(dob.age_on(Date::from_ymd(2023, 3, 1)), 19);
    }
}
