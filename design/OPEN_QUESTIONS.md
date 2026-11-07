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

Next up: § Battle direction (TAKOAI-109), B1 first, plus picking the first
batch from design/BATTLE.md § Prioritised change list.

## Battle direction (design/BATTLE.md)

_Raised by the Diablo / Path of Exile gap analysis (TAKOAI-109). Each entry
lists options and Mika's recommendation; the call is Wei's. None of these
blocks the proposed first batch (BATTLE.md § Prioritised change list, #1–#4)
except B6, which decides #4._

- **B1. Bullet-hell vs. ARPG balance:** where does the danger come from?
  (a) Bullet-hell first: fewer, tougher enemies, danger from shot patterns.
  (b) ARPG horde: big crowds, danger from being swarmed. (c) Hybrid: chaff
  dies in one hit and is there to be mowed down; the danger comes from
  readable shots and telegraphs on ranged enemies, elites and bosses.
  _Recommended: (c)._ _(BATTLE.md § Gap analysis)_
  `Deferred` · TAKOAI-109
- **B2. Battleship mobility:** the ship is slow on purpose (DESIGN.md
  § Player). (a) Keep it slow; only the Kite has a dash. (b) Raise base
  speed ~30%. (c) Keep it slow, but every ship gets one mobility or
  defensive answer and enemy shots are tuned to the slowest ship.
  _Recommended: (c)._ _(DESIGN.md § Player)_
  `Deferred` · TAKOAI-109
- **B3. How deep battleship building goes:** (a) Module sockets that change
  skills and the basic attack (Diablo 3 runes / PoE support gems).
  (b) (a) plus hull, weapon and thruster parts with random affixes (PoE
  gear). (c) (b) plus E/R skill picks and the Pilot's Combat Operation tree
  as the passive tree. _Recommended: (a) first, built so (b) and (c) can be
  added; random-affix parts only once module drops prove fun._
  _(BATTLE.md § Ship modules)_
  `Deferred` · TAKOAI-109
- **B4. Where modules come from:** (a) random mission drops only;
  (b) crafted on the carrier from blueprints and Void Crystal; (c) random
  drops, and unwanted ones salvaged into materials that craft or upgrade
  modules (so no drop is useless; later the Manufacturing Module becomes
  the crafting bench). _Recommended: (c)._ _(BATTLE.md § Prioritised change
  list #9)_
  `Deferred` · TAKOAI-109
- **B5. Mission rhythm:** (a) keep Elimination as kill N; (b) kill N, then a
  Fissure Guardian comes out and the mission ends when it dies;
  (c) a new "close the fissures" objective, each fissure guarded.
  _Recommended: (b) for Elimination now, (c) as the next mission type._
  _(DESIGN.md § Elimination, BATTLE.md § Enemy roles)_
  `Deferred` · TAKOAI-109
- **B6. Sound in the demo:** the demo scope says no audio, and audio
  direction is not started (§ Not yet started). (a) Stay silent until audio
  direction is done. (b) A placeholder pack (licensed or CC0) now, replaced
  later. (c) Commission bespoke sound now. _Recommended: (b): hits and kills
  without sound lose much of their feel._ _(READINESS.md § First Playable
  Scope)_
  `Deferred` · TAKOAI-109 / TAKOAI-7
- **B7. Difficulty:** (a) one fixed tuning; (b) the player picks a Threat
  level at the Berth: more elites and affixes, better loot; (c) the game
  scales to the ship's power by itself. _Recommended: (b)._
  _(BATTLE.md § Prioritised change list #10)_
  `Deferred` · TAKOAI-109

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
- **Crew task assignment:** the main character walks around the carrier
  between missions (decided 2026-10-03), and the player builds rooms and
  places stations and furniture freely (decided 2026-10-17, TAKOAI-88,
  DESIGN.md § Carrier). Manufacturing stations will use the same placement.
  Still open: how the player assigns companions their jobs, whether crew
  carry out construction (the demo's sites finish on a timer), and whether/how the
  carrier itself "moves". _(§ Player, § Crew / Companions, § Carrier)_
  `Deferred` · TAKOAI-2 — the First Playable Demo has neither manufacturing
  nor companion jobs.
- **Room bonuses and comfort:** should an enclosed area count as a room type
  from the furniture in it (ONI room bonuses), and should furniture feed a
  comfort or morale stat? The room-and-item model allows both without a
  save change. _(§ Carrier)_
  `Deferred` · TAKOAI-4 — furniture is cosmetic in the First Playable Demo.
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
  `Deferred` · TAKOAI-4 — the first playable builds 6 × 4 and 13 × 9 rooms,
  stations and furniture inside a fixed 36 × 22-tile hull on one deck
  (DESIGN.md § Carrier). Growing the hull, and extra decks for base
  expansion (Wei, 2026-10-17: a good idea for later), are still open.
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
  whether there is also Oni opposition (rival Oni groups). TAKOAI-109
  proposes four roles, elite affixes and a Fissure Guardian for this
  (design/BATTLE.md § Enemy roles). _(§ Enemies / Opposition)_
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
