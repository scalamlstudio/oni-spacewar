TAKOAI-108 — exploratory painted berth floor option

Nine opaque 128×128 tiles match the existing floor cell size. Select fill,
edge-n/e/s/w, or corner-nw/ne/se/sw from the berth boundary, never the hull
boundary. North is screen-up. PNG names are berth-floor-<piece>-v1.png.
The catalogue berth-floor-import-v1.json lists every exact path and dimension;
it is an art handoff, not a patch for the existing furniture importer.

For each existing 4×4 berth at (2,8) and (8,8), place the nine-piece tiles
on the floor layer and composite berth-floor-marking-2x2-v1.png at local
cell offset (1,1). The overlay is 256×256 with genuine transparent alpha.
It is one whole overlay, not a sprite sheet to slice. The ring remains
circular and all cells remain square; no headroom, trapezoid or side face.
For other rectangular sizes ≥3×3, centre the same overlay geometrically.

The material is painted teal, with bevel seams, warm trim lights and gold
corner markings carried forward from berth-v1.png. Only berth art is added;
all other images in the layout are existing painted art. The mock is an art
assembly of the occupied 22×16 starting region, not a runtime screenshot.
berth-floor-comparison-v1.png pairs the original whole sprite with the tile
assembly; berth-floor-starter-grid-v1.png shows both berths on the cell grid.

The built-in ImageGen tool generated the material (not a cutout sheet).
Prompt: one seamless square orthographic top-down teal metal floor texture;
edge-to-edge, no perspective or raised sides; hand-painted worn teal metal,
subtle warm upper-left sheen, tiny scratches and restrained mottling;
medium muted teal outer edges, no border, ring, symbols, text or humans.
Native editable SVG adds deterministic connected trim, plate bevels and gold
markings over this material. Reproduce all PNGs and mocks with:
python3 design/art/carrier-furniture/build-berth-floor-v1.py
Requires macOS Swift, sips and Google Chrome. Swift crops whole browser
renders to their authored viewport, never strips from a sprite sheet.

These are exploration assets for Engineer integration, not locked art direction.
