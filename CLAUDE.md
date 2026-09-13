# merl — notes for agents

- `src/app.rs` `KEYS` is the single source of truth for the key bindings: the `?` overlay, the
  README `## Keys` table (a test compares them) and the tutorial all derive from it.
- `merl --tutor` (`src/tutor.rs`, sample project in `tutor/notes/`) teaches the navigation keys.
  When you add, remove or rebind a key, or change what an overlay or jump does, update the
  affected lesson or add one. The test `every_key_is_taught_or_skipped_on_purpose` fails on a new
  `KEYS` row until it is taught or listed in `NOT_TAUGHT`; `tutorial_is_completable` fails when a
  lesson's keys no longer reach its target.
