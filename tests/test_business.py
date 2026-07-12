from datetime import date, datetime

import pytest

import timefuzz as tf
from timefuzz import Config, Instant, Range


def test_end_of_q3_matches_spec_example(now):
    r = tf.parse("end of q3", now=now)
    assert isinstance(r, Range)
    assert r.start == datetime(2026, 9, 1)
    assert r.end == datetime(2026, 9, 30, 23, 59, 59)
    assert r.confidence == pytest.approx(0.85)
    assert "last month of Q3" in r.interpretation


def test_end_of_past_quarter_rolls_forward(now):
    r = tf.parse("end of q1", now=now)  # Q1 2026 has passed
    assert r.start == datetime(2027, 3, 1)
    assert "assuming 2027" in r.interpretation


def test_start_of_q4(now):
    r = tf.parse("start of q4", now=now)
    assert r.start == datetime(2026, 10, 1)
    assert r.end == datetime(2026, 10, 31, 23, 59, 59)


def test_next_business_day_from_sunday(now):
    r = tf.parse("next business day", now=now)  # Sunday -> Monday
    assert isinstance(r, Instant)
    assert r.when == datetime(2026, 7, 13, 9, 0)


def test_next_business_day_skips_weekend():
    friday = datetime(2026, 7, 17, 10, 0)
    r = tf.parse("next business day", now=friday)
    assert r.when == datetime(2026, 7, 20, 9, 0)  # Monday


def test_holiday_hook(now):
    cfg = Config(holidays=lambda d: d == date(2026, 7, 13))
    r = tf.parse("next business day", now=now, config=cfg)
    assert r.when == datetime(2026, 7, 14, 9, 0)  # Monday is a holiday -> Tuesday


def test_in_n_business_days(now):
    r = tf.parse("in 3 business days", now=now)  # Sun + Mon/Tue/Wed
    assert r.when == datetime(2026, 7, 15, 9, 0)
    r = tf.parse("5 business days from now", now=now)
    assert r.when == datetime(2026, 7, 17, 9, 0)


def test_end_of_month_is_last_day(now):
    r = tf.parse("end of month", now=now)
    assert isinstance(r, Instant)
    assert r.when == datetime(2026, 7, 31, 9, 0)


def test_end_of_next_month(now):
    r = tf.parse("end of next month", now=now)
    assert r.when == datetime(2026, 8, 31, 9, 0)


def test_start_of_next_quarter(now):
    r = tf.parse("start of next quarter", now=now)
    assert r.when == datetime(2026, 10, 1, 9, 0)


def test_end_of_year(now):
    r = tf.parse("end of the year", now=now)
    assert r.when == datetime(2026, 12, 31, 9, 0)


def test_last_business_day_of_month(now):
    r = tf.parse("last business day of the month", now=now)
    assert r.when == datetime(2026, 7, 31, 9, 0)  # Jul 31 2026 is a Friday


def test_last_business_day_respects_holidays():
    cfg = Config(holidays=lambda d: d == date(2026, 7, 31))
    r = tf.parse("last business day of the month", now=datetime(2026, 7, 12), config=cfg)
    assert r.when == datetime(2026, 7, 30, 9, 0)


def test_ordinal_in_month(now):
    r = tf.parse("2nd monday of march", now=now)  # 2026's has passed -> 2027
    assert r.when == datetime(2027, 3, 8, 9, 0)
    assert "assuming 2027" in r.interpretation


def test_ordinal_with_explicit_year(now):
    r = tf.parse("2nd monday of march 2026", now=now)
    assert r.when == datetime(2026, 3, 9, 9, 0)


def test_last_weekday_of_month(now):
    r = tf.parse("last friday of october", now=now)
    assert r.when == datetime(2026, 10, 30, 9, 0)


def test_nonexistent_ordinal_raises(now):
    with pytest.raises(tf.ParseError, match="has no 5th Friday"):
        tf.parse("5th friday of february", now=now)
