//! Tokenizer: splits input into typed tokens.

use chrono::Weekday;

#[derive(Debug, Clone, PartialEq)]
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
    Unit(Unit),
    Kw(Kw),
    /// A matched, named anchor (normalized name).
    Anchor(String),
    /// Unrecognized word.
    Word(String),
}

fn weekday_of(w: &str) -> Option<Weekday> {
    Some(match w {
        "monday" | "mon" => Weekday::Mon,
        "tuesday" | "tue" | "tues" => Weekday::Tue,
        "wednesday" | "wed" => Weekday::Wed,
        "thursday" | "thu" | "thur" | "thurs" => Weekday::Thu,
        "friday" | "fri" => Weekday::Fri,
        "saturday" | "sat" => Weekday::Sat,
        "sunday" | "sun" => Weekday::Sun,
        _ => return None,
    })
}

fn month_of(w: &str) -> Option<u32> {
    Some(match w {
        "january" | "jan" => 1,
        "february" | "feb" => 2,
        "march" | "mar" => 3,
        "april" | "apr" => 4,
        "may" => 5,
        "june" | "jun" => 6,
        "july" | "jul" => 7,
        "august" | "aug" => 8,
        "september" | "sep" | "sept" => 9,
        "october" | "oct" => 10,
        "november" | "nov" => 11,
        "december" | "dec" => 12,
        _ => return None,
    })
}

fn unit_of(w: &str) -> Option<Unit> {
    Some(match w {
        "minute" | "minutes" | "min" | "mins" => Unit::Minute,
        "hour" | "hours" | "hr" | "hrs" => Unit::Hour,
        "day" | "days" => Unit::Day,
        "week" | "weeks" | "wk" | "wks" => Unit::Week,
        "month" | "months" => Unit::Month,
        "quarter" | "quarters" => Unit::Quarter,
        "year" | "years" | "yr" | "yrs" => Unit::Year,
        "weekend" | "weekends" => Unit::Weekend,
        _ => return None,
    })
}

/// Business shorthand that expands to multiple tokens: "eom" == "end of month".
fn shorthand(w: &str) -> Option<Vec<Tok>> {
    let unit = match w {
        "eow" => Unit::Week,
        "eom" => Unit::Month,
        "eoq" => Unit::Quarter,
        "eoy" => Unit::Year,
        _ => return None,
    };
    Some(vec![Tok::Kw(Kw::End), Tok::Kw(Kw::Of), Tok::Unit(unit)])
}

fn kw_of(w: &str) -> Option<Kw> {
    Some(match w {
        "next" | "upcoming" => Kw::Next,
        "last" | "previous" | "past" => Kw::Last,
        "this" | "current" => Kw::This,
        "after" | "following" => Kw::After,
        "before" | "preceding" => Kw::Before,
        "sometime" | "somewhere" | "around" => Kw::Sometime,
        "early" => Kw::Early,
        "mid" | "middle" => Kw::Mid,
        "late" => Kw::Late,
        "end" => Kw::End,
        "start" | "beginning" => Kw::Start,
        "of" => Kw::Of,
        "in" | "within" => Kw::In,
        "ago" => Kw::Ago,
        "from" => Kw::From,
        "now" => Kw::Now,
        "today" => Kw::Today,
        "tomorrow" => Kw::Tomorrow,
        "yesterday" => Kw::Yesterday,
        "business" | "working" => Kw::Business,
        _ => return None,
    })
}

fn number_word(w: &str) -> Option<i64> {
    Some(match w {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        _ => return None,
    })
}

fn ordinal_word(w: &str) -> Option<u32> {
    Some(match w {
        "first" | "1st" => 1,
        "second" | "2nd" => 2,
        "third" | "3rd" => 3,
        "fourth" | "4th" => 4,
        "fifth" | "5th" => 5,
        _ => {
            // "6th" ... "31st"
            let digits: String = w.chars().take_while(|c| c.is_ascii_digit()).collect();
            let suffix = &w[digits.len()..];
            if !digits.is_empty() && matches!(suffix, "st" | "nd" | "rd" | "th") {
                return digits.parse().ok();
            }
            return None;
        }
    })
}

fn quarter_of(w: &str) -> Option<u32> {
    let rest = w.strip_prefix('q')?;
    let q: u32 = rest.parse().ok()?;
    if (1..=4).contains(&q) {
        Some(q)
    } else {
        None
    }
}

/// Tokenize `text`. `anchor_names` are normalized (lowercase, trimmed) anchor
/// keys; occurrences in the text become `Tok::Anchor`.
pub fn tokenize(text: &str, anchor_names: &[String]) -> Vec<Tok> {
    let mut s = text.to_lowercase();
    for c in [',', '.', '!', '?', ';', ':', '(', ')', '"'] {
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
        if w == "the" || w == "on" || w == "at" || w == "for" {
            continue; // articles / filler
        }
        if let Some(extra) = shorthand(w) {
            toks.extend(extra);
            continue;
        }
        if let Ok(n) = w.parse::<i64>() {
            toks.push(Tok::Num(n));
        } else if let Some(o) = ordinal_word(w) {
            toks.push(Tok::Ord(o));
        } else if let Some(wd) = weekday_of(w) {
            toks.push(Tok::Weekday(wd));
        } else if let Some(m) = month_of(w) {
            toks.push(Tok::Month(m));
        } else if let Some(q) = quarter_of(w) {
            toks.push(Tok::Quarter(q));
        } else if let Some(u) = unit_of(w) {
            toks.push(Tok::Unit(u));
        } else if let Some(k) = kw_of(w) {
            toks.push(Tok::Kw(k));
        } else if let Some(n) = number_word(w) {
            toks.push(Tok::Num(n));
        } else {
            toks.push(Tok::Word(w.to_string()));
        }
    }

    // Merge "business" + day-unit -> BusinessDay.
    let mut merged = Vec::with_capacity(toks.len());
    let mut i = 0;
    while i < toks.len() {
        if i + 1 < toks.len()
            && toks[i] == Tok::Kw(Kw::Business)
            && toks[i + 1] == Tok::Unit(Unit::Day)
        {
            merged.push(Tok::Unit(Unit::BusinessDay));
            i += 2;
        } else {
            merged.push(toks[i].clone());
            i += 1;
        }
    }
    merged
}
