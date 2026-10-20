"""Tests for the store: plain asserts over a file in a temporary directory."""

import json
from pathlib import Path

import pytest

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


def test_find_keeps_only_notes_with_every_tag(tmp_path: Path) -> None:
    store = store_in(tmp_path)
    store.add("standup notes", ["work"])
    store.add("quarterly plan", ["work", "plan"])
    store.add("holiday plan", ["home", "plan"])
    assert [n.text for n in store.find("", ["work"])] == ["standup notes", "quarterly plan"]
    assert [n.text for n in store.find("plan", ["work", "plan"])] == ["quarterly plan"]
    assert store.find("", ["nothing"]) == []


def test_find_without_tags_is_today_s_search(tmp_path: Path) -> None:
    store = store_in(tmp_path)
    store.add("Buy Milk", ["shop"])
    assert len(store.find("milk")) == 1


def test_tags_counts_each_tag_once_per_note(tmp_path: Path) -> None:
    store = store_in(tmp_path)
    store.add("a", ["work", "work"])
    store.add("b", ["work", "home"])
    assert store.tags() == {"home": 1, "work": 2}


def test_a_duplicate_id_on_disk_is_refused(tmp_path: Path) -> None:
    path = tmp_path / "notes.json"
    note = {"id": 1, "text": "x", "tags": [], "created": "2026-01-01"}
    path.write_text(json.dumps([note, note]), encoding="utf-8")
    with pytest.raises(ValueError, match="used twice"):
        NoteStore(Config(path=path))
