# Open Questions

_The single canonical list of unresolved design questions, grouped by topic. Other
docs may flag a spot as open inline (e.g. "TBD", "Open:") for context, but the
enumerated list lives only here — when a question is answered, update the relevant
doc's content and remove it from this list, rather than keeping a second copy._

## Mechanics (design/DESIGN.md)

- **Companion combat skill trigger conditions:** each skill has its own trigger
  condition (e.g. repair skill → ship HP low, damage skill → enemy in range) —
  what's the full range of condition types, and what happens when multiple
  skills' conditions are met simultaneously (priority/cooldowns)?
  _(§ Crew Skill Trees)_
- **Skill tree progress source:** how are points/progress earned for the
  Research and Combat Operation trees (missions, training, both)?
  _(§ Crew Skill Trees)_
- **Ship flight/weapon controls:** beyond skill-tree abilities, what's the base
  flight control scheme (thrust/rotate vs. waypoint, etc.) and is there a basic
  weapon-fire action separate from Combat Operation skills? _(§ Player)_
- **Key-resource trading target:** does trading for key resources unlock ship
  *modules*, or feed the *recipes* crew leveling unlocks, or both?
  _(§ Ship Modules)_
- **Module roster:** what modules exist besides Manufacturing (the one example
  given so far), and what area/mechanic does each introduce? _(§ Ship Modules)_
- **Resource categories:** raw/refined/key-trade breakdown feeding ship
  progression. _(§ Ship Modules)_
- **Machines/rooms progression:** which ones exist early vs. late game.
  _(§ Ship Modules)_
- **Failure reward exceptions:** the mechanism is decided (per-reward-type
  configurable "survives failure" flag), but which specific rewards get flagged
  that way is still open — deliberately left pending for later tuning.
  _(§ Missions)_
- **Enemies/opposition:** entirely undefined — who/what the player fights.
  _(§ Enemies / Opposition)_
- **Level design within missions:** procedural vs. hand-designed, mission
  objective types/variety. _(implied by § Missions, not yet written up)_

## Story (design/STORY.md)

- **Wormhole accident:** what caused it, and what state it leaves the crew/ship
  in narratively? _(§ Opening: The Wormhole Accident)_
- **Setting / known space:** what does known space look like, and where does the
  hub sit in it? _(§ Setting)_
- **The Oni:** origin and culture of the species — why are they the ones out
  exploring space? _(§ The Oni)_
- **Factions:** what do they represent in-fiction (political/military groups,
  trade guilds, tied to the Oni or the accident)? What do they each want?
  _(§ Factions)_

## Concept (design/CONCEPT.md)

- No open questions currently — pitch and double-meaning are settled.

## Not yet started

- Art direction / visual style beyond "Oni = walking fat cat."
- Audio direction.
- Controls/input scheme specifics (the inherited scaffold uses mouse-move +
  click-to-shoot; not yet confirmed as final).
