# Batch B furniture — TAKOAI-92

19 S-view furniture/decor designs for the TAKOAI-93 import. These are the
selected first-pass assets, with later art-direction refinements still possible.
Built-in imagegen was used, one generated object per call; no sheet slicing,
painted-room extraction, or painted repairs. No crew or humans appear.

`batch-b-import-v1.json` maps every selected versioned PNG to its stable ID,
intended `assets/source/core/`-relative destination, footprint, canvas, bottom-centre
anchor, placement and layer. Files remain under `design/art/`; Engineer C owns
runtime import. `*-source-v1.png` preserves each original generated transparent
image. Import the named `*-v1.png`, not the source or contact sheet.

The footprint is the lower depth × 128 pixels. Headroom is a multiple of 32.
Wall pieces use width × 128 by 96 canvases. Resizing preserves aspect ratio,
whole subjects and alpha; transparent padding alone is trimmed before placing
the whole artwork with a minimum four-pixel margin. Tall-object content is
capped using the reference physical height plus a half-tile depth allowance.
This keeps the 180 px locker, 200 px cat tree and 160 px lamp in the intended
scale range beside the 112 px Pilot. The narrow bunk uses a full tile depth
allowance for its two-tile footprint. No in-game code or manifest is changed.

## Prompt set

Use case: stylized-concept. Single whole standalone furniture game sprite
for Oni Spacewar's cozy cat spaceship. Painted detailed teal and warm amber/
orange, material shading and highlights, panel seams, varied dark outline.
Orthographic 3/4 top-down SOUTH view; horizontal front edges, visible tops,
vertical heights, no diamond rotation. Top-left light. Genuine transparent
background, no floor, no ground shadow, no vignette or backdrop. Entire object
inside generous clear margins. No text, labels, numbers, humans or characters.
For wall pieces: south-facing front with slightly visible upper edge, no wall.
For the rug: flat square top-down surface. Individual subjects and reference
heights are recorded in the import catalogue's `description` and
`reference_height` fields.

## Verification

`prepare-batch-b-v1.swift` sizes whole transparent sources.
`swift design/art/carrier-furniture/inspect-batch-b-v1.swift design/art/carrier-furniture`
checks alpha margins, records bounding boxes and builds a labelled preview.
The preview's filenames are presentation labels only, outside sprite pixels.
`batch-b-contact-sheet-v1.png` normalizes previews for browsing;
`batch-b-pilot-scale-v1.png` shows native 2× item pixels beside the established
Pilot with its visible height scaled to 112 px. All 19 selected PNGs passed
canvas-dimension and alpha-margin checks and were visually inspected together.
