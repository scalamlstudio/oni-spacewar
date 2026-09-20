# Design

_How the game actually plays, once the concept in CONCEPT.md is settled._

## Core Structure

The player's persistent base **is the ship** — not a colony on the ground. All the
ONI-style build/fix/life-support depth applies to the ship itself. The hub is a
separate, dev-managed MMO-style social space (see § Hub), not something the player
builds.

## Core Loop / Story Structure

1. **Opening (wormhole accident):** Story starts with a wormhole accident leaving
   the player with one semi-functional spaceship. Many systems need to be built/
   fixed — this introduces the ship's build/repair/life-support mechanics.
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
- Purpose: trading, and a base for **factions**.
- **Factions:** players can join a faction and earn credits/reputation by
  completing missions for it.
- Also offers **resource-gathering quests** the player can do at/from the hub.
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
- Open: does a mission failure/ship-loss still grant partial rewards (Warframe-
  style partial extraction) or zero rewards?
- **Squads:** a mission can have up to 4 players join, but every mission must be
  fully completable solo. Ships and crews always belong to a single player only —
  never shared/co-owned. In a multiplayer mission, each player brings their own
  ship into the shared instance — missions play out as a small co-present fleet,
  not a single shared ship. (Implication: mission-instance combat/space needs to
  support multiple independently-piloted ships plus their respective companion
  crews at once.)

## Enemies / Opposition

_TBD_

## World / Levels

- **Ship interior:** the player's persistent base — rooms/systems to fix, build
  out, and upgrade over the course of the game.
- **Hub:** dev-managed MMO social/trade space; not player-buildable.
- **Space / missions:** discrete, self-contained mission instances (see § Missions)
  where exploration, looting, and 2D ship combat happen.

## Economy

- Credits earned via faction missions and resource-gathering quests.
- Spent on... ship upgrades/parts, presumably (see § Open Questions — what exactly
  is purchasable, and where: only at the hub, or also from loot/crafting on the
  ship?).

## Open Questions

- How does credit-earning (faction missions, resource quests) translate into
  concrete ship upgrades — is there crafting on the ship too, or is the hub the
  only source of new ship parts/tech?
- Does mission failure/ship-loss grant partial rewards, or none?
- What do factions actually represent (in-fiction) — political/military groups,
  trade guilds, something tied to the Oni species or the wormhole accident?
