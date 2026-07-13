//! The i18n seam: every language-specific word table lives in a `Locale`.
//!
//! Grammar rules and the token types are language-neutral; the tokenizer
//! resolves words through the `Locale` it is given. English is the built-in
//! default (`EN`). Adding a language means writing one more `Locale` table —
//! no grammar changes.
//!
//! Note: `interpretation` strings in results are English in v0.x/v1.0;
//! localizing *output* is a separate, later concern.

use crate::tokenize::{Kw, Unit};
use chrono::Weekday;

pub struct Locale {
    pub name: &'static str,
    /// Word -> weekday ("monday", "mon").
    pub weekdays: &'static [(&'static str, Weekday)],
    /// Word -> month number 1-12.
    pub months: &'static [(&'static str, u32)],
    /// Word -> unit ("week", "wks").
    pub units: &'static [(&'static str, Unit)],
    /// Word -> keyword ("next", "after", "sometime").
    pub keywords: &'static [(&'static str, Kw)],
    /// Word -> number ("three", "a").
    pub number_words: &'static [(&'static str, i64)],
    /// Word -> ordinal ("first", "third").
    pub ordinal_words: &'static [(&'static str, u32)],
    /// Suffixes that turn digits into ordinals ("st" in "31st").
    pub ordinal_suffixes: &'static [&'static str],
    /// Prefixes that turn a digit 1-4 into a quarter ("q" in "q3").
    pub quarter_prefixes: &'static [&'static str],
    /// Articles / filler words dropped entirely ("the", "at").
    pub fillers: &'static [&'static str],
    /// Shorthand expanding to "end of <unit>" ("eom").
    pub end_of_shorthand: &'static [(&'static str, Unit)],
    /// Suffix/word marking a morning clock time ("am").
    pub am_words: &'static [&'static str],
    /// Suffix/word marking an afternoon clock time ("pm").
    pub pm_words: &'static [&'static str],
    /// Named clock times ("noon" -> 12:00).
    pub special_times: &'static [(&'static str, (u32, u32))],
}

fn find<T: Copy>(table: &[(&str, T)], w: &str) -> Option<T> {
    table.iter().find(|(k, _)| *k == w).map(|(_, v)| *v)
}

impl Locale {
    pub fn weekday(&self, w: &str) -> Option<Weekday> {
        find(self.weekdays, w)
    }
    pub fn month(&self, w: &str) -> Option<u32> {
        find(self.months, w)
    }
    pub fn unit(&self, w: &str) -> Option<Unit> {
        find(self.units, w)
    }
    pub fn keyword(&self, w: &str) -> Option<Kw> {
        find(self.keywords, w)
    }
    pub fn number_word(&self, w: &str) -> Option<i64> {
        find(self.number_words, w)
    }
    pub fn special_time(&self, w: &str) -> Option<(u32, u32)> {
        find(self.special_times, w)
    }
    pub fn end_of_shorthand(&self, w: &str) -> Option<Unit> {
        find(self.end_of_shorthand, w)
    }
    pub fn is_filler(&self, w: &str) -> bool {
        self.fillers.contains(&w)
    }

    /// "2nd", "31st", "first" -> ordinal number.
    pub fn ordinal(&self, w: &str) -> Option<u32> {
        if let Some(n) = find(self.ordinal_words, w) {
            return Some(n);
        }
        let digits: String = w.chars().take_while(|c| c.is_ascii_digit()).collect();
        let suffix = &w[digits.len()..];
        if !digits.is_empty() && self.ordinal_suffixes.contains(&suffix) {
            return digits.parse().ok();
        }
        None
    }

    /// "q3" -> 3.
    pub fn quarter(&self, w: &str) -> Option<u32> {
        for p in self.quarter_prefixes {
            if let Some(rest) = w.strip_prefix(p) {
                if let Ok(q) = rest.parse::<u32>() {
                    if (1..=4).contains(&q) {
                        return Some(q);
                    }
                }
            }
        }
        None
    }

    /// Strip an am/pm marker: "3pm" -> ("3", Some(true)), "7:30" -> ("7:30", None).
    pub fn strip_meridiem<'a>(&self, w: &'a str) -> (&'a str, Option<bool>) {
        for s in self.pm_words {
            if let Some(rest) = w.strip_suffix(s) {
                return (rest, Some(true));
            }
        }
        for s in self.am_words {
            if let Some(rest) = w.strip_suffix(s) {
                return (rest, Some(false));
            }
        }
        (w, None)
    }
}

