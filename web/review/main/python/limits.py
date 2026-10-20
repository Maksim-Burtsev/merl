import math
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

from bucket import TokenBucket


@dataclass(frozen=True)
class Limits:
    rate_per_minute: int
    burst: int

    def new_bucket(self, now: float) -> TokenBucket:
        return TokenBucket(
            capacity=self.burst,
            refill_per_second=self.rate_per_minute / 60,
            tokens=self.burst,
            updated_at=now,
        )

    def idle_ttl(self) -> int:
        """Seconds until an untouched bucket is full again and can be dropped."""
        return math.ceil(self.burst * 60 / self.rate_per_minute) + 1


@dataclass(frozen=True)
class LimitsConfig:
    default: Limits
    overrides: dict[str, Limits] = field(default_factory=dict)

    def for_key(self, api_key: str) -> Limits:
        return self.overrides.get(api_key, self.default)


def load_config(path: Path = Path("config.toml")) -> LimitsConfig:
    with path.open("rb") as f:
        raw = tomllib.load(f)
    return LimitsConfig(
        default=Limits(**raw["default"]),
        overrides={key: Limits(**values) for key, values in raw.get("overrides", {}).items()},
    )
