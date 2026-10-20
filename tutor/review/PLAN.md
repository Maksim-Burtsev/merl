# Plan: find notes by tag

`notes find` reads only the text of a note. Tags are stored, but nothing
searches them. Goal: `notes find --tag work` lists the notes tagged `work`,
and words after it narrow the list further.

## Steps

1. `NoteStore.find` takes the tags to match
2. `cli.py` passes `--tag` to `find`, not only to `add`
   - several `--tag` flags narrow the list
   - no `--tag` keeps today's search
3. Tests for both
4. `notes tags` lists every tag with its count

## Status

- [x] design agreed
- [x] implementation
- [x] tests
- [ ] README

| Step | File | Risk |
|---|---|---|
| filter | store.py, models.py | low |
| flag | cli.py | medium: `--tag` already tags a new note in `add` |
| tests | tests/test_store.py, tests/test_cli.py | low |
| tags | store.py, cli.py | low |

> [!NOTE]
> Tags match as typed: `Work` and `work` are two tags, as they are today.

```python
def find(self, needle: str, tags: list[str] | None = None) -> list[Note]:
    wanted = set(tags or [])
    lowered = needle.lower()
    return [
        note
        for note in self.notes
        if lowered in note.text.lower() and wanted <= set(note.tags)
    ]
```

---

Done when `make test` is green.
