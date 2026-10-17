# Carrier furniture and stations (TAKOAI-88)

Design art for carrier interior Revision 3: Small and Large rooms built
from floor, wall and doorway tiles, with work stations and furniture placed
freely, all facing S (design/READINESS.md § Demo Spec ›
Carrier). Art batches A and B go here once drawn.

- `make-starter-mock.mjs` holds the starting layout and the autoplay's
  expansion, checks them against the rules (rooms in the hull and sharing
  walls, doorways and their clearance, floor, wall behind, wall face, front
  strips clear and reachable) and writes `mockup-starter-carrier.svg`:
  `node design/art/carrier-furniture/make-starter-mock.mjs`
- `mockup-starter-carrier.png` is a rough diagram, not art. Render it with
  headless Chrome like `design/art/demo-v2/render-svg.sh` (window
  1470 × 720).
