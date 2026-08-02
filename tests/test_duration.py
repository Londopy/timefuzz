"""Bare durations (v0.4): "<qty> <unit>" with no leading "in".

The corpus pins the resolved values; this file pins the *properties* a
corpus line cannot express — that the new rule does not collide with the
marked forms, does not introduce ambiguity, and stays narrow enough that
garbage still fails.
"""

from datetime import date, datetime, timedelta

import pytest

import timefuzz as tf
from timefuzz import Ambiguous, Instant, Range


class TestBareDurationsMatchTheMarkedForm:
    """"30 minutes" and "in 30 minutes" are the same request."""

    @pytest.mark.parametrize(
        "bare,marked",
        [
            ("30 minutes", "in 30 minutes"),
            ("2 hrs", "in 2 hours"),
            ("3 days", "in 3 days"),
            ("a week", "in a week"),
            ("6 months", "in 6 months"),
            ("3 business days", "in 3 business days"),
        ],
    )
    def test_same_instant(self, now, bare, marked):
        assert tf.parse(bare, now=now).when == tf.parse(marked, now=now).when

    @pytest.mark.parametrize("text", ["30 minutes", "2 hrs", "3 days", "a week"])
    def test_never_ambiguous(self, now, text):
        """Overlapping with relative.rs must dedup, not produce candidates."""
        assert isinstance(tf.parse(text, now=now), Instant)

    def test_marked_form_keeps_its_own_wording(self, now):
        """The older rule runs first, so its interpretation survives dedup."""
        assert "from today" in tf.parse("in 3 business days", now=now).interpretation
        assert "from now" in tf.parse("3 business days", now=now).interpretation


class TestArithmetic:
    def test_offsets_keep_the_clock(self, now):
        assert tf.parse("3 days", now=now).when == now + timedelta(days=3)
        assert tf.parse("90 minutes", now=now).when == now + timedelta(minutes=90)

    def test_months_use_calendar_arithmetic_with_clamping(self):
        # 1 month from Jan 31 clamps to the last day of February.
        r = tf.parse("1 month", now=datetime(2026, 1, 31, 9, 0))
        assert r.when.date() == date(2026, 2, 28)

    def test_business_days_skip_weekends(self, now):
        # now is a Sunday; three business days lands on Wednesday.
        assert tf.parse("3 business days", now=now).when.date() == date(2026, 7, 15)

    def test_business_days_respect_the_holidays_hook(self, now):
        cfg = tf.Config(holidays=lambda d: d == date(2026, 7, 14))
        r = tf.parse("3 business days", now=now, config=cfg)
        assert r.when.date() == date(2026, 7, 16)

    def test_distant_horizon_is_penalised(self, now):
        near = tf.parse("2 years", now=now)
        far = tf.parse("400 months", now=now)
        assert far.confidence < near.confidence
        assert "distant horizon" in far.interpretation


class TestHalf:
    @pytest.mark.parametrize(
        "text,delta",
        [
            ("half a minute", timedelta(seconds=30)),
            ("half an hour", timedelta(minutes=30)),
            ("half hour", timedelta(minutes=30)),
            ("half a day", timedelta(hours=12)),
            ("half a week", timedelta(days=3, hours=12)),
        ],
    )
    def test_fixed_length_units_halve_exactly(self, now, text, delta):
        assert tf.parse(text, now=now).when == now + delta

    def test_half_a_year_is_six_months(self, now):
        assert tf.parse("half a year", now=now).when == tf.parse("6 months", now=now).when

    def test_half_is_exact_not_fuzzy(self, now):
        """A half is a precise amount, so it scores like an exact count."""
        assert tf.parse("half an hour", now=now).confidence == (
            tf.parse("30 minutes", now=now).confidence
        )

    @pytest.mark.parametrize("text", ["half a month", "half a quarter"])
    def test_calendar_units_are_not_halved(self, text, now):
        """No exact meaning, so it fails rather than silently rounding."""
        with pytest.raises(tf.ParseError):
            tf.parse(text, now=now)


class TestFuzzyQuantities:
    @pytest.mark.parametrize(
        "text,count", [("a couple hours", 2), ("a few hours", 3), ("several hours", 3)]
    )
    def test_counts(self, now, text, count):
        assert tf.parse(text, now=now).when == now + timedelta(hours=count)

    def test_article_is_optional(self, now):
        for text in ("a couple hours", "couple hours", "a couple of hours"):
            assert tf.parse(text, now=now).when == now + timedelta(hours=2)

    def test_scored_below_an_exact_count(self, now):
        fuzzy = tf.parse("a couple hours", now=now)
        exact = tf.parse("2 hours", now=now)
        assert fuzzy.confidence < exact.confidence
        assert "approximately" in fuzzy.interpretation

    def test_low_enough_to_trigger_confirmation(self, now):
        """The README's worked example auto-schedules at >= 0.8."""
        assert tf.parse("a couple hours", now=now).confidence < 0.8


class TestLeadingWords:
    """Only this rule tolerates unknown words, and only leading ones."""

    def test_reads_past_a_leading_word(self, now):
        r = tf.parse("back in a couple hours", now=now)
        assert r.when == now + timedelta(hours=2)
        assert '"back"' in r.interpretation

    def test_costs_confidence(self, now):
        assert (
            tf.parse("gimme 5 mins", now=now).confidence
            < tf.parse("5 mins", now=now).confidence
        )

    def test_names_every_word_it_ignored(self, now):
        r = tf.parse("ok cool 5 mins", now=now)
        assert '"ok"' in r.interpretation and '"cool"' in r.interpretation

    def test_words_after_the_quantity_are_not_ignored(self, now):
        with pytest.raises(tf.ParseError):
            tf.parse("3 whole days", now=now)

    def test_trailing_words_are_not_ignored(self, now):
        with pytest.raises(tf.ParseError):
            tf.parse("5 mins ok", now=now)

    def test_other_rules_stay_strict(self, now):
        """The tolerance must not leak into the rest of the grammar."""
        with pytest.raises(tf.ParseError):
            tf.parse("blah next friday", now=now)


class TestStaysNarrow:
    @pytest.mark.parametrize(
        "text",
        [
            "week",  # a bare unit is a period, not a duration
            "month",
            "weekend",
            "half",
            "a couple",
            "5 apples",
            "2 weekends",
            "an hour and a half",
            "90 seconds",
        ],
    )
    def test_still_unparseable(self, text, now):
        with pytest.raises(tf.ParseError):
            tf.parse(text, now=now)

    @pytest.mark.parametrize(
        "text,kind",
        [
            ("next week", Range),
            ("last quarter", Range),
            ("mid month", Range),
            ("sometime next week", Range),
            ("end of month", Instant),
            ("eom", Instant),
            ("3 days ago", Instant),
            ("2 weeks from now", Instant),
            ("5 business days ago", Instant),
            ("first business day of next month", Instant),
        ],
    )
    def test_existing_phrases_keep_their_shape(self, now, text, kind):
        assert isinstance(tf.parse(text, now=now), kind)

    def test_anchored_phrases_are_untouched(self, now):
        anchors = {"my birthday": date(2026, 8, 3)}
        r = tf.parse("2 weeks after my birthday", now=now, anchors=anchors)
        assert r.when.date() == date(2026, 8, 17)


class TestTrailingClockTime:
    def test_day_granular_durations_take_the_time(self, now):
        assert tf.parse("3 days at 5pm", now=now).when == datetime(2026, 7, 15, 17, 0)

    def test_sub_day_durations_keep_their_own_clock(self, now):
        assert tf.parse("30 minutes at 5pm", now=now).when == datetime(2026, 7, 12, 16, 0)
