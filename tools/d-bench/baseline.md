# `d` bench baseline

merl at `2af1537`, 2026-10-01, release build, `vm.loadavg` { 1.89 2.46 3.21 } before the run, { 1.85 2.40 3.04 } after. php (koel) again at `3d72d68c` (#351), `vm.loadavg` { 18.71 18.96 28.80 }: its times are high by that load. swift (Alamofire) again at `1bde563f` (#375), `vm.loadavg` { 6.21 8.92 18.31 } before, { 11.91 9.53 17.86 } after: its times are high by that load. csharp (eShop) again at `b8e99295` (#349, #360). js (eslint) again at `fb3cf164` (#347), `vm.loadavg` { 8.42 7.51 11.17 }: its times are high by that load. c (redis) and cpp (leveldb) again at `0a54d385` (#382), `vm.loadavg` { 18.14 9.24 7.07 }. objc (SDWebImage) added at `a84351e7` and its review fixes (#417), `vm.loadavg` { 6.05 6.09 7.73 } before, { 7.29 6.82 7.88 } after. nix (nix-darwin) added at `35684017` (#620), `vm.loadavg` { 24.49 30.54 24.43 }: its times are high by that load. css (bootstrap) added at `dce59c0d` (#590), `vm.loadavg` { 8.51 15.26 31.33 }. vue (gitea), svelte (immich) and astro (starlight) added at `35684017` (#595), `vm.loadavg` { 9.25 15.62 31.64 } before, { 26.57 19.87 27.63 } after: their times are high by that load.

| language (project) | scored | direct hit | picker with the answer | wrong jump | miss | not scored | p50 ms | p90 ms |
|---|---|---|---|---|---|---|---|---|
| python (paperless-ngx) | 215 | 175 (81%) | 21 (10%) | 0 (0%) | 19 (9%) | 55 | 15.3 | 945.9 |
| ts (outline) | 122 | 109 (89%) | 8 (7%) | 1 (1%) | 4 (3%) | 178 | 48.5 | 116.9 |
| js (eslint) | 147 | 119 (81%) | 21 (14%) | 0 (0%) | 7 (5%) | 123 | 45.8 | 1546.3 |
| go (caddy) | 229 | 224 (98%) | 3 (1%) | 0 (0%) | 2 (1%) | 41 | 6.2 | 14.8 |
| rust (ripgrep) | 231 | 183 (79%) | 44 (19%) | 0 (0%) | 4 (2%) | 69 | 7.1 | 125.0 |
| c (redis) | 250 | 232 (93%) | 18 (7%) | 0 (0%) | 0 (0%) | 20 | 42.8 | 59.9 |
| cpp (leveldb) | 233 | 127 (55%) | 95 (41%) | 2 (1%) | 9 (4%) | 67 | 14.1 | 113.9 |
| php (koel) | 104 | 99 (95%) | 5 (5%) | 0 (0%) | 0 (0%) | 196 | 22.8 | 43.0 |
| swift (Alamofire) | 154 | 115 (75%) | 37 (24%) | 0 (0%) | 2 (1%) | 116 | 22.0 | 48.6 |
| java (halo) | 37 | 34 (92%) | 3 (8%) | 0 (0%) | 0 (0%) | 43 | 38.9 | 84.1 |
| kotlin (nowinandroid) | 30 | 30 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 50 | 19.8 | 35.0 |
| csharp (eShop) | 27 | 23 (85%) | 4 (15%) | 0 (0%) | 0 (0%) | 53 | 22.7 | 44.5 |
| ruby (mastodon) | 28 | 22 (79%) | 5 (18%) | 0 (0%) | 1 (4%) | 52 | 47.3 | 152.6 |
| objc (SDWebImage) | 229 | 92 (40%) | 122 (53%) | 0 (0%) | 15 (7%) | 41 | 25.1 | 257.9 |
| nix (nix-darwin) | 49 | 39 (80%) | 10 (20%) | 0 (0%) | 0 (0%) | 51 | 5.9 | 14.6 |
| css (bootstrap) | 230 | 164 (71%) | 56 (24%) | 1 (0%) | 9 (4%) | 70 | 4.1 | 36.2 |
| vue (gitea) | 218 | 123 (56%) | 42 (19%) | 3 (1%) | 50 (23%) | 52 | 60.3 | 1935.5 |
| svelte (immich) | 182 | 115 (63%) | 42 (23%) | 5 (3%) | 20 (11%) | 88 | 52.1 | 1487.7 |
| astro (starlight) | 167 | 112 (67%) | 13 (8%) | 1 (1%) | 41 (25%) | 103 | 81.4 | 2063.8 |
