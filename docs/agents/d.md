# Changing `d`

A wrong jump is worse than a picker or "don't know", and no lookup may get worse than on master.
Only the path the change narrows gets new rules: every other lookup, and every other language,
matches exactly as master does. Rules that start answering a new question tend to leak into the
fallback paths, so a `d` PR is ready to merge only after the bench shows no cursor worse than
master.

- **The bench**, `tools/d-bench/run [--lang go,rust]` (`tools/d-bench/README.md`): recorded
  cursors in real projects pinned to a commit, one per language, merl's answer scored against a
  language server's (or a judgement read from the code, where the README says so) and diffed
  against `baseline.tsv`, master's. It prints per language the direct hits, pickers with the
  answer, wrong jumps, misses, wrong jumps on rows it does not score and p50 / p90 ms, lists every
  cursor that got worse (a new wrong jump first, on a row not scored too), and exits 1 when there
  is any. Run the languages the change touches, all of them
  when a shared path moves; the table goes into the commit message. A PR that changes the table
  commits the new baseline with it (`--update-baseline`), so the next PR compares against what
  master will be.
- It times in release; the times mean something only with `sysctl -n vm.loadavg` under ~8:
  parallel sessions' builds push it to 30–90 and skew timings two- to threefold.
- The answers are recorded once, never in CI and never in merl (`record.py`); a cursor whose
  recorded answer is debatable is marked `skip` with the reason in `answers/LANG.tsv`, not argued
  with in the code.

The expected answers of `d` are written in the fixtures, under the line they probe
(`tests/fixtures/README.md`, #307). A fixed case flips its annotation from `today; want … (#N)`
to the wanted answer, or adds one when no annotation covers it; a miss found in a real project
becomes an annotation with the issue that will fix it.

An adversarial fixture per language (shadowed imports, namesake types, declaration-shaped lines
in strings) finds what real projects do not. To prove a test can fail, revert one fix at a time
and watch it go red.
