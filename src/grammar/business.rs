//! Business calendar: "next business day", "in 3 business days",
//! "end of q3", "end of month", "start of next quarter",
//! "last business day of the month".

use super::{day_instant, day_range, period_of, RuleResult};
use crate::confidence;
use crate::config::Ctx;
use crate::dates::*;
use crate::tokenize::{Kw, Tok, Unit};
use chrono::{Datelike, Duration, NaiveDate};

fn add_business_days(ctx: &Ctx, from: NaiveDate, n: i64) -> NaiveDate {
    let mut d = from;
    let mut remaining = n;
    while remaining > 0 {
        d += Duration::days(1);
        if ctx.is_business_day(d) {
            remaining -= 1;
        }
    }
    d
}

pub fn try_match(tokens: &[Tok], ctx: &Ctx) -> RuleResult {
    let today = ctx.today();
    match tokens {
        // "next business day"
        [Tok::Kw(Kw::Next), Tok::Unit(Unit::BusinessDay)] => {
            let d = add_business_days(ctx, today, 1);
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::EXACT,
                "the next business day (skipping weekends and holidays)".into(),
            ))
        }
        // "in N business days" / "N business days from now"
        [Tok::Kw(Kw::In), Tok::Num(n), Tok::Unit(Unit::BusinessDay)]
        | [Tok::Num(n), Tok::Unit(Unit::BusinessDay), Tok::Kw(Kw::From), Tok::Kw(Kw::Now)] => {
            if *n < 0 {
                return RuleResult::None;
            }
            let d = add_business_days(ctx, today, *n);
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::STRONG,
                format!("{n} business days from today"),
            ))
        }
        // "end of q<N>" -> the last month of that quarter, as a range (per spec)
        [Tok::Kw(Kw::End), Tok::Kw(Kw::Of), Tok::Quarter(q)] => {
            let mut year = today.year();
            let (_, q_end) = quarter_range(year, *q);
            let mut note = String::new();
            if q_end < today {
                year += 1;
                note = format!(" (this year's Q{q} has passed; assuming {year})");
            }
            let last_month = *q * 3;
            let (s, e) = month_range(year, last_month);
            RuleResult::One(day_range(
                ctx,
                s,
                e,
                confidence::CALENDAR,
                format!("last month of Q{q}{note}"),
            ))
        }
        // "start of q<N>" -> the first month of that quarter
        [Tok::Kw(Kw::Start), Tok::Kw(Kw::Of), Tok::Quarter(q)] => {
            let mut year = today.year();
            let (_, q_end) = quarter_range(year, *q);
            let mut note = String::new();
            if q_end < today {
                year += 1;
                note = format!(" (this year's Q{q} has passed; assuming {year})");
            }
            let first_month = (*q - 1) * 3 + 1;
            let (s, e) = month_range(year, first_month);
            RuleResult::One(day_range(
                ctx,
                s,
                e,
                confidence::CALENDAR,
                format!("first month of Q{q}{note}"),
            ))
        }
        // "end of <period>" / "start of <period>" for week|month|quarter|year,
        // optionally with next/this/last: "end of next month"
        [Tok::Kw(k @ (Kw::End | Kw::Start)), Tok::Kw(Kw::Of), rest @ ..] => {
            // Normalize a bare unit to "this <unit>".
            let normalized: Vec<Tok>;
            let rest = match rest {
                [Tok::Unit(u)] => {
                    normalized = vec![Tok::Kw(Kw::This), Tok::Unit(*u)];
                    &normalized[..]
                }
                r => r,
            };
            let Some(p) = period_of(rest, ctx) else {
                return RuleResult::None;
            };
            let end = matches!(k, Kw::End);
            let d = if end { p.end } else { p.start };
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::CALENDAR.min(p.base_confidence),
                format!(
                    "the {} day of {}",
                    if end { "last" } else { "first" },
                    p.description
                ),
            ))
        }
        // "last business day of <period>"
        [Tok::Kw(Kw::Last), Tok::Unit(Unit::BusinessDay), Tok::Kw(Kw::Of), rest @ ..] => {
            let normalized: Vec<Tok>;
            let rest = match rest {
                [Tok::Unit(u)] => {
                    normalized = vec![Tok::Kw(Kw::This), Tok::Unit(*u)];
                    &normalized[..]
                }
                r => r,
            };
            let Some(p) = period_of(rest, ctx) else {
                return RuleResult::None;
            };
            let mut d = p.end;
            while !ctx.is_business_day(d) {
                d -= Duration::days(1);
            }
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::CALENDAR.min(p.base_confidence),
                format!("the last business day of {}", p.description),
            ))
        }
        _ => RuleResult::None,
    }
}
