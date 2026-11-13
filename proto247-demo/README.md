# The recordings of #247, round 7

Throwaway, never merged. Each GIF in the issue is one line of this shape, run from the checkout
after `cargo build --release`:

    proto247-demo/rec.sh OUT.gif STEPS BIN SIZE SCENE READ GONE [cast.py options]

`STEPS` is a file of `steps/` without its extension, `BIN` a merl binary, `SCENE` a name not used
yet (the demo folder is built at `/tmp/247r7/SCENE/services`), `READ` how many branches of the
batch are already ticked (`none` for no batch yet), `GONE` a `repo:branch` deleted on origin or
`-`. The first run clones the two source projects into `proto247-demo/src/`.

| GIF | Line |
|---|---|
| now | `rec.sh now.gif now MASTER 160x50 now none -` |
| tour | `rec.sh tour.gif tour BIN 160x50 tour none -` |
| start | `rec.sh start.gif start BIN 160x50 start none -` |
| cross | `rec.sh cross.gif cross BIN 160x50 cross 0 -` |
| list | `rec.sh list.gif list BIN 160x50 list 2 -` |
| nextday | `rec.sh nextday.gif nextday BIN 160x50 nextday 3 -` |
| end | `rec.sh end.gif end BIN 160x50 end 4 -` |
| agent | `rec.sh agent.gif agent BIN 160x50 agent 3 -` |
| empty | `rec.sh empty.gif empty BIN 160x50 empty 5 -` |
| narrow | `rec.sh narrow.gif narrow BIN 80x24 narrow 0 -` |
| start-a | `rec.sh start-a.gif startplain BIN 160x50 start-a 0 -` |
| start-b | `MERL_PROTO_247_START=list rec.sh start-b.gif startlist BIN 160x50 start-b 0 -` |
| gone-a | `rec.sh gone-a.gif gone BIN 160x50 gone-a 2 web:ACME-91` |
| gone-b | `MERL_PROTO_247_GONE=leave rec.sh gone-b.gif gone BIN 160x50 gone-b 2 web:ACME-91` |
| says-a | `rec.sh says-a.gif says BIN 160x50 says-a 2 -` |
| says-b | `MERL_PROTO_247_SAYS=less rec.sh says-b.gif says BIN 160x50 says-b 2 -` |
| light | `rec.sh light.gif light BIN 160x50 light 0 - --ghostty 'TokyoNight Day'` |
| plain-master, plain-proto | `rec.sh plain.gif plain MASTER-or-BIN 160x50 plain none -` |
