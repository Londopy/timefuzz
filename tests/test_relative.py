from datetime import datetime, timedelta

import pytest

import timefuzz as tf
from timefuzz import Instant, Range


def test_now(now):
    r = tf.parse("now", now=now)
    assert isinstance(r, Instant)
    assert r.when == now
    assert r.confidence == 1.0


def test_today_uses_default_time(now):
    r = tf.parse("today", now=now)
    assert r.when == datetime(2026, 7, 12, 9, 0)


def test_tomorrow_and_yesterday(now):
    assert tf.parse("tomorrow", now=now).when == datetime(2026, 7, 13, 9, 0)
    assert tf.parse("yesterday", now=now).when == datetime(2026, 7, 11, 9, 0)


@pytest.mark.parametrize(
    "text,delta",
    [
        ("in 3 days", timedelta(days=3)),
        ("in 2 weeks", timedelta(weeks=2)),
        ("in 5 hours", timedelta(hours=5)),
        ("in 45 minutes", timedelta(minutes=45)),
        ("3 days from now", timedelta(days=3)),
        ("2 days ago", timedelta(days=-2)),
        ("2 weeks ago", timedelta(weeks=-2)),
    ],
)
def test_arithmetic_offsets_keep_clock_time(now, text, delta):
    r = tf.parse(text, now=now)
    assert isinstance(r, Instant)
    assert r.when == now + delta
    assert r.confidence >= 0.9


def test_word_numbers(now):
    assert tf.parse("in three days", now=now).when == now + timedelta(days=3)
    assert tf.parse("in a week", now=now).when == now + timedelta(weeks=1)


def test_in_n_months_calendar_arithmetic(now):
    r = tf.parse("in 18 months", now=now)
    assert r.when == datetime(2028, 1, 12, 15, 30)


def test_month_end_clamping():
    # Jan 31 + 1 month clamps to Feb 29 (2024 is a leap year).
    r = tf.parse("in 1 month", now=datetime(2024, 1, 31, 12, 0))
    assert r.when == datetime(2024, 2, 29, 12, 0)


def test_next_week_is_a_range(now):
    r = tf.parse("next week", now=now)
    assert isinstance(r, Range)
    assert r.start == datetime(2026, 7, 13, 0, 0)
    assert r.end == datetime(2026, 7, 19, 23, 59, 59)


def test_this_month_range(now):
    r = tf.parse("this month", now=now)
    assert r.start == datetime(2026, 7, 1)
    assert r.end == datetime(2026, 7, 31, 23, 59, 59)


def test_last_quarter_range(now):
    r = tf.parse("last quarter", now=now)
    assert r.start == datetime(2026, 4, 1)
    assert r.end == datetime(2026, 6, 30, 23, 59, 59)


def test_unparseable_raises():
    with pytest.raises(tf.ParseError):
        tf.parse("the heat death of the universe")


def test_empty_input_raises():
    with pytest.raises(tf.ParseError):
        tf.parse("   ")
