"""timefuzz — fuzzy, natural-language time parsing with confidence scores.

Resolves phrases like "sometime next week", "the tuesday after my birthday",
or "end of q3" into concrete datetimes or ranges, plus a confidence score and
the interpretation the parser chose.

    >>> import timefuzz as tf
    >>> tf.parse("next friday")
    Instant(when=datetime(...), confidence=0.95, interpretation='next Friday, default 09:00')
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from datetime import date, datetime, time, tzinfo
from enum import IntEnum
from typing import Callable

from timefuzz._core import __version__ as _core_version
from timefuzz._core import parse_raw as _parse_raw

__version__ = _core_version
__all__ = [
    "parse",
    "Config",
    "Weekday",
    "Instant",
    "Range",
    "Ambiguous",
    "TimefuzzError",
    "ParseError",
    "__version__",
]


class TimefuzzError(Exception):
    """Base class for timefuzz errors."""


class ParseError(TimefuzzError):
    """The input did not match any grammar rule."""


class Weekday(IntEnum):
    MON = 0
    TUE = 1
    WED = 2
    THU = 3
    FRI = 4
    SAT = 5
    SUN = 6


@dataclass(frozen=True)
class Config:
    """Parser configuration.

    Attributes:
        default_time: Time of day applied to date-only results.
        week_start: First day of the week (affects "this <weekday>", week spans).
        next_skips_today: Whether "next friday" said on a Friday skips today.
        tz: If set, `now` is interpreted in this zone and results carry it.
            Naive by default.
        holidays: Optional predicate ``date -> bool``; dates returning True are
            skipped by business-day rules (weekends are always skipped).
    """

    default_time: time = time(9, 0)
    week_start: Weekday = Weekday.MON
    next_skips_today: bool = True
    tz: tzinfo | None = None
    holidays: Callable[[date], bool] | None = None


@dataclass(frozen=True)
class Instant:
    """A single resolved point in time."""

    when: datetime
    confidence: float
    interpretation: str


@dataclass(frozen=True)
class Range:
    """An inclusive resolved span: ``start <= end``."""

    start: datetime
    end: datetime
    confidence: float
    interpretation: str


@dataclass(frozen=True)
class Ambiguous:
    """The phrase has multiple viable readings (or references an unknown anchor).

    ``candidates`` may be empty, e.g. for an unknown anchor; ``reason`` always
    explains why the parse is ambiguous.
    """

    candidates: list[Instant | Range] = field(default_factory=list)
    reason: str = ""


_DEFAULT_CONFIG = Config()


def _attach_tz(dt: datetime, tz: tzinfo | None) -> datetime:
    return dt.replace(tzinfo=tz) if tz is not None else dt


def _decode_candidate(obj: dict, tz: tzinfo | None) -> Instant | Range:
    conf = round(obj["confidence"], 4)  # hide f32 representation noise
    if obj["kind"] == "instant":
        return Instant(
            when=_attach_tz(datetime.fromisoformat(obj["when"]), tz),
            confidence=conf,
            interpretation=obj["interpretation"],
        )
    return Range(
        start=_attach_tz(datetime.fromisoformat(obj["start"]), tz),
        end=_attach_tz(datetime.fromisoformat(obj["end"]), tz),
        confidence=conf,
        interpretation=obj["interpretation"],
    )


def parse(
    text: str,
    now: datetime | None = None,
    anchors: dict[str, date] | None = None,
    config: Config | None = None,
) -> Instant | Range | Ambiguous:
    """Parse a fuzzy time phrase.

    Args:
        text: The phrase, e.g. ``"sometime next week"``.
        now: Reference moment; defaults to the current local time. If tz-aware
            and ``config.tz`` is set, it is converted to that zone first.
        anchors: Named dates for anchored phrases, e.g.
            ``{"my birthday": date(2026, 8, 3)}``. Matching is case-insensitive
            substring matching on the anchor name.
        config: A :class:`Config`; defaults to ``Config()``.

    Returns:
        :class:`Instant`, :class:`Range`, or :class:`Ambiguous`.

    Raises:
        ParseError: If no grammar rule matches the input.
        ValueError: If ``now`` or an anchor date is invalid.
    """
    cfg = config if config is not None else _DEFAULT_CONFIG

    if now is None:
        now = datetime.now(cfg.tz) if cfg.tz is not None else datetime.now()
    if now.tzinfo is not None:
        if cfg.tz is not None:
            now = now.astimezone(cfg.tz)
        now = now.replace(tzinfo=None)  # core math is naive wall-clock

    anchor_map = {
        str(k): v.isoformat() for k, v in (anchors or {}).items()
    }

    holidays_cb = None
    if cfg.holidays is not None:
        user_cb = cfg.holidays

        def holidays_cb(iso: str) -> bool:  # called from Rust with an ISO date
            return bool(user_cb(date.fromisoformat(iso)))

    raw = _parse_raw(
        text,
        now.replace(microsecond=0).isoformat(),
        anchor_map,
        (cfg.default_time.hour, cfg.default_time.minute, cfg.default_time.second),
        int(cfg.week_start),
        cfg.next_skips_today,
        holidays_cb,
    )
    obj = json.loads(raw)

    kind = obj["kind"]
    if kind in ("instant", "range"):
        return _decode_candidate(obj, cfg.tz)
    if kind == "ambiguous":
        return Ambiguous(
            candidates=[_decode_candidate(c, cfg.tz) for c in obj["candidates"]],
            reason=obj["reason"],
        )
    raise ParseError(obj["reason"])
