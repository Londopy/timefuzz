//! Tokenizer: splits input into typed tokens.
//!
//! All language-specific word tables live in `crate::locale::Locale`; the
//! token types below are language-neutral. Adding a language means adding a
//! `Locale` table — this file and the grammar rules stay untouched.

use crate::locale::Locale;
use chrono::Weekday;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unit {
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
    Weekend,
    BusinessDay,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kw {
    Next,
    Last,
    This,
    After,
    Before,
    Sometime,
    Early,
    Mid,
    Late,
    End,
    Start,
    Of,
    In,
    Ago,
    From,
    Now,
    Today,
    Tomorrow,
    Yesterday,
    Business,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Num(i64),
    /// Ordinal: "2nd", "third" -> 3.
    Ord(u32),
    Weekday(Weekday),
    /// Month 1-12.
    Month(u32),
    /// Quarter 1-4 ("q3").
    Quarter(u32),
    /// Clock time in 24h: "3pm" -> (15, 0), "7:30" -> (7, 30), "noon" -> (12, 0).
    Time(u32, u32),
    Unit(Unit),
    Kw(Kw),
    /// A matched, named anchor (normalized name).
    Anchor(String),
    /// Unrecognized word.
    Word(String),
}

/// Validate an (hour, minute) pair, applying an am/pm marker if present.
fn valid_time(h: u32, m: u32, meridiem: Option<bool>) -> Option<(u32, u32)> {
    if m >= 60 {
        return None;
    }
    match meridiem {
        Some(pm) => {
            if !(1..=12).contains(&h) {
                return None;
            }
            Some(((h % 12) + if pm { 12 } else { 0 }, m))
        }
        None => {
            if h < 24 {
                Some((h, m))
            } else {
                None
            }
        }
    }
}

/// Recognize a clock time: "3pm", "3:30pm", "15:45", "noon".
/// A bare number ("3") is a time only with an explicit am/pm marker.
fn time_of(w: &str, loc: &Locale) -> Option<(u32, u32)> {
    if let Some((h, m)) = loc.special_time(w) {
        return Some((h, m));
    }
    let (base, meridiem) = loc.strip_meridiem(w);
    if base.is_empty() {
        return None;
    }
    match base.split_once(':') {
        Some((hs, ms)) => {
            if ms.len() != 2 {
                return None;
            }
            valid_time(hs.parse().ok()?, ms.parse().ok()?, meridiem)
        }
        None => {
            meridiem?; // no colon and no am/pm marker: it's just a number
            valid_time(base.parse().ok()?, 0, meridiem)
        }
    }
}

/// Tokenize `text`. `anchor_names` are normalized (lowercase, trimmed) anchor
/// keys; occurrences in the text become `Tok::Anchor`.
pub fn tokenize(text: &str, anchor_names: &[String], loc: &Locale) -> Vec<Tok> {
    let mut s = text.to_lowercase();
    // NOTE: ':' is intentionally kept — it belongs to clock times ("7:30").
    for c in [',', '.', '!', '?', ';', '(', ')', '"'] {
        s = s.replace(c, " ");
    }
    s = s.replace('-', " ");

    // Substitute anchors (longest names first, so "my birthday party" wins
    // over "my birthday" when both are registered).
    let mut order: Vec<usize> = (0..anchor_names.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(anchor_names[i].len()));
    for i in order {
        let name = &anchor_names[i];
        if !name.is_empty() && s.contains(name.as_str()) {
            s = s.replace(name.as_str(), &format!(" \u{1}{i}\u{1} "));
        }
    }

    let mut toks = Vec::new();
    for w in s.split_whitespace() {
        // Anchor marker?
        if let Some(idx) = w
            .strip_prefix('\u{1}')
            .and_then(|r| r.strip_suffix('\u{1}'))
            .and_then(|r| r.parse::<usize>().ok())
        {
            toks.push(Tok::Anchor(anchor_names[idx].clone()));
            continue;
        }
        if loc.is_filler(w) {
            continue;
        }
        if let Some(u) = loc.end_of_shorthand(w) {
            toks.extend([Tok::Kw(Kw::End), Tok::Kw(Kw::Of), Tok::Unit(u)]);
            continue;
        }
        if let Some((h, m)) = time_of(w, loc) {
            toks.push(Tok::Time(h, m));
        } else if let Ok(n) = w.parse::<i64>() {
            toks.push(Tok::Num(n));
        } else if let Some(o) = loc.ordinal(w) {
            toks.push(Tok::Ord(o));
        } else if let Some(wd) = loc.weekday(w) {
            toks.push(Tok::Weekday(wd));
        } else if let Some(m) = loc.month(w) {
            toks.push(Tok::Month(m));
        } else if let Some(q) = loc.quarter(w) {
            toks.push(Tok::Quarter(q));
        } else if let Some(u) = loc.unit(w) {
            toks.push(Tok::Unit(u));
        } else if let Some(k) = loc.keyword(w) {
            toks.push(Tok::Kw(k));
        } else if let Some(n) = loc.number_word(w) {
            toks.push(Tok::Num(n));
        } else {
            toks.push(Tok::Word(w.to_string()));
        }
    }

    // Merge passes:
    //   "business" + day-unit          -> BusinessDay
    //   Num + standalone am/pm word    -> Time ("3 pm")
    let mut merged: Vec<Tok> = Vec::with_capacity(toks.len());
    let mut i = 0;
    while i < toks.len() {
        if i + 1 < toks.len()
            && toks[i] == Tok::Kw(Kw::Business)
            && toks[i + 1] == Tok::Unit(Unit::Day)
        {
            merged.push(Tok::Unit(Unit::BusinessDay));
            i += 2;
            continue;
        }
        if let (Tok::Num(n), Some(Tok::Word(w2))) = (&toks[i], toks.get(i + 1)) {
            let meridiem = if loc.pm_words.contains(&w2.as_str()) {
                Some(true)
            } else if loc.am_words.contains(&w2.as_str()) {
                Some(false)
            } else {
                None
            };
            if meridiem.is_some() && *n >= 0 {
                if let Some((h, m)) = valid_time(*n as u32, 0, meridiem) {
                    merged.push(Tok::Time(h, m));
                    i += 2;
                    continue;
                }
            }
        }
        merged.push(toks[i].clone());
        i += 1;
    }
    merged
}
