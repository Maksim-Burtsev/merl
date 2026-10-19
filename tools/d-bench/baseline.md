# `d` bench baseline

merl at `75ae4ef7`, 2026-10-19, release build, `vm.loadavg` { 6.62 8.01 7.63 } before the run, { 6.73 7.45 7.50 } after. The julia row and its cursors are kept from master: this machine has no Julia, so its standard library is not found and 54 cursors read worse with master's build too.
The go row with the fixes of #1101 and #1102: `fd32efdb`, 2026-10-19, release build, `vm.loadavg` { 10.63 9.74 8.39 } before the run, { 10.42 9.77 8.50 } after.
The csharp row with the fixes of #1089 and #1090: `b81c63d9`, 2026-10-19, release build, `vm.loadavg` { 8.59 7.21 7.29 } before the run, { 6.82 6.94 7.18 } after.

| language (project) | scored | direct hit | picker with the answer | wrong jump | miss | not scored | p50 ms | p90 ms |
|---|---|---|---|---|---|---|---|---|
| python (paperless-ngx) | 257 | 214 (83%) | 25 (10%) | 0 (0%) | 18 (7%) | 68 | 19.5 | 386.8 |
| ts (outline) | 122 | 116 (95%) | 3 (2%) | 1 (1%) | 2 (2%) | 178 | 42.8 | 97.7 |
| js (eslint, uptime-kuma) | 217 | 174 (80%) | 33 (15%) | 1 (0%) | 9 (4%) | 177 | 40.1 | 174.0 |
| go (caddy, gitea) | 318 | 310 (97%) | 6 (2%) | 0 (0%) | 2 (1%) | 51 | 6.6 | 19.1 |
| rust (ripgrep) | 231 | 203 (88%) | 24 (10%) | 0 (0%) | 4 (2%) | 69 | 5.9 | 61.7 |
| c (redis) | 250 | 239 (96%) | 11 (4%) | 0 (0%) | 0 (0%) | 20 | 24.6 | 34.5 |
| cpp (leveldb) | 232 | 197 (85%) | 34 (15%) | 0 (0%) | 1 (0%) | 68 | 17.0 | 63.1 |
| php (koel) | 392 | 238 (61%) | 132 (34%) | 0 (0%) | 22 (6%) | 88 | 31.4 | 244.8 |
| swift (Alamofire) | 448 | 419 (94%) | 28 (6%) | 0 (0%) | 1 (0%) | 281 | 9.0 | 34.1 |
| java (halo) | 115 | 92 (80%) | 23 (20%) | 0 (0%) | 0 (0%) | 88 | 64.8 | 181.4 |
| kotlin (nowinandroid) | 56 | 51 (91%) | 4 (7%) | 0 (0%) | 1 (2%) | 95 | 27.5 | 57.9 |
| csharp (eShop) | 85 | 61 (72%) | 12 (14%) | 0 (0%) | 12 (14%) | 130 | 20.9 | 37.7 |
| ruby (mastodon) | 140 | 126 (90%) | 13 (9%) | 0 (0%) | 1 (1%) | 179 | 72.0 | 638.9 |
| objc (SDWebImage) | 229 | 94 (41%) | 120 (52%) | 0 (0%) | 15 (7%) | 41 | 22.4 | 88.3 |
| groovy (nextflow) | 25 | 13 (52%) | 4 (16%) | 0 (0%) | 8 (32%) | 47 | 83.6 | 160.7 |
| jenkins (pipeline-library) | 18 | 11 (61%) | 7 (39%) | 0 (0%) | 0 (0%) | 54 | 57.5 | 90.5 |
| solidity (openzeppelin-contracts) | 158 | 103 (65%) | 55 (35%) | 0 (0%) | 0 (0%) | 112 | 7.5 | 9.8 |
| starlark (rules_go) | 84 | 79 (94%) | 2 (2%) | 0 (0%) | 3 (4%) | 186 | 4.5 | 7.2 |
| julia (DataFrames.jl) | 177 | 67 (38%) | 78 (44%) | 7 (4%) | 25 (14%) | 123 | 14.0 | 31.6 |
| nix (nix-darwin) | 49 | 39 (80%) | 10 (20%) | 0 (0%) | 0 (0%) | 51 | 3.1 | 10.9 |
| css (bootstrap) | 230 | 164 (71%) | 56 (24%) | 1 (0%) | 9 (4%) | 70 | 3.4 | 24.5 |
| vue (gitea) | 218 | 123 (56%) | 42 (19%) | 2 (1%) | 51 (23%) | 52 | 55.4 | 185.7 |
| svelte (immich) | 182 | 115 (63%) | 43 (24%) | 5 (3%) | 19 (10%) | 88 | 38.5 | 194.9 |
| astro (starlight) | 167 | 112 (67%) | 14 (8%) | 1 (1%) | 40 (24%) | 103 | 37.2 | 114.5 |
| clojure (babashka) | 100 | 65 (65%) | 22 (22%) | 0 (0%) | 13 (13%) | 170 | 9.2 | 18.6 |
| elisp (magit) | 39 | 28 (72%) | 4 (10%) | 0 (0%) | 7 (18%) | 41 | 4.2 | 5.2 |
| racket (drracket) | 108 | 65 (60%) | 14 (13%) | 0 (0%) | 29 (27%) | 128 | 13.8 | 19.3 |
| scheme (chibi-scheme) | 54 | 29 (54%) | 16 (30%) | 0 (0%) | 9 (17%) | 17 | 8.0 | 9.6 |
| commonlisp (lem) | 52 | 34 (65%) | 3 (6%) | 0 (0%) | 15 (29%) | 26 | 7.2 | 8.3 |
| bash (bats-core, acme.sh) | 97 | 84 (87%) | 5 (5%) | 0 (0%) | 8 (8%) | 12 | 5.2 | 6.6 |
| lua (nvim-treesitter, kong) | 63 | 57 (90%) | 2 (3%) | 0 (0%) | 4 (6%) | 47 | 1.4 | 14.8 |
| dart (dio) | 97 | 63 (65%) | 29 (30%) | 0 (0%) | 5 (5%) | 17 | 33.3 | 103.4 |
| sql (lemmy) | 79 | 22 (28%) | 13 (16%) | 0 (0%) | 44 (56%) | 31 | 11.2 | 15.4 |
