//! Anchored relatives: "<weekday> after/before <anchor>", "the day after
//! <anchor>", "2 weeks before <anchor>". Anchors are caller-supplied named
//! dates; unknown anchors yield `Ambiguous` with a clear reason.

use super::{day_instant, RuleResult};
use crate::confidence;
use crate::config::Ctx;
use crate::dates::*;
use crate::tokenize::{Kw, Tok, Unit};
use chrono::{Duration, NaiveDate};

fn anchor_date(ctx: &Ctx, name: &str) -> Option<NaiveDate> {
    ctx.anchors.get(name).copied()
}

fn dir_label(after: bool) -> &'static str {
    if after {
        "after"
    } else {
        "before"
    }
}

pub fn try_match(tokens: &[Tok], ctx: &Ctx) -> RuleResult {
    match tokens {
        // "<weekday> after/before <anchor>"
        [Tok::Weekday(w), Tok::Kw(k @ (Kw::After | Kw::Before)), Tok::Anchor(a)] => {
            let Some(base) = anchor_date(ctx, a) else {
                return RuleResult::None;
            };
            let after = matches!(k, Kw::After);
            let d = if after {
                next_weekday(base, *w, true)
            } else {
                prev_weekday(base, *w, true)
            };
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::STRONG,
                format!(
                    "first {} strictly {} {}",
                    weekday_name(*w),
                    dir_label(after),
                    base
                ),
            ))
        }
        // "the day after/before <anchor>"
        [Tok::Unit(Unit::Day), Tok::Kw(k @ (Kw::After | Kw::Before)), Tok::Anchor(a)] => {
            let Some(base) = anchor_date(ctx, a) else {
                return RuleResult::None;
            };
            let after = matches!(k, Kw::After);
            let d = base + Duration::days(if after { 1 } else { -1 });
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::EXACT,
                format!("the day {} {}", dir_label(after), base),
            ))
        }
        // "N days/weeks/months/years after/before <anchor>"
        [Tok::Num(n), Tok::Unit(u), Tok::Kw(k @ (Kw::After | Kw::Before)), Tok::Anchor(a)] => {
            let Some(base) = anchor_date(ctx, a) else {
                return RuleResult::None;
            };
            let after = matches!(k, Kw::After);
            let sign: i64 = if after { 1 } else { -1 };
            let (d, unit_name) = match u {
                Unit::Day => (base + Duration::days(sign * n), "day"),
                Unit::Week => (base + Duration::days(7 * sign * n), "week"),
                Unit::Month => (add_months(base, (sign * n) as i32), "month"),
                Unit::Year => (add_months(base, (12 * sign * n) as i32), "year"),
                _ => return RuleResult::None,
            };
            let plural = if *n == 1 { "" } else { "s" };
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::STRONG,
                format!("{n} {unit_name}{plural} {} {}", dir_label(after), base),
            ))
        }
        // Unknown anchor: "<...> after/before <unrecognized words>"
        _ => {
            let pos = tokens
                .iter()
                .position(|t| matches!(t, Tok::Kw(Kw::After | Kw::Before)));
            if let Some(i) = pos {
                let tail = &tokens[i + 1..];
                let words: Vec<&str> = tail
                    .iter()
                    .filter_map(|t| match t {
                        Tok::Word(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .collect();
                if !words.is_empty() && words.len() == tail.len() {
                    return RuleResult::Ambiguous(
                        vec![],
                        format!(
                            "unknown anchor \"{}\" — pass it via anchors={{\"{}\": date(...)}}",
                            words.join(" "),
                            words.join(" ")
                        ),
                    );
                }
            }
            RuleResult::None
        }
    }
}
