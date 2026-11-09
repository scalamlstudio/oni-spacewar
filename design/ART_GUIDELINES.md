# Art Guidelines

These rules are binding for every future Art and Engineer issue that creates,
imports or displays Oni Spacewar art.

## Characters

Characters must come from the concept crew in
`design/art/concept-art/01-oni-crew-v3.png` and match that sheet:

- Pilot: orange cat with goggles and a teal suit.
- Engineer: black cat with a hard hat.
- Researcher: cream cat with glasses.
- Gunner: grey tabby with a red band.

Do not invent new cat faces, use generic stand-ins, or substitute silhouettes
that read as different characters.

**Scale (carrier).** A carrier tile is 64 world px (128 px at 2×). The Pilot
and every crew member are about **56 px tall in game, 112 px at 2×**
(TAKOAI-94; was 70 / 140), and their footprint is no wider than 0.6 tile, so a
character fits on one tile. Draw stations and furniture against that height
(reference heights: design/READINESS.md § Carrier › Art spec).

**No hull plating.** The carrier has no bare-hull tiles: anywhere that isn't a
room's floor, wall or doorway shows the shared space sky, and the rooms' outer
walls are the ship's edge.

## Rendering Style

Finished demo assets use painted detail at the level of
`assets/source/core/battle/ship/kite.png`,
`assets/source/core/battle/ship/bulwark.png` and
`design/art/ships/03-mission-combat-v3.png`: soft shading and highlights,
panel lines and material detail, glows, and a dark outline whose weight varies
with the form.

Do not use flat single-colour fills with uniform thick outlines. This replaces
the earlier TAKOAI-51 direction of bold outlines, flat vibrant colour and no
gradients.

## Text

Do not paint text into image assets. No titles, logos, labels, numbers or UI
words belong in the pixels. Leave calm space where needed and let the game lay
out text in the UI. Icons and non-word glyphs are allowed.

## Scene Backdrops

Title, Result and later full-screen scene art must not paint the space sky.
The shared runtime nebula shader owns the sky, so scene art should contain only
subjects, interiors, frames, foreground debris and local glows on transparent
pixels where space shows through.

## Cropping

Every sprite or frame must keep the whole subject inside its canvas with a
transparent margin. Ears, tails, antennas and other protrusions must not be
clipped, flattened or patched back on.

Before shipping a sprite or frame, check its alpha bounding box. Directional
character frames must keep a consistent head shape across facings, especially
when comparing north with south.

## Animation frames

Every frame of a character, idle and animation alike, is one whole drawing
(TAKOAI-79 ears, TAKOAI-130 legs). When a pose looks wrong, fix the source
art by redrawing or regenerating the whole frame. Never fix it in the import
step.

- **No compositing.** Do not cut a frame into regions and paste new parts
  onto it: no new legs or feet under a kept body, no heads or ears from
  another frame or direction, no polygon or ellipse body parts, no erasing
  and repainting. An import script may only cut frames from a sheet, remove
  the background, scale them and place them on the canvas.
- **One scale per sheet.** Draw an animation's frames together, with the idle
  pose, on one sheet, and scale the whole sheet by one factor. Do not fit
  each frame to a target height, since that changes the body ratio whenever
  a pose's outline changes.
- **Keep the proportions.** New poses keep the character's established head,
  body and leg ratio. Walk cycles alternate: each leg takes its turn in front
  in every facing, diagonals included.
- **Arms counter the legs.** Only one hand is forward at a time, opposite
  to the leg on its side, and hands stay at or below belly height. On a
  round body seen from the side, the far arm is hidden behind the body and
  is not drawn.
- **Keep the source.** Commit the generated or painted sheet and its exact
  prompt next to it (`assets/authoring/`), together with the importer that
  turns it into runtime frames, so it can be redrawn and re-imported.
