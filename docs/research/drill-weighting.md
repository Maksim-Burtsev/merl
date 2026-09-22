# How drills pick the next item, score a miss against a slow hit, and retire a key

Research for [#191](https://github.com/Maksim-Burtsev/merl/issues/191), part of the wayfinder
map [#187](https://github.com/Maksim-Burtsev/merl/issues/187). Question: what do shortcut
trainers, adaptive typing tutors and spaced-repetition schedulers do about (a) the next item,
(b) a miss versus a slow correct answer, (c) declaring an item learned, and what of it transfers
to a 20-task terminal session. Primary sources only; a claim we could not trace to the product's
own docs or code is marked *unverified*. Read 2026-09-22.

## Shortcut trainers

| Tool | Next item | Miss vs slow | Learned / retired | Source |
|---|---|---|---|---|
| **KeyCombiner** | Weighted sampling inside a run: low-confidence combos "occur more often". Between runs a day-bucketed schedule. | `confidence = (correct − skipped − 2·errors) / avg_time_s`. A miss costs two correct answers; slowness divides the whole score. | Mastered at confidence ≥ 3, revoked when it drops below. Between runs: more wrong than right in the last run → back tomorrow; else delay by confidence: <1 → 1 d, 1–2 → 4 d, 2–5 → 16 d, >5 → 32 d. | [wiki: Confidence](https://github.com/tkainrad/keycombiner/wiki/Confidence-Value), [wiki: Spaced Repetition](https://github.com/tkainrad/keycombiner/wiki/Spaced-Repetition), [author's post](https://tkainrad.dev/posts/how-i-learned-50-new-keyboard-shortcuts-in-42-minutes/) |
| **ShortcutFoo** | SRS across days: "one day … then maybe 2 days, then 4, then 7, then 18"; one Learn pass per unit per day. | Self-rated after each attempt, Anki style; time is not measured. | No published threshold; six rank names labelled by time span (days to ~1 year). | [How it works, part I](https://www.shortcutfoo.com/blog/how-does-shortcutfoo-work-part-i-learn-section), [interval training](https://www.shortcutfoo.com/blog/introducing-interval-training-for-shortcuts) |
| **Vim Genius** | Uniform random among the level's unmastered commands. | A correct answer counts only if given in ≤ 15 s; slower is simply not counted. Wrong keys never register. | `MASTERY_NUMBER = 2`: two on-time correct answers retire the command from the level. | [source](https://github.com/vicramon/vimgenius) (`command.rb`, `commands_controller.rb`, `command.js`) |
| **Vim Adventures** | Fixed 13-level progression; a level is passed by solving its puzzle. | None. | None. | [site](https://vim-adventures.com/) |
| **vimtutor** | Linear text, two chapters. | None. | None. | [usr_01](https://vimhelp.org/usr_01.txt.html) |
| **vim-be-good** | `math.random(1, #self.rounds)`; results logged, never read back. | Per-round timeout by difficulty: 10 s easy … 2 s "tpope". | None. | [game-runner.lua](https://raw.githubusercontent.com/ThePrimeagen/vim-be-good/master/lua/vim-be-good/game-runner.lua) |
| **Emacs key-quiz** | Uniform random, item removed on a correct answer; 20 questions a game. | Correct part +5, wrong −10; no timer. | Per game only, nothing persists. | [key-quiz.el](https://raw.githubusercontent.com/federicotdn/key-quiz/master/key-quiz.el) |
| **keyzen** (typing) | Uniform over chars with a streak < 5. | Error resets the streak to 0; time ignored. | Next char unlocks when no char is under the streak. | [keyzen.js](https://raw.githubusercontent.com/wwwtyro/keyzen/master/keyzen.js) |
| **JetBrains Feature Trainer / Key Promoter X** | Hand-ordered lessons / nudges on mouse actions. | – | Lesson done when its tasks are done. | [docs](https://www.jetbrains.com/help/idea/feature-trainer.html), [README](https://raw.githubusercontent.com/halirutan/IntelliJ-Key-Promoter-X/master/README.md) |

Only KeyCombiner publishes a formula. Only KeyCombiner and ShortcutFoo do spaced repetition, and
both at the scale of days between sessions, not within one. Everything else is uniform random with
at most a streak counter or remove-on-correct.

## Adaptive typing tutors

| Tool | Next item | Miss vs slow | Learned | Source |
|---|---|---|---|---|
| **keybr** | Focus = the included key with the lowest confidence; the focus letter is in every generated word. Six letters always in; the next unlocks when every included key has `bestConfidence ≥ 1`. | Speed only. `confidence = time_at_target / EMA(time_to_type)`, target 175 CPM (≈ 343 ms per char), EMA alpha 0.1, no minimum sample count. A typo'd keystroke contributes **no time sample**: the miss is neither a slow hit nor a penalty, it is dropped. Users complain that 90 % accuracy still unlocks keys. | Confidence ≥ 1 once ("were once above the target speed"); keys are never removed, they just stop being focus. | [guided.ts](https://github.com/aradzie/keybr.com/blob/master/packages/keybr-lesson/lib/guided.ts), [target.ts](https://github.com/aradzie/keybr.com/blob/master/packages/keybr-lesson/lib/target.ts), `packages/keybr-textinput/lib/histogram.ts`, `packages/keybr-result/lib/keystats.ts`, [issue #34](https://github.com/aradzie/keybr.com/issues/34) |
| **TIPP10** | Top-4 chars by error rate `errors·100/occurrences`; snippets rich in them, every 2nd query plain random; last 10 snippets not repeated. | Error rate only; speed ignored. | None, the rate just decays. | [intelligence](https://www.tipp10.com/en/intelligence/), `sql/trainingsql.cpp` |
| **Klavaro** | Random words; a targeted mode kicks in when errors ≥ 60 or the 10 slowest keys average > 2.2× the 40 slowest. Then words alternate letters from the error list (∝ wrong count) and the slow list (∝ mean time). | Both, in separate lists. | Per-course pass goals: 95–98 % accuracy, 10 or 50 WPM. | [site](https://klavaro.sourceforge.io/en/), `src/accuracy.h`, `src/tutor.c` |
| **Amphetype** | Ranks by `damage = count·time²·(1 + misses/count)`; review = mistyped words + slowest quarter. | Time squared, miss rate as a multiplier. | None. | `amphetype/StatWidgets.py`, `Quizzer.py` |
| **GNU Typist** | Fixed scripts; a drill repeats when errors > 3 %. | Error rate only. | – | [docs](https://www.gnu.org/software/gtypist/doc/) |
| **Monkeytype** | Not adaptive; opt-in "practise words" repeats missed words and the slowest 20 % of the last test. | – | – | `frontend/src/ts/test/practise-words.ts` |
| TypingClub, Typing.com, Ratatype | Fixed lesson order, star thresholds, formulas unpublished. | TypingClub: "value accuracy over speed", "double-penalty for every error" (*from search snippets*). | – | [TypingClub results](https://s.typingclub.com/docs/student-management/track-progress/results-page.html) |

## Spaced-repetition schedulers

- **SM-2** ([Wozniak](https://www.supermemo.com/en/blog/application-of-a-computer-to-improve-the-results-obtained-in-working-with-the-supermemo-method)).
  Grades 0–5: 5 perfect, 4 "correct after a hesitation", 3 "correct with serious difficulty",
  2 wrong but the answer "seemed easy", 1 wrong, 0 blackout. `EF' = EF + (0.1 − (5−q)(0.08 +
  (5−q)·0.02))`, start 2.5, floor 1.3, so per grade: +0.10, 0, −0.14, −0.32, −0.54, −0.80.
  A slow correct answer (3) costs about a seventh of a blackout. Intervals 1, 6, then ×EF; a
  grade < 3 restarts the interval. **Within a session**: "repeat again all items that scored
  below four … until all of these items score at least four" — grade 4 is the exit bar, a 3 is
  re-asked the same day.
- **Anki** ([deck options](https://docs.ankiweb.net/deck-options.html), [leeches](https://docs.ankiweb.net/leeches.html)).
  Learning steps `1m 10m`; Again → first step, Good → next step, Easy graduates, Hard repeats the
  step. No timer input to scheduling. The only give-up rule in any scheduler: a card that lapses
  8 times is tagged a leech and suspended.
- **FSRS** ([fsrs-rs](https://github.com/open-spaced-repetition/fsrs-rs), [algorithm wiki](https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-Algorithm), [KDD 2022](https://dl.acm.org/doi/10.1145/3534678.3539081)).
  Difficulty / stability / retrievability; target retention 0.9. Its whole input is `{rating
  1–4, delta_t days}`: **response time is not used**. A Hard rating still grows stability, by a
  factor of 0.60 of the normal growth; a lapse resets it to a fresh small value. The same-day
  rule: Hard/Good/Easy never lower stability, Again multiplies it by roughly 0.36.
- **Leitner boxes**: correct → next box, wrong → box 1, learned = last box. Binary; no grade for
  slow. ([Wikipedia](https://en.wikipedia.org/wiki/Leitner_system); the 1972 book is not online.)

None of these use time, and all of them schedule across days. They answer "when to show it again
next week", not "which of 37 keys to ask in the next two minutes".

## Response time as the signal

- **ARTS** (Mettler, Massey & Kellman; [2011](https://escholarship.org/uc/item/2xs4n8wz),
  [2014](https://pmc.ncbi.nlm.nih.gov/articles/PMC6124487/),
  [2016](https://pmc.ncbi.nlm.nih.gov/articles/PMC6028005/)) is the published scheme closest to
  our case: one session, accuracy plus response time, items retire. Priority
  `P = a·(N − D)·[b·(1 − α)·log(RT/r) + α·W]`, where N = trials since the item was last shown,
  D = enforced delay (2, later 1) so nothing repeats back to back, α = 1 after an error, W = 20
  chosen so that any error outranks the slowest correct answer, and `log(RT/r)` for correct
  answers ("20 vs 30 s matters less than 2 vs 12 s"). Retirement: 3 correct in a row under 10 s
  (2011, exp. 1), 4 of the last 5 under 6.5 s (2011, exp. 2), 5 of 6 under 3 s (2014). Beat
  fixed spacing at a delayed test, d ≈ 0.56.
- **Pyc & Rawson 2009** ([pdf](https://andymatuschak.org/prompts/Pyc2009.pdf)): "difficult but
  successful retrievals are better for memory than easier successful retrievals", difficulty
  measured as first-keypress latency. A slow correct answer is good practice, not a defect.
- **Lindsey, Shroyer, Pashler & Mozer 2014** ([pdf](https://home.cs.colorado.edu/~mozer/Research/Selected%20Publications/reprints/LindseyShroyerPashlerMozer2014.pdf)):
  review the item whose predicted recall is nearest θ = 0.33, not the weakest one; +16.5 % over
  massed practice.

## Motor learning within a session

- **Shea & Morgan 1979** ([pdf](https://gwern.net/doc/psychology/spaced-repetition/1979-shea.pdf)):
  random order of movement patterns, "no more than 2 trials on the same task consecutively", is
  slower during practice (1.69 vs 1.32 s) and faster at retention (1.31 vs 1.73 s) than blocked
  practice. [Magill & Hall 1990](https://www.sciencedirect.com/science/article/abs/pii/016794579090005X)
  and [Schmidt & Bjork 1992](https://journals.sagepub.com/doi/abs/10.1111/j.1467-9280.1992.tb00029.x)
  generalise it: what maximises performance during training hurts retention. So in-session
  speed under blocked practice overstates learning; interleave.
- [Lee & Genovese 1989](https://pubmed.ncbi.nlm.nih.gov/2489826/): for a discrete task like a
  keypress, pauses between trials do not help; interleaving is what matters.
- **Practice curve** ([Heathcote, Brown & Mewhort 2000](https://link.springer.com/article/10.3758/BF03212979)):
  per-trial RT falls exponentially to an asymptote set by physical limits; a plateau is three
  near-identical times in a row.
- **Chord latency** ([Card, Moran & Newell 1980, KLM](http://iihm.imag.fr/blanch/ens/2010-2011/M1/EIHM/cours/1980-Card-KLM.pdf)):
  a keystroke K = 0.2–0.28 s for a typist, a modifier is its own K, mental preparation M = 1.35 s.
  A recalled chord ≈ 0.5 s; one that needs thought ≈ 1.8 s. [Lane et al. 2005](https://www.ruf.rice.edu/~lane/papers/hidden_costs.pdf)
  measured 1.36 s for a shortcut from prompt to keypress (reading included) against 2.17 s icon
  and 3.13 s menu. [Remington, Yuen & Pashler 2016](https://pubmed.ncbi.nlm.nih.gov/26651347/):
  shortcuts overtake menus after ~10 uses per item; "frequency of shortcut use is a function of
  ease of retrieval". No primary source states a "200–400 ms expert chord" range; KLM is the
  closest.

## What transfers to a 20-task terminal session

Facts that bind: 37 keys, 20 tasks, so most keys are not asked in a session; a key is learned
only when it shows up in real work (settled in #187); weak keys are asked more often, strong
ones never dropped.

1. **Not SRS.** SM-2, Anki and FSRS schedule across days, ignore time, and their "learned"
   is redundant with ours (real-work presses). KeyCombiner, the one shortcut trainer with a real
   formula, does weighted sampling inside a run and reserves its SRS for the day scale; we have
   no day scale to speak of, so weighted sampling is the whole thing.

2. **A miss outranks any slow correct answer.** Every scheme that has both agrees: SM-2 −0.32
   vs −0.14, FSRS reset vs ×0.6, ARTS W = 20 vs log(RT), KeyCombiner −2 vs a divisor. And
   slow-correct is desirable retrieval effort (Pyc & Rawson), so it raises the weight, it does
   not get punished. keybr's "drop the sample on a typo" is the one thing not to copy.

3. **Weights, per key, from work stats and the last 5 drill attempts**:
   - never pressed in real work: 4
   - a miss among the last 5 drill attempts: 3
   - correct but slow: `1 + log2(median_ms / cap)` clamped to [1, 2] (ARTS uses log RT)
   - fast and correct: 1, never 0 (strong keys stay in)
   Pick by weighted random. Enforce a gap of at least 2 tasks between the same key (ARTS D = 2,
   Shea & Morgan's "no more than 2 in a row"). No grouping by category: interleaving is what
   builds retention.

4. **Within the session**: a missed key is re-queued once, at least 2 tasks later (SM-2's
   "repeat until ≥ 4", ARTS's α = 1). A slow correct one is not re-asked; 20 slots are too few,
   its weight carries to the next session. On a miss the task ends by naming the key: that is the
   drill's one teaching moment.

5. **Miss and cap**: a miss is a press that is not the target chord (or a timeout of 8 s with no
   press, cf. Vim Genius 15 s, ARTS 10 s). Time is measured from the task appearing to the
   correct press, so reading is included; the cap is 2 s (Lane's 1.36 s measured, plus KLM's
   M = 1.35 s for a chord that needs thought lands at ~1.8 s). Make `cap` and the timeout named
   constants and recheck them after two weeks of the user's own timings.

6. **Weak list at the end**: never in work, or a miss in the last 5, or median of the last 3
   correct above the cap. There is no in-session retirement: ARTS's "3 fast in a row" needs
   more trials per item than 20 over 37 keys allows, so the streak spans sessions and only
   changes the weight, and the only retirement is the one already settled: the key shows up in
   real work.

7. **Placement pass** (first run, one task per key in `KEYS` order) stays as settled; it is a
   tour. Its timings seed the per-key medians, so the second run is already weighted.
