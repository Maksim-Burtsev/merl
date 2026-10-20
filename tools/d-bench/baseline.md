# `d` bench baseline

merl at `75ae4ef7`, 2026-10-19, release build, `vm.loadavg` { 6.62 8.01 7.63 } before the run, { 6.73 7.45 7.50 } after. The julia row and its cursors are kept from master: this machine has no Julia, so its standard library is not found and 54 cursors read worse with master's build too.
The go row with the fixes of #1101 and #1102: `fd32efdb`, 2026-10-19, release build, `vm.loadavg` { 10.63 9.74 8.39 } before the run, { 10.42 9.77 8.50 } after.
The csharp row with the fixes of #1089 and #1090: `b81c63d9`, 2026-10-19, release build, `vm.loadavg` { 8.59 7.21 7.29 } before the run, { 6.82 6.94 7.18 } after.
The sql row with the fixes of #1065 and #1066: over `b81c63d9`, 2026-10-19, release build, `vm.loadavg` { 6.87 5.88 6.62 } before the run.
The java, kotlin, swift and go rows with #1072: `9f4c7433`, 2026-10-19, release build, `vm.loadavg` { 4.76 5.68 6.38 } before the run, { 4.76 5.68 6.38 } after.
The dart row with the fixes of #1084, #1085 and #1086: over `76c2d351`, 2026-10-19, release build, `vm.loadavg` { 6.98 6.92 7.40 } before the run.
The rust row with the fixes of #1044 and #1045 (a `use` inside an inline `mod`, a crate handed on by `pub extern crate`, a receiver whose call's arguments span lines): over `da06cd37`, 2026-10-20, release build, `vm.loadavg` { 8.81 9.06 8.23 } before the run.
The swift row with #1047: over `06a28815`, 2026-10-19, release build, `vm.loadavg` { 6.95 10.10 9.35 } before the run.
The go row with #1072's chains of calls: over `72304a83`, 2026-10-20, release build, `vm.loadavg` { 5.32 4.55 4.67 } before the run, { 6.51 4.92 4.79 } after.
The go row with #1116's element of a named slice type: over `278338c8`, 2026-10-20, release build, `vm.loadavg` { 16.44 16.00 15.35 } before the run, { 32.60 21.85 17.71 } after (the times read high under that load).
The `not scored, jumps elsewhere` column (#1092): every row but julia's played over `a7e01566`, 2026-10-20, release build, `vm.loadavg` { 10.33 15.89 16.80 } before the run; only the verdicts of the rows not scored moved, the times and the scored rows are as they were. julia's 0 is unmeasured.
The ruby row with the fixes of #1093 and #1114: `2594316e`, 2026-10-20, release build, `vm.loadavg` { 7.36 8.57 7.73 } before the run, { 11.24 9.40 8.07 } after.
The c, cpp and objc rows with the fixes of #1140 and #1138: over `c7694294`, 2026-10-20, release build, `vm.loadavg` { 5.58 6.33 7.33 } before the run.
The php row with the fixes of #1177, #1176, #1175 and #1174: `85c29236`, 2026-10-20, release build, `vm.loadavg` { 8.74 7.48 6.93 } before the run, { 11.93 8.47 7.32 } after.
The swift row with #1068's fields of a value from outside the project: over `d99cae84`, 2026-10-21, release build, `vm.loadavg` { 7.31 10.68 15.68 } before the run (the times read high under that load).
The sql row with #1064 (the `CREATE` in effect in a migration): over `d99cae84`, 2026-10-21, release build, `vm.loadavg` { 6.58 9.92 15.09 } before the run, { 6.18 9.23 14.47 } after.
The sql row with #1063 (a column through its table or alias): over `2a700f71`, 2026-10-21, release build, `vm.loadavg` { 7.63 12.11 14.89 } before the run, { 5.95 10.78 14.19 } after.

| language (project) | scored | direct hit | picker with the answer | wrong jump | miss | not scored | not scored, jumps elsewhere | p50 ms | p90 ms |
|---|---|---|---|---|---|---|---|---|---|
| python (paperless-ngx) | 257 | 214 (83%) | 25 (10%) | 0 (0%) | 18 (7%) | 68 | 11 | 19.5 | 386.8 |
| ts (outline) | 122 | 116 (95%) | 3 (2%) | 1 (1%) | 2 (2%) | 178 | 4 | 42.8 | 97.7 |
| js (eslint, uptime-kuma) | 217 | 174 (80%) | 33 (15%) | 1 (0%) | 9 (4%) | 177 | 11 | 40.1 | 174.0 |
| go (caddy, gitea) | 318 | 312 (98%) | 5 (2%) | 0 (0%) | 1 (0%) | 51 | 1 | 9.6 | 26.6 |
| rust (ripgrep) | 231 | 208 (90%) | 19 (8%) | 0 (0%) | 4 (2%) | 69 | 5 | 5.8 | 61.0 |
| c (redis) | 250 | 239 (96%) | 11 (4%) | 0 (0%) | 0 (0%) | 20 | 8 | 25.0 | 35.2 |
| cpp (leveldb) | 232 | 199 (86%) | 32 (14%) | 0 (0%) | 1 (0%) | 68 | 15 | 19.2 | 70.7 |
| php (koel) | 392 | 239 (61%) | 131 (33%) | 0 (0%) | 22 (6%) | 88 | 3 | 28.0 | 249.3 |
| swift (Alamofire) | 448 | 420 (94%) | 27 (6%) | 0 (0%) | 1 (0%) | 281 | 28 | 13.4 | 49.5 |
| java (halo) | 115 | 93 (81%) | 22 (19%) | 0 (0%) | 0 (0%) | 88 | 2 | 59.0 | 161.1 |
| kotlin (nowinandroid) | 56 | 51 (91%) | 4 (7%) | 0 (0%) | 1 (2%) | 95 | 0 | 28.0 | 63.9 |
| csharp (eShop) | 85 | 61 (72%) | 12 (14%) | 0 (0%) | 12 (14%) | 130 | 1 | 20.9 | 37.7 |
| ruby (mastodon) | 140 | 127 (91%) | 12 (9%) | 0 (0%) | 1 (1%) | 179 | 2 | 66.3 | 382.6 |
| objc (SDWebImage) | 229 | 94 (41%) | 120 (52%) | 0 (0%) | 15 (7%) | 41 | 13 | 22.0 | 92.5 |
| groovy (nextflow) | 25 | 13 (52%) | 4 (16%) | 0 (0%) | 8 (32%) | 47 | 4 | 83.6 | 160.7 |
| jenkins (pipeline-library) | 18 | 11 (61%) | 7 (39%) | 0 (0%) | 0 (0%) | 54 | 0 | 57.5 | 90.5 |
| solidity (openzeppelin-contracts) | 158 | 103 (65%) | 55 (35%) | 0 (0%) | 0 (0%) | 112 | 34 | 7.5 | 9.8 |
| starlark (rules_go) | 84 | 79 (94%) | 2 (2%) | 0 (0%) | 3 (4%) | 186 | 17 | 4.5 | 7.2 |
| julia (DataFrames.jl) | 177 | 67 (38%) | 78 (44%) | 7 (4%) | 25 (14%) | 123 | 0 | 14.0 | 31.6 |
| nix (nix-darwin) | 49 | 39 (80%) | 10 (20%) | 0 (0%) | 0 (0%) | 51 | 0 | 3.1 | 10.9 |
| css (bootstrap) | 230 | 164 (71%) | 56 (24%) | 1 (0%) | 9 (4%) | 70 | 1 | 3.4 | 24.5 |
| vue (gitea) | 218 | 123 (56%) | 42 (19%) | 2 (1%) | 51 (23%) | 52 | 3 | 55.4 | 185.7 |
| svelte (immich) | 182 | 115 (63%) | 43 (24%) | 5 (3%) | 19 (10%) | 88 | 3 | 38.5 | 194.9 |
| astro (starlight) | 167 | 112 (67%) | 14 (8%) | 1 (1%) | 40 (24%) | 103 | 9 | 37.2 | 114.5 |
| clojure (babashka) | 100 | 65 (65%) | 22 (22%) | 0 (0%) | 13 (13%) | 170 | 33 | 9.2 | 18.6 |
| elisp (magit) | 39 | 28 (72%) | 4 (10%) | 0 (0%) | 7 (18%) | 41 | 0 | 4.2 | 5.2 |
| racket (drracket) | 108 | 65 (60%) | 14 (13%) | 0 (0%) | 29 (27%) | 128 | 31 | 13.8 | 19.3 |
| scheme (chibi-scheme) | 54 | 29 (54%) | 16 (30%) | 0 (0%) | 9 (17%) | 17 | 2 | 8.0 | 9.6 |
| commonlisp (lem) | 52 | 34 (65%) | 3 (6%) | 0 (0%) | 15 (29%) | 26 | 2 | 7.2 | 8.3 |
| bash (bats-core, acme.sh) | 97 | 84 (87%) | 5 (5%) | 0 (0%) | 8 (8%) | 12 | 2 | 5.2 | 6.6 |
| lua (nvim-treesitter, kong) | 63 | 57 (90%) | 2 (3%) | 0 (0%) | 4 (6%) | 47 | 1 | 1.4 | 14.8 |
| dart (dio) | 97 | 70 (72%) | 23 (24%) | 0 (0%) | 4 (4%) | 17 | 0 | 44.3 | 124.2 |
| sql (lemmy) | 79 | 54 (68%) | 1 (1%) | 0 (0%) | 24 (30%) | 31 | 0 | 8.6 | 15.3 |
