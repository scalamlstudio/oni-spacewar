Clean office tile option — TAKOAI-89 reopened scope

Selected defaults: cool light-grey floor; charcoal/slate wall; cyan guide light;
48 px cap/side runs and 32 px downward face extrusion. Entire 128×128 tile
is opaque with floor showing outside the wall strip. Rounded cap endpoints
also serve as slim frame posts at the outer wall ends of paired doorways.
Thresholds contain no repeated post. No noise, texture, rivets or tile frames.

Files: floor-0..2-v3, wall-<NESW>-v3 (16), doorway-ns/ew-v4.
Existing stations, scaffold and front marker remain unchanged.
`batch-a-import-v3.json` preserves all 29 stable IDs and source destinations.
Engineer imports the catalogue; no assets/source or manifest writes in this pass.

Reproduce: `uv run --with pillow python design/art/carrier-furniture/build-batch-a-v3.py`
Requires the retained v2 script solely for coordinates/mock composition and diagnostics.

Validation:
- All 16 masks remain 128×128; every compatible E/W and N/S edge pair
  matches byte for byte, including corners and T-joins.
- Floor centre grayscale value exceeds wall face by over 140 / 255.
- Visual inspection of wall diagnostics shows continuous unframed runs,
  rounded outer ends, clean joins, and open two-tile thresholds.
- Starting-layout mock supplied at native 2×, 1× (2304×1408), and
  0.5× (1152×704), plus grayscale at 1×. Coordinates and starfield retained.
- Six-station comparison on the new floor: command console, upgrade bench,
  sample analyzer, salvage sorter, combat simulator and berth all retain
  readable silhouettes. No station needs a colour tweak for contrast.
  Their existing detailed industrial style remains more ornate than the
  new architecture; that is a later style option, not a redraw in this pass.
- No optional vent/light-panel variant added: plain runs meet the cleaner brief.
