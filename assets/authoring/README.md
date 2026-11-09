## Pilot idle and walk: whole drawings, all eight directions (TAKOAI-130)

Every Pilot frame is a complete drawing; see `design/ART_GUIDELINES.md`
§ Animation frames. Do not composite new parts onto an existing frame. To
change a pose, regenerate or repaint that facing's sheet and re-import it.

`pilot-walk-<dir>.png` (E, SE, S, NE, N) is one row of five whole characters
on flat magenta: idle, then walk_1 contact, walk_2 passing, walk_3 opposite
contact (legs exchanged), walk_4 opposite passing. Each sheet was produced in
one generation with Codex image generation (`codex exec`, image generation
tool), using that facing's previous idle frame on white at 4× as the
character and style reference. The exact prompt is saved beside it as
`pilot-walk-<dir>-prompt.txt`. W, SW and NW mirror E, SE and NE in the client.

Arms follow the walk (TAKOAI-130 review): only one hand is forward at a
time, opposite to the leg on its side, and hands stay at or below belly
height. In the E side view the far arm is hidden behind the body and not
drawn. The SE sheet was regenerated with two references, the SE idle frame
and the previous SE sheet (`pilot-walk-se.png` at commit `de1673a`), to keep
its alternating leg poses while redrawing the arms; regenerating it from
text alone repeatedly gave the same foot forward in both contact frames.

`pilot_walk_import` keys out the magenta, cuts the five drawings at the empty
columns between them, and scales the whole sheet once so idle is 150 px tall.
It places each frame on the 165×192 canvas with its feet on row 181 and its
head at idle's column. It doesn't paint, erase or move any part of a drawing.

Rebuild from the repository root:

```sh
export CARGO_TARGET_DIR=/Users/chenwei/.cache/oni-spacewar-target
cargo run -p oni-content --bin pilot_walk_import
cargo run -p oni-content --bin content-pipeline
cargo test -p oni-content --test character_frames
```

Runtime frames are `assets/source/core/carrier/pilot/<dir>/{idle,walk_1..4}.png`.
Processed copies, compressed bundles and manifest hashes are rebuilt together.
`--autoplay --shots DIR --clip` records every facing in the real game window
as `11c-stride-<dir>-00.png` through `10.png`.
