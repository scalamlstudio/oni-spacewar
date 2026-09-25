# Readiness

_What "the design is solid enough to start building" means for Oni Spacewar, and
which open questions stand in the way. Questions themselves live only in
design/OPEN_QUESTIONS.md — this doc references them by name, it doesn't copy
them. Task tracking lives on the Multica board (TAKOAI-2 … TAKOAI-7)._

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

- Research/study action (TAKOAI-4): a timed task a crew member is assigned to,
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
2. **Every "Blocks first playable" question below is decided** — reflected in
   the relevant design doc and removed from design/OPEN_QUESTIONS.md.
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
7. **Deferred questions are explicitly deferred** — each "Can defer" item below
   is acknowledged as out of the first playable, so it can't silently block
   implementation.

Not required before development: final numbers/balance, art, audio, story
beyond the opening premise, multiplayer or hub design.

## Question Triage

Every entry in design/OPEN_QUESTIONS.md, tagged against the first playable
scope above. "Partial" = a first-playable answer is needed now, the full answer
can wait.

### Mechanics

| Question (OPEN_QUESTIONS.md) | Tag | Board | Reason |
|---|---|---|---|
| Carrier control outside missions | **Blocks** | TAKOAI-2 | Half the core loop happens here; can't build the carrier scene without its controls. |
| Basic weapon-fire action | **Blocks** | TAKOAI-3 | Defines the moment-to-moment combat input and the whole enemy balance. |
| Companion combat skill trigger conditions | **Blocks (partial)** | TAKOAI-3 | Need the trigger model + multi-trigger rule for 1-2 skills; the full condition range can grow later. |
| Research/training as an action | **Blocks (partial)** | TAKOAI-4 | Needed to close the loop from loot to skill points; one source is enough for now. |
| Resource categories | **Blocks (partial)** | TAKOAI-4 | Need the handful of resources in the one production chain; full taxonomy can wait. |
| Machines/rooms progression | **Blocks (partial)** | TAKOAI-4 | Need the 2-3 starting buildings + one unlockable; early/late curve can wait. |
| Enemies/opposition | **Blocks (partial)** | TAKOAI-5 | Need 2 void-monster types; human/Oni opposition can wait. |
| Level design within missions | **Blocks (partial)** | TAKOAI-5 | Need one objective type and hand-built vs. procedural for the first segment. |
| Extraordinary sample variety | Can defer | TAKOAI-4 | One sample kind (or none, if tech documents are the chosen source) is enough. |
| Key-resource trading target | Can defer | TAKOAI-4 | Trading is out of first-playable scope. |
| Module roster | Can defer | TAKOAI-4 | First playable uses only Docking + Manufacturing, both already decided. |
| Ship size extension | Can defer | TAKOAI-4 | Carrier is fixed-size in the first playable. |
| Failure reward exceptions | Can defer | TAKOAI-5 | Mechanism is decided; the flag defaults to off. Already deliberately pending. |

### Story

| Question (OPEN_QUESTIONS.md) | Tag | Board | Reason |
|---|---|---|---|
| Timeline & map | Can defer | TAKOAI-6 | One mission segment needs no map. |
| Wormhole accident specifics | Can defer | TAKOAI-6 | The premise (stranded, semi-functional carrier) is enough for a title card. |
| Oni evolution & culture | Can defer | TAKOAI-6 | "Magical abilities" doesn't need to be concrete until skill trees grow past a stub. |
| Factions | Can defer | TAKOAI-6 | Hub and factions are out of scope. |

### Not yet started

| Question (OPEN_QUESTIONS.md) | Tag | Board | Reason |
|---|---|---|---|
| Art direction | Can defer | TAKOAI-7 | Placeholder shapes are fine for the first playable. |
| Audio direction | Can defer | TAKOAI-7 | No audio needed for the first playable. |

## Recommended Decision Order

For the blocking questions, in the order that lets each answer constrain the
next:

1. **Confirm the first playable scope** (this doc, TAKOAI-1) — it decides which
   questions below are "partial".
2. **Basic weapon-fire action** (TAKOAI-3) — cheapest decision with the biggest
   downstream effect; the existing code already has mouse movement and Q/W/E/R
   skill hotkeys, so combat can be prototyped first.
3. **Companion skill triggers** — trigger model + multi-trigger rule
   (TAKOAI-3). Settles what companions do in combat.
4. **Enemies — first two void-monster types** (TAKOAI-5). Only designable once
   the player's attack/skills are known.
5. **Level design — one objective, hand-built vs. procedural** (TAKOAI-5).
   Completes the mission half of the loop.
6. **Carrier control outside missions** (TAKOAI-2). Includes how a mission is
   launched and what the main character does aboard.
7. **Resource categories + machines for the one production chain** (TAKOAI-4).
   Depends on what missions drop (4-5) and how building works (6).
8. **Research/training as an action** (TAKOAI-4). Last, because it's the link
   between the production chain (7) and skill trees.

Steps 2-5 and step 6 are independent and can run in parallel if Wei prefers to
alternate between combat and carrier topics.

_Full list of open questions across all design docs: design/OPEN_QUESTIONS.md._
