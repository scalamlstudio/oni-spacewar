Battle feedback v1 is an exploratory import option for TAKOAI-111. The issue explicitly requests TAKOAI-51 flat battle styling; current general guidance is `design/ART_GUIDELINES.md` (the issue's `design/art/ART_GUIDELINES.md` path does not exist). Sources extend the existing demo-v2 SVG effects workflow; no image generation or character redesign is involved.

All runtime PNGs have transparent backgrounds, no text, and one horizontal row of authored 256 × 256 frames. Frames have a common center (128,128) and at least eight transparent pixels at every edge. Read left to right, once; remove at end, do not loop. SVGs are editable sources. These are constructed frames, not slices of an AI sheet. `preview-v1.png` is documentation only and must not be imported as runtime art.

| PNG stem (append `-v1.png`) | Frames | Strip size | Suggested ms/frame | Suggested world size |
| --- | --- | --- | --- | --- |
| hit-enemy | 4 | 1024 × 256 | 35 | 48 |
| hit-ship | 4 | 1024 × 256 | 35 | 48 |
| death-swarmer | 6 | 1536 × 256 | 45 | 80 |
| death-spitter | 6 | 1536 × 256 | 45 | 112 |
| shockwave-impact | 4 | 1024 × 256 | 45 | 160 |
| scatter-impact | 4 | 1024 × 256 | 35 | 64 |
| spitter-windup | 4 | 1024 × 256 | 100 | 96 |
| spitter-aim-line | 1 | 256 × 256 | static | see below |

Timing and world sizes except the requested 0.4 s wind-up are starting suggestions for Engineer to tune at battle zoom. No gameplay code or timing is changed. Effects use normal straight-alpha blending; optional additive blending should be judged in game. Fade the last burst/impact frame to zero rather than holding its debris/ring. Render effects above enemy sprites so the early flash reads. Swarmer and Spitter bursts share a palette but use differently oriented flash/fragments and suggested sizes.

For the aim line, use the inner UV region `(16,120,224,16)` and stretch along +X. Position its cropped start at the muzzle, rotate toward the aim point, stretch its length to that point, and use roughly 4–6 world px thickness. The unused padding is deliberately transparent. Line position does not encode a new attack rule. Wind-up overlay is centered on the Spitter; four 100 ms frames cover 0.4 seconds.

`russo_one.ttf` is an unmodified copy of the existing core font with `OFL.txt`. Render damage as runtime text, never as digit-sheet pixels. Suggested normal `#FFF3CC` at 18 world px; crit/big `#FFD15C` at 25; ship damage `#FF789E` at 20 with a minus sign. Use a 2 px `#241032` outline. Size and sign provide additional cues besides colour. These are style options, not locked art direction.

`manifest-v1.json` carries frame counts, pivots, timing, colours and suggested sizes. Rebuild PNGs on macOS with `python3 design/art/battle/feedback/build-v1.py` (Google Chrome required). Verify output dimensions, nonempty frames and transparent margins with `python3 design/art/battle/feedback/verify-v1.py`. Preview inspected on a dark battle-like background; final in-game readability is part of Engineer's import step.
