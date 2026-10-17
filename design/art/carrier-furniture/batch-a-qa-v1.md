# Batch A generation QA — not ready for import

TAKOAI-89. Generated with the built-in image_gen tool. This is an exploratory
set, not finished or locked art. No assets/source files or manifest entries
were changed. The issue's shipping requirements are not satisfied.

## Saved outputs

All six station concepts, three floor attempts, sixteen wall-mask attempts,
two doorway attempts, two hull attempts, scaffold and front marker are saved
here as numbered PNGs. `*-source-v1.png` preserves each original generation.
`doorway-ns-v2.png` is a second geometry attempt; its original is
`doorway-ns-revision-source-v1.png`.

`batch-a-contact-sheet-v1.png` shows normalized previews in this order:
command console, upgrade bench, sample analyzer, salvage sorter, combat
simulator, berth, floors 0–2, hulls 0–1, doorway ns, doorway ew, scaffold,
front marker, then walls 0000–1111 in n-e-s-w bit order. It is a review
montage of separately generated assets, not a sheet to slice into sprites.

`starter-assembled-v1.png` is a diagnostic composition at 48 px per tile,
using the approved 36 × 22 hull, five starting rooms and six paired doorways.
The three initial desks and two berth pads occupy the approved coordinates.
Batch B furniture, crew and ships are intentionally absent from this Batch A
diagnostic; the sorter and simulator are shown in the contact sheet rather
than added to the starting layout.

## Checks and failures

- The approved layout validator (`make-starter-mock.mjs`) passes.
- Normalized tiles are 128 × 128. Stations use footprint-width canvases with
  headroom in multiples of 32. Whole transparent subjects are resized without
  cutting any painted background out or slicing a sheet.
- Wall generations have transparent edge padding. The assembled preview
  confirms disconnected wall runs. The 32 px cap / 96 px exposed face
  geometry and neighbour-mask connectivity are not reliable. Several
  south-connected masks still paint an exposed face.
- Doorway v1 generations depict individual frames; repeating them divides
  the two-tile passage with a central post. The ns revision is flatter, but
  does not resolve the full doorway family or exact matching wall joins.
- Hull 0 depicts an isolated raised object instead of bare-hull plating.
- Floor variants differ in edge treatment and value; cross-variant seamless
  compatibility is not established.
- The console's diagonal front edge and relative station heights need a
  consistent S-view and Pilot-scale pass before shipping.

The image generator returned pictures but failed these required geometry
constraints, including after the targeted doorway retry. Importing this set
would conceal those failures behind valid filenames and stable IDs. Keep it
out of the content pipeline until corrected whole-piece generations pass
the same assembled-layout check. Revision 1 runtime art remains in place.

## Reproduce previews

Prompts and requested dimensions: `batch-a-prompts-v1.json`.
`prepare-batch-a-v1.swift` normalizes a whole generated image to its requested
canvas; `assemble-batch-a-v1.swift` composes the diagnostic and contact sheet.
Both use macOS AppKit. They make no runtime or gameplay changes.

    swift prepare-batch-a-v1.swift SOURCE DEST WIDTH HEIGHT object
    swift assemble-batch-a-v1.swift design/art/carrier-furniture
