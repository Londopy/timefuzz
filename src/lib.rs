//! timefuzz — fuzzy, natural-language time parsing with confidence scores.
//!
//! Rust core; thin Python API via PyO3 (see `python/timefuzz/__init__.py`).

pub mod confidence;
pub mod config;
pub mod dates;
pub mod grammar;
pub mod locale;
pub mod tokenize;
pub mod types;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use config::{Cfg, Ctx};
use std::collections::HashMap;
use types::Outcome;

/// Pure-Rust entry point, used by the Python binding, tests, and benches.
pub fn parse_str(
    text: &str,
    now: NaiveDateTime,
    anchors: &HashMap<String, NaiveDate>,
    cfg: &Cfg,
    is_holiday: &dyn Fn(NaiveDate) -> bool,
) -> Outcome {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Outcome::NoParse {
            reason: "empty input".into(),
        };
    }
    let names: Vec<String> = anchors.keys().cloned().collect();
    let tokens = tokenize::tokenize(trimmed, &names, cfg.locale);
    let ctx = Ctx {
        now,
        cfg,
        anchors,
        is_holiday,
        tod: cfg.default_time,
        explicit_time: false,
    };
    grammar::resolve(trimmed, &tokens, &ctx)
}

// ---------------- PyO3 binding ----------------

/// The `#[pymodule]`/`#[pyfunction]` expansion of pyo3 0.22 trips
/// `clippy::useless_conversion` on recent clippy; scoped allow.
#[allow(clippy::useless_conversion)]
mod py_binding {
    use super::*;

    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    fn weekday_from_u8(n: u8) -> PyResult<Weekday> {
        Ok(match n {
            0 => Weekday::Mon,
            1 => Weekday::Tue,
            2 => Weekday::Wed,
            3 => Weekday::Thu,
            4 => Weekday::Fri,
            5 => Weekday::Sat,
            6 => Weekday::Sun,
            _ => {
                return Err(PyValueError::new_err(
                    "week_start must be 0 (Mon) .. 6 (Sun)",
                ))
            }
        })
    }

    /// Low-level binding. `now` is an ISO datetime string, anchor values are ISO
    /// dates, `holidays` is an optional callable `(iso_date_str) -> bool`.
    /// Returns the outcome as a JSON string (see `types::WireOutcome`).
    #[pyfunction]
    #[pyo3(signature = (text, now, anchors, default_time, week_start, next_skips_today, holidays=None))]
    #[allow(clippy::too_many_arguments)]
    fn parse_raw(
        py: Python<'_>,
        text: &str,
        now: &str,
        anchors: HashMap<String, String>,
        default_time: (u32, u32, u32),
        week_start: u8,
        next_skips_today: bool,
        holidays: Option<Py<PyAny>>,
    ) -> PyResult<String> {
        let now: NaiveDateTime = now
            .parse()
            .map_err(|e| PyValueError::new_err(format!("invalid `now` datetime: {e}")))?;

        let mut anchor_dates: HashMap<String, NaiveDate> = HashMap::new();
        for (k, v) in &anchors {
            let d: NaiveDate = v.parse().map_err(|e| {
                PyValueError::new_err(format!("invalid anchor date for {k:?}: {e}"))
            })?;
            anchor_dates.insert(k.trim().to_lowercase(), d);
        }

        let (h, m, s) = default_time;
        let cfg = Cfg {
            default_time: NaiveTime::from_hms_opt(h, m, s)
                .ok_or_else(|| PyValueError::new_err("invalid default_time"))?,
            week_start: weekday_from_u8(week_start)?,
            next_skips_today,
            locale: &crate::locale::EN,
        };

        let is_holiday = |d: NaiveDate| -> bool {
            match &holidays {
                Some(cb) => cb
                    .bind(py)
                    .call1((d.to_string(),))
                    .and_then(|r| r.extract::<bool>())
                    .unwrap_or(false),
                None => false,
            }
        };

        let outcome = parse_str(text, now, &anchor_dates, &cfg, &is_holiday);
        Ok(outcome.to_json())
    }

    #[pymodule]
    fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(parse_raw, m)?)?;
        m.add("__version__", env!("CARGO_PKG_VERSION"))?;
        Ok(())
    }
}
