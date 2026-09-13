"""Storage: every note lives in one JSON file, read and written as a whole."""

import json

from config import Config, load_config
from models import Note


class NoteStore:
    """Reads and writes the notes of a single JSON file."""

    def __init__(self, config: Config | None = None) -> None:
        self.config = config or load_config()
        self.notes: list[Note] = self._read()

    def _read(self) -> list[Note]:
        """Load the notes from disk; a missing file means no notes yet."""
        if not self.config.path.exists():
            return []
        raw = json.loads(self.config.path.read_text(encoding="utf-8"))
        return [Note.from_dict(item) for item in raw]

    def _write(self) -> None:
        """Write every note back to disk."""
        raw = [note.to_dict() for note in self.notes]
        self.config.path.write_text(json.dumps(raw, indent=2), encoding="utf-8")

    def next_id(self) -> int:
        """One past the highest id in use."""
        return max((note.id for note in self.notes), default=0) + 1

    def add(self, text: str, tags: list[str] | None = None) -> Note:
        """Append a note and save the file."""
        note = Note(id=self.next_id(), text=text, tags=tags or [])
        self.notes.append(note)
        self._write()
        return note

    def remove(self, note_id: int) -> bool:
        """Delete the note with this id and report whether it existed."""
        keep = [note for note in self.notes if note.id != note_id]
        if len(keep) == len(self.notes):
            return False
        self.notes = keep
        self._write()
        return True

    def find(self, needle: str) -> list[Note]:
        """Every note whose text contains `needle`, case-insensitively."""
        lowered = needle.lower()
        return [note for note in self.notes if lowered in note.text.lower()]

    def page(self, number: int = 1) -> list[Note]:
        """One page of notes, newest first."""
        newest = sorted(self.notes, key=lambda note: note.created, reverse=True)
        start = (number - 1) * self.config.page_size
        return newest[start : start + self.config.page_size]
