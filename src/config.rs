//! Parser configuration and the parse context threaded through grammar rules.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use std::collections::HashMap;

pub struct Cfg {
    /// Time of day applied to date-only results (default 09:00).
    pub default_time: NaiveTime,
    /// First day of the week (default Monday).
    pub week_start: Weekday,
    /// Whether "next <weekday>" skips today when today is that weekday.
    pub next_skips_today: bool,
}

impl Default for Cfg {
    fn default() -> Self {
        Cfg {
            default_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            week_start: Weekday::Mon,
            next_skips_today: true,
        }
    }
}

/// Everything a grammar rule needs to resolve a phrase.
pub struct Ctx<'a> {
    pub now: NaiveDateTime,
    pub cfg: &'a Cfg,
    /// Anchor name (normalized, lowercase) -> date.
    pub anchors: &'a HashMap<String, NaiveDate>,
    /// Business-calendar holiday predicate (weekends handled separately).
    pub is_holiday: &'a dyn Fn(NaiveDate) -> bool,
}

impl Ctx<'_> {
    pub fn today(&self) -> NaiveDate {
        self.now.date()
    }

    /// Attach the configured default time to a date.
    pub fn at_default(&self, d: NaiveDate) -> NaiveDateTime {
        d.and_time(self.cfg.default_time)
    }

    /// Inclusive day span: 00:00:00 .. 23:59:59.
    pub fn day_span(&self, start: NaiveDate, end: NaiveDate) -> (NaiveDateTime, NaiveDateTime) {
        (
            start.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
            end.and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
        )
    }

    pub fn is_business_day(&self, d: NaiveDate) -> bool {
        use chrono::Datelike;
        let wd = d.weekday();
        wd != Weekday::Sat && wd != Weekday::Sun && !(self.is_holiday)(d) && d.year() > 0
    }
}
