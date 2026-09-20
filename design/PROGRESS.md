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
