//! The wall clock and the time zone the original reads through `java.util.Calendar`: new
//! calendars start at "now" in the default time zone, and the game only changes their date
//! fields, so its dates carry the time of day they were created at.

use jiff::civil::{Date, DateTime};
use jiff::tz::{Disambiguation, TimeZone};
use jiff::Timestamp;

/// A date's fields as `Calendar.get` returns them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateFields {
    pub year: i32,
    /// `Calendar.MONTH`: 0 for January.
    pub month: i32,
    pub day: i32,
    /// `Calendar.DAY_OF_WEEK`: 1 for Sunday to 7 for Saturday.
    pub weekday: i32,
}

#[derive(Clone, Debug)]
pub struct Clock {
    /// A fixed "now" (the Java harness's `GregorianCalendar.bfSetNow`), or the system clock.
    fixed_now: Option<i64>,
    zone: TimeZone,
}

impl Default for Clock {
    fn default() -> Self {
        Self::system()
    }
}

impl PartialEq for Clock {
    fn eq(&self, other: &Self) -> bool {
        self.fixed_now == other.fixed_now && self.zone.iana_name() == other.zone.iana_name()
    }
}

impl Clock {
    /// The computer's clock and time zone, as the game sees them.
    pub fn system() -> Self {
        Clock { fixed_now: None, zone: TimeZone::system() }
    }

    /// A stopped clock in a named zone, for the differential tests.
    pub fn fixed(now_millis: i64, zone: &str) -> Self {
        Clock { fixed_now: Some(now_millis), zone: TimeZone::get(zone).expect("a known time zone") }
    }

    /// `System.currentTimeMillis()`
    pub fn now_millis(&self) -> i64 {
        self.fixed_now.unwrap_or_else(|| Timestamp::now().as_millisecond())
    }

    fn local(&self, millis: i64) -> DateTime {
        self.zone.to_datetime(Timestamp::from_millisecond(millis).expect("a representable instant"))
    }

    /// `Calendar.get(YEAR, MONTH, DAY_OF_MONTH, DAY_OF_WEEK)` of an instant.
    pub fn fields(&self, millis: i64) -> DateFields {
        let local = self.local(millis);
        DateFields {
            year: local.year() as i32,
            month: local.month() as i32 - 1,
            day: local.day() as i32,
            weekday: local.weekday().to_sunday_one_offset() as i32,
        }
    }

    /// `Calendar.get(DAY_OF_YEAR)`: 1 for the 1st of January.
    pub fn day_of_year(&self, millis: i64) -> i32 {
        self.local(millis).day_of_year() as i32
    }

    /// A clone of a calendar at `millis` with its date set (`set(YEAR..DAY_OF_MONTH)`): the
    /// same time of day.
    pub fn date_at(&self, millis: i64, year: i32, month: i32, day: i32) -> i64 {
        let time = self.local(millis).time();
        let date = Date::new(year as i16, (month + 1) as i8, day as i8).expect("a valid date");
        let zoned = self.zone.to_ambiguous_zoned(date.to_datetime(time)).disambiguate(Disambiguation::Later).expect("a representable date");
        zoned.timestamp().as_millisecond()
    }

    /// A clone of a calendar at `millis` with `add(DAY_OF_MONTH, days)`: the same time of day.
    pub fn add_days(&self, millis: i64, days: i32) -> i64 {
        let date = self.local(millis).date().checked_add(jiff::Span::new().days(days as i64)).expect("a representable date");
        self.date_at(millis, date.year() as i32, date.month() as i32 - 1, date.day() as i32)
    }

    /// `Calendar.getInstance()`, `clear()`, `set(year, month, day)`: the local midnight.
    pub fn midnight(&self, year: i32, month: i32, day: i32) -> i64 {
        let date = Date::new(year as i16, (month + 1) as i8, day as i8).expect("a valid date");
        let zoned = self.zone.to_ambiguous_zoned(date.to_datetime(jiff::civil::Time::midnight())).disambiguate(Disambiguation::Later).expect("a representable date");
        zoned.timestamp().as_millisecond()
    }

    /// A clone of a calendar at `millis` with `set(DAY_OF_YEAR, day)`.
    pub fn with_day_of_year(&self, millis: i64, day: i32) -> i64 {
        let year = self.local(millis).year();
        let date = Date::new(year, 1, 1).expect("a valid year").checked_add(jiff::Span::new().days((day - 1) as i64)).expect("a day of the year");
        self.date_at(millis, date.year() as i32, date.month() as i32 - 1, date.day() as i32)
    }

    /// `Calendar.getInstance()` then `set(year, month, day)`: the date at the clock's local
    /// time of day. A time skipped by a change of offset moves forward; a repeated one takes
    /// the later instant, like `GregorianCalendar`.
    pub fn date_now(&self, year: i32, month: i32, day: i32) -> i64 {
        let time = self.local(self.now_millis()).time();
        let date = Date::new(year as i16, (month + 1) as i8, day as i8).expect("a valid date");
        let zoned = self
            .zone
            .to_ambiguous_zoned(date.to_datetime(time))
            .disambiguate(Disambiguation::Later)
            .expect("a representable date");
        zoned.timestamp().as_millisecond()
    }

    /// The dates of a year, in order, as `f.r(int)` walks them.
    /// `Calendar.getActualMaximum(DAY_OF_MONTH)` for a month (0 to 11).
    pub fn days_in_month(&self, year: i32, month: i32) -> i32 {
        match month {
            1 => {
                if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 { 29 } else { 28 }
            }
            3 | 5 | 8 | 10 => 30,
            _ => 31,
        }
    }

    pub fn days_of_year(year: i32) -> impl Iterator<Item = (i32, i32, i32, i32)> {
        let first = Date::new(year as i16, 1, 1).expect("a valid year");
        first.series(jiff::Span::new().days(1)).take_while(move |d| d.year() as i32 == year).map(|d| {
            (d.year() as i32, d.month() as i32 - 1, d.day() as i32, d.weekday().to_sunday_one_offset() as i32)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_keep_the_time_of_day_of_now() {
        // 2026-10-20 10:30:15.250 in Sao Paulo (UTC-3)
        let clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
        let millis = clock.date_now(2023, 0, 1);
        let fields = clock.fields(millis);
        assert_eq!((fields.year, fields.month, fields.day, fields.weekday), (2023, 0, 1, 1));
        assert_eq!(millis.rem_euclid(86_400_000), clock.now_millis().rem_euclid(86_400_000));
    }

    #[test]
    fn a_year_has_its_days_in_order() {
        let days: Vec<_> = Clock::days_of_year(2024).collect();
        assert_eq!(days.len(), 366);
        assert_eq!(days[0], (2024, 0, 1, 2));
        assert_eq!(days[365], (2024, 11, 31, 3));
    }
}
