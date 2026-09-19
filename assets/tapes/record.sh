#!/bin/sh
# Records one tape with vhs and writes the GIF next to it:
#     assets/tapes/setup.sh && assets/tapes/record.sh assets/demo.tape
# Run from the repository root after cargo build --release. vhs records the frames; the GIF is
# (2x, at the tape's Framerate 25) are scaled to 1200 px, padded and put together here, because the ffmpeg call inside vhs 0.12 writes nothing with ffmpeg 9.
set -eu
tape=$1
gif=${tape%.tape}.gif
frames=target/demo-frames/$(basename "$tape" .tape)
rm -rf "$frames" && mkdir -p "$(dirname "$frames")"
sed "s|^Output .*|Output $frames/|" "$tape" > "$frames.tape"
vhs "$frames.tape" > /dev/null
ffmpeg -y -v error -framerate 25 -i "$frames/frame-text-%05d.png" \
    -framerate 25 -i "$frames/frame-cursor-%05d.png" -filter_complex \
    "[0][1]overlay,scale=1176:-2:flags=lanczos,pad=1200:ih+24:12:12:color=0x222436[m];[m]fps=${FPS:-12.5},split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle" \
    "$gif"
ls -l "$gif"
