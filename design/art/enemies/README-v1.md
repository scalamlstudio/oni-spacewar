Exploratory battle enemy art options for TAKOAI-114. Generated with the built-in imagegen tool; issue-specific bold flat styling overrides the painted rendering guidance. Violet outlines and pink/lavender crystals connect the new catlike creatures to the existing void family. These are source sheets for the asset pipeline, not runtime-ready strips or locked art direction.

All seven `*-source-v1.png` files are **2172 × 724 px**, with one left-to-right row and a white background. They contain 35 semantic items total. `manifest-v1.json` records exact ordering. `preview-v1.html` embeds all originals; `preview-v1.png` is a documentation contact sheet.

| Source filename | Items, left to right |
| --- | --- |
| lancer-source-v1.png | move, wind-up, dash smear, ground charge line, death burst |
| bloater-source-v1.png | move, swelling with ring, standalone warning ring, explosion, fading death burst |
| brood-source-v1.png | idle, sac wind-up, standalone pulsing sac, death burst |
| warden-source-v1.png | idle, tether beam, shield shimmer, death burst |
| fissure-guardian-source-v1.png | phase 1, phase 2, shot-ring flash, charge line, death burst |
| elite-affixes-source-v1.png | Swift, Volatile, Warding, Blinking, Splitting, Scorching, Chilling |
| elite-effects-source-v1.png | plain aura option, jagged aura option, crown aura option, burning patch, teleport mark |

Use the pipeline's cutout tool; do not equal-width slice these sheets. Disconnected particles belong to their parent effect. Remove background inside rings too, while preserving white creature highlights. Source poses have different bounding boxes and are not animation cells. No sheets have been sliced here.

Suggested padded output frame sizes after semantic cutout: ordinary enemies, auras, patches and normal bursts **256 × 256**; Guardian phases and death **512 × 512**; icons **64 × 64** (display around 24 px); charge lines **512 × 64**; tether **256 × 32**. Keep centered pivots for bodies/rings and an origin at the start of beams/lines. Suggested ordinary body display size is 56 px and Guardian 140 px, so the boss reads distinctly larger. These are import suggestions, not simulation changes.

Use scale/brightness/opacity animation for the key poses: the Lancer line covers the design's 0.7 s warning; the Bloater ring grows over 1 s. Render its explosion to match the authoritative AoE (approximately 180 px diameter if 90 px is the radius). Stretch charge lines to the simulation's aim segment and tethers between entity positions; decorative arrowheads are not extra range. The Warden shimmer and elite rings have empty centers for overlay use. Guardian phase 2 exposes a brighter core and cracked armor; its flash is a warning effect, not a projectile pattern. The client owns shot positions and safe gaps.

Three elite auras are alternatives for later comparison; none is selected as final. In-game scale, icon readability, alpha cleanup and import remain Engineer's integration work. Sources contain no gameplay implementation.

Generation brief: `prompts-v1.json` records the prompt specification and cleanup pass. All sheets were visually inspected for item order, gaps, margins and coverage. Regenerate the HTML preview and manifest from existing PNGs with `python3 design/art/enemies/prepare-v1.py`. PNG preview is a browser capture of that HTML, not sliced art.
