# Readiness

_What "the design is solid enough to start building" means for Oni Spacewar.
Questions themselves — including which ones block this scope and the order to
decide them in — live only in design/OPEN_QUESTIONS.md; this doc doesn't copy
them. Task tracking lives on the Multica board (TAKOAI-1 … TAKOAI-8)._

## First Playable Scope

_**Proposal — for Wei to confirm or cut.** Nothing below is decided until it's
logged in design/PROGRESS.md._

The smallest vertical slice that proves the core loop: **build on the carrier →
fly a battleship mission → bring loot home → build/level something new.** One
pass through that loop, repeatable, is the whole target.

**In scope**

- **Carrier (between missions):** one small, fixed-size interior grid. Two
  modules pre-unlocked: **Docking** (holds one battleship) and
  **Manufacturing** (2-3 buildings, Mindustry-style placement + conveyors, one
  short input/output chain: raw → refined → something the battleship or carrier
  uses). No module unlocking or ship-size extension yet.
- **Crew:** main character + 2 companions. Companions take ONI-style task
  assignments on the carrier (build, operate a machine). Life support, if any, is
  a single resource with a visible fail state — not a full ONI web.
- **Mission:** one hand-built space segment, one objective type (e.g. clear the
  area / reach the exit with loot), solo only. Launched directly from the
  carrier — no hub.
- **Battleship combat:** click-to-move battleship, basic attack (if kept — see
  TAKOAI-3), 2-3 main-character skills on Q/W/E/R, 1-2 companion auto-trigger
  skills, 2 void-monster enemy types.
- **Progression:** Combat points from kills and Research points from one source
  (tech document as mission loot, *or* one kind of extraordinary sample). Each
  crew member has a stub of both skill trees, 3-5 nodes each — enough to unlock
  one new recipe/building and one new combat skill after a mission.
- **Failure:** losing the battleship ends the mission, loot is dropped, carrier
  and crew progression untouched. Every reward type carries the "survives
  failure" flag, defaulting to *off*.
- **Art/audio:** placeholder shapes (existing Circle/Triangle/Square/Hexagon)
  and no audio.

**Out of scope for the first playable** (deferred, not rejected)

- Hub, trading, credits, factions, player-to-player anything.
- Multiplayer missions (up to 4 players) — the design keeps it, but the first
  build is single-player and offline. Code should avoid assuming there is only
  ever one battleship in a mission instance, and stop there.
- Lab Module, module unlocking, carrier growth, additional modules.
- Procedural levels, multiple objective types, story content beyond a title
  card for the wormhole accident.

**Defaults proposed where the scope needs a call** (flagged for Wei)

- Research/study action (TAKOAI-8): a timed task a crew member is assigned to,
  same as any other ONI-style job — reuses the task system instead of adding a
  new one.
- Level design (TAKOAI-5): hand-built for the first playable; procedural stays
  an open question.
- Life support: minimal (one resource) or none — Wei to pick; the checklist only
  requires that the choice is written down.

## Design-Done Checklist

Development on the first playable can start when all of these hold:

1. **Scope confirmed** — Wei has accepted, edited, or replaced § First Playable
   Scope above, and design/PROGRESS.md logs it.
2. **Every blocking question is decided** — each design/OPEN_QUESTIONS.md
   entry tagged `Blocks first playable` or `Blocks (partial)` is reflected in the
   relevant design doc and removed from design/OPEN_QUESTIONS.md.
3. **Between-missions controls are written down** — how the player places
   buildings and assigns crew tasks on the carrier, and how a mission is
   launched from it (design/DESIGN.md § Player, § Crew / Companions).
4. **One combat round is specified end to end** — what the player presses,
   what the battleship does, what companions do on their own, what enemies do,
   and how a mission is won or lost.
5. **One progression round is specified end to end** — which resource comes
   back from a mission, what it's refined into, which points it earns, and which
   skill node or building it unlocks. Concrete names and rough numbers, not
   categories.
6. **Docs are consistent** — no doc contradicts a decision logged in
   design/PROGRESS.md (e.g. design/CONCEPT.md's pitch and reference games still
   describe "the ship" as both base and combat vessel, and cite FTL for combat;
   both predate the carrier/battleship split and the LoL-style control scheme).
7. **Deferred questions are explicitly deferred** — each design/OPEN_QUESTIONS.md
   entry tagged `Deferred` is acknowledged as out of the first playable, so it
   can't silently block implementation.

Not required before development: final numbers/balance, art, audio, story
beyond the opening premise, multiplayer or hub design.

_Open questions — tagged as blocking or deferred against this scope, with their
board issues and a recommended decision order: design/OPEN_QUESTIONS.md._
