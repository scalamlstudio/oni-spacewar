## Pilot walk: original proportions, all eight directions (TAKOAI-130)

Wei requested alternating legs in every direction while retaining the original
short-legged body ratio. Runtime sprites keep the original upper-body pixels;
only a 19 px foot band below row 162 can animate, ending at the original row
181 baseline. No generated head, torso or longer thigh is imported.

`pilot-walk-bases/` contains the original idle frames from commit `781acf3`.
The N view uses that commit's `walk_1` head above row 96, since the legacy N
idle ears sit lower; its original idle torso remains unchanged. This retains
the existing character-height contract without resizing any body part.

The five transparent `pilot-short-walk-<dir>.png` sheets were produced with
the built-in imagegen tool, referencing the original idle sprite for that
view. Exact prompts are saved beside them as `*-prompt.txt`. The importer
extracts a stable pair of tiny boots from the first cell's bottom nine
percent. Each boot keeps its identity throughout the cycle; the importer
moves them in opposite projected directions, rather than relying on the
generator to keep left/right identities consistent across four new bodies.

Phases: first foot forward / second back; first planted / second passing;
first back / second forward; first passing / second planted. Diagonals use
both horizontal travel and depth; N/S swap foot depth. Original rounded
waist/belly and the back-view tail remain in front of the feet. E, SE, S,
NE and N are authored; W, SW and NW mirror E, SE and NE in the client.
All eight facings advance one pose per 24 px travelled: a 0.6 s cycle at
160 px/s. Character scale and movement speed are unchanged.

Rebuild from the repository root:

```sh
export CARGO_TARGET_DIR=/Users/chenwei/.cache/oni-spacewar-target
cargo run -p oni-content --bin pilot_stride_import
cargo run -p oni-content --bin content-pipeline
cargo test -p oni-content --test character_frames
```

The source frames remain `assets/source/core/carrier/pilot/<dir>/walk_1.png`
through `walk_4.png`; processed copies, compressed bundles and manifest hashes
are rebuilt together. Tests enforce the original upper pixels, canvas,
feet baseline, per-view distinct poses and packaged-source consistency.
`--autoplay --shots DIR --clip` records every facing in the real game window
as `11c-stride-<dir>-00.png` through `10.png`.
