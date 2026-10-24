"""Command line entry point: add, remove, list, search and count notes."""

import argparse
import sys

from config import load_config
from models import Formatter, Note, PlainFormatter
from store import NoteStore


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    """Build the argument parser and read the command line."""
    parser = argparse.ArgumentParser(prog="notes", description="A tiny note keeper.")
    parser.add_argument("command", choices=["add", "remove", "list", "find", "tags"])
    parser.add_argument("words", nargs="*", help="note text, id, or search query")
    parser.add_argument(
        "--tag",
        action="append",
        default=[],
        help="tag a new note, or keep only the notes with this tag",
    )
    return parser.parse_args(argv)


def show(notes: list[Note], formatter: Formatter | None = None) -> None:
    """Print one line per note, the way the formatter renders it."""
    formatter = formatter or PlainFormatter()
    for note in notes:
        print(formatter.render(note))


def show_tags(counts: dict[str, int]) -> None:
    """Print every tag beside the number of notes that carry it."""
    width = max((len(tag) for tag in counts), default=0)
    for tag, count in counts.items():
        print(f"#{tag:<{width}}  {count}")


def main(argv: list[str] | None = None) -> int:
    """Run one command and return the process exit code."""
    args = parse_args(argv)
    store = NoteStore(load_config())
    if args.command == "add":
        if not args.words:
            print("a note needs some text", file=sys.stderr)
            return 2
        note = store.add(" ".join(args.words), args.tag)
        print(f"added {note.id}")
    elif args.command == "remove":
        if not store.remove(int(args.words[0])):
            print("no such note", file=sys.stderr)
            return 1
    elif args.command == "find":
        found = store.find(" ".join(args.words), args.tag)
        if not found:
            print("nothing found", file=sys.stderr)
            return 1
        show(found)
    elif args.command == "tags":
        show_tags(store.tags())
    else:
        show(store.page())
    return 0


if __name__ == "__main__":
    sys.exit(main())
