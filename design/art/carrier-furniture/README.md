# Carrier furniture and stations (TAKOAI-88)

Design art for carrier interior Revision 2: floor tiles, walls and doors,
work stations and furniture placed freely (design/READINESS.md § Demo Spec ›
Carrier). Art batches A and B go here once drawn.

- `make-starter-mock.mjs` holds the starting layout, checks it against the
  placement rules (floor, wall behind, wall face, doors, front strips clear
  and reachable) and writes `mockup-starter-carrier.svg`:
  `node design/art/carrier-furniture/make-starter-mock.mjs`
- `mockup-starter-carrier.png` is a rough diagram, not art. Render it with
  headless Chrome like `design/art/demo-v2/render-svg.sh` (window
  1340 × 720).
