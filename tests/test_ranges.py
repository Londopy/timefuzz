"""Vague spans, range invariants, timezone attachment, and property tests."""

from datetime import datetime, timedelta
from zoneinfo import ZoneInfo

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

import timefuzz as tf
from timefuzz import Ambiguous, Config, Instant, Range

from conftest import NOW


def test_sometime_next_week_matches_spec_example(now):
    r = tf.parse("sometime next week", now=now)
    assert isinstance(r, Range)
    assert r.start == datetime(2026, 7, 13)
    assert r.end == datetime(2026, 7, 19, 23, 59, 59)
    assert r.confidence == pytest.approx(0.8)
    assert "week after this one" in r.interpretation


def test_sometime_in_august(now):
    r = tf.parse("sometime in august", now=now)
    assert r.start == datetime(2026, 8, 1)
    assert r.end == datetime(2026, 8, 31, 23, 59, 59)
    assert r.confidence <= 0.8  # month name assumes the year


def test_month_name_rolls_to_next_year(now):
    r = tf.parse("sometime in march", now=now)  # March 2026 has passed
    assert r.start == datetime(2027, 3, 1)
    assert "passed" in r.interpretation


def test_early_mid_late_thirds_of_month(now):
    early = tf.parse("early next month", now=now)
    mid = tf.parse("mid next month", now=now)
    late = tf.parse("late next month", now=now)
    assert early.start == datetime(2026, 8, 1)
    assert early.end == datetime(2026, 8, 10, 23, 59, 59)
    assert mid.start == datetime(2026, 8, 11)
    assert mid.end == datetime(2026, 8, 20, 23, 59, 59)
    assert late.start == datetime(2026, 8, 21)
    assert late.end == datetime(2026, 8, 31, 23, 59, 59)
    # Thirds tile the month without gaps or overlap.
    assert early.end + timedelta(seconds=1) == mid.start
    assert mid.end + timedelta(seconds=1) == late.start


def test_mid_next_week_is_midweek(now):
    r = tf.parse("mid next week", now=now)
    assert r.start == datetime(2026, 7, 15)  # Wednesday
    assert r.end == datetime(2026, 7, 16, 23, 59, 59)  # Thursday


def test_late_this_year(now):
    r = tf.parse("late this year", now=now)
    assert r.start == datetime(2026, 9, 1)
    assert r.end == datetime(2026, 12, 31, 23, 59, 59)


def test_vague_confidence_capped(now):
    for text in ("sometime next week", "early next month", "late this year"):
        r = tf.parse(text, now=now)
        assert r.confidence <= 0.8, text


# ---- timezone handling ----


def test_naive_by_default(now):
    r = tf.parse("next friday", now=now)
    assert r.when.tzinfo is None


def test_tz_aware_opt_in(now):
    cfg = Config(tz=ZoneInfo("America/New_York"))
    r = tf.parse("next friday", now=now, config=cfg)
    assert r.when.tzinfo is not None
    assert r.when.hour == 9  # wall-clock time preserved


def test_wall_clock_stable_across_dst_boundary():
    # US DST springs forward 2026-03-08; "tomorrow at 09:00" stays 09:00 wall time.
    cfg = Config(tz=ZoneInfo("America/New_York"))
    r = tf.parse("tomorrow", now=datetime(2026, 3, 7, 12, 0), config=cfg)
    assert r.when == datetime(2026, 3, 8, 9, 0, tzinfo=ZoneInfo("America/New_York"))
    assert r.when.utcoffset() == timedelta(hours=-4)  # 09:00 is post-jump, EDT


def test_aware_now_is_converted_to_config_tz():
    cfg = Config(tz=ZoneInfo("America/New_York"))
    # 01:00 UTC on Jul 13 is 21:00 Jul 12 in New York -> "tomorrow" is Jul 13.
    now = datetime(2026, 7, 13, 1, 0, tzinfo=ZoneInfo("UTC"))
    r = tf.parse("tomorrow", now=now, config=cfg)
    assert (r.when.year, r.when.month, r.when.day) == (2026, 7, 13)


# ---- property tests ----

PHRASES = st.sampled_from(
    [
        "now", "today", "tomorrow", "yesterday", "next friday", "this tuesday",
        "last monday", "friday", "sunday", "in 3 days", "2 weeks ago",
        "next week", "this month", "last quarter", "next year",
        "sometime next week", "early march", "mid q3", "late next month",
        "end of q3", "start of q4", "end of month", "next business day",
        "in 5 business days", "2nd monday of march", "last friday of october",
    ]
)


@given(text=PHRASES)
@settings(max_examples=60, deadline=None)
def test_confidence_always_in_unit_interval(text):
    r = tf.parse(text, now=NOW)
    results = r.candidates if isinstance(r, Ambiguous) else [r]
    for c in results:
        assert 0.0 <= c.confidence <= 1.0


@given(text=PHRASES)
@settings(max_examples=60, deadline=None)
def test_ranges_are_ordered(text):
    r = tf.parse(text, now=NOW)
    results = r.candidates if isinstance(r, Ambiguous) else [r]
    for c in results:
        if isinstance(c, Range):
            assert c.start <= c.end


@given(n=st.integers(min_value=1, max_value=365))
@settings(max_examples=40, deadline=None)
def test_roundtrip_in_n_days(n):
    # parse(fmt(x)) == x for round-trippable phrasings.
    r = tf.parse(f"in {n} days", now=NOW)
    assert isinstance(r, Instant)
    assert r.when == NOW + timedelta(days=n)


@given(n=st.integers(min_value=1, max_value=52))
@settings(max_examples=30, deadline=None)
def test_roundtrip_weeks_ago(n):
    r = tf.parse(f"{n} weeks ago", now=NOW)
    assert r.when == NOW - timedelta(weeks=n)
