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
- Combat / interaction: the main character's Combat Operation skills are
  manually triggered by the player (hotkeys, LoL-ability-style) while the
  battleship moves (see § Crew Skill Trees) — unlike companions, whose combat
  skills auto-trigger automatically when their condition is matched.
- Progression: earn credits via faction missions and resource-gathering, spend on
  growing/upgrading the carrier (see § Hub, § Economy).
- Open: how the player controls/manages the carrier itself outside of missions
  (building placement, task assignment) — the click-to-move/ability description
  above is specifically about piloting the battleship in a mission.

## Crew / Companions

- Main character: **the battleship's pilot during missions** — player-controlled
  directly, flying the battleship and manually triggering their own Combat
  Operation skills (see § Crew Skill Trees). Role while the carrier is docked/
  being built up (outside of missions) is still open.
- 2-3 companions: ONI-style task assignment — the player assigns jobs/priorities
  (fix this, mine that, defend here) and companions execute autonomously, rather
  than being directly piloted. They can be trained (skills/roles improve over time,
  ONI-dupe-style), and their combat skills auto-trigger on condition rather than
  being manually activated (see § Crew Skill Trees).

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
- Open: the full range of condition types for companion skills beyond the two
  examples above, what happens when multiple companion skills' conditions are
  met at once (priority/cooldowns); the range/tiers of extraordinary samples and
  how their point values differ; and how "researching" a sample or "studying" a
  document plays out as an action (time cost, task assignment, etc.) — see
  design/OPEN_QUESTIONS.md.

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
  system rework. Which rewards actually get flagged that way is still open.
- **Loot** can include advanced tech documents (see § Hub, § Crew Skill Trees) on
  top of whatever other resources missions grant.
- **Squads:** a mission can have up to 4 players join, but every mission must be
  fully completable solo. Carriers, battleships, and crews always belong to a
  single player only — never shared/co-owned. In a multiplayer mission, each
  player brings their own battleship into the shared instance — missions play
  out as a small co-present fleet, not a single shared battleship. (Implication:
  mission-instance combat/space needs to support multiple independently-piloted
  battleships plus their respective companion crews at once.)

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
     Other modules beyond these three are still open.
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

## Enemies / Opposition

- Narrative source is likely the **void monsters** occupying space segments
  (see design/STORY.md § Setting), but gameplay specifics — enemy types,
  behaviors, whether all opposition is void monsters or there is also Oni
  opposition (rival Oni groups) to fight — are still entirely open.

## World / Levels

- **Carrier interior:** the player's persistent base — modules, rooms, and
  systems to fix, build out, and upgrade over the course of the game (see
  § Ship Modules).
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
