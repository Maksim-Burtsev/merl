# `d` bench baseline

merl at `2771821d`, 2026-10-03, release build, `vm.loadavg` { 3.17 8.28 10.83 } before the run, { 4.83 6.51 9.47 } after, the epic of 18 PRs scored from the run's `last-merl.tsv` (#649). julia (DataFrames.jl) added at master `41df0a48` merged into #672 (#429, #653, #674), `vm.loadavg` { 48.49 38.25 23.30 }: its times are high by that load.

| language (project) | scored | direct hit | picker with the answer | wrong jump | miss | not scored | p50 ms | p90 ms |
|---|---|---|---|---|---|---|---|---|
| python (paperless-ngx) | 213 | 199 (93%) | 8 (4%) | 0 (0%) | 6 (3%) | 57 | 13.3 | 338.3 |
| ts (outline) | 122 | 109 (89%) | 8 (7%) | 1 (1%) | 4 (3%) | 178 | 35.6 | 84.1 |
| js (eslint) | 147 | 119 (81%) | 21 (14%) | 0 (0%) | 7 (5%) | 123 | 29.3 | 422.9 |
| go (caddy) | 229 | 224 (98%) | 3 (1%) | 0 (0%) | 2 (1%) | 41 | 5.5 | 16.0 |
| rust (ripgrep) | 231 | 183 (79%) | 44 (19%) | 0 (0%) | 4 (2%) | 69 | 5.7 | 69.2 |
| c (redis) | 250 | 232 (93%) | 18 (7%) | 0 (0%) | 0 (0%) | 20 | 22.6 | 32.4 |
| cpp (leveldb) | 233 | 127 (55%) | 95 (41%) | 2 (1%) | 9 (4%) | 67 | 13.1 | 62.7 |
| php (koel) | 104 | 99 (95%) | 5 (5%) | 0 (0%) | 0 (0%) | 196 | 13.7 | 25.4 |
| swift (Alamofire) | 154 | 115 (75%) | 37 (24%) | 0 (0%) | 2 (1%) | 116 | 10.0 | 16.4 |
| java (halo) | 37 | 35 (95%) | 2 (5%) | 0 (0%) | 0 (0%) | 43 | 49.5 | 95.5 |
| kotlin (nowinandroid) | 34 | 34 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 46 | 25.1 | 53.6 |
| csharp (eShop) | 27 | 23 (85%) | 4 (15%) | 0 (0%) | 0 (0%) | 53 | 18.2 | 31.7 |
| ruby (mastodon) | 28 | 22 (79%) | 5 (18%) | 0 (0%) | 1 (4%) | 52 | 44.4 | 122.0 |
| objc (SDWebImage) | 229 | 92 (40%) | 122 (53%) | 0 (0%) | 15 (7%) | 41 | 18.0 | 131.9 |
| groovy (nextflow) | 25 | 13 (52%) | 4 (16%) | 0 (0%) | 8 (32%) | 47 | 71.8 | 157.7 |
| jenkins (pipeline-library) | 18 | 11 (61%) | 7 (39%) | 0 (0%) | 0 (0%) | 54 | 63.2 | 93.5 |
| solidity (openzeppelin-contracts) | 158 | 103 (65%) | 55 (35%) | 0 (0%) | 0 (0%) | 112 | 7.8 | 10.5 |
| starlark (rules_go) | 84 | 79 (94%) | 2 (2%) | 0 (0%) | 3 (4%) | 186 | 5.1 | 8.6 |
| julia (DataFrames.jl) | 177 | 67 (38%) | 78 (44%) | 7 (4%) | 25 (14%) | 123 | 26.2 | 65.4 |
