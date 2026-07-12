from datetime import date, datetime, time, tzinfo
from enum import IntEnum
from typing import Callable

__version__: str

class TimefuzzError(Exception): ...
class ParseError(TimefuzzError): ...

class Weekday(IntEnum):
    MON = 0
    TUE = 1
    WED = 2
    THU = 3
    FRI = 4
    SAT = 5
    SUN = 6

class Config:
    default_time: time
    week_start: Weekday
    next_skips_today: bool
    tz: tzinfo | None
    holidays: Callable[[date], bool] | None
    def __init__(
        self,
        default_time: time = ...,
        week_start: Weekday = ...,
        next_skips_today: bool = ...,
        tz: tzinfo | None = ...,
        holidays: Callable[[date], bool] | None = ...,
    ) -> None: ...

class Instant:
    when: datetime
    confidence: float
    interpretation: str

class Range:
    start: datetime
    end: datetime
    confidence: float
    interpretation: str

class Ambiguous:
    candidates: list[Instant | Range]
    reason: str

def parse(
    text: str,
    now: datetime | None = None,
    anchors: dict[str, date] | None = None,
    config: Config | None = None,
) -> Instant | Range | Ambiguous: ...
