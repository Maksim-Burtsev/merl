# Releases

Only when the owner asks for one, and with nothing open under `release-blocker`. In order:

1. The changelog covers every PR since the last tag (`git log vX.Y.Z..origin/master`): check
   each commit's issue and content, since PRs merge without an entry; a missing one is written as
   its `changes/N.md`. Dependabot, `docs:`, `refactor:` and tooling commits get no entry. Entries
   cite the issue. Then `tools/gather-changes.py` moves every `changes/N.md` under
   `## [Unreleased]`, one heading each in Keep a Changelog's order, entries by issue number, and
   deletes the files (`--print` shows the section and touches nothing). This happens on the
   release branch: its PR (step 4) carries the section and the deletions.
2. Smoke test: `.claude/skills/smoke-test/SKILL.md` (`/smoke-test` in Claude Code), from that
   branch, so the report reads the finished changelog. No release without its GO.
3. The release note, `docs/releases/X.Y.Z.md`: `release.yml` publishes it as the release, titled
   `merl X.Y.Z`, and fails without it. It is a list, not the changelog section:
   - `#### New` from `### Added`, `#### Better` from `### Changed`, `#### Fixed` from `### Fixed`;
     a heading with nothing under it is left out.
   - One line per changelog entry of the version, in the changelog's order: what you can do or
     what changed, the key or flag in backticks, the issue in parentheses. Up to ~12 words; no
     prose, no second sentence.
   - Under `#### Better` and `#### Fixed`, the `d` lines leave the list for a folded block at the
     end of their section, sorted so each language's lines stand together:
     `<details><summary>N more <code>d</code> fixes, by language</summary>` (`improvements`
     under Better). The owner asked for it on 0.8.0, whose 88 fixes were 63 `d` lines.
   - Last line, with the anchor GitHub builds from the version's heading (`## [0.8.0] -
     2026-09-27` is `#080---2026-09-27`); open the link to check it lands on the section:
     `` [Full changelog](https://github.com/Maksim-Burtsev/merl/blob/master/CHANGELOG.md#080---2026-09-27) · `brew upgrade merl` ``
   - No emoji, no intro paragraph, no contributor list. The model is the previous note in
     `docs/releases/`, or for the first one the sample in #272.
   - A GIF on top, above `#### New`, only when the release changes something visible that looks
     good in motion: `tools/cast.py --size 160x50` on a real project with the tree open. Show it
     to the owner and ask whether it goes in; without a yes the note has none. It goes to `media`
     as `releases/X.Y.Z.gif` (a new name for every upload, as for PR screencasts) and into the
     note as `![](https://raw.githubusercontent.com/Maksim-Burtsev/merl/media/releases/X.Y.Z.gif)`.
4. A release PR, `release: X.Y.Z`, from a branch `release/X.Y.Z`: only such a branch's CI
   presses `d`, `u` and `D` at every cursor of every fixture (#543, in four shards, ~25 minutes
   with the other checks), and a panic it finds is fixed on that branch. `## [Unreleased]` becomes `## [X.Y.Z] - YYYY-MM-DD` with its
   link, the version goes into `Cargo.toml` and `Cargo.lock`, it adds `docs/releases/X.Y.Z.md`,
   it carries the `tests/budgets.tsv` the smoke test's time budgets wrote, and its body holds the
   smoke test's verdict table.
5. After the merge, an annotated tag `vX.Y.Z` (message `merl X.Y.Z`) on that commit, pushed;
   `release.yml` builds the GitHub release and its binaries.
6. The Homebrew tap: `release.yml` bumps it when the `TAP_TOKEN` secret is set; otherwise bump
   `Formula/merl.rb` in `Maksim-Burtsev/homebrew-tap` by hand (the version and the four sha256
   of the `.sha256` assets), commit `merl X.Y.Z` and push. The first release that ships
   `merl-x86_64-apple-darwin.tar.gz` adds an `on_intel` block under `on_macos`, with that
   asset's url and sha256; `bump-tap` fails on an asset the formula has no block for.
