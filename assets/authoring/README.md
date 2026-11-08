## Pilot side walk v2 (TAKOAI-130)

`pilot-side-walk-v2.png` is the transparent four-column sheet generated with
the built-in imagegen tool using the previous east-facing `walk_1.png` as
the edit reference (`781acf3:assets/source/core/carrier/pilot/e/walk_1.png`). Wei requested legs that swing alternately forward and
backward instead of holding the same split stance. Left-facing movement
mirrors these east-facing frames in the client.

The four phases are near-leg forward contact, near-leg planted/far-leg
passing, far-leg forward contact, and far-leg planted/near-leg passing.
The planted boot baseline, head and hip origins share one transform across
all four cells; the importer never independently recentres each pose.

Regenerate the runtime files from the repository root:

```sh
export CARGO_TARGET_DIR=/Users/chenwei/.cache/oni-spacewar-target
cargo run -p oni-content --bin pilot_stride_import
cargo run -p oni-content --bin content-pipeline
cargo test -p oni-content --test character_frames
```

The runtime frames remain `assets/source/core/carrier/pilot/e/walk_1.png`
through `walk_4.png`, on the existing 165×192 canvas. The client uses 24 px
travel per phase for these sideways poses: 0.6 seconds per full cycle at
the unchanged 160 px/s walking speed. Other facing timings are unchanged.

### Generation prompt

Edit target: the attached game sprite. Create ONE production sprite sheet,
4 equal-width columns in one horizontal row on a genuinely transparent
background, no labels. Same orange cat pilot in teal flight suit, goggles,
backpack and orange tail, facing RIGHT in exact side profile, same style,
scale, head, face, torso and hip position in every column. ONLY change the
leg and arm walking poses. The current image has static split legs;
replace with a readable natural alternating bipedal WALK cycle, NOT
running/hopping. Four phases left to right: (1) near leg extends
FORWARD/right with heel touching ground, far leg extends BACK/left with
toe touching; (2) near foot passes directly BELOW hip planted, far knee
bent lifting to pass forward; (3) near leg extends BACK/left, far leg
extends FORWARD/right heel planted, the exact opposite of phase1; (4) far
foot planted BELOW hip, near knee bent lifting forward. Both distinct
teal boots must visibly exchange front/back positions between columns 1
and 3; do not keep the same foot in front across frames. Opposite arm
swings subtly. Keep upper body and head absolutely consistent in position,
proportions and size, no head turning, no changing costume or expression,
no dramatic bob, all planted feet on the exact same horizontal baseline.
Full body each cell, generous transparent margins, no cropping, no ground,
no shadow, no text, no checkerboard. Crisp illustrated sprite matching
the reference.
