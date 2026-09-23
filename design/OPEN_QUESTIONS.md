# Open Questions

_The single canonical list of unresolved design questions, grouped by topic. Other
docs may flag a spot as open inline (e.g. "TBD", "Open:") for context, but the
enumerated list lives only here — when a question is answered, update the relevant
doc's content and remove it from this list, rather than keeping a second copy._

## Mechanics (design/DESIGN.md)

- **Companion combat skill trigger conditions:** each skill has its own trigger
  condition (e.g. repair skill → battleship HP low, damage skill → enemy in
  range) — what's the full range of condition types, and what happens when
  multiple skills' conditions are met simultaneously (priority/cooldowns)?
  _(§ Crew Skill Trees)_
- **Extraordinary sample variety:** what different kinds of samples exist, and
  how do their Research point payouts differ? _(§ Crew Skill Trees, § Ship
  Modules)_
- **Research/training as an action:** how does "researching" a sample or
  "studying" a tech document actually play out mechanically — likely happens in
  the Lab Module (unconfirmed), but is it a timed task assignment, passive over
  time, something else? _(§ Crew Skill Trees, § Ship Modules)_
- **Basic weapon-fire action:** is there a plain attack separate from Combat
  Operation skills (LoL champions have basic attacks alongside abilities), or is
  all offense skill-based? _(§ Player)_
- **Carrier control outside missions:** how does the player actually
  control/manage the carrier itself (building placement, task assignment,
  whether/how it "moves") when not on a mission with the battleship? What does
  the main character do aboard the carrier between missions? _(§ Player,
  § Crew / Companions)_
- **Key-resource trading target:** does trading for key resources unlock ship
  *modules*, or feed the *recipes* crew leveling unlocks, or both?
  _(§ Ship Modules)_
- **Module roster:** what modules exist besides Docking, Manufacturing, and Lab,
  and what area/mechanic does each introduce? _(§ Ship Modules)_
- **Ship size extension:** mechanically, how does adding a module grow the
  carrier's buildable space — new rooms/tiles attached to the existing layout, a
  pre-designed set of expansion shapes, something else? _(§ Ship Modules)_
- **Resource categories:** raw/refined/key-trade breakdown feeding ship
  progression. _(§ Ship Modules)_
- **Machines/rooms progression:** which ones exist early vs. late game.
  _(§ Ship Modules)_
- **Failure reward exceptions:** the mechanism is decided (per-reward-type
  configurable "survives failure" flag), but which specific rewards get flagged
  that way is still open — deliberately left pending for later tuning.
  _(§ Missions)_
- **Enemies/opposition:** likely narrative source is the void monsters (see
  Story § Setting below), but enemy types/behaviors and whether there's also
  human/Oni opposition are undefined. _(§ Enemies / Opposition)_
- **Level design within missions:** procedural vs. hand-designed, mission
  objective types/variety. _(implied by § Missions, not yet written up)_

## Story (design/STORY.md)

- **Timeline & map:** how long ago the Big Rip/segmentation happened, how much
  of the universe is explored/reclaimed vs. still void-monster territory, and
  where the hub and the player's starting point sit in that map. _(§ Setting)_
- **Wormhole accident specifics:** what specifically caused it, and what
  state/location it leaves the crew/ship in (near void-monster territory?).
  _(§ Opening: The Wormhole Accident)_
- **Oni evolution & culture:** what specifically triggers the human→Oni
  evolution, what "magical abilities" means concretely (ties to design/DESIGN.md
  § Crew Skill Trees), whether Oni and un-evolved humans coexist/have distinct
  roles, and why Oni specifically are the ones out exploring space now.
  _(§ The Oni)_
- **Factions:** human groups, Oni-specific groups, trade guilds, or organized
  around fighting the void monsters? What does each faction want?
  _(§ Factions)_

## Concept (design/CONCEPT.md)

- No open questions currently — pitch and double-meaning are settled.

## Not yet started

- Art direction / visual style beyond "Oni = human-like, with variants like cat
  ears" and "ship-building interior = Mindustry-style factory/conveyor visuals"
  (see design/DESIGN.md § Ship Modules) — ship exterior, characters, UI, etc.
  still undefined.
- Audio direction.
