"""The one thing this project stores: a note."""

from dataclasses import dataclass, field
from datetime import date
from typing import Protocol


@dataclass
class Note:
    """A single note, identified by an integer id."""

    id: int
    text: str
    tags: list[str] = field(default_factory=list)
    created: date = field(default_factory=date.today)

    def to_dict(self) -> dict:
        """The note as plain, JSON-friendly data."""
        return {
            "id": self.id,
            "text": self.text,
            "tags": self.tags,
            "created": self.created.isoformat(),
        }

    @classmethod
    def from_dict(cls, raw: dict) -> "Note":
        """Rebuild a note from what `to_dict` wrote."""
        return cls(
            id=int(raw["id"]),
            text=raw["text"],
            tags=list(raw.get("tags", [])),
            created=date.fromisoformat(raw["created"]),
        )

    def has_tags(self, wanted: set[str]) -> bool:
        """Whether the note carries every tag in `wanted`, as typed."""
        return wanted <= set(self.tags)

    def summary(self, width: int = 40) -> str:
        """One line for a listing: the text, cut to `width` characters."""
        text = self.text if len(self.text) <= width else self.text[: width - 1] + "…"
        tags = " ".join(f"#{tag}" for tag in sorted(self.tags))
        return f"{self.id:>4}  {text}  {tags}".rstrip()


class Formatter(Protocol):
    """How one note is turned into a line of output."""

    def render(self, note: Note) -> str: ...


class PlainFormatter:
    """The default: the note's own one-line summary."""

    def render(self, note: Note) -> str:
        return note.summary()
