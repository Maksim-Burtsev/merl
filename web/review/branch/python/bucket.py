from dataclasses import dataclass
from math import ceil
from typing import TYPE_CHECKING

from burst import burst_for

if TYPE_CHECKING:
    from plans import Limits


@dataclass
class TokenBucket:
    """Token bucket for one API key. Times are Unix seconds."""

    capacity: int
    refill_per_second: float
    tokens: float
    updated_at: float

    @classmethod
    def for_plan(cls, plan: "Limits", now: float) -> "TokenBucket":
        burst = burst_for(plan)
        return cls(
            capacity=burst,
            refill_per_second=plan.rate_per_minute / 60,
            tokens=burst,
            updated_at=now,
        )

    def take(self, now: float) -> bool:
        self._refill(now)
        if self.tokens < 1:
            return False
        self.tokens -= 1
        return True

    def retry_after(self, now: float) -> float:
        self._refill(now)
        missing = 1 - self.tokens
        if missing <= 0:
            return 0.0
        return ceil(missing / self.refill_per_second)

    def _refill(self, now: float) -> None:
        elapsed = max(0.0, now - self.updated_at)
        self.tokens = min(self.capacity, self.tokens + elapsed * self.refill_per_second)
        self.updated_at = now

    def to_fields(self) -> dict[str, str]:
        return {
            "capacity": str(self.capacity),
            "refill_per_second": str(self.refill_per_second),
            "tokens": str(self.tokens),
            "updated_at": str(self.updated_at),
        }

    @classmethod
    def from_fields(cls, fields: dict[str, str]) -> "TokenBucket":
        return cls(
            capacity=int(fields["capacity"]),
            refill_per_second=float(fields["refill_per_second"]),
            tokens=float(fields["tokens"]),
            updated_at=float(fields["updated_at"]),
        )
