//! Weekday navigation: "next friday", "this tuesday", "last monday", "friday".

use super::{day_instant, RuleResult};
use crate::confidence;
use crate::config::Ctx;
use crate::dates::*;
use crate::tokenize::{Kw, Tok};
use chrono::{Datelike, Duration};

pub fn try_match(tokens: &[Tok], ctx: &Ctx) -> RuleResult {
    let today = ctx.today();
    match tokens {
        // "next <weekday>"
        [Tok::Kw(Kw::Next), Tok::Weekday(w)] => {
            let name = weekday_name(*w);
            if today.weekday() == *w {
                // Today *is* that weekday: convention decided by config.
                let (d, note) = if ctx.cfg.next_skips_today {
                    (
                        today + Duration::days(7),
                        "skipping today per next_skips_today=true",
                    )
                } else {
                    (today, "today counts per next_skips_today=false")
                };
                RuleResult::One(day_instant(
                    ctx,
                    d,
                    confidence::STRONG,
                    format!("next {name} — today is {name}; {note}"),
                ))
            } else {
                RuleResult::One(day_instant(
                    ctx,
                    next_weekday(today, *w, true),
                    confidence::EXACT,
                    format!(
                        "next {name}, default {}",
                        ctx.cfg.default_time.format("%H:%M")
                    ),
                ))
            }
        }
        // "this <weekday>": the occurrence within the current week
        [Tok::Kw(Kw::This), Tok::Weekday(w)] => {
            let (ws, _) = week_range(today, ctx.cfg.week_start);
            let d = next_weekday(ws, *w, false);
            let name = weekday_name(*w);
            let (conf, note) = if d < today {
                (confidence::PART, " (already passed this week)")
            } else if d == today {
                (confidence::STRONG, " (that is today)")
            } else {
                (confidence::CALENDAR, "")
            };
            RuleResult::One(day_instant(
                ctx,
                d,
                conf,
                format!("{name} of the current week{note}"),
            ))
        }
        // "last <weekday>": most recent strictly before today
        [Tok::Kw(Kw::Last), Tok::Weekday(w)] => RuleResult::One(day_instant(
            ctx,
            prev_weekday(today, *w, true),
            confidence::EXACT,
            format!("the most recent {} before today", weekday_name(*w)),
        )),
        // bare "<weekday>"
        [Tok::Weekday(w)] => {
            let name = weekday_name(*w);
            if today.weekday() == *w {
                // Said on that very day: could mean today or next week.
                let c1 = day_instant(ctx, today, confidence::BARE, format!("{name} — today"));
                let c2 = day_instant(
                    ctx,
                    today + Duration::days(7),
                    confidence::BARE - 0.1,
                    format!("{name} — a week from today"),
                );
                RuleResult::Ambiguous(
                    vec![c1, c2],
                    format!(
                        "today is {name}; bare \"{}\" could mean today or next week",
                        name.to_lowercase()
                    ),
                )
            } else {
                RuleResult::One(day_instant(
                    ctx,
                    next_weekday(today, *w, true),
                    confidence::BARE,
                    format!("the upcoming {name}"),
                ))
            }
        }
        _ => RuleResult::None,
    }
}
