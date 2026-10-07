# `d` bench baseline

merl at the merge of master `7155f91b` into the epic `0deed2e8` (#645), 2026-10-04, release build, `vm.loadavg` { 2.74 4.76 4.30 } before the run, { 23.02 11.82 7.34 } after: the later rows' times are high by that load. julia (DataFrames.jl) again with Julia 1.11.7 on the PATH and the depot of #653, `vm.loadavg` { 10.27 10.30 7.08 }. clojure (babashka), elisp (magit), racket (drracket), scheme (chibi-scheme) and commonlisp (lem) added at `af4311bd` (#428), 2026-10-06, `vm.loadavg` { 2.26 2.69 8.36 } before, { 2.69 2.73 7.99 } after. The five Lisps again on the branch of #731–#735, 2026-10-06, `vm.loadavg` { 10.05 12.30 21.87 } before. ts (outline) and js (eslint) again on the branch of #318, 2026-10-07, `vm.loadavg` { 4.93 11.18 27.84 } before, { 5.10 10.59 27.05 } after.

| language (project) | scored | direct hit | picker with the answer | wrong jump | miss | not scored | p50 ms | p90 ms |
|---|---|---|---|---|---|---|---|---|
| python (paperless-ngx) | 213 | 199 (93%) | 8 (4%) | 0 (0%) | 6 (3%) | 57 | 13.6 | 258.1 |
| ts (outline) | 122 | 114 (93%) | 5 (4%) | 1 (1%) | 2 (2%) | 178 | 41.5 | 82.3 |
| js (eslint) | 147 | 119 (81%) | 21 (14%) | 0 (0%) | 7 (5%) | 123 | 38.6 | 152.5 |
| go (caddy) | 229 | 224 (98%) | 3 (1%) | 0 (0%) | 2 (1%) | 41 | 5.3 | 15.3 |
| rust (ripgrep) | 231 | 183 (79%) | 44 (19%) | 0 (0%) | 4 (2%) | 69 | 5.7 | 67.7 |
| c (redis) | 250 | 232 (93%) | 18 (7%) | 0 (0%) | 0 (0%) | 20 | 22.5 | 32.3 |
| cpp (leveldb) | 232 | 177 (76%) | 53 (23%) | 0 (0%) | 2 (1%) | 68 | 14.6 | 61.2 |
| php (koel) | 104 | 99 (95%) | 5 (5%) | 0 (0%) | 0 (0%) | 196 | 13.8 | 25.9 |
| swift (Alamofire) | 154 | 115 (75%) | 37 (24%) | 0 (0%) | 2 (1%) | 116 | 10.2 | 16.7 |
| java (halo) | 37 | 35 (95%) | 2 (5%) | 0 (0%) | 0 (0%) | 43 | 51.2 | 97.0 |
| kotlin (nowinandroid) | 34 | 34 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 46 | 26.5 | 55.9 |
| csharp (eShop) | 27 | 23 (85%) | 4 (15%) | 0 (0%) | 0 (0%) | 53 | 18.6 | 30.7 |
| ruby (mastodon) | 28 | 22 (79%) | 5 (18%) | 0 (0%) | 1 (4%) | 52 | 64.6 | 141.3 |
| objc (SDWebImage) | 229 | 92 (40%) | 122 (53%) | 0 (0%) | 15 (7%) | 41 | 34.5 | 144.3 |
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
