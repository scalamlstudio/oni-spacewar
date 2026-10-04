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
- Trigger conditions are per-skill, e.g. repair skill → ship HP low, damage skill
  → enemy in range. Full condition range and multi-trigger priority still open.
- Main character confirmed as the ship's pilot — player directly controls ship
  flight. Main character's Combat Operation skills are manually triggered while
  flying (unlike companions' auto-trigger); their Research skills lean toward
  passive production/research boosts.
- Confirmed passive-boost Research nodes aren't main-character-specific — any
  crew member's Research tree can include boosting-type skills.
- Skill point sources detailed: Combat points from defeating enemies. Research
  points from (1) researching "extraordinary samples" — a chance byproduct of
  manufacturing, different sample kinds give different point payouts — and
  (2) studying "advanced tech documents," bought at the hub or found as mission
  loot. Exact mechanics of the research/study action itself still open.
- Ship control scheme settled: click-to-move, League of Legends champion-style —
  player always controls their own ship (no unit selection), moves slowly, uses
  abilities on hotkeys. StarCraft II Mothership was the movement-feel reference
  (slow, ability-driven). Companions still auto-trigger skills on condition.
- Revised Oni visual design, superseding the earlier "walking fat cat"
  description in CONCEPT.md/STORY.md (since redesigned — see 2026-09-25).
- Wrote the universe's core cosmology into STORY.md § Setting: the Oni
  civilization fractured the universe into "space segments" to survive an
  oncoming Big Rip, later invented wormhole-jump engines to travel between
  segments, and found many segments already held by hostile "void monsters."
  Oni origin resolved: the Oni gained magical abilities through the war against
  the void monsters. Void monsters are the likely narrative source for
  DESIGN.md § Enemies / Opposition.
- Ship building/art direction referenced to Mindustry (2D factory building —
  machines, conveyors, resource routing), bounded by the ship's current interior
  space rather than Mindustry's unbounded map. Ship size grows over time by
  adding more modules (see § Ship Modules).
- Major structural clarification: the persistent base is a **carrier**, not the
  ship the player fights with. Missions are played with a separate **battleship**
  (built/stored via a new **Docking Module**), selected along with companions and
  teleported to a space segment. Also confirmed **Manufacturing Module** and
  **Lab Module** (research) as known modules. Reworked DESIGN.md throughout to
  distinguish carrier (persistent, stays behind) from battleship (mission
  vessel, at risk each run) — this also sharpens why mission stakes are low: the
  carrier itself is never directly at risk.

## 2026-09-25

- Added `design/READINESS.md`: a proposed first-playable scope (one carrier with
  Docking + Manufacturing, one solo hand-built mission, stub skill trees, no hub
  or multiplayer), a design-done checklist, a triage of every
  `OPEN_QUESTIONS.md` entry as blocking or deferrable, and a recommended decision
  order. The scope is a proposal pending Wei's confirmation — nothing in it is
  decided yet.
- Task tracking moved to the Multica board (TAKOAI-2 … TAKOAI-7, one issue per
  decision area). `README.md`'s Todo list was removed in favor of pointers to
  `design/`; `OPEN_QUESTIONS.md` stays the single canonical question list.
- Consolidated open questions into `design/OPEN_QUESTIONS.md` only: each entry
  now carries its first-playable tag (`Blocks first playable` /
  `Blocks (partial)` / `Deferred`) and board issue, and a "Decide next" list at
  the top gives the recommended order by name. `design/READINESS.md` dropped its
  Question Triage and Recommended Decision Order sections (which restated the
  questions) and keeps only § First Playable Scope and § Design-Done Checklist.
  First-playable resources, machines/rooms and the research/training action now
  track under TAKOAI-8. No question's substance changed.
- Oni redesigned as a cat-like species (decided with collaborators). They are
  now their own species rather than descended from another one — every such
  reference is removed from the design and cosmology — and the Oni are the
  civilization that segmented the universe. Updated CONCEPT.md, STORY.md,
  DESIGN.md and OPEN_QUESTIONS.md to match, and edited the 2026-09-22 entries
  above to match too.
- Added exploratory concept art in `design/concept-art/` (TAKOAI-12): four
  flat-vector SVG sheets with PNG renders — the Oni crew lineup, the carrier
  (exterior + Mindustry-style interior), a battleship-vs-void-monsters mission
  moment with a Q/W/E/R bar, and a mood/palette board. These are for reaction
  only, not a decided art direction; art direction stays open under TAKOAI-7.

## 2026-09-26

- Revised the concept art in `design/concept-art/` after Wei's feedback on
  TAKOAI-12 (still exploratory, not a decided art direction; palette kept):
  rounder, chubbier Oni; the carrier redrawn as a spine of repeatable module
  sockets (several Docks each holding a different battleship, several
  Manufacturing modules each on a different production line); and the mission
  sheet redrawn as a bullet-hell fight with a much smaller battleship. The
  design docs are not yet updated for the multi-module carrier or bullet-hell
  combat — those await Wei's confirmation as design decisions.
- Moved the game code into `src/` (main.lua, requirement.lua, class/, general/,
  enemy/, projectile/, shader/, world/), separating it from `design/` at the
  repo root. No Lua changed; `love.sh`, `README.md` and `ARCHITECTURE.md`
  updated to point at `src/`.
- Basic attack decided (TAKOAI-3): the battleship's main weapon auto-fires at
  the nearest enemy within weapon range at a fixed rate, rather than a
  targeted LoL-style auto-attack or no basic attack at all. Click-to-move
  stays the movement input; Q/W/E/R Combat Operation skills are unaffected.
  Updated DESIGN.md § Player and READINESS.md § First Playable Scope;
  removed from OPEN_QUESTIONS.md.
- Companion combat skill trigger conditions (TAKOAI-3) deliberately left
  undecided: Wei doesn't want the full condition range or multi-trigger
  priority rule fixed this early, since it'll need fine-tuning once detailed
  skill design starts. The existing two examples in DESIGN.md § Crew Skill
  Trees (repair on low HP, damage on enemy in range) stand as-is for the
  first playable. Re-tagged `Deferred` in OPEN_QUESTIONS.md instead of
  resolving it.

## 2026-09-28

- Removed the superseded `design/concept-art/` folder (v1/v2 sheets plus
  `generate.py` and `render.sh`) after Wei approved the v3 renders
  (TAKOAI-16). `design/art/` replaces it; the old files remain in git history.
  README now links to `design/art/` (TAKOAI-17).

## 2026-10-03

- **First Playable Demo scope confirmed** by Wei (TAKOAI-36), replacing the
  2026-09-25 proposal. The demo is one repeatable loop: Title → Carrier → Dock
  (pick battleship + briefing) → Battle (Elimination) → Result → Carrier →
  spend rewards at the Workshop → go again. It is single-player and offline,
  with battle logic kept in the deterministic `sim` crate so multiplayer stays
  possible. Rewrote design/READINESS.md § First Playable Scope.
- Moved out of first-playable scope (deferred, not rejected): the Manufacturing
  Module and production chain, companion task assignment, and the research
  loop (Research points, samples, tech documents, skill-tree stubs).
- Carrier control decided (TAKOAI-2): the main character (the Pilot) walks
  around the carrier interior in a side-view, ONI-style cross-section, talks
  to crew and uses modules (A / D to walk, E to interact). Building placement
  and task assignment are deferred with manufacturing. Updated DESIGN.md
  § Player and § Crew / Companions.
- Enemies and levels decided for the demo (TAKOAI-5, in part): 2 void-monster
  types (Void Swarmer, Void Spitter) in DESIGN.md § Enemies / Opposition. New
  mission type **Elimination** (kill N enemies across waves, one hand-built
  arena) in DESIGN.md § Missions. No reward survives failure in the demo.
- New demo-only module, the **Workshop**: Credits and Void Crystal from
  mission loot buy 3 upgrades (Hull Plating, Weapon Tuning, Thruster Tuning;
  2 levels each) that apply to every battleship. Two battleships, **Kite**
  and **Bulwark**, each with 2 skills.
- Added design/READINESS.md § Demo Spec: carrier layout, ship and enemy stats,
  wave plan, kill target (20), loot rates, upgrade costs, crew dialogue lines,
  tutorial hints and save contents. Mika wrote these numbers as defaults for
  Wei to tune (TAKOAI-37).
- design/OPEN_QUESTIONS.md: no question blocks the first playable any more.
  Carrier control, enemies and level design are answered for the demo; the
  rest are re-tagged `Deferred`.

## 2026-10-04

- **2.5D expandable carrier** (TAKOAI-52, First Playable Demo Revision 1):
  the one-deck side-view corridor is replaced by a 3/4 top-down grid of rooms
  and corridors that the player expands at the Workshop, dungeon-builder
  style. Wei approved the 3/4 view and building at the Workshop; Mika chose
  the rest as defaults. New DESIGN.md § Carrier; § Player updated (WASD,
  8 directions).
- Layout model: 12 × 8-cell hull, 128 px cells; rooms with fixed footprints
  and door sockets (no rotation); corridors shape themselves from their
  neighbours; everything must connect to the Bridge. Player-built pieces can
  be demolished for a full refund if nothing gets cut off.
- Starting carrier: Bridge, Crew Quarters, Workshop and Dock joined by 7
  corridor cells. Buildable: **Salvage Bay** (+25% mission haul on success,
  120 cr + 3 VC), **Training Room** (−15% Q/W cooldowns, 200 cr + 6 VC),
  corridors (10 cr).
- Dock: one battleship per berth, drawn with its combat sprite; two berths in
  the demo. More berths come later from a Hangar Bay when there are more ships.
- Save version 3 adds the carrier layout; v1/v2 saves get the starting
  layout. design/READINESS.md § Demo Spec › Carrier holds the numbers, the
  Stage 7 art list and the Stage 8 engineer scope.
