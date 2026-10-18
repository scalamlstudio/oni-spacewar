TAKOAI-104 — orthographic carrier redraw option

All 25 requested pieces are redrawn whole with deterministic Python/Pillow
geometry: six stations including the berth, and 19 furniture/decor pieces.
These remain exploratory source art; they do not lock the project's art direction.

Import `orthographic-import-patch-v1.json` as an art-only patch catalogue.
It contains changed rows only, in the same array format as the orientation patch.
Stable IDs, source destinations, canvas sizes and existing furniture footprint,
headroom, anchor and layer metadata are preserved. Station rows retain their
original metadata shape, as required by the importer's patch comparison.
No runtime assets, manifest, code or design rules are changed.

Reproduce with Python 3 and Pillow:

```
python design/art/carrier-furniture/build-orthographic-v1.py
```

Projection and visual checks:

- Top planes are axis-aligned rectangles or ellipses. All parallel edges remain
  parallel, front/back widths are equal, and vertical extrusions go straight up.
  Surface depth uses a common 0.60 compression within the ground footprint;
  footprint cells themselves stay 128 by 128 px throughout the canvas.
- Berth: 512 by 544 canvas, rectangular 4 by 4 footprint at y=32, visible
  rectangle (4,36)–(508,540). Rug: 256 by 256 canvas, visible rectangle
  (4,4)–(252,252). These are filled rectangles inset only by the required margin.
- Every PNG has at least four transparent pixels around its whole subject.
  No drop shadows extend into adjacent cells. The grid does not converge.
- Teal/warm materials use upper-left soft light, bevel highlights, material
  grain, seams, rivets and glyph details. No words or humans are drawn into art.
- `orthographic-before-after-v1.png`: every piece paired with its current
  predecessor, with equal-cell ground grids overlaid after compositing.
- `orthographic-starter-grid-v1.png`: all starting furniture at the existing
  layout coordinates. Cyan lines are the 128 px grid; pink rectangles are
  reserved footprints, which may extend behind raised surfaces. The existing
  Pilot source sprite is shown at approximately 112 px alpha height for scale.
  The mock shows the occupied 22 by 16 region, excluding unused expansion space.
- `orthographic-unchanged-tiles-v1.png`: checked the three v3 floors, all sixteen
  v3 wall masks and all seventeen hull-edge variants from TAKOAI-101. Their cap
  edges and extrusions already follow the grid; none is redrawn.
- `orthographic-geometry-v1.json`: ground rectangles, anchors and the authored
  orthographic box coordinates for reproducible inspection.

Validation: all 25 PNG canvas sizes, alpha bounding boxes, margins and flat
rectangles pass the builder's assertions. The patch rows were also compared
against the combined existing catalogues using the importer's art-only metadata
invariants. Before/after, layout and unchanged-tile sheets were visually inspected.
The source importer has a fixed catalogue list; registering this new patch in
that list and importing runtime assets remains Engineer work outside this issue.
