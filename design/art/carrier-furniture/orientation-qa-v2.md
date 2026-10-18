# South-facing orientation correction — TAKOAI-99

Reviewed all six Batch A stations and all 19 Batch B pieces against the v3 floor. Command Console and Salvage Sorter had diagonal front edges and unequal side exposure. Batch B also had yaw in locker, sofa, crate stack, snack dispenser and aquarium. Remaining pieces were already axis aligned or rotationally symmetric; organic plants retain their natural asymmetry.

Seven whole-object transparent redraws are stored as `*-source-v2.png`; normalized import sprites are `*-v2.png`. Apply `orientation-import-patch-v2.json` over the existing A v3 and B v1 catalogues. Stable IDs, source destinations, canvas dimensions and existing footprint metadata are unchanged. Batch A continues its existing implicit bottom-centre anchor; Batch B retains explicit anchors. Content fits its previous occupied envelope, bottom aligned four pixels above the canvas edge. Head-on projections naturally change silhouette width/height. No runtime assets changed.

Built-in imagegen was used, one edit per source. Prompt template (subject substituted for each of the seven names above):

> Change ONLY yaw/orientation: redraw this exact same object facing straight SOUTH head-on in 3/4 top-down orthographic view. Front horizontal edges parallel to canvas bottom; depth edges vertically up canvas; no diagonal yaw, no perspective vanishing point or broad side panel. Preserve design, parts, teal/amber palette, painted textures, variable dark outlines and top-left illumination. Keep top surfaces visible at the same elevation angle. Whole object centered on genuine transparency, no ground, outside shadows, text or humans; single object.

`prepare-orientation-v2.py` reproduces normalization, catalogue patch and both comparison sheets with Pillow. It removes near-invisible alpha noise outside each whole generated subject, fits without stretching and checks canvas dimensions, bottom alignment and minimum four-pixel margins. Visual inspection confirms horizontal front edges on all seven redraws and all six stations together on the v3 floor. Source artwork remains exploratory; this supplies the requested orientation correction for Engineer to re-import.
