# Batch A wall assembly revision — TAKOAI-89

Wei's 2026-10-18 direction replaces individually generated masks with shared
material assembly. All changes stay in `design/art/`. Runtime import remains
outstanding: Art's explicit commit scope is `design/art/` only, whereas the
issue asks for `assets/source` and manifest writes. These are exploratory
options, not locked art or imported runtime assets.

## Deliverables and reproduction

Run `python3 design/art/carrier-furniture/build-batch-a-v2.py` with Pillow.
It assembles all sixteen `wall-<NESW>-v2.png` masks and the two repeatable
`doorway-<ns|ew>-v3.png` thresholds. Earlier ns-v2 is preserved as history.
Five original generated textures are saved as `*-source-v2.png`, alongside
their normalized face, cap, inner/outer corner and jamb samples.

`starter-assembled-full-v2.png` is 4608 × 2816 at native 2× scale.
`starter-assembled-v2.png` is its smaller inspection copy. Both show the
five starter rooms, twelve doorway tiles, three starting desks and two
berths at approved coordinates, with dark seeded starfield outside rooms.
No hull tile is used. Ships, crew and Batch B furniture are outside this mock.

`wall-diagnostics-v2.png` shows all masks and paired thresholds (NS across
two columns, EW across two rows). `station-scale-v2.png` compares all six
unchanged Batch A stations with the existing Pilot at 112 px alpha height.
Station image height includes projected footprint depth as well as raised
structure, so alpha height alone is not physical object height.

`batch-a-import-v2.json` supplies 29 stable IDs, selected versions, source
destinations, sizes and alpha bounds for the outstanding import. It contains
no hull IDs and removes no Revision 1 art.

## Assembly and checks

Masks are N,E,S,W. Every tile is 128 × 128. S=0 has 32 px cap and 96 px face;
S=1 shows only top material. Material edges are blended before assembly;
exposed ends get trim, connected edges get no post. Corner trim overlays an
opaque common substrate, so corners and T-joins expose no gaps. Tiles have
no transparent margin because they must reach the grid boundaries.

Doorways repeat floor thresholds, with jambs attached to terminating WALL
ends rather than repeated inside the passage. Two adjacent tiles have no
centre post or door leaf.

Passed: all sixteen masks opaque; exact opposing edge bytes for horizontal
and vertical straight wall runs; clear doorway centres; six whole station
alpha bounds with at least 3 px margin including antialiasing; 29 catalogue
entries. The existing `node design/art/carrier-furniture/make-starter-mock.mjs`
layout and expansion validator also passes. Assembled mock and scale sheet
were visually inspected. No gameplay code changed.

Floors 0–2, scaffold, marker and station images are unchanged as requested.
The earlier QA's concern about cross-variant floor edge designs remains;
these checks certify wall continuity, not floor seamlessness. The console
retains its original angled front. The scale sheet makes the retained
proportions visible rather than silently rescaling accepted artwork.

## Prompts and source

Built-in image_gen, `stylized-concept`, five separate source images:

- Face: seamless full-bleed horizontal wall-face swatch, front-on; painted
  softly shaded teal steel, brass trim, fine panels, top-left light, cozy
  sci-fi cat-carrier aesthetic; no characters, humans, text, floor,
  perspective, external frame or padding; target 128 × 96.
- Cap: seamless full-bleed wall TOP swatch, teal-grey brushed metal, narrow
  brass rim, top-left light, orthographic down view; no objects, text,
  perspective, external frame or padding; target 128 × 32.
- Inner corner: compact concave L trim, teal steel, inward brass rim,
  painted, axis-aligned top-down, top-left light, whole transparent subject;
  no external shadow, text or characters; texture source, not a room.
- Outer corner: same, convex rounded L and outward brass rim.
- Jamb: one narrow vertical teal-and-brass post, 3/4 orthographic S view,
  painted metal, little teal light, whole transparent subject; no door leaf,
  second post, lintel, floor, text, humans or cats. Exterior passage edge.

Original generated alpha is preserved. Material sampling and assembly are
performed by the deterministic script explicitly requested in the issue.
