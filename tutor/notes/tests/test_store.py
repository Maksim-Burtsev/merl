"""Tests for the store: plain asserts over a file in a temporary directory."""

from pathlib import Path

from config import Config
from store import NoteStore


def store_in(tmp_path: Path) -> NoteStore:
    """A store backed by a fresh file under `tmp_path`."""
    return NoteStore(Config(path=tmp_path / "notes.json"))


def test_add_assigns_increasing_ids(tmp_path: Path) -> None:
    store = store_in(tmp_path)
    first = store.add("buy milk")
    second = store.add("water the plants", ["home"])
    assert first.id == 1
    assert second.id == 2
    assert second.tags == ["home"]


def test_notes_survive_a_reload(tmp_path: Path) -> None:
    store_in(tmp_path).add("remember this")
    reopened = store_in(tmp_path)
    assert len(reopened.notes) == 1
    assert reopened.notes[0].text == "remember this"


def test_remove_reports_a_missing_id(tmp_path: Path) -> None:
    store = store_in(tmp_path)
    note = store.add("temporary")
    assert store.remove(note.id) is True
    assert store.remove(note.id) is False


def test_find_ignores_case(tmp_path: Path) -> None:
    store = store_in(tmp_path)
    store.add("Buy Milk")
    assert len(store.find("milk")) == 1
    assert store.find("bread") == []
