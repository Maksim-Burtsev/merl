# `d` bench baseline

merl at `f20c52b1` with the fix of #975 (ts rows), 2026-10-17, release build, `vm.loadavg` { 10.10 8.84 8.35 } before the run; the csharp row with the fixes of #935, `vm.loadavg` { 5.00 5.48 5.37 }; the other rows as in the night's second epic.
The rust row: merl at `20ac5110` with the `d` fixes of branch `b1019/rust`, 2026-10-19, release build, `vm.loadavg` { 6.98 6.29 6.45 } before the run and after.
The swift row (#1053, #1052, #1055, branch `b1019/swift`): 2026-10-19, release build, `vm.loadavg` { 5.07 5.68 5.67 } before the run.
The lua row with the fixes of branch `b1019/lua` (#1061, #1062), 2026-10-19, release build, `vm.loadavg` { 8.08 7.25 6.44 } before the run, { 6.75 6.94 6.39 } after.
The bash row with the fixes of #1056 to #1059, 2026-10-19, release build, `vm.loadavg` { 4.86 5.49 5.61 } before the run.
The ts, js, vue, svelte and astro rows: merl at `168369f0` with the `d` fix of #1078 on branch `b1019/jsts`, 2026-10-19, release build, `vm.loadavg` { 5.60 7.28 6.96 } before the run and after.
The ruby row with the fixes of branch `b1019/ruby` (#1095, #1094, #1096, #1097, #1098), 2026-10-19, release build, `vm.loadavg` { 5.64 7.46 7.01 } before the run.
The go row with the fixes of #1101 and #1102 (branch `b1019/go`), 2026-10-19, release build, `vm.loadavg` { 5.66 6.55 7.11 } before the run, { 6.57 6.73 7.17 } after.

| language (project) | scored | direct hit | picker with the answer | wrong jump | miss | not scored | p50 ms | p90 ms |
|---|---|---|---|---|---|---|---|---|
| python (paperless-ngx) | 213 | 199 (93%) | 8 (4%) | 0 (0%) | 6 (3%) | 57 | 13.6 | 258.1 |
| ts (outline) | 122 | 116 (95%) | 3 (2%) | 1 (1%) | 2 (2%) | 178 | 43.2 | 91.7 |
| js (eslint) | 147 | 119 (81%) | 23 (16%) | 0 (0%) | 5 (3%) | 123 | 45.9 | 208.8 |
| go (caddy) | 229 | 227 (99%) | 2 (1%) | 0 (0%) | 0 (0%) | 41 | 6.2 | 15.8 |
| rust (ripgrep) | 231 | 203 (88%) | 24 (10%) | 0 (0%) | 4 (2%) | 69 | 6.5 | 77.2 |
| c (redis) | 250 | 232 (93%) | 18 (7%) | 0 (0%) | 0 (0%) | 20 | 24.7 | 33.3 |
| cpp (leveldb) | 232 | 182 (78%) | 48 (21%) | 0 (0%) | 2 (1%) | 68 | 22.8 | 94.6 |
| php (koel) | 104 | 99 (95%) | 5 (5%) | 0 (0%) | 0 (0%) | 196 | 13.8 | 25.9 |
| swift (Alamofire) | 154 | 125 (81%) | 28 (18%) | 0 (0%) | 1 (1%) | 116 | 18.4 | 40.2 |
| java (halo) | 37 | 35 (95%) | 2 (5%) | 0 (0%) | 0 (0%) | 43 | 51.2 | 97.0 |
| kotlin (nowinandroid) | 34 | 34 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 46 | 26.5 | 55.9 |
| csharp (eShop) | 27 | 25 (93%) | 2 (7%) | 0 (0%) | 0 (0%) | 53 | 18.0 | 28.9 |
| ruby (mastodon) | 28 | 23 (82%) | 4 (14%) | 0 (0%) | 1 (4%) | 52 | 57.0 | 166.5 |
| objc (SDWebImage) | 229 | 92 (40%) | 122 (53%) | 0 (0%) | 15 (7%) | 41 | 26.3 | 110.1 |
| groovy (nextflow) | 25 | 13 (52%) | 4 (16%) | 0 (0%) | 8 (32%) | 47 | 73.4 | 164.6 |
| jenkins (pipeline-library) | 18 | 11 (61%) | 7 (39%) | 0 (0%) | 0 (0%) | 54 | 59.0 | 91.8 |
| solidity (openzeppelin-contracts) | 158 | 103 (65%) | 55 (35%) | 0 (0%) | 0 (0%) | 112 | 7.6 | 9.2 |
| starlark (rules_go) | 84 | 79 (94%) | 2 (2%) | 0 (0%) | 3 (4%) | 186 | 5.1 | 8.4 |
| julia (DataFrames.jl) | 177 | 67 (38%) | 78 (44%) | 7 (4%) | 25 (14%) | 123 | 14.0 | 31.6 |
| nix (nix-darwin) | 49 | 39 (80%) | 10 (20%) | 0 (0%) | 0 (0%) | 51 | 8.1 | 24.4 |
| css (bootstrap) | 230 | 164 (71%) | 56 (24%) | 1 (0%) | 9 (4%) | 70 | 4.9 | 36.1 |
| vue (gitea) | 218 | 123 (56%) | 42 (19%) | 2 (1%) | 51 (23%) | 52 | 50.4 | 170.4 |
| svelte (immich) | 182 | 115 (63%) | 43 (24%) | 5 (3%) | 19 (10%) | 88 | 34.8 | 179.2 |
| astro (starlight) | 167 | 112 (67%) | 14 (8%) | 1 (1%) | 40 (24%) | 103 | 35.7 | 107.0 |
| clojure (babashka) | 100 | 65 (65%) | 22 (22%) | 0 (0%) | 13 (13%) | 170 | 5.0 | 14.0 |
| elisp (magit) | 39 | 28 (72%) | 4 (10%) | 0 (0%) | 7 (18%) | 41 | 4.1 | 5.1 |
| racket (drracket) | 108 | 65 (60%) | 14 (13%) | 0 (0%) | 29 (27%) | 128 | 13.2 | 16.1 |
| scheme (chibi-scheme) | 54 | 29 (54%) | 16 (30%) | 0 (0%) | 9 (17%) | 17 | 8.0 | 9.7 |
| commonlisp (lem) | 52 | 34 (65%) | 3 (6%) | 0 (0%) | 15 (29%) | 26 | 7.2 | 8.1 |
| bash (bats-core, acme.sh) | 97 | 84 (87%) | 5 (5%) | 0 (0%) | 8 (8%) | 12 | 4.8 | 7.1 |
| lua (nvim-treesitter, kong) | 63 | 57 (90%) | 2 (3%) | 0 (0%) | 4 (6%) | 47 | 2.5 | 15.4 |
