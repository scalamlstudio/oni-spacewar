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

## Cropping

Every sprite or frame must keep the whole subject inside its canvas with a
transparent margin. Ears, tails, antennas and other protrusions must not be
clipped, flattened or patched back on.

Before shipping a sprite or frame, check its alpha bounding box. Directional
character frames must keep a consistent head shape across facings, especially
when comparing north with south.
