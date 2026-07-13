//! Relative offsets: "in 3 days", "2 weeks ago", "tomorrow", "next month".

use super::{day_instant, RuleResult};
use crate::confidence;
use crate::config::Ctx;
use crate::dates::*;
use crate::tokenize::{Kw, Tok, Unit};
use crate::types::{Resolution, Value};
use chrono::{Datelike, Duration, NaiveDateTime};

fn shift(now: NaiveDateTime, unit: &Unit, n: i64) -> Option<(NaiveDateTime, &'static str)> {
    Some(match unit {
        Unit::Minute => (now + Duration::minutes(n), "minute"),
        Unit::Hour => (now + Duration::hours(n), "hour"),
        Unit::Day => (now + Duration::days(n), "day"),
        Unit::Week => (now + Duration::days(7 * n), "week"),
        Unit::Month => (
            add_months(now.date(), n as i32).and_time(now.time()),
            "month",
        ),
        Unit::Quarter => (
            add_months(now.date(), 3 * n as i32).and_time(now.time()),
            "quarter",
        ),
        Unit::Year => (
            add_months(now.date(), 12 * n as i32).and_time(now.time()),
            "year",
        ),
        // handled by the business/ext rules
        Unit::BusinessDay | Unit::Weekend => return None,
    })
}

/// An explicit trailing time ("in 3 days at 5pm") overrides the clock for
/// date-granular units; minute/hour arithmetic keeps its own clock.
fn apply_tod(ctx: &Ctx, unit: &Unit, when: NaiveDateTime) -> NaiveDateTime {
    if ctx.explicit_time && !matches!(unit, Unit::Minute | Unit::Hour) {
        when.date().and_time(ctx.tod)
    } else {
        when
    }
}

fn plural(n: i64, s: &str) -> String {
    if n == 1 {
        format!("1 {s}")
    } else {
        format!("{n} {s}s")
    }
}

pub fn try_match(tokens: &[Tok], ctx: &Ctx) -> RuleResult {
    let today = ctx.today();
    match tokens {
        [Tok::Kw(Kw::Now)] => RuleResult::One(Resolution {
            value: Value::Instant { when: ctx.now },
            confidence: confidence::CERTAIN,
            interpretation: "the current moment".into(),
        }),
        [Tok::Kw(Kw::Today)] => RuleResult::One(day_instant(
            ctx,
            today,
            confidence::EXACT,
            format!("today, {}", ctx.tod_label()),
        )),
        [Tok::Kw(Kw::Tomorrow)] => RuleResult::One(day_instant(
            ctx,
            today + Duration::days(1),
            confidence::EXACT,
            format!("tomorrow, {}", ctx.tod_label()),
        )),
        [Tok::Kw(Kw::Yesterday)] => RuleResult::One(day_instant(
            ctx,
            today - Duration::days(1),
            confidence::EXACT,
            format!("yesterday, {}", ctx.tod_label()),
        )),
        // bare clock time: "3pm", "15:45", "noon"
        [Tok::Time(h, m)] => {
            let t = chrono::NaiveTime::from_hms_opt(*h, *m, 0).unwrap();
            let (d, note) = if ctx.now.time() < t {
                (today, "today")
            } else {
                (
                    today + Duration::days(1),
                    "tomorrow (that time has already passed today)",
                )
            };
            RuleResult::One(Resolution {
                value: Value::Instant {
                    when: d.and_time(t),
                },
                confidence: confidence::STRONG,
                interpretation: format!("{h:02}:{m:02} {note}"),
            })
        }
        // "in N <unit>" / "N <unit> from now"
        [Tok::Kw(Kw::In), Tok::Num(n), Tok::Unit(u)]
        | [Tok::Num(n), Tok::Unit(u), Tok::Kw(Kw::From), Tok::Kw(Kw::Now)] => {
            match shift(ctx.now, u, *n) {
                Some((when, name)) => {
                    let when = apply_tod(ctx, u, when);
                    let days = (when.date() - ctx.today()).num_days();
                    let conf = confidence::horizon_penalty(confidence::EXACT, days);
                    let note = if conf < confidence::EXACT {
                        " (distant horizon)"
                    } else {
                        ""
                    };
                    RuleResult::One(Resolution {
                        value: Value::Instant { when },
                        confidence: conf,
                        interpretation: format!("{} from now{note}", plural(*n, name)),
                    })
                }
                None => RuleResult::None,
            }
        }
        // "N <unit> ago"
        [Tok::Num(n), Tok::Unit(u), Tok::Kw(Kw::Ago)] => match shift(ctx.now, u, -*n) {
            Some((when, name)) => {
                let when = apply_tod(ctx, u, when);
                let days = (when.date() - ctx.today()).num_days();
                let conf = confidence::horizon_penalty(confidence::EXACT, days);
                let note = if conf < confidence::EXACT {
                    " (distant horizon)"
                } else {
                    ""
                };
                RuleResult::One(Resolution {
                    value: Value::Instant { when },
                    confidence: conf,
                    interpretation: format!("{} before now{note}", plural(*n, name)),
                })
            }
            None => RuleResult::None,
        },
        // "next/this/last week|month|quarter|year" -> span of that period
        [Tok::Kw(Kw::Next | Kw::This | Kw::Last), Tok::Unit(Unit::Week | Unit::Month | Unit::Quarter | Unit::Year)] => {
            match super::period_of(tokens, ctx) {
                Some(p) => RuleResult::One(super::day_range(
                    ctx,
                    p.start,
                    p.end,
                    confidence::STRONG,
                    p.description,
                )),
                None => RuleResult::None,
            }
        }
        _ => {
            let _ = today.year();
            RuleResult::None
        }
    }
}
