"""Configuration: where the notes live and how many are listed at once."""

import json
from dataclasses import dataclass
from pathlib import Path

DEFAULT_PATH = Path.home() / ".notes.json"


@dataclass
class Config:
    """Runtime settings of the notes CLI."""

    path: Path = DEFAULT_PATH
    page_size: int = 20


def load_config(path: Path | None = None) -> Config:
    """Read the settings from a JSON file, falling back to the defaults."""
    if path is None or not path.exists():
        return Config()
    raw = json.loads(path.read_text(encoding="utf-8"))
    return Config(
        path=Path(raw.get("path", DEFAULT_PATH)),
        page_size=int(raw.get("page_size", 20)),
    )
