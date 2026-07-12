//! v0.2+ grammar extensions: richer anchored phrases, more business-calendar
//! rules, weekend support, and additional `Ambiguous` cases.

use super::{day_instant, day_range, period_of, RuleResult};
use crate::confidence;
use crate::config::Ctx;
use crate::dates::*;
use crate::tokenize::{Kw, Tok, Unit};
use chrono::{Datelike, Duration, NaiveDate, Weekday};

fn ord_label(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// Walk `n` business days from `from` (negative walks backwards).
fn walk_business_days(ctx: &Ctx, from: NaiveDate, n: i64) -> NaiveDate {
    let mut d = from;
    let step = if n >= 0 { 1 } else { -1 };
    let mut remaining = n.abs();
    while remaining > 0 {
        d += Duration::days(step);
        if ctx.is_business_day(d) {
            remaining -= 1;
        }
    }
    d
}

/// Same day-count thirds as the vague rules.
fn third_span(start: NaiveDate, end: NaiveDate, which: Kw) -> (NaiveDate, NaiveDate, &'static str) {
    let len = (end - start).num_days() + 1;
    let a = len / 3;
    let b = 2 * len / 3;
    match which {
        Kw::Early => (start, start + Duration::days(a.max(1) - 1), "early part of"),
        Kw::Mid => (
            start + Duration::days(a),
            start + Duration::days(b.max(a + 1) - 1),
            "middle part of",
        ),
        _ => (start + Duration::days(b), end, "late part of"),
    }
}

fn dir_label(after: bool) -> &'static str {
    if after {
        "after"
    } else {
        "before"
    }
}

