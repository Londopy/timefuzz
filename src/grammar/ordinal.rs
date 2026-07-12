//! Ordinal-in-period: "2nd monday of march", "last friday of october",
//! "first monday of next month".

use super::{day_instant, RuleResult};
use crate::confidence;
use crate::config::Ctx;
use crate::dates::*;
use crate::tokenize::{Kw, Tok, Unit};
use chrono::Datelike;

fn ordinal_label(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

pub fn try_match(tokens: &[Tok], ctx: &Ctx) -> RuleResult {
    let today = ctx.today();
    match tokens {
        // "<nth> <weekday> of <month> [year]"
        [Tok::Ord(n), Tok::Weekday(w), Tok::Kw(Kw::Of), Tok::Month(m), rest @ ..]
            if rest.is_empty() || matches!(rest, [Tok::Num(_)]) =>
        {
            let (year, explicit_year) = match rest {
                [Tok::Num(y)] => (*y as i32, true),
                _ => (today.year(), false),
            };
            let mut year = year;
            let mut note = String::new();
            if !explicit_year {
                // If that date has already passed this year, roll to next year.
                if let Some(d) = nth_weekday_of_month(year, *m, *n, *w) {
                    if d < today {
                        year += 1;
                        note = format!(" (this year's has passed; assuming {year})");
                    }
                }
            }
            match nth_weekday_of_month(year, *m, *n, *w) {
                Some(d) => RuleResult::One(day_instant(
                    ctx,
                    d,
                    if explicit_year {
                        confidence::STRONG
                    } else {
                        confidence::CALENDAR
                    },
                    format!(
                        "the {} {} of {} {}{}",
                        ordinal_label(*n),
                        weekday_name(*w),
                        month_name(*m),
                        year,
                        note
                    ),
                )),
                None => RuleResult::Invalid(format!(
                    "{} {} has no {} {}",
                    month_name(*m),
                    year,
                    ordinal_label(*n),
                    weekday_name(*w)
                )),
            }
        }
        // "last <weekday> of <month> [year]"
        [Tok::Kw(Kw::Last), Tok::Weekday(w), Tok::Kw(Kw::Of), Tok::Month(m), rest @ ..]
            if rest.is_empty() || matches!(rest, [Tok::Num(_)]) =>
        {
            let (mut year, explicit_year) = match rest {
                [Tok::Num(y)] => (*y as i32, true),
                _ => (today.year(), false),
            };
            let mut note = String::new();
            if !explicit_year && last_weekday_of_month(year, *m, *w) < today {
                year += 1;
                note = format!(" (this year's has passed; assuming {year})");
            }
            let d = last_weekday_of_month(year, *m, *w);
            RuleResult::One(day_instant(
                ctx,
                d,
                if explicit_year {
                    confidence::STRONG
                } else {
                    confidence::CALENDAR
                },
                format!(
                    "the last {} of {} {}{}",
                    weekday_name(*w),
                    month_name(*m),
                    year,
                    note
                ),
            ))
        }
        // "<nth> <weekday> of next/this month"
        [Tok::Ord(n), Tok::Weekday(w), Tok::Kw(Kw::Of), Tok::Kw(k @ (Kw::Next | Kw::This)), Tok::Unit(Unit::Month)] =>
        {
            let anchor = add_months(
                today.with_day(1).unwrap(),
                if matches!(k, Kw::Next) { 1 } else { 0 },
            );
            let label = if matches!(k, Kw::Next) {
                "next"
            } else {
                "the current"
            };
            match nth_weekday_of_month(anchor.year(), anchor.month(), *n, *w) {
                Some(d) => RuleResult::One(day_instant(
                    ctx,
                    d,
                    confidence::CALENDAR,
                    format!(
                        "the {} {} of {} month ({} {})",
                        ordinal_label(*n),
                        weekday_name(*w),
                        label,
                        month_name(anchor.month()),
                        anchor.year()
                    ),
                )),
                None => RuleResult::Invalid(format!(
                    "{} {} has no {} {}",
                    month_name(anchor.month()),
                    anchor.year(),
                    ordinal_label(*n),
                    weekday_name(*w)
                )),
            }
        }
        _ => RuleResult::None,
    }
}
