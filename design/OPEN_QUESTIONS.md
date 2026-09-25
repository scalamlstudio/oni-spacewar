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

The blocking questions, in the order that lets each answer constrain the next.
Confirming design/READINESS.md § First Playable Scope (TAKOAI-1) comes first,
since it decides what "partial" means below.

1. Basic weapon-fire action
2. Companion combat skill trigger conditions
3. Enemies/opposition
4. Level design within missions
5. Carrier control outside missions
6. Resource categories + Machines/rooms progression
7. Research/training as an action

Steps 1-4 (combat) and step 5 (carrier) are independent and can run in parallel.

## Mechanics (design/DESIGN.md)

- **Companion combat skill trigger conditions:** each skill has its own trigger
  condition (e.g. repair skill → battleship HP low, damage skill → enemy in
  range) — what's the full range of condition types, and what happens when
  multiple skills' conditions are met simultaneously (priority/cooldowns)?
  _(§ Crew Skill Trees)_
  `Blocks (partial)` · TAKOAI-3 — needed now: the trigger model and
  multi-trigger rule for 1-2 skills.
- **Extraordinary sample variety:** what different kinds of samples exist, and
  how do their Research point payouts differ? _(§ Crew Skill Trees, § Ship
  Modules)_
  `Deferred` · TAKOAI-4 — one sample kind (or none, if tech documents are the
  chosen Research source) is enough for the first playable.
- **Research/training as an action:** how does "researching" a sample or
  "studying" a tech document actually play out mechanically — likely happens in
  the Lab Module (unconfirmed), but is it a timed task assignment, passive over
  time, something else? _(§ Crew Skill Trees, § Ship Modules)_
  `Blocks (partial)` · TAKOAI-8 — needed now: one Research source, enough to
  close the loop from loot to skill points.
- **Basic weapon-fire action:** is there a plain attack separate from Combat
  Operation skills (LoL champions have basic attacks alongside abilities), or is
  all offense skill-based? _(§ Player)_
  `Blocks first playable` · TAKOAI-3
- **Carrier control outside missions:** how does the player actually
  control/manage the carrier itself (building placement, task assignment,
  whether/how it "moves") when not on a mission with the battleship? What does
  the main character do aboard the carrier between missions? _(§ Player,
  § Crew / Companions)_
  `Blocks first playable` · TAKOAI-2
- **Key-resource trading target:** does trading for key resources unlock ship
  *modules*, or feed the *recipes* crew leveling unlocks, or both?
  _(§ Ship Modules)_
  `Deferred` · TAKOAI-4 — trading is out of first-playable scope.
- **Module roster:** what modules exist besides Docking, Manufacturing, and Lab,
  and what area/mechanic does each introduce? _(§ Ship Modules)_
  `Deferred` · TAKOAI-4 — the first playable uses only Docking + Manufacturing.
- **Ship size extension:** mechanically, how does adding a module grow the
  carrier's buildable space — new rooms/tiles attached to the existing layout, a
  pre-designed set of expansion shapes, something else? _(§ Ship Modules)_
  `Deferred` · TAKOAI-4 — the carrier is fixed-size in the first playable.
- **Resource categories:** raw/refined/key-trade breakdown feeding ship
  progression. _(§ Ship Modules)_
  `Blocks (partial)` · TAKOAI-8 — needed now: the handful of resources in the
  one production chain.
- **Machines/rooms progression:** which ones exist early vs. late game.
  _(§ Ship Modules)_
  `Blocks (partial)` · TAKOAI-8 — needed now: the 2-3 starting buildings plus
  one unlockable.
- **Failure reward exceptions:** the mechanism is decided (per-reward-type
  configurable "survives failure" flag), but which specific rewards get flagged
  that way is still open — deliberately left pending for later tuning.
  _(§ Missions)_
  `Deferred` · TAKOAI-5
- **Enemies/opposition:** likely narrative source is the void monsters (see
  Story § Setting below), but enemy types/behaviors and whether there's also
  Oni opposition (rival Oni groups) are undefined. _(§ Enemies / Opposition)_
  `Blocks (partial)` · TAKOAI-5 — needed now: 2 void-monster types.
- **Level design within missions:** procedural vs. hand-designed, mission
  objective types/variety. _(implied by § Missions, not yet written up)_
  `Blocks (partial)` · TAKOAI-5 — needed now: one objective type, and
  hand-built vs. procedural for the first segment.

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