pub fn try_match(tokens: &[Tok], ctx: &Ctx) -> RuleResult {
    let today = ctx.today();
    match tokens {
        // bare anchor: "my birthday"
        [Tok::Anchor(a)] => {
            let Some(d) = ctx.anchors.get(a).copied() else {
                return RuleResult::None;
            };
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::EXACT,
                format!("the date of \"{a}\""),
            ))
        }
        // "<nth> <weekday> after <anchor>"
        [Tok::Ord(n), Tok::Weekday(w), Tok::Kw(Kw::After), Tok::Anchor(a)] => {
            let Some(base) = ctx.anchors.get(a).copied() else {
                return RuleResult::None;
            };
            let d = next_weekday(base, *w, true) + Duration::days(7 * (*n as i64 - 1));
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::STRONG,
                format!(
                    "the {} {} strictly after {}",
                    ord_label(*n),
                    weekday_name(*w),
                    base
                ),
            ))
        }
        // "the week/month after/before <anchor>" -> Range
        [Tok::Unit(u @ (Unit::Week | Unit::Month)), Tok::Kw(k @ (Kw::After | Kw::Before)), Tok::Anchor(a)] =>
        {
            let Some(base) = ctx.anchors.get(a).copied() else {
                return RuleResult::None;
            };
            let after = matches!(k, Kw::After);
            let sign = if after { 1 } else { -1 };
            let (s, e, what) = match u {
                Unit::Week => {
                    let (ws, _) = week_range(base, ctx.cfg.week_start);
                    let s = ws + Duration::days(7 * sign);
                    (s, s + Duration::days(6), "calendar week")
                }
                _ => {
                    let anchor_month = add_months(base.with_day(1).unwrap(), sign as i32);
                    let (s, e) = month_range(anchor_month.year(), anchor_month.month());
                    (s, e, "calendar month")
                }
            };
            RuleResult::One(day_range(
                ctx,
                s,
                e,
                confidence::STRONG,
                format!(
                    "the {what} {} the one containing {}",
                    dir_label(after),
                    base
                ),
            ))
        }
        // "the weekend after <anchor>"
        [Tok::Unit(Unit::Weekend), Tok::Kw(Kw::After), Tok::Anchor(a)] => {
            let Some(base) = ctx.anchors.get(a).copied() else {
                return RuleResult::None;
            };
            let sat = next_weekday(base, Weekday::Sat, true);
            RuleResult::One(day_range(
                ctx,
                sat,
                sat + Duration::days(1),
                confidence::STRONG,
                format!("the first weekend (Sat-Sun) after {base}"),
            ))
        }
        // "N business days after/before <anchor>"
        [Tok::Num(n), Tok::Unit(Unit::BusinessDay), Tok::Kw(k @ (Kw::After | Kw::Before)), Tok::Anchor(a)] =>
        {
            let Some(base) = ctx.anchors.get(a).copied() else {
                return RuleResult::None;
            };
            let after = matches!(k, Kw::After);
            let d = walk_business_days(ctx, base, if after { *n } else { -*n });
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::STRONG,
                format!("{n} business days {} {}", dir_label(after), base),
            ))
        }
        // "N business days ago"
        [Tok::Num(n), Tok::Unit(Unit::BusinessDay), Tok::Kw(Kw::Ago)] => {
            let d = walk_business_days(ctx, today, -*n);
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::STRONG,
                format!("{n} business days before today"),
            ))
        }
        // "<nth> business day of <period>"  ("first business day of the month")
        [Tok::Ord(n), Tok::Unit(Unit::BusinessDay), Tok::Kw(Kw::Of), rest @ ..] => {
            let Some(p) = period_of(rest, ctx) else {
                return RuleResult::None;
            };
            let d = walk_business_days(ctx, p.start - Duration::days(1), *n as i64);
            if d > p.end {
                return RuleResult::Invalid(format!(
                    "{} has fewer than {} business days",
                    p.description, n
                ));
            }
            RuleResult::One(day_instant(
                ctx,
                d,
                confidence::CALENDAR.min(p.base_confidence),
                format!("the {} business day of {}", ord_label(*n), p.description),
            ))
        }
        // "<weekday> next/this/last week"
        [Tok::Weekday(w), Tok::Kw(k @ (Kw::Next | Kw::This | Kw::Last)), Tok::Unit(Unit::Week)] => {
            let offset = match k {
                Kw::Next => 1,
                Kw::Last => -1,
                _ => 0,
            };
            let (ws, _) = week_range(today, ctx.cfg.week_start);
            let d = next_weekday(ws + Duration::days(7 * offset), *w, false);
            let (mut conf, label) = match k {
                Kw::Next => (confidence::EXACT, "next"),
                Kw::Last => (confidence::EXACT, "last"),
                _ => (confidence::CALENDAR, "the current"),
            };
            let mut note = "";
            if matches!(k, Kw::This) && d < today {
                conf = confidence::PART;
                note = " (already passed this week)";
            }
            RuleResult::One(day_instant(
                ctx,
                d,
                conf,
                format!("{} of {} week{note}", weekday_name(*w), label),
            ))
        }
        // "next/this/last weekend"
        [Tok::Kw(k @ (Kw::Next | Kw::This | Kw::Last)), Tok::Unit(Unit::Weekend)] => {
            let (ws, _) = week_range(today, ctx.cfg.week_start);
            let sat_this = next_weekday(ws, Weekday::Sat, false);
            match k {
                Kw::This => RuleResult::One(day_range(
                    ctx,
                    sat_this,
                    sat_this + Duration::days(1),
                    confidence::STRONG,
                    "the weekend of the current week (Sat-Sun)".into(),
                )),
                Kw::Last => {
                    let sat = sat_this - Duration::days(7);
                    RuleResult::One(day_range(
                        ctx,
                        sat,
                        sat + Duration::days(1),
                        confidence::STRONG,
                        "the weekend of the previous week (Sat-Sun)".into(),
                    ))
                }
                _ => {
                    if today < sat_this {
                        // Said midweek, "next weekend" is genuinely ambiguous.
                        let coming = day_range(
                            ctx,
                            sat_this,
                            sat_this + Duration::days(1),
                            0.65,
                            "the coming weekend (Sat-Sun)".into(),
                        );
                        let following = day_range(
                            ctx,
                            sat_this + Duration::days(7),
                            sat_this + Duration::days(8),
                            0.55,
                            "the weekend after the coming one (Sat-Sun)".into(),
                        );
                        RuleResult::Ambiguous(
                            vec![coming, following],
                            "\"next weekend\" said midweek can mean the coming weekend \
                             or the one after it"
                                .into(),
                        )
                    } else {
                        let sat = sat_this + Duration::days(7);
                        RuleResult::One(day_range(
                            ctx,
                            sat,
                            sat + Duration::days(1),
                            confidence::STRONG,
                            "the weekend of next week (Sat-Sun)".into(),
                        ))
                    }
                }
            }
        }
        // bare month name: "august"
        [Tok::Month(m)] => {
            if *m == today.month() {
                // Said during that very month: current month vs. next year.
                let (s1, e1) = month_range(today.year(), *m);
                let (s2, e2) = month_range(today.year() + 1, *m);
                let c1 = day_range(
                    ctx,
                    s1,
                    e1,
                    0.6,
                    format!("{} — the current month", month_name(*m)),
                );
                let c2 = day_range(
                    ctx,
                    s2,
                    e2,
                    0.55,
                    format!("{} {}", month_name(*m), today.year() + 1),
                );
                RuleResult::Ambiguous(
                    vec![c1, c2],
                    format!(
                        "\"{}\" said during {} could mean the current month or {} {}",
                        month_name(*m).to_lowercase(),
                        month_name(*m),
                        month_name(*m),
                        today.year() + 1
                    ),
                )
            } else {
                let Some(p) = period_of(tokens, ctx) else {
                    return RuleResult::None;
                };
                RuleResult::One(day_range(
                    ctx,
                    p.start,
                    p.end,
                    confidence::BARE,
                    p.description,
                ))
            }
        }
        // "sometime early/mid/late <period>" — doubly hedged
        [Tok::Kw(Kw::Sometime), Tok::Kw(k @ (Kw::Early | Kw::Mid | Kw::Late)), rest @ ..] => {
            let rest = match rest {
                [Tok::Kw(Kw::In), r @ ..] => r,
                r => r,
            };
            let Some(p) = period_of(rest, ctx) else {
                return RuleResult::None;
            };
            let (s, e, label) = third_span(p.start, p.end, *k);
            RuleResult::One(day_range(
                ctx,
                s,
                e,
                confidence::DOUBLE_VAGUE.min(p.base_confidence),
                format!("the {label} {} (doubly hedged)", p.description),
            ))
        }
        _ => RuleResult::None,
    }
}
