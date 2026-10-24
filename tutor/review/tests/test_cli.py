"""Tests for the command line: run `main` and read what it printed."""

from pathlib import Path

import pytest

import cli
from config import Config
from store import NoteStore


@pytest.fixture
def store(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> NoteStore:
    """A store under `tmp_path` that `cli.main` opens instead of the real one."""
    config = Config(path=tmp_path / "notes.json")
    monkeypatch.setattr(cli, "load_config", lambda: config)
    return NoteStore(config)


def test_find_by_tag(store: NoteStore, capsys: pytest.CaptureFixture[str]) -> None:
    store.add("standup notes", ["work"])
    store.add("buy milk", ["shop"])
    assert cli.main(["find", "--tag", "work"]) == 0
    out = capsys.readouterr().out
    assert "standup notes" in out
    assert "buy milk" not in out


def test_find_with_nothing_found_fails(store: NoteStore) -> None:
    store.add("buy milk")
    assert cli.main(["find", "bread"]) == 1


def test_add_needs_text(store: NoteStore) -> None:
    assert cli.main(["add", "--tag", "work"]) == 2
    assert store.notes == []


def test_tags_lists_the_counts(store: NoteStore, capsys: pytest.CaptureFixture[str]) -> None:
    store.add("a", ["work"])
    store.add("b", ["work", "home"])
    assert cli.main(["tags"]) == 0
    assert capsys.readouterr().out.splitlines() == ["#home  1", "#work  2"]