pub static EN: Locale = Locale {
    name: "en",
    weekdays: &[
        ("monday", Weekday::Mon),
        ("mon", Weekday::Mon),
        ("tuesday", Weekday::Tue),
        ("tue", Weekday::Tue),
        ("tues", Weekday::Tue),
        ("wednesday", Weekday::Wed),
        ("wed", Weekday::Wed),
        ("thursday", Weekday::Thu),
        ("thu", Weekday::Thu),
        ("thur", Weekday::Thu),
        ("thurs", Weekday::Thu),
        ("friday", Weekday::Fri),
        ("fri", Weekday::Fri),
        ("saturday", Weekday::Sat),
        ("sat", Weekday::Sat),
        ("sunday", Weekday::Sun),
        ("sun", Weekday::Sun),
    ],
    months: &[
        ("january", 1),
        ("jan", 1),
        ("february", 2),
        ("feb", 2),
        ("march", 3),
        ("mar", 3),
        ("april", 4),
        ("apr", 4),
        ("may", 5),
        ("june", 6),
        ("jun", 6),
        ("july", 7),
        ("jul", 7),
        ("august", 8),
        ("aug", 8),
        ("september", 9),
        ("sep", 9),
        ("sept", 9),
        ("october", 10),
        ("oct", 10),
        ("november", 11),
        ("nov", 11),
        ("december", 12),
        ("dec", 12),
    ],
    units: &[
        ("minute", Unit::Minute),
        ("minutes", Unit::Minute),
        ("min", Unit::Minute),
        ("mins", Unit::Minute),
        ("hour", Unit::Hour),
        ("hours", Unit::Hour),
        ("hr", Unit::Hour),
        ("hrs", Unit::Hour),
        ("day", Unit::Day),
        ("days", Unit::Day),
        ("week", Unit::Week),
        ("weeks", Unit::Week),
        ("wk", Unit::Week),
        ("wks", Unit::Week),
        ("month", Unit::Month),
        ("months", Unit::Month),
        ("quarter", Unit::Quarter),
        ("quarters", Unit::Quarter),
        ("year", Unit::Year),
        ("years", Unit::Year),
        ("yr", Unit::Year),
        ("yrs", Unit::Year),
        ("weekend", Unit::Weekend),
        ("weekends", Unit::Weekend),
    ],
    keywords: &[
        ("next", Kw::Next),
        ("upcoming", Kw::Next),
        ("last", Kw::Last),
        ("previous", Kw::Last),
        ("past", Kw::Last),
        ("this", Kw::This),
        ("current", Kw::This),
        ("after", Kw::After),
        ("following", Kw::After),
        ("before", Kw::Before),
        ("preceding", Kw::Before),
        ("sometime", Kw::Sometime),
        ("somewhere", Kw::Sometime),
        ("around", Kw::Sometime),
        ("early", Kw::Early),
        ("mid", Kw::Mid),
        ("middle", Kw::Mid),
        ("late", Kw::Late),
        ("end", Kw::End),
        ("start", Kw::Start),
        ("beginning", Kw::Start),
        ("of", Kw::Of),
        ("in", Kw::In),
        ("within", Kw::In),
        ("ago", Kw::Ago),
        ("from", Kw::From),
        ("now", Kw::Now),
        ("today", Kw::Today),
        ("tomorrow", Kw::Tomorrow),
        ("yesterday", Kw::Yesterday),
        ("business", Kw::Business),
        ("working", Kw::Business),
    ],
    number_words: &[
        ("a", 1),
        ("an", 1),
        ("one", 1),
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
    ],
    ordinal_words: &[
        ("first", 1),
        ("second", 2),
        ("third", 3),
        ("fourth", 4),
        ("fifth", 5),
    ],
    ordinal_suffixes: &["st", "nd", "rd", "th"],
    quarter_prefixes: &["q"],
    fillers: &["the", "on", "at", "for"],
    end_of_shorthand: &[
        ("eow", Unit::Week),
        ("eom", Unit::Month),
        ("eoq", Unit::Quarter),
        ("eoy", Unit::Year),
    ],
    am_words: &["am"],
    pm_words: &["pm"],
    special_times: &[("noon", (12, 0)), ("midnight", (0, 0))],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenize::{tokenize, Tok};

    /// A miniature Spanish-flavoured locale proving the seam: new language,
    /// zero grammar changes.
    static ES_MINI: Locale = Locale {
        name: "es-mini",
        weekdays: &[("viernes", Weekday::Fri), ("martes", Weekday::Tue)],
        months: &[("agosto", 8)],
        units: &[("semana", Unit::Week), ("dias", Unit::Day)],
        keywords: &[
            ("proximo", Kw::Next),
            ("proxima", Kw::Next),
            ("en", Kw::In),
            ("despues", Kw::After),
            ("de", Kw::Of),
        ],
        number_words: &[("tres", 3)],
        ordinal_words: &[("primero", 1)],
        ordinal_suffixes: &[],
        quarter_prefixes: &["t"],
        fillers: &["el", "la"],
        end_of_shorthand: &[],
        am_words: &[],
        pm_words: &[],
        special_times: &[("mediodia", (12, 0))],
    };

    #[test]
    fn spanish_mini_locale_tokenizes_without_grammar_changes() {
        let toks = tokenize("el proximo viernes", &[], &ES_MINI);
        assert_eq!(toks, vec![Tok::Kw(Kw::Next), Tok::Weekday(Weekday::Fri)]);
        let toks = tokenize("en tres dias", &[], &ES_MINI);
        assert_eq!(
            toks,
            vec![Tok::Kw(Kw::In), Tok::Num(3), Tok::Unit(Unit::Day)]
        );
    }

    #[test]
    fn spanish_mini_locale_parses_end_to_end() {
        use crate::config::Cfg;
        use crate::types::{Outcome, Value};
        let cfg = Cfg {
            locale: &ES_MINI,
            ..Cfg::default()
        };
        let now = "2026-07-12T15:30:00".parse().unwrap();
        let anchors = std::collections::HashMap::new();
        let out = crate::parse_str("el proximo viernes", now, &anchors, &cfg, &|_| false);
        match out {
            Outcome::Resolved(r) => match r.value {
                Value::Instant { when } => {
                    assert_eq!(when.to_string(), "2026-07-17 09:00:00")
                }
                _ => panic!("expected instant"),
            },
            other => panic!("expected resolved, got {other:?}"),
        }
    }

    #[test]
    fn english_tables_have_no_duplicate_keys() {
        fn dup<T>(t: &[(&str, T)]) -> Option<String> {
            let mut seen = std::collections::HashSet::new();
            t.iter()
                .find(|(k, _)| !seen.insert(*k))
                .map(|(k, _)| k.to_string())
        }
        assert_eq!(dup(EN.weekdays), None);
        assert_eq!(dup(EN.months), None);
        assert_eq!(dup(EN.units), None);
        assert_eq!(dup(EN.keywords), None);
        assert_eq!(dup(EN.number_words), None);
    }
}
