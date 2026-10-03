# Open Questions

_The single canonical list of unresolved design questions, grouped by topic. Other
docs may flag a spot as open inline (e.g. "TBD", "Open:") for context, but the
enumerated list lives only here — when a question is answered, update the relevant
doc's content and remove it from this list, rather than keeping a second copy._

_Each entry is tagged against the first playable scope in design/READINESS.md,
and names the Multica board issue that tracks it:_

- _`Blocks first playable` — must be decided before development starts._
- _`Blocks (partial)` — a first-playable answer is needed now (noted on the
  entry); the full answer can wait._
- _`Deferred` — out of first-playable scope; can't block implementation._

## Decide next

Nothing blocks the first playable. Wei confirmed the First Playable Demo scope
on 2026-10-03 (design/READINESS.md § First Playable Scope, TAKOAI-36). The
demo answered the questions that used to block it (carrier control, enemies,
level design), and the rest left scope with manufacturing and the research loop
(resources, machines, research action). Each is re-tagged below.

## Mechanics (design/DESIGN.md)

- **Companion combat skill trigger conditions:** each skill has its own trigger
  condition (e.g. repair skill → battleship HP low, damage skill → enemy in
  range) — what's the full range of condition types, and what happens when
  multiple skills' conditions are met simultaneously (priority/cooldowns)?
  _(§ Crew Skill Trees)_
  `Deferred` · TAKOAI-3 — Wei decided this needs the detailed skill design
  pass to fine-tune, not a call made this early; the existing two examples
  (repair on low HP, damage on enemy in range) are enough for the first
  playable.
- **Extraordinary sample variety:** what different kinds of samples exist, and
  how do their Research point payouts differ? _(§ Crew Skill Trees, § Ship
  Modules)_
  `Deferred` · TAKOAI-4 — the First Playable Demo has no samples.
- **Research/training as an action:** how does "researching" a sample or
  "studying" a tech document actually play out mechanically — likely happens in
  the Lab Module (unconfirmed), but is it a timed task assignment, passive over
  time, something else? _(§ Crew Skill Trees, § Ship Modules)_
  `Deferred` · TAKOAI-8 — the First Playable Demo has no research loop; loot
  is spent at the Workshop instead.
- **Carrier building placement & crew task assignment:** the main character
  walks around the carrier between missions (decided 2026-10-03). Still open:
  how the player places buildings, how they assign companions their jobs, and
  whether/how the carrier itself "moves". _(§ Player, § Crew / Companions)_
  `Deferred` · TAKOAI-2 — the First Playable Demo has neither manufacturing
  nor companion jobs.
- **Key-resource trading target:** does trading for key resources unlock ship
  *modules*, or feed the *recipes* crew leveling unlocks, or both?
  _(§ Ship Modules)_
  `Deferred` · TAKOAI-4 — trading is out of first-playable scope.
- **Module roster:** what modules exist besides Docking, Manufacturing, and Lab,
  and what area/mechanic does each introduce? _(§ Ship Modules)_
  `Deferred` · TAKOAI-4 — the First Playable Demo uses only the Dock and the
  Workshop.
- **Ship size extension:** mechanically, how does adding a module grow the
  carrier's buildable space — new rooms/tiles attached to the existing layout, a
  pre-designed set of expansion shapes, something else? _(§ Ship Modules)_
  `Deferred` · TAKOAI-4 — the carrier is fixed-size in the first playable.
- **Resource categories:** raw/refined/key-trade breakdown feeding ship
  progression. _(§ Ship Modules)_
  `Deferred` · TAKOAI-8 — the First Playable Demo has only Credits and Void
  Crystal, and no production chain.
- **Machines/rooms progression:** which ones exist early vs. late game.
  _(§ Ship Modules)_
  `Deferred` · TAKOAI-8 — the First Playable Demo has no manufacturing.
- **Failure reward exceptions:** the mechanism is decided (per-reward-type
  configurable "survives failure" flag), but which specific rewards get flagged
  that way is still open — deliberately left pending for later tuning.
  _(§ Missions)_
  `Deferred` · TAKOAI-5 — the First Playable Demo flags none.
- **Enemies/opposition beyond the demo:** the 2 void-monster types are
  decided (Void Swarmer, Void Spitter). Still open: the wider enemy roster, and
  whether there is also Oni opposition (rival Oni groups). _(§ Enemies /
  Opposition)_
  `Deferred` · TAKOAI-5
- **Level design within missions:** the demo has one hand-built arena and one
  objective type, Elimination (§ Missions). Still open: procedural vs.
  hand-designed long term, and which other objective types exist.
  _(§ Missions)_
  `Deferred` · TAKOAI-5

## Story (design/STORY.md)

- **Timeline & map:** how long ago the Big Rip/segmentation happened, how much
  of the universe is explored/reclaimed vs. still void-monster territory, and
  where the hub and the player's starting point sit in that map. _(§ Setting)_
  `Deferred` · TAKOAI-6 — one mission segment needs no map.
- **Wormhole accident specifics:** what specifically caused it, and what
  state/location it leaves the crew/ship in (near void-monster territory?).
  _(§ Opening: The Wormhole Accident)_
  `Deferred` · TAKOAI-6 — the premise is enough for a title card.
- **Oni evolution & culture:** what specifically triggered the awakening of
  the Oni's magical abilities, what "magical abilities" means concretely (ties
  to design/DESIGN.md § Crew Skill Trees), and why the Oni are out exploring
  space now.
  _(§ The Oni)_
  `Deferred` · TAKOAI-6 — not needed until skill trees grow past a stub.
- **Factions:** Oni political/military groups, trade guilds, or organized
  around fighting the void monsters? What does each faction want?
  _(§ Factions)_
  `Deferred` · TAKOAI-6 — the hub and factions are out of first-playable scope.

## Concept (design/CONCEPT.md)

- No open questions currently — pitch and double-meaning are settled.

## Not yet started

- Art direction / visual style beyond "Oni = cat-like creatures" and
  "ship-building interior = Mindustry-style factory/conveyor visuals" (see
  design/DESIGN.md § Ship Modules) — ship exterior, characters, UI, etc. still
  undefined.
  `Deferred` · TAKOAI-7
- Audio direction.
  `Deferred` · TAKOAI-7
