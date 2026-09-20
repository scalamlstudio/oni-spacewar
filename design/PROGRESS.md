# Progress

_Running log of what's decided and what's built. Newest entries on top._

## 2026-09-20

- Bootstrapped project from the `space-sandbox` architecture (Agent/Enemy/Object/Shape
  class hierarchy, World/Camera/Panel/Physics/Control systems). No game-specific code yet.
- Created `design/` folder to work out concept and design before writing more code.
- Concept defined: colony/base management (ONI-style) + space exploration/looting +
  2D space combat. "Oni" = Oxygen Not Included nod + the in-game species (walking fat
  cat) the player manages.
- Story structure decided: start on one semi-functional ship (wormhole accident) with
  1 main character + 2-3 companions, introducing repair/build mechanics; ship later
  returns to a colony/hub that expands into the fuller base-management loop.
- Companions use ONI-style task assignment (not direct control).
- Core structure clarified: the **ship** is the player's persistent base (all ONI-
  style building/repair lives there). The hub is a dev-managed MMO-style social/
  trade space — players join factions, do missions and resource-gathering quests
  for credits, but don't build or shape the hub itself.

## 2026-09-21

- Mission structure modeled on Warframe: discrete, self-contained runs taken on
  separately from the persistent ship/hub state.
- Stakes clarified: losing the ship in a mission only costs the time/rewards of
  that mission — it doesn't threaten the ship's persistent upgrades or crew
  progression.
- Ships and crews belong only to a single player (never shared/co-owned). Missions
  support up to 4 players joining, but every mission must be completable solo.
  Each player brings their own ship into the shared mission instance (a small
  co-present fleet, not one shared ship).
- Mission failure: most rewards are lost, but there can be exceptions (specifics
  TBD).
- Split narrative/lore into its own `design/STORY.md`, separate from mechanics in
  `design/DESIGN.md`.
- Added `design/OPEN_QUESTIONS.md` as a rollup of every unresolved question across
  the other docs.
- Removed the duplicate "Open Questions" list sections from DESIGN.md and
  STORY.md — OPEN_QUESTIONS.md is now the single canonical list; other docs just
  point to it.
- On-ship crafting confirmed (ONI-style resource/machine input-output balancing) —
  new § Crafting & Tech Tree in DESIGN.md. Trading (player-to-player and with hub
  NPCs) is the source of key resources that expand the tech tree.
- Failure-reward exceptions: mechanism decided (per-reward-type configurable
  "survives failure" flag), specific rewards left pending on purpose for future
  tuning.
- New § Crew Skill Trees in DESIGN.md: each crew member has 2 branching, node-
  based skill trees (Research, Combat Operation), Warframe Focus-School style —
  nodes have stages, unlocking one opens further nodes/branches. Relationship to
  the ship's crafting tech tree is still open.

## 2026-09-22

- Split ship progression into two confirmed-separate layers: **Ship Modules**
  (coarse unlocks that introduce a basic mechanic + ship area, e.g. unlocking
  Manufacturing lets you start building manufacturing buildings at all) vs.
  **Crew Skill Trees** (the actual progression — leveling crew unlocks which
  specific buildings/recipes are available within an already-unlocked module).
  Renamed DESIGN.md's "Crafting & Tech Tree" section to "Ship Modules" to match.
- Resolved which crew skill tree does what: Research unlocks recipes/buildings;
  Combat Operation unlocks combat skills/passives that trigger automatically when
  a condition is met (not manually activated), fitting the existing ONI-style
  autonomous behavior for companions.
