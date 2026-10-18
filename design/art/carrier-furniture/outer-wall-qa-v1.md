# Carrier hull edge option v1 — TAKOAI-101

Scripted continuation of v3, offered as an exploratory hull option. The issue's
explicit clean scripted style overrides the general painted-art guideline here.
No gameplay or layout changes; no assets/source writes.

## Deliverables

- 17 whole 128 × 128 RGBA tiles, 64 world px at 2×.
- `outer-wall-import-v1.json`: stable IDs, exact masks, source destinations,
  dimensions, placement rules and all 72 starting hull-cell selections.
- `build-outer-wall-v1.py`: deterministic Python 3 + Pillow generator/checks.
- `starter-hull-full-v1.png`: native 2× geometry mock.
- `starter-hull-1x-v1.png`: 1× mock, 1536 × 1152.
- `starter-hull-half-v1.png`: 0.5× mock, 768 × 576.
- `hull-edge-before-after-v1.png`: same north edge/T junction; before above,
  after below. The upper white exterior strip becomes continuous charcoal.
- `hull-wall-contact-sheet-v1.png`: tiles in catalogue order, six per row.
  This is a diagnostic only; import the separate PNGs, never slice this sheet.

## Geometry and mechanical selection

File: `hull-wall-<connections>-x<exterior>-v1.png`.
ID: `core.carrier.tile.hull_wall_<connections>_x<exterior>`.

Connections retain v3's four NESW bits: 1 for an adjacent wall cell. Exterior
uses eight bits N E S W NE SE SW NW: 1 for space outside the union of room
floor, wall and doorway cells. Clear a diagonal bit when either adjoining
cardinal bit is set, since the cardinal strip already supplies that corner.
The two masks are independent: connection mask alone cannot distinguish top
from bottom walls or a hull step from an internal junction.

Examples:

| Case | Connections | Exterior |
| --- | --- | --- |
| North straight | 0101 | 10000000 |
| North shell + inner wall from south | 0111 | 10000000 |
| East straight | 1010 | 01000000 |
| South straight | 0101 | 00100000 |
| West straight | 1010 | 00010000 |
| Convex NE corner | 0011 | 11000000 |
| Concave NE step, four wall neighbors | 1111 | 00001000 |

Select the exact catalogue key. With no exterior bits retain the current v3
wall. The catalogue contains every starter case plus all four orientations of
straight runs, inward T joins, convex and concave corners. The generator's
`render()` can emit additional combinations for future layouts; those must be
catalogued explicitly before import. Never approximate with another key.

The shell occupies the outer 64 px, with a darker south-facing surface kept
inside that footprint and a subtle top-left highlight. Cyan follows the inner
edge and connects to the existing 48 px inner-wall centerline. Corners are unions
of the same shell geometry, not overlays with repeated end caps. Concave steps
use the missing diagonal to fill the outer quarter of the tile.

Every exposed edge is opaque charcoal. Interior floor is included. Replace the
centered wall tile on the same cell; do not add a second centered wall over it.
Outside cells continue to show space. No bare hull-plating tiles are introduced.
Do not rotate these images, because their top-left lighting is baked in.

## Starting layout and checks

The five room rectangles and six two-cell doorways match
`crates/client/src/layout.rs` (master at f750a59). The mock includes the five
large station objects for scale; small furniture is omitted to expose joins.
It is an art assembly, not a runtime screenshot. The berth has no external
opening: all doorways connect occupied rooms. Default: no threshold tile.

Run `python3 design/art/carrier-furniture/build-outer-wall-v1.py` with Pillow
installed. Checks verify:

- Every exposed cardinal pixel is opaque dark shell, never floor or cyan.
- Every adjacent starter hull seam matches byte-for-byte.
- Every connected hull-to-inner-wall seam matches unchanged v3 byte-for-byte.
- All starter doorways are internal; no missing outer opening case.

Visually inspected the half-scale assembly, edge comparison and contact sheet:
continuous outer shell, clean stepped corner, uninterrupted cyan at T joins,
and no floor beyond the shell. All file writes stay in design/art/.

Engineer handoff: import the separate tiles using the catalogue, derive both
masks from occupied-cell union, and replace only hull cells. Runtime import and
render integration are intentionally left to Engineer as the issue requests.
