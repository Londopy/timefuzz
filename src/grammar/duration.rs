//! Bare durations: "30 minutes", "2 hrs", "half an hour", "a couple hours".
//!
//! `relative.rs` covers the *marked* forms — "in 30 minutes", "30 minutes from
//! now". This rule covers the same interval written without the marker, which
//! is how people usually type a short ETA. A bare duration means the same
//! thing as the `in …` form and resolves to the same `Instant`, so when both
//! rules fire the dispatcher's value-dedup collapses them to one candidate.
//!
//! The rule is deliberately narrow:
//!
//! * it needs an explicit quantity — a bare "week" stays a period, not "in a
//!   week";
//! * the unit must end the phrase, so "3 days ago", "2 weeks from now" and
//!   "3 business days after the wedding" fall through to their own rules;
//! * it is the only rule that tolerates unrecognized *leading* words, enough
//!   to read "back in a couple hours" without letting every other rule start
//!   ignoring words it does not understand. Doing so costs confidence and is
//!   named in the interpretation.

use super::{relative, RuleResult};
use crate::confidence;
use crate::config::Ctx;
use crate::dates::add_months;
use crate::tokenize::{Kw, Tok, Unit};
use crate::types::{Resolution, Value};
use chrono::{Duration, NaiveDate};

/// How the quantity was written. This is what separates "2 hrs" from "a
/// couple hours": same arithmetic, different confidence.
#[derive(Clone, Copy, PartialEq)]
enum Qty {
    /// An explicit count: "30 minutes".
    Exact(i64),
    /// An approximate count: "a couple hours".
    Approx(i64),
    /// "half an hour" — a fraction, but an exact one.
    Half,
}

impl Qty {
    fn count(self) -> i64 {
        match self {
            Qty::Exact(n) | Qty::Approx(n) => n,
            Qty::Half => 1,
        }
    }

    fn is_approx(self) -> bool {
        matches!(self, Qty::Approx(_))
    }
}

/// Half a unit, in seconds, plus its article form for the interpretation.
///
/// A unit is halved only when the halving is *exact* under the arithmetic
/// this crate already uses for it. Minutes through weeks are fixed spans of
/// seconds, so they halve cleanly. Months and quarters are calendar
/// arithmetic — "half a month" would have to be rounded to some number of
/// days, so it stays unparsed instead of guessing. (Years are the exception
/// and are handled separately: half a year is exactly six months.)
fn half_of(unit: &Unit) -> Option<(i64, &'static str)> {
    Some(match unit {
        Unit::Minute => (30, "a minute"),
        Unit::Hour => (30 * 60, "an hour"),
        Unit::Day => (12 * 3600, "a day"),
        Unit::Week => (3 * 24 * 3600 + 12 * 3600, "a week"),
        Unit::Month | Unit::Quarter | Unit::Year | Unit::Weekend | Unit::BusinessDay => {
            return None
        }
    })
}

/// Walk `n` business days forward. Mirrors `business.rs` so that
/// "3 business days" and "in 3 business days" land on the same value.
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

/// Read the quantity at the front of `toks`, returning it and what follows.
fn take_qty(toks: &[Tok]) -> Option<(Qty, &[Tok])> {
    let (qty, rest) = match toks {
        [Tok::Num(n), rest @ ..] if *n >= 0 => (Qty::Exact(*n), rest),
        [Tok::Fuzzy(n), rest @ ..] if *n >= 0 => (Qty::Approx(*n), rest),
        [Tok::Half, rest @ ..] => (Qty::Half, rest),
        _ => return None,
    };
    // "a couple of hours" — the "of" is noise between quantity and unit.
    match rest {
        [Tok::Kw(Kw::Of), tail @ ..] => Some((qty, tail)),
        tail => Some((qty, tail)),
    }
}

pub fn try_match(tokens: &[Tok], ctx: &Ctx) -> RuleResult {
    // Leading words the tokenizer did not recognize: "back in a couple
    // hours", "gimme 5 mins".
    let skipped: Vec<&String> = tokens
        .iter()
        .map_while(|t| match t {
            Tok::Word(w) => Some(w),
            _ => None,
        })
        .collect();
    let rest = &tokens[skipped.len()..];

    // An optional "in". With an exact count this is also relative.rs's
    // pattern; both rules produce the same value, so the dispatcher dedups.
    let rest = match rest {
        [Tok::Kw(Kw::In), tail @ ..] => tail,
        tail => tail,
    };

    let Some((qty, rest)) = take_qty(rest) else {
        return RuleResult::None;
    };
    let [Tok::Unit(unit)] = rest else {
        return RuleResult::None;
    };

    // Resolve to an instant, the label for the amount, and the confidence
    // tier the way the quantity was written earns.
    let (when, amount, base) = match (qty, unit) {
        // Half a year is six months exactly, in the same calendar arithmetic
        // "in 1 year" uses (twelve months).
        (Qty::Half, Unit::Year) => (
            relative::apply_tod(
                ctx,
                &Unit::Year,
                add_months(ctx.now.date(), 6).and_time(ctx.now.time()),
            ),
            "half a year".to_string(),
            confidence::EXACT,
        ),
        (Qty::Half, u) => match half_of(u) {
            Some((secs, article_form)) => (
                ctx.now + Duration::seconds(secs),
                format!("half {article_form}"),
                confidence::EXACT,
            ),
            None => return RuleResult::None,
        },
        // Business days are whole days at the default time, per business.rs.
        (_, Unit::BusinessDay) => (
            ctx.at_default(add_business_days(ctx, ctx.today(), qty.count())),
            relative::plural(qty.count(), "business day"),
            if qty.is_approx() {
                confidence::BARE
            } else {
                confidence::STRONG
            },
        ),
        (_, u) => match relative::shift(ctx.now, u, qty.count()) {
            Some((when, name)) => (
                relative::apply_tod(ctx, u, when),
                relative::plural(qty.count(), name),
                if qty.is_approx() {
                    confidence::BARE
                } else {
                    // Unmarked but explicit is as clear as "in 30 minutes".
                    confidence::EXACT
                },
            ),
            None => return RuleResult::None,
        },
    };

    let amount = if qty.is_approx() {
        format!("approximately {amount}")
    } else {
        amount
    };

    let days = (when.date() - ctx.today()).num_days();
    let conf = confidence::horizon_penalty(base, days);
    let horizon = if conf < base {
        " (distant horizon)"
    } else {
        ""
    };

    // Reading past words we did not understand is a guess; say so, and score
    // it below the same phrase written cleanly.
    let (conf, ignored) = if skipped.is_empty() {
        (conf, String::new())
    } else {
        let words: Vec<String> = skipped.iter().map(|w| format!("\"{w}\"")).collect();
        (
            confidence::clamp(conf - 0.05),
            format!(" (ignoring {})", words.join(", ")),
        )
    };

    RuleResult::One(Resolution {
        value: Value::Instant { when },
        confidence: conf,
        interpretation: format!("{amount} from now{horizon}{ignored}"),
    })
}
