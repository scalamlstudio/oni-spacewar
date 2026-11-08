# Design

_How the game actually plays, once the concept in CONCEPT.md is settled._

## Core Structure

The player's persistent base **is the carrier** — a modularized carrier ship, not
a colony on the ground. All the ONI-style build/fix/life-support depth applies to
the carrier itself. The hub is a separate, dev-managed MMO-style social space (see
§ Hub), not something the player builds.

The carrier is **not** what the player pilots into a mission. Missions are played
out with a separate, smaller **battleship** — built/stored via the carrier's
Docking Module — that the player selects and teleports into a mission's space
segment (see § Missions, § Ship Modules).

## Core Loop

1. **Opening:** the player starts with one semi-functional carrier (see
   design/STORY.md § Opening: The Wormhole Accident for the narrative reason).
   Many systems need to be built/fixed — this introduces the carrier's build/
   repair/life-support mechanics.
2. **Crew:** 1 main controllable character + 2-3 companions aboard the carrier.
   Companions can be trained and work alongside the main character (ONI-style task
   assignment — see § Crew / Companions).
3. **Ongoing loop:** the player builds up the carrier (modules, buildings,
   research) between missions. To play a mission, they select a battleship and
   their companions and teleport to a space segment to explore, loot, and fight
   (2D combat) — see § Missions. The carrier periodically docks at the hub to
   trade, take on faction missions, and spend earned credits. The carrier (not
   the hub) is where progression/building happens.

## Player

