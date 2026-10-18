# `d` bench baseline

merl at `f20c52b1` with the fix of #975 (ts rows), 2026-10-17, release build, `vm.loadavg` { 10.10 8.84 8.35 } before the run; the csharp row with the fixes of #935, `vm.loadavg` { 5.00 5.48 5.37 }; the other rows as in the night's second epic.
The bash and lua rows (#1091): merl at `0e47807c`, 2026-10-18, release build, `vm.loadavg` { 4.68 5.99 6.51 } before the run and after.
The python, js, go, php, swift, java, kotlin, csharp, ruby, dart and sql rows (#1091): merl at `c812fbcc` (its code as master's `33396ebe`), 2026-10-19, release build, koel with `vendor/` installed, `vm.loadavg` { 5.54 6.66 6.89 } before the run, { 9.16 7.85 7.34 } after.

| language (project) | scored | direct hit | picker with the answer | wrong jump | miss | not scored | p50 ms | p90 ms |
|---|---|---|---|---|---|---|---|---|
| python (paperless-ngx) | 257 | 214 (83%) | 25 (10%) | 0 (0%) | 18 (7%) | 68 | 16.5 | 300.5 |
| ts (outline) | 122 | 116 (95%) | 3 (2%) | 1 (1%) | 2 (2%) | 178 | 46.2 | 99.5 |
| js (eslint, uptime-kuma) | 217 | 174 (80%) | 30 (14%) | 2 (1%) | 11 (5%) | 177 | 38.6 | 157.0 |
| go (caddy, gitea) | 318 | 286 (90%) | 18 (6%) | 0 (0%) | 14 (4%) | 51 | 6.1 | 26.5 |
| rust (ripgrep) | 231 | 183 (79%) | 44 (19%) | 0 (0%) | 4 (2%) | 69 | 5.7 | 67.7 |
| c (redis) | 250 | 232 (93%) | 18 (7%) | 0 (0%) | 0 (0%) | 20 | 24.7 | 33.3 |
| cpp (leveldb) | 232 | 182 (78%) | 48 (21%) | 0 (0%) | 2 (1%) | 68 | 22.8 | 94.6 |
| php (koel) | 392 | 185 (47%) | 143 (36%) | 0 (0%) | 64 (16%) | 88 | 53.4 | 246.6 |
| swift (Alamofire) | 448 | 409 (91%) | 37 (8%) | 0 (0%) | 2 (0%) | 281 | 8.1 | 32.5 |
| java (halo) | 115 | 57 (50%) | 42 (37%) | 0 (0%) | 16 (14%) | 88 | 59.9 | 135.8 |
| kotlin (nowinandroid) | 56 | 50 (89%) | 5 (9%) | 0 (0%) | 1 (2%) | 95 | 28.4 | 61.2 |
| csharp (eShop) | 85 | 55 (65%) | 13 (15%) | 0 (0%) | 17 (20%) | 130 | 18.6 | 32.7 |
| ruby (mastodon) | 140 | 96 (69%) | 33 (24%) | 0 (0%) | 11 (8%) | 179 | 71.4 | 629.6 |
| objc (SDWebImage) | 229 | 92 (40%) | 122 (53%) | 0 (0%) | 15 (7%) | 41 | 26.3 | 110.1 |
| groovy (nextflow) | 25 | 13 (52%) | 4 (16%) | 0 (0%) | 8 (32%) | 47 | 73.4 | 164.6 |
| jenkins (pipeline-library) | 18 | 11 (61%) | 7 (39%) | 0 (0%) | 0 (0%) | 54 | 59.0 | 91.8 |
| solidity (openzeppelin-contracts) | 158 | 103 (65%) | 55 (35%) | 0 (0%) | 0 (0%) | 112 | 7.6 | 9.2 |
| starlark (rules_go) | 84 | 79 (94%) | 2 (2%) | 0 (0%) | 3 (4%) | 186 | 5.1 | 8.4 |
| julia (DataFrames.jl) | 177 | 67 (38%) | 78 (44%) | 7 (4%) | 25 (14%) | 123 | 14.0 | 31.6 |
| nix (nix-darwin) | 49 | 39 (80%) | 10 (20%) | 0 (0%) | 0 (0%) | 51 | 8.1 | 24.4 |
| css (bootstrap) | 230 | 164 (71%) | 56 (24%) | 1 (0%) | 9 (4%) | 70 | 4.9 | 36.1 |
| vue (gitea) | 218 | 123 (56%) | 42 (19%) | 3 (1%) | 50 (23%) | 52 | 52.4 | 441.4 |
| svelte (immich) | 182 | 115 (63%) | 42 (23%) | 5 (3%) | 20 (11%) | 88 | 76.2 | 500.0 |
| astro (starlight) | 167 | 112 (67%) | 13 (8%) | 1 (1%) | 41 (25%) | 103 | 83.2 | 358.9 |
| clojure (babashka) | 100 | 65 (65%) | 22 (22%) | 0 (0%) | 13 (13%) | 170 | 5.0 | 14.0 |
| elisp (magit) | 39 | 28 (72%) | 4 (10%) | 0 (0%) | 7 (18%) | 41 | 4.1 | 5.1 |
| racket (drracket) | 108 | 65 (60%) | 14 (13%) | 0 (0%) | 29 (27%) | 128 | 13.2 | 16.1 |
| scheme (chibi-scheme) | 54 | 29 (54%) | 16 (30%) | 0 (0%) | 9 (17%) | 17 | 8.0 | 9.7 |
| commonlisp (lem) | 52 | 34 (65%) | 3 (6%) | 0 (0%) | 15 (29%) | 26 | 7.2 | 8.1 |
| bash (bats-core, acme.sh) | 97 | 34 (35%) | 17 (18%) | 0 (0%) | 46 (47%) | 12 | 3.3 | 11.4 |
| lua (nvim-treesitter, kong) | 63 | 44 (70%) | 3 (5%) | 1 (2%) | 15 (24%) | 47 | 4.3 | 24.4 |
| dart (dio) | 97 | 46 (47%) | 33 (34%) | 8 (8%) | 10 (10%) | 17 | 26.9 | 87.6 |
| sql (lemmy) | 79 | 22 (28%) | 13 (16%) | 0 (0%) | 44 (56%) | 31 | 10.7 | 14.9 |
