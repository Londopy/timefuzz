//! Calendar arithmetic shared by grammar rules.

use chrono::{Datelike, Duration, Months, NaiveDate, Weekday};

pub fn add_months(d: NaiveDate, n: i32) -> NaiveDate {
    if n >= 0 {
        d.checked_add_months(Months::new(n as u32)).unwrap_or(d)
    } else {
        d.checked_sub_months(Months::new((-n) as u32)).unwrap_or(d)
    }
}

pub fn month_range(year: i32, month: u32) -> (NaiveDate, NaiveDate) {
    let start = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let end = add_months(start, 1) - Duration::days(1);
    (start, end)
}

pub fn quarter_of_month(month: u32) -> u32 {
    (month - 1) / 3 + 1
}

pub fn quarter_range(year: i32, q: u32) -> (NaiveDate, NaiveDate) {
    let first_month = (q - 1) * 3 + 1;
    let start = NaiveDate::from_ymd_opt(year, first_month, 1).unwrap();
    let end = add_months(start, 3) - Duration::days(1);
    (start, end)
}

pub fn year_range(year: i32) -> (NaiveDate, NaiveDate) {
    (
        NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(year, 12, 31).unwrap(),
    )
}

/// The week containing `d`, given the configured first day of the week.
pub fn week_range(d: NaiveDate, week_start: Weekday) -> (NaiveDate, NaiveDate) {
    let offset = (7 + d.weekday().num_days_from_monday() as i64
        - week_start.num_days_from_monday() as i64)
        % 7;
    let start = d - Duration::days(offset);
    (start, start + Duration::days(6))
}

/// First occurrence of `target` at or after `from` (`strict` = strictly after).
pub fn next_weekday(from: NaiveDate, target: Weekday, strict: bool) -> NaiveDate {
    let mut d = if strict {
        from + Duration::days(1)
    } else {
        from
    };
    while d.weekday() != target {
        d += Duration::days(1);
    }
    d
}

/// Most recent occurrence of `target` at or before `from` (`strict` = strictly before).
pub fn prev_weekday(from: NaiveDate, target: Weekday, strict: bool) -> NaiveDate {
    let mut d = if strict {
        from - Duration::days(1)
    } else {
        from
    };
    while d.weekday() != target {
        d -= Duration::days(1);
    }
    d
}

/// nth (1-based) `target` weekday of a month, if it exists.
pub fn nth_weekday_of_month(year: i32, month: u32, n: u32, target: Weekday) -> Option<NaiveDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let first_hit = next_weekday(first, target, false);
    let d = first_hit + Duration::days(7 * (n as i64 - 1));
    if d.month() == month && d.year() == year {
        Some(d)
    } else {
        None
    }
}

pub fn last_weekday_of_month(year: i32, month: u32, target: Weekday) -> NaiveDate {
    let (_, end) = month_range(year, month);
    prev_weekday(end, target, false)
}

pub fn weekday_name(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

pub fn month_name(m: u32) -> &'static str {
    [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ][(m - 1) as usize]
}