- Movement: **click-to-move**, League of Legends champion-style — during a
  mission, the player always controls their own **battleship** directly, with no
  unit selection step (there's only ever the one battleship to control). The
  battleship moves **slowly** and leans on abilities rather than fast
  dogfighting, the same feel as controlling the Mothership unit in StarCraft II.
  Every battleship has one mobility or defensive skill, and enemy shots are
  tuned so the slowest ship can dodge them (design/BATTLE.md § Decisions, B2).
- Combat / interaction: the main character's Combat Operation skills are
  manually triggered by the player (hotkeys, LoL-ability-style) while the
  battleship moves (see § Crew Skill Trees) — unlike companions, whose combat
  skills auto-trigger automatically when their condition is matched.
- Basic attack: the battleship's main weapon auto-fires at the nearest enemy
  within weapon range at a fixed rate; the player's inputs are movement
  (click-to-move) and Combat Operation skills (Q/W/E/R, cooldown-based). A
  player-set focus target may be added later.
- Progression: earn credits via faction missions and resource-gathering, spend on
  growing/upgrading the carrier (see § Hub, § Economy).
- **Aboard the carrier (outside missions):** the player walks the main
  character around the carrier interior — a 3/4 top-down (2.5D) view of
  rooms furnished with work stations and furniture — talks to crew and
  walks up to a station to use it (click, press-and-hold or WASD to walk, E
  or a click to use). The existing battleship Berth opens component customization; a separate
  mission-selection station selects and launches missions (Wei, playtest #2,
  TAKOAI-120/130; see § Missions). The player shapes the
  carrier by building rooms and placing stations and furniture freely in
  Build mode (see § Carrier). The ability scheme above is only for piloting the
  battleship in a mission. Manufacturing stations will go through the same
  placement; how the player assigns crew tasks is left for when companion
  jobs come back into scope (see design/OPEN_QUESTIONS.md).

## Crew / Companions

- Main character: **the battleship's pilot during missions** — player-controlled
  directly, flying the battleship and manually triggering their own Combat
  Operation skills (see § Crew Skill Trees). Outside missions the main
  character is the player's avatar aboard the carrier: they walk its interior,
  talk to companions and use modules directly (see § Player). In the concept
  art the main character is the **Pilot**; the companions are the
  **Engineer**, **Researcher** and **Gunner**.
- 2-3 companions: ONI-style task assignment — the player assigns jobs/priorities
  (fix this, mine that, defend here) and companions execute autonomously, rather
  than being directly piloted. They can be trained (skills/roles improve over time,
  ONI-dupe-style), and their combat skills auto-trigger on condition rather than
  being manually activated (see § Crew Skill Trees). In the First Playable Demo
  companions only stand at their stations and talk (short lines with expression
  portraits); task assignment and coming along on missions are deferred.

## Crew Skill Trees

- Each crew member (main character and each companion) has their own **2 skill
  trees**: one for **Research**, one for **Combat Operation**.
- Trees are branching, node-based — modeled on Warframe's Focus School trees.
  Each node can have several stages, and unlocking a node opens up further nodes/
  branches beyond it.
- This is the **actual progression** system of the game — distinct from Ship
  Modules (see § Ship Modules), which just introduce base mechanics.
- **Research tree:** unlocks recipes and buildings within whatever modules are
  already active (see § Ship Modules for the example). Nodes can also be
  **passive production/research boosts** (buffs to output/speed) rather than
  direct unlocks — this applies to any crew member, not just the main character.
- **Combat Operation tree:** unlocks combat skills/passive skills, but the
  trigger mechanism differs by crew role:
  - **Companions:** auto-triggered — a companion uses a skill automatically when
    its condition is met, specific to each skill, e.g. a **repair skill**
    triggers when the battleship's HP is low, a **damage skill** triggers when
    an enemy is in range.
  - **Main character:** manually triggered by the player while flying the
    battleship (see § Player) — not condition-based.
- **Points:** Combat points are earned by defeating enemies. Research points come
  from two crew activities:
  - **Researching extraordinary samples:** manufacturing (see § Ship Modules) has
    a chance to produce a bonus "extraordinary sample" alongside its normal
    output. A crew member can research that sample to gain Research points.
    There are many different kinds of samples, and different samples yield
    different point results.
  - **Training (studying):** crew gain knowledge by studying "advanced tech
    documents," obtained either by purchasing them at the hub or finding them
    as mission loot (see § Hub, § Missions).
- Open: the range/tiers of extraordinary samples and how their point values
  differ, and how "researching" a sample or "studying" a document plays out as
  an action (time cost, task assignment, etc.) — see design/OPEN_QUESTIONS.md.
  The full range of condition types for companion skills beyond the two
  examples above, and what happens when multiple companion skills' conditions
  are met at once (priority/cooldowns), is deliberately left open too: Wei
  decided this needs the detailed skill design pass to fine-tune rather than a
  call made this early.

## Hub

- An MMO-style social/trade space where players' carriers dock between
  expeditions.
- Purpose: trading (both player-to-player and with hub NPCs), and a base for
  **factions**.
- **Factions:** players can join a faction and earn credits/reputation by
  completing missions for it.
- Also offers **resource-gathering quests** the player can do at/from the hub.
- Trading here is a key source of the resources needed to expand the carrier (see
  § Ship Modules) — not just a credit sink.
- **Advanced tech documents** are purchasable here — studying them is one of the
  two ways crew earn Research points (see § Crew Skill Trees).
- Important distinction from a typical base-builder: the player has little to no
  impact on the hub's own growth or appearance — it's managed by the developers,
  not built up by players. All the "building" fantasy lives on the carrier.
- Multiplayer: the hub is the shared social space where players see/interact with
  each other (trade, faction presence).

## Missions

- Mission structure is modeled on **Warframe**: discrete, self-contained runs with
  their own objective(s), taken on (from the hub / via factions) and then
  played out separately from the persistent carrier/hub state.
- **Entry:** before a mission, the player selects **1 battleship** (built/stored
  via the carrier's Docking Module) and their companions, then teleports to the
  target space segment to complete the mission. The carrier itself stays behind
  — it's the battleship (plus crew) that enters the mission.
- **Stakes:** losing the battleship during a mission costs the player only the
  time and rewards tied to *that mission* — it does not threaten the carrier's
  persistent build/upgrades or the crew's overall progression. Low permanent
  risk, same spirit as dying/failing a Warframe mission (you lose that mission's
  run, not your Warframe or account progress). This is reinforced by the
  carrier/battleship split: even a lost battleship never puts the home base
  itself at risk.
- **Failure rewards:** on mission failure, most rewards are lost. Which rewards
  are exceptions (still pay out on failure) is undecided — but each reward type
  should carry its own configurable "survives failure" flag/rule, rather than one
  global rule, so exceptions can be tuned/expanded per reward later without a
  system rework. Which rewards actually get flagged that way is still open; in
  the First Playable Demo none are (failure loses all mission loot).
- **Loot** can include advanced tech documents (see § Hub, § Crew Skill Trees) on
  top of whatever other resources missions grant.
- **Squads:** a mission can have up to 4 players join, but every mission must be
  fully completable solo. Carriers, battleships, and crews always belong to a
  single player only — never shared/co-owned. In a multiplayer mission, each
  player brings their own battleship into the shared instance — missions play
  out as a small co-present fleet, not a single shared battleship. (Implication:
  mission-instance combat/space needs to support multiple independently-piloted
  battleships plus their respective companion crews at once.)

### Elimination

The first mission type, and the only one in the First Playable Demo.

- **Objective:** kill N enemies, N rolled per mission, 1–100, never above
  100, then the Fissure Guardian. The HUD shows "Kills x/N", then "destroy
  the Fissure Guardian".
  Kills by any battleship in the instance count toward the shared total.
- **Waves:** enemies arrive in a fixed sequence of waves. The next wave comes
  once the current one is cleared or a timeout passes, after a short pause with
  a "Wave N" banner. More enemies spawn in total than N, so the player never
  has to hunt down the last one.
- **Fissure Guardian:** when the kill count reaches N the fissures go quiet,
  the monsters still out are drawn back into them, and a **Fissure
  Guardian** comes out (design/BATTLE.md B5). Phase 1: rings of slow shots
  with gaps to fly through, plus Swarmer packs. Phase 2 (below half HP):
  faster rings, a Lancer-style charge, and Bloaters. Every attack has a
  warning (the ring flashes and shows its gaps; the charge draws its line).
  A boss bar shows its name and HP; its phase change and death get a flash,
  shake and sound, and it dies in a loot burst. Numbers: design/READINESS.md
  § Elimination mission.
- **Success:** the Fissure Guardian dies. Remaining enemies vanish and any
  loot still on the field is pulled in and collected.
- **Failed:** every battleship in the instance is destroyed (solo: the
  player's), or the player quits the mission. Mission loot is then lost unless
  its reward type's "survives failure" flag is on.
- **Level:** open space with no edge (TAKOAI-58); enemies pour out of void
  fissures placed around the start. No time limit.
- **Loot:** enemies drop pickups (Credits, Void Crystal) that a battleship
  collects by flying over them.

Numbers (wave plan, kill target, drop rates): design/READINESS.md § Demo Spec.

## Ship Modules

- The carrier is a **modularized carrier ship**. Ship progression has two
  distinct layers, and they should stay separate (confirmed — not the same
  system as § Crew Skill Trees):
  1. **Ship Modules** (this section) — coarse unlocks that introduce a *basic
     game mechanic* plus a dedicated area on the carrier. This is about
     gradually introducing systems to the player, not deep progression. Known
     modules so far:
     - **Docking Module** — lets the player build/store **battleships**, the
       vessels taken into missions (see § Missions).
     - **Manufacturing Module** — opens up an area where manufacturing
       buildings/equipment can be placed to produce items.
     - **Lab Module** — for research. Likely (not yet confirmed) the place
       where crew carry out the "researching a sample" / "studying a tech
       document" actions that earn Research points (see § Crew Skill Trees).
     - **Workshop** — First Playable Demo stand-in for the progression
       layer: the player spends Credits and Void Crystal on upgrades that
       apply to every battleship (Hull Plating, Weapon Tuning, Thruster
       Tuning). See design/READINESS.md § Demo Spec.
     Other modules beyond these are still open. On the carrier a module is
     a set of **work stations** the player places (see § Carrier), not a
     pre-drawn room.
  2. **Buildings & recipes within a module** — the actual depth/progression,
     unlocked by leveling up crew members via their skill trees (see § Crew
     Skill Trees). E.g. once Manufacturing is unlocked, further crew leveling is
     what unlocks *which* manufacturing buildings/recipes become available.
- On-ship crafting itself is ONI-style: buildings consume and produce resources,
  and the player has to balance input/output chains (e.g. a machine that turns
  raw material A + power into refined material B) the same way ONI balances
  power, oxygen, and water webs.
- **Building style/art direction reference: Mindustry** — a 2D factory-building
  game (place machines/conveyors, route resources between them). Key difference
  from Mindustry: Mindustry's factory can keep expanding indefinitely across an
  open map, but our carrier building is bounded by the carrier's current
  interior space/size. The carrier's size is itself extended over time by
  adding more modules (see this section) — so unlocking a module both
  introduces a mechanic *and* grows the buildable area.
- Manufacturing has a chance to produce a bonus **"extraordinary sample"**
  alongside normal output — these feed Research points when studied by a crew
  member (see § Crew Skill Trees).
- **Key resources** — needed to unlock/expand things — are obtained through
  trading, both with other players and with hub NPCs (see § Hub, § Economy).
- Open: does trading-for-key-resources unlock *modules*, or feed the *recipes*
  crew leveling unlocks, or both? What other modules exist besides Docking,
  Manufacturing, and Lab? (see design/OPEN_QUESTIONS.md)

## Carrier

_Revision 3 (TAKOAI-88), **approved by Wei on 2026-10-17**. It replaces
Revision 1 (TAKOAI-52, 2026-10-04: pre-drawn rooms joined by corridors) and
Revision 2 (2026-10-12, proposed: floor painted tile by tile). Wei's
direction: stop drawing whole rooms; draw lots of furniture and work
stations and let players place them freely, inside simple rooms the player
builds. Numbers, the catalogue, the starting layout, the art spec and the
build scope: design/READINESS.md § Demo Spec › Carrier._

The carrier interior is a set of **rooms** the player builds inside the
hull, filled with **work stations** (things that do something) and
**furniture and decor** (things that make it theirs). A room is only space:
what makes it a bunk room or a lab is what the player puts in it.

- **View:** 3/4 top-down (2.5D), as in Zelda or Stardew Valley,
  orthographic and axis-aligned, lit from the top left. A wall tile with
  floor south of it shows its front face; other walls show only their top.
  Sprites are drawn in order of their feet's y. One deck.
- **Camera and walking:** unchanged from Revision 1. The camera follows the
  Pilot with three zoom levels; the Pilot walks by click, press-and-hold
  or WASD, the same controls as in battle, and uses stations and talks to
  crew with a click or E.
- **Grid:** 64 px tiles (the Pilot is about one tile tall) in a fixed hull.
- **Rooms:** two sizes, **Small (6 × 4 floor)** and **Large (13 × 9)**, each
  with a one-tile wall ring, built with Credits and Void Crystal. Rooms
  placed side by side share the wall between them and open a 2-tile
  **doorway** through it by themselves; the player never builds doors or
  corridors. A new room must share a wall with an existing one. Rooms are
  assembled from floor, wall and doorway tiles, never drawn whole.
- **Items:** every item has a footprint (1 × 1, 2 × 1, 2 × 2, 4 × 4…) and
  **faces S** (no rotation, one drawing each). Layers: **object** (blocks
  walking), **floor** (rugs, walkable) or **wall** (screens and lights hung
  on a north wall's face). Placement rules are "on floor", "against a
  wall" (its back touches the north wall) or "on a wall".
- **Use and pathing:** a station has a **front strip**, the row south of
  it, which must stay clear and be reachable from the Command Console.
  Doorways keep a clear tile on each side. Any edit that would block a
  station or cut it off is refused with a reason.
- **Stations** keep every function the demo has. The Command Console
  (where the Pilot starts), the Upgrade Bench (Workshop upgrades), one
  Berth per battleship (component customization), plus a separate mission-selection
  station (briefing and launch; TAKOAI-130) and the Sample Analyzer
  (the Researcher's post; using it opens the Codex, which closes back to
  the Carrier; Settings › Codex stays a second way in) are in the starting
  carrier: they can be moved
  but not removed. The Salvage Sorter and the Combat Simulator are bought
  once and carry the old Salvage Bay and Training Room effects.
- **Furniture and decor** are cosmetic in the demo: beds, lockers, tables,
  sofas, plants, lamps, an aquarium, cat beds and cat trees, wall screens
  and lights. A comfort stat is deferred to the economy pass (TAKOAI-4).
- **Building:** B opens Build mode anywhere aboard. It uses the ONI-style
  builder screen: categories (Rooms, Stations, Furniture, Decor), a ghost
  that shows green or red with one reason, plus Move and Remove tools.
  Placing pays and starts a **construction site** that finishes on its own
  after a few seconds (placeholder times while testing); nobody walks
  there. Crew carrying out construction comes with companion jobs.
  Everything is unlocked from the start; removing refunds in full, moving
  is free, and the starting carrier can't be sold.
- **Room types and bonuses:** not tracked in the demo. ONI-style labels
  and bonuses ("two beds and a locker make a Bunk Room") can come later
  without changing the save. Deferred to TAKOAI-4.
- **Battle effects** still reach the battle only through the battleship's
  `Loadout`, so they stay inside the deterministic `sim`. Economy effects
  stay in `client`.
- **Hull and decks:** one deck of fixed size in the demo. Growing the hull
  and adding decks (the "ship size extension" question) stay deferred.

## Enemies / Opposition

- Opposition comes from the **void monsters** occupying space segments (see
  design/STORY.md § Setting). Whether there is also Oni opposition (rival Oni
  groups) to fight is still open.
- Enemies target the nearest battleship, so a mission instance with several
  battleships works without changes.
- **First Playable Demo enemy types** (stats in design/READINESS.md § Demo
  Spec):
  - **Void Swarmer:** weak and fast, comes in groups. Rushes the nearest
    battleship and deals contact damage. Drops Credits, and sometimes Void
    Crystal.
  - **Void Spitter:** fewer of them, and tougher. Keeps its distance from the
    nearest battleship and fires slow projectiles that can be dodged. Drops
    more Credits, and Void Crystal more often.
- **Enemy roles** (TAKOAI-116, from design/BATTLE.md § Enemy roles; numbers
  are starting points in `crates/sim/src/tuning.rs`). Each attack has a
  readable warning first:
  - **Void Lancer (charger):** walks in, stops and draws a charge line at a
    battleship for 0.7 s, then dashes ~400 px along it, hitting each ship
    it passes once. Sidestep the line.
  - **Void Bloater (death-bomber):** slow. Bursts 1 s after it dies, or
    after it touches a battleship (it stops and swells); a warning ring grows
    to the burst's ~90 px reach. The burst hurts ships and monsters alike,
    so it can be popped at range or chained into a pack.
  - **Void Brood (summoner):** hangs back near the fissure it came from and
    releases 3 Swarmers every ~6 s; its sac swells and throbs for 1 s first.
    Dive it first.
  - **Void Warden (buffer):** stays with its pack, away from the battleship,
    and shields every other monster within ~120 px (they take half damage),
    shown by a tether and a shimmer. Kill it first or pull the pack away.
- **Packs mix the roles.** Plain packs are Swarmers with some Spitters. New
  roles join as the mission's kills rise — Lancer at 10% of the kill
  target, Bloater 25%, Warden 40%, Brood 55% — and the first pack after each
  unlock always brings the new role. After that about half the packs are
  role packs: Swarmers with Lancers, Swarmers with Bloaters among them, a
  Warden over Swarmers (sometimes with a Spitter), or a Brood behind
  Spitters and a few Swarmers.

## Battleship components

Wei's accepted playtest #2 direction (2026-11-08): replace engine, armor, main
gun, automatic side gun, targeting system and extra equipment at the existing
battleship Berth. Mission selection/launch is at a separate station. These
combat components are distinct from carrier Ship Modules.

The concrete six-socket mapping (engine → Q, extra equipment → W, main gun →
basic attack), catalogue, stock migration, rarity, salvage/crafting and Threat
1–5 tables are **proposed** in BATTLE.md § Ship modules. They preserve the
existing two skills per hull; E/R remain deferred. Confirmation is tracked only
in OPEN_QUESTIONS.md § Component and loot proposal. Downstream implementation
uses the accepted interaction separation, with proposed balance awaiting Wei.

## Battle Direction

Accepted direction (TAKOAI-109, decided provisionally 2026-11-07 in
TAKOAI-110): battle moves toward a Diablo / Path of Exile feel on top of the
bullet-hell dodging — one-hit chaff with the danger in readable telegraphs,
elites and a Fissure Guardian, ship modules that change skills, module
drops with salvage, and Threat levels. The gap analysis, ranked change
list, decisions (B1–B7) and build order are in design/BATTLE.md. The
sections above still describe the battle as built; each is updated as its
step lands.

## World / Levels

- **Carrier interior:** the player's persistent base — rooms, stations,
  furniture and systems to fix, build out, and upgrade over the course of the game (see
  § Carrier, § Ship Modules).
- **Hub:** dev-managed MMO social/trade space; not player-buildable.
- **Space / missions:** discrete, self-contained mission instances (see
  § Missions), reached by teleporting a selected battleship + crew there. This
  is where exploration, looting, and 2D ship combat happen.

## Economy

- Credits earned via faction missions and resource-gathering quests; spent at the
  hub (trading with NPCs and other players).
- Trading is also how the player obtains **key resources** that expand the
  carrier, on top of whatever's gathered/looted directly on missions.
- Carrier upgrades themselves come from on-ship crafting, gated by Ship Modules
  and Crew Skill Trees (see § Ship Modules, § Crew Skill Trees), not direct
  purchase — credits/trading get you the resources and unlocks, building still
  happens on the carrier.

_Full list of open questions across all design docs: design/OPEN_QUESTIONS.md._
