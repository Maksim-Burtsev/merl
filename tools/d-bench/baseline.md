# `d` bench baseline

merl at `f20c52b1` with the fix of #975 (ts rows), 2026-10-17, release build, `vm.loadavg` { 10.10 8.84 8.35 } before the run; the csharp row with the fixes of #935, `vm.loadavg` { 5.00 5.48 5.37 }; the other rows as in the night's second epic.
The bash and lua rows (#1091): merl at `0e47807c`, 2026-10-18, release build, `vm.loadavg` { 4.68 5.99 6.51 } before the run and after.
The rust row: merl at `20ac5110` with the `d` fixes of branch `b1019/rust`, 2026-10-19, release build, `vm.loadavg` { 6.98 6.29 6.45 } before the run and after.
The ts, js, vue, svelte and astro rows: merl at `aaa71b2c` with the `d` fixes of branch `b1019/jsts` (#1079), 2026-10-19, release build, `vm.loadavg` { 7.19 7.26 6.59 } before the run and after.

| language (project) | scored | direct hit | picker with the answer | wrong jump | miss | not scored | p50 ms | p90 ms |
|---|---|---|---|---|---|---|---|---|
| python (paperless-ngx) | 213 | 199 (93%) | 8 (4%) | 0 (0%) | 6 (3%) | 57 | 13.6 | 258.1 |
| ts (outline) | 122 | 116 (95%) | 3 (2%) | 1 (1%) | 2 (2%) | 178 | 45.8 | 110.8 |
| js (eslint) | 147 | 119 (81%) | 21 (14%) | 0 (0%) | 7 (5%) | 123 | 41.9 | 184.2 |
| go (caddy) | 229 | 224 (98%) | 3 (1%) | 0 (0%) | 2 (1%) | 41 | 5.3 | 15.3 |
| rust (ripgrep) | 231 | 203 (88%) | 24 (10%) | 0 (0%) | 4 (2%) | 69 | 6.5 | 77.2 |
| c (redis) | 250 | 232 (93%) | 18 (7%) | 0 (0%) | 0 (0%) | 20 | 24.7 | 33.3 |
| cpp (leveldb) | 232 | 182 (78%) | 48 (21%) | 0 (0%) | 2 (1%) | 68 | 22.8 | 94.6 |
| php (koel) | 104 | 99 (95%) | 5 (5%) | 0 (0%) | 0 (0%) | 196 | 13.8 | 25.9 |
| swift (Alamofire) | 154 | 115 (75%) | 37 (24%) | 0 (0%) | 2 (1%) | 116 | 10.2 | 16.7 |
| java (halo) | 37 | 35 (95%) | 2 (5%) | 0 (0%) | 0 (0%) | 43 | 51.2 | 97.0 |
| kotlin (nowinandroid) | 34 | 34 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 46 | 26.5 | 55.9 |
| csharp (eShop) | 27 | 25 (93%) | 2 (7%) | 0 (0%) | 0 (0%) | 53 | 18.0 | 28.9 |
| ruby (mastodon) | 28 | 22 (79%) | 5 (18%) | 0 (0%) | 1 (4%) | 52 | 64.6 | 141.3 |
| objc (SDWebImage) | 229 | 92 (40%) | 122 (53%) | 0 (0%) | 15 (7%) | 41 | 26.3 | 110.1 |
| groovy (nextflow) | 25 | 13 (52%) | 4 (16%) | 0 (0%) | 8 (32%) | 47 | 73.4 | 164.6 |
| jenkins (pipeline-library) | 18 | 11 (61%) | 7 (39%) | 0 (0%) | 0 (0%) | 54 | 59.0 | 91.8 |
| solidity (openzeppelin-contracts) | 158 | 103 (65%) | 55 (35%) | 0 (0%) | 0 (0%) | 112 | 7.6 | 9.2 |
| starlark (rules_go) | 84 | 79 (94%) | 2 (2%) | 0 (0%) | 3 (4%) | 186 | 5.1 | 8.4 |
| julia (DataFrames.jl) | 177 | 67 (38%) | 78 (44%) | 7 (4%) | 25 (14%) | 123 | 14.0 | 31.6 |
| nix (nix-darwin) | 49 | 39 (80%) | 10 (20%) | 0 (0%) | 0 (0%) | 51 | 8.1 | 24.4 |
| css (bootstrap) | 230 | 164 (71%) | 56 (24%) | 1 (0%) | 9 (4%) | 70 | 4.9 | 36.1 |
| vue (gitea) | 218 | 123 (56%) | 42 (19%) | 2 (1%) | 51 (23%) | 52 | 50.3 | 169.6 |
| svelte (immich) | 182 | 115 (63%) | 42 (23%) | 5 (3%) | 20 (11%) | 88 | 35.5 | 175.6 |
| astro (starlight) | 167 | 112 (67%) | 13 (8%) | 1 (1%) | 41 (25%) | 103 | 34.5 | 100.8 |
| clojure (babashka) | 100 | 65 (65%) | 22 (22%) | 0 (0%) | 13 (13%) | 170 | 5.0 | 14.0 |
| elisp (magit) | 39 | 28 (72%) | 4 (10%) | 0 (0%) | 7 (18%) | 41 | 4.1 | 5.1 |
| racket (drracket) | 108 | 65 (60%) | 14 (13%) | 0 (0%) | 29 (27%) | 128 | 13.2 | 16.1 |
| scheme (chibi-scheme) | 54 | 29 (54%) | 16 (30%) | 0 (0%) | 9 (17%) | 17 | 8.0 | 9.7 |
| commonlisp (lem) | 52 | 34 (65%) | 3 (6%) | 0 (0%) | 15 (29%) | 26 | 7.2 | 8.1 |
| bash (bats-core, acme.sh) | 97 | 34 (35%) | 17 (18%) | 0 (0%) | 46 (47%) | 12 | 3.3 | 11.4 |
| lua (nvim-treesitter, kong) | 63 | 44 (70%) | 3 (5%) | 1 (2%) | 15 (24%) | 47 | 4.3 | 24.4 |
