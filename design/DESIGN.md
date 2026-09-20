# Design

_How the game actually plays, once the concept in CONCEPT.md is settled._

## Core Structure

The player's persistent base **is the ship** — not a colony on the ground. All the
ONI-style build/fix/life-support depth applies to the ship itself. The hub is a
separate, dev-managed MMO-style social space (see § Hub), not something the player
builds.

## Core Loop

1. **Opening:** the player starts with one semi-functional spaceship (see
   design/STORY.md § Opening: The Wormhole Accident for the narrative reason).
   Many systems need to be built/fixed — this introduces the ship's build/repair/
   life-support mechanics.
2. **Crew:** 1 main controllable character + 2-3 companions aboard the ship.
   Companions can be trained and work alongside the main character (ONI-style task
   assignment — see § Crew / Companions).
3. **Ongoing loop:** The ship explores space, loots, and fights (2D combat).
   Periodically it docks at the hub to trade, take on faction missions, and spend
   earned credits — then heads back out. The ship (not the hub) is where
   progression/building happens.

## Player

- Movement:
- Combat / interaction:
- Progression: earn credits via faction missions and resource-gathering, spend on
  growing/upgrading the ship (see § Hub, § Economy).

## Crew / Companions

- Main character: player-controlled directly (likely direct control during combat/
  exploration, given the 2D shooter half of the game).
- 2-3 companions: ONI-style task assignment — the player assigns jobs/priorities
  (fix this, mine that, defend here) and companions execute autonomously, rather
  than being directly piloted. They can be trained (skills/roles improve over time,
  ONI-dupe-style).

## Hub

- An MMO-style social/trade space where players' ships dock between expeditions.
- Purpose: trading (both player-to-player and with hub NPCs), and a base for
  **factions**.
- **Factions:** players can join a faction and earn credits/reputation by
  completing missions for it.
- Also offers **resource-gathering quests** the player can do at/from the hub.
- Trading here is a key source of the resources needed to expand the ship's tech
  tree (see § Crafting & Tech Tree) — not just a credit sink.
- Important distinction from a typical base-builder: the player has little to no
  impact on the hub's own growth or appearance — it's managed by the developers,
  not built up by players. All the "building" fantasy lives on the ship.
- Multiplayer: the hub is the shared social space where players see/interact with
  each other (trade, faction presence).

## Missions

- Mission structure is modeled on **Warframe**: discrete, self-contained runs with
  their own objective(s), taken on (from the hub / via factions) and then
  played out separately from the persistent ship/hub state.
- **Stakes:** losing the ship during a mission costs the player only the time and
  rewards tied to *that mission* — it does not threaten the ship's persistent
  build/upgrades or the crew's overall progression. Low permanent risk, same
  spirit as dying/failing a Warframe mission (you lose that mission's run, not
  your Warframe or account progress).
- **Failure rewards:** on mission failure, most rewards are lost. Which rewards
  are exceptions (still pay out on failure) is undecided — but each reward type
  should carry its own configurable "survives failure" flag/rule, rather than one
  global rule, so exceptions can be tuned/expanded per reward later without a
  system rework. Which rewards actually get flagged that way is still open.
- **Squads:** a mission can have up to 4 players join, but every mission must be
  fully completable solo. Ships and crews always belong to a single player only —
  never shared/co-owned. In a multiplayer mission, each player brings their own
  ship into the shared instance — missions play out as a small co-present fleet,
  not a single shared ship. (Implication: mission-instance combat/space needs to
  support multiple independently-piloted ships plus their respective companion
  crews at once.)

## Crafting & Tech Tree

- On-ship crafting is central, ONI-style: the player builds machines/stations on
  the ship that consume and produce resources, and has to balance input/output
  chains (e.g. a machine that turns raw material A + power into refined material B)
  the same way ONI balances power, oxygen, and water webs.
- This is the main way the ship is upgraded/expanded over time — not just buying
  finished upgrades, but building the production chain that makes them.
- **Key resources** — needed to unlock/expand the tech tree — are obtained through
  trading, both with other players and with hub NPCs (see § Hub, § Economy).
- Open: shape of the tech tree (tiers, branching vs. linear), the resource
  categories (raw/refined/key-trade), and which machines/rooms exist early vs.
  late game (see design/OPEN_QUESTIONS.md).

## Enemies / Opposition

_TBD_

## World / Levels

- **Ship interior:** the player's persistent base — rooms/systems to fix, build
  out, and upgrade over the course of the game.
- **Hub:** dev-managed MMO social/trade space; not player-buildable.
- **Space / missions:** discrete, self-contained mission instances (see § Missions)
  where exploration, looting, and 2D ship combat happen.

## Economy

- Credits earned via faction missions and resource-gathering quests; spent at the
  hub (trading with NPCs and other players).
- Trading is also how the player obtains **key resources** that expand the ship's
  tech tree, on top of whatever's gathered/looted directly on missions.
- Ship upgrades themselves come from on-ship crafting (see § Crafting & Tech
  Tree), not direct purchase — credits/trading get you the resources and unlocks,
  building still happens on the ship.

_Full list of open questions across all design docs: design/OPEN_QUESTIONS.md._
