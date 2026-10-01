# `d` bench baseline

merl at `2af1537`, 2026-10-01, release build, `vm.loadavg` { 1.89 2.46 3.21 } before the run, { 1.85 2.40 3.04 } after. php (koel) again at `3d72d68c` (#351), `vm.loadavg` { 18.71 18.96 28.80 }: its times are high by that load. swift (Alamofire) again at `1bde563f` (#375), `vm.loadavg` { 6.21 8.92 18.31 } before, { 11.91 9.53 17.86 } after: its times are high by that load. csharp (eShop) again at `b8e99295` (#349, #360).

| language (project) | scored | direct hit | picker with the answer | wrong jump | miss | not scored | p50 ms | p90 ms |
|---|---|---|---|---|---|---|---|---|
| python (paperless-ngx) | 215 | 175 (81%) | 21 (10%) | 0 (0%) | 19 (9%) | 55 | 15.3 | 945.9 |
| ts (outline) | 122 | 109 (89%) | 8 (7%) | 1 (1%) | 4 (3%) | 178 | 48.5 | 116.9 |
| js (eslint) | 147 | 116 (79%) | 21 (14%) | 1 (1%) | 9 (6%) | 123 | 40.4 | 556.8 |
| go (caddy) | 229 | 224 (98%) | 3 (1%) | 0 (0%) | 2 (1%) | 41 | 6.2 | 14.8 |
| rust (ripgrep) | 231 | 183 (79%) | 44 (19%) | 0 (0%) | 4 (2%) | 69 | 7.1 | 125.0 |
| c (redis) | 250 | 226 (90%) | 24 (10%) | 0 (0%) | 0 (0%) | 20 | 36.9 | 118.9 |
| cpp (leveldb) | 233 | 118 (51%) | 96 (41%) | 2 (1%) | 17 (7%) | 67 | 12.7 | 100.5 |
| php (koel) | 104 | 99 (95%) | 5 (5%) | 0 (0%) | 0 (0%) | 196 | 22.8 | 43.0 |
| swift (Alamofire) | 154 | 115 (75%) | 37 (24%) | 0 (0%) | 2 (1%) | 116 | 22.0 | 48.6 |
| java (halo) | 37 | 34 (92%) | 3 (8%) | 0 (0%) | 0 (0%) | 43 | 38.9 | 84.1 |
| kotlin (nowinandroid) | 30 | 30 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 50 | 19.8 | 35.0 |
| csharp (eShop) | 27 | 23 (85%) | 4 (15%) | 0 (0%) | 0 (0%) | 53 | 22.7 | 44.5 |
| ruby (mastodon) | 28 | 22 (79%) | 5 (18%) | 0 (0%) | 1 (4%) | 52 | 47.3 | 152.6 |
