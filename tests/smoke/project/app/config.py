"""Settings, read from the environment once at startup."""
import os
from dataclasses import dataclass


@dataclass(frozen=True)
class Settings:
    database_url: str
    api_token: str
    page_size: int = 50
    refund_window_days: int = 30


def load() -> Settings:
    return Settings(
        database_url=os.environ.get("DATABASE_URL", "postgresql://orders@localhost/orders"),
        api_token=os.environ["API_TOKEN"],
        page_size=int(os.environ.get("PAGE_SIZE", "50")),
    )
