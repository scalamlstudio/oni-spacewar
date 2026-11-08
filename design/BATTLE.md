# Battle Direction

_**Accepted direction** (TAKOAI-109 proposed 2026-11-02; accepted
2026-11-07, TAKOAI-110). It covers how to make the battle fun: a gap
analysis against a Diablo / Path of Exile feel, a ranked change list, the
decisions it raised (§ Decisions, provisional) and the build order
(§ Build order). Today's battle, until each step lands, is specified in
design/DESIGN.md § Missions and design/READINESS.md § Demo Spec._

## Target feel

Wei: battle is the main reason the game isn't fun yet. The target is closer
to **Diablo / Path of Exile**: monsters chase you and use many different
abilities, and you build your battleship to beat the challenge and move the
story on. Six pillars:

1. **Game feel:** hits and kills feel good to land and to take.
2. **Monsters as puzzles:** distinct roles, readable telegraphs, mixed packs,
   elites with affixes, bosses with phases.
3. **Build crafting:** skills that change how you play, modifiers on them,
   gear with random affixes, build-defining uniques, clear synergies.
4. **Loot and reward:** frequent small drops, rare big ones that look and
   sound special, power you can see, and no drop that's useless.
5. **Pacing:** chaff → elites → boss; short, varied objectives; opt-in
   rising difficulty; risk you can feel.
6. **Story in combat:** story bosses, enemy families per region, story beats
   that unlock enemies, skills and ship parts.

**What stays:** dodging slow, readable shots (the bullet-hell core),
click-to-move, a slow battleship that leans on abilities (DESIGN.md
§ Player), void fissures as the place enemies come from, and battle logic
in the deterministic `sim` crate so multiplayer stays possible.

**Mapping:** the battleship's hull, weapon and modules play the part of
PoE's gear and gems; the Pilot's Combat Operation tree (DESIGN.md § Crew
Skill Trees) is the passive tree; fissures are the portals, each with an
enemy family and elite modifiers.

## Gap analysis

Read from the code on `master` (`crates/sim`, `crates/client/src/render.rs`)
and the Demo Spec, not from a played session.

| Pillar | Rating | What exists today |
|---|---|---|
| 1. Game feel | **Missing** | A hit subtracts HP and deletes the shot (`state.rs` `projectile_hits`); nothing else happens. Enemies vanish on death (`kill_and_drop` drops them from the list). No hit flash (sprites are always drawn untinted), no death burst, no damage numbers, no screen shake, no hit-stop. Taking damage only moves the hull bar. No audio at all (out of demo scope; no audio crate). Knockback exists only on Bulwark's Shockwave. The sim emits no events, and enemies have no stable IDs (they're addressed by list index), so the client can't tell what was hit or what died. |
| 2. Monsters as puzzles | **Weak** | Two roles: Swarmer (rusher, contact damage) and Spitter (keeps 200–320 px, strafes, fires a slow dodgeable shot). The Spitter fires the instant its cooldown ends, with no wind-up. No elites, affixes, bosses, summoners, healers, chargers or bombers. Enemies arrive one at a time from a random fissure, so there are no packs; the only change over a mission is more of both (spawn gap 1.33 s → 0.5 s, Spitter share 10% → 40% over 120 s). |
| 3. Build crafting | **Missing** | Two ships with 2 fixed skills each (Kite: Afterburn, Scatter; Bulwark: Bastion, Shockwave). E and R are wired as inputs but unused. Progression is 3 flat stat upgrades × 2 levels (+25% hull, +25% damage, +15% speed) plus the Combat Simulator (−15% Q/W cooldowns), the same for every ship. No choice inside a ship; everything is maxed after ~6 wins (1200 cr + 30 VC). |
| 4. Loot and reward | **Weak** | Drops are always Credits (every kill) and sometimes Void Crystal. Nothing rare, nothing that changes the ship. Pickups drift in within 80 px and expire after 20 s. The Workshop is the only sink and runs out. The Result screen shows the haul. |
| 5. Pacing and structure | **Weak** | One objective (kill 40), one linear ramp that flattens after 120 s, no elite or boss beats, no difficulty choice. Wins take 52–74 s. Risk is "lose all loot on failure", which is real but not something the player opts into. Strengths to keep: no arena edge, fissure pointers, the 36-enemy cap. |
| 6. Story in combat | **Missing** | Briefing lines at the Berth and the void-monster premise (STORY.md § Setting). No story bosses, enemy families or story beats tied to combat. |

**Two structural causes** behind most of the gaps:

- **Kill speed vs. crowd size.** A Kite needs 3 basic hits per Swarmer and
  enemies trickle in one at a time, so the screen is never a crowd to mow
  through, and never a pack to read. ARPGs get their feel from killing lots
  of weak things fast while a few dangerous things demand attention.
- **Nothing to choose.** With two fixed skills and flat upgrades, every run
  of a ship plays the same, so even good enemies would have nothing to be
  "solved" with.

## Prioritised change list

Ranked by fun per effort. Size: **S** ≈ a few days, **M** ≈ a week or two,
**L** = several weeks or needs a decision first. Owner: **Eng** (engineer),
**Art**, **Design** (design or story pass).

| # | Change | Pillar | Size | Owner |
|---|---|---|---|---|
| 1 | **Hit and kill feedback.** The sim reports hits, kills and damage taken as events with stable entity IDs. The client adds hit flash, a small knock-back on hit, death bursts, damage numbers (with a setting to turn them off), screen shake on taking damage and big skills, and a short render-only freeze on elite or boss kills. | 1 | M | Eng + Art (bursts, sparks) |
| 2 | **Packs and one-hit chaff.** Fissures pulse out packs (e.g. 6–10 Swarmers at once) instead of a trickle; Swarmer HP is tuned so every ship kills one in a single basic hit; more enemies on screen, each weaker. | 1, 5 | S | Eng + Design (tuning) |
| 3 | **Telegraphs.** The Spitter glows and shows its aim line for ~0.4 s before it fires; every new attack in the roster ships with a wind-up the player can read. | 2 | S | Eng + Art |
| 4 | **Placeholder sound.** ~12 sounds (basic fire, hit, enemy death, taking damage, Q/W per ship, credit pickup, crystal pickup, rare drop, elite spawn, success, failure) and one battle loop, from a CC0 (or licensed) placeholder pack (B6). | 1, 4 | S–M | Eng + Art |
| 5 | **Four new enemy roles** (§ Enemy roles): Lancer (charger), Bloater (death-bomber), Brood (summoner), Warden (shields allies). Fed into TAKOAI-5 rather than designed twice. | 2 | M | Design + Eng + Art |
| 6 | **Elite packs with affixes.** A pack leader that's bigger, tinted, named, has ~4× HP and 1–2 affixes (§ Elite affixes), and drops better loot. | 2, 4, 5 | M | Eng + Design |
| 7 | **Fissure Guardian.** A mini-boss with 2 phases (§ Enemy roles) that ends the mission, so a run goes chaff → elites → boss. | 2, 5 | M | Design + Eng + Art |
| 8 | **Ship modules that change skills.** Each battleship gets module sockets; modules modify Q, W or the basic attack (§ Ship modules). They reach the sim through `Loadout`, like upgrades today. | 3 | M–L | Design + Eng |
| 9 | **Loot rarity and module drops.** Modules drop in missions in rarity tiers (common, magic, rare, unique) with their own colour, beam and sound; unwanted modules salvage into Void Crystal. | 4, 3 | M | Eng + Art |
| 10 | **Threat levels.** The Berth offers Threat 1–5: more enemy HP and damage, more elites and affixes, better loot. | 5 | S–M | Design + Eng |
| 11 | **E and R skills.** Each ship picks 2 more skills from a small per-ship pool at the Berth. | 3 | M | Design + Eng + Art |
| 12 | **More objectives:** close the fissures, survive, defend a drone, chase a loot runner. | 5 | L | Design + Eng |
| 13 | **Pilot Combat Operation tree** as the passive tree (already in DESIGN.md, deferred from the demo). | 3 | L | Design + Eng |
| 14 | **Enemy families and story bosses** per region, tied to STORY.md. | 6 | L | Design (story) + Art |

**The three expected front-runners, checked:**

- **Hit feedback / game feel — confirmed, #1.** It's missing entirely, it
  multiplies the value of everything after it, and most of it is
  client-only. Its one sim change (events with stable IDs) is also what
  later items need for elite names, boss bars and loot beams.
- **Enemy roles with telegraphs and elite affixes — confirmed, split.**
  Telegraphs on the existing Spitter are cheap (#3). New roles (#5), elites
  (#6) and the Guardian (#7) are a week or more each, so they make up the
  second batch.
- **Ship modules that modify skills — confirmed as the core of building,
  ranked after enemies.** Builds only matter once there are enemies worth
  solving, and the design depends on the build-depth and loot calls (§ Decisions,
  B3 and B4).

**Proposed first batch:** #1 Hit and kill feedback, #2 Packs and one-hit
chaff, #3 Telegraphs, and #4 Placeholder sound (B6). All are S–M, and
together they change how every second of the current battle feels.

**Second batch:** #5–#7 (enemies and the Guardian). **Third:** #8–#11
(building and loot). The order is fixed in § Build order.

## Decisions

_Decided 2026-11-07 (TAKOAI-110), **provisional**: Wei took the
recommendations to start building and can override any of them after
playtests. The options that were weighed are in the PR #58 history of
design/OPEN_QUESTIONS.md._

- **B1. Bullet-hell vs. ARPG balance → hybrid.** Chaff dies in one hit and
  is there to be mowed down; the danger comes from readable shots and
  telegraphs on ranged enemies, elites and bosses. (Not: bullet-hell first
  with fewer, tougher enemies; not: an ARPG horde where the danger is being
  swarmed.)
- **B2. Battleship mobility → stay slow, one answer per ship.** Base speed
  stays slow (DESIGN.md § Player), but every battleship has one mobility or
  defensive skill (Kite: Afterburn; Bulwark: Bastion), and enemy shots are
  tuned so the slowest ship can dodge them.
- **B3. Build depth → modules first.** Module sockets that change Q, W and
  the basic attack (§ Ship modules), built so random-affix hull / weapon /
  thruster parts, E/R picks and the Pilot's Combat Operation tree can be
  added later. Random-affix parts only once module drops prove fun.
- **B4. Where modules come from → drops plus salvage.** Modules drop in
  missions by rarity; unwanted ones salvage into materials that craft or
  upgrade modules, so no drop is useless. Later the Manufacturing Module
  becomes the crafting bench.
- **B5. Mission rhythm → kill N, then the Fissure Guardian.** Elimination
  becomes: reach the kill count, the Fissure Guardian comes out, and the
  mission succeeds when it dies. "Close the fissures" is the next mission
  type (§ Prioritised change list #12).
- **B6. Sound in the demo → placeholder CC0 pack.** A placeholder pack
  (CC0, or licensed if CC0 has gaps) goes in now and is replaced once audio
  direction is done (OPEN_QUESTIONS.md § Not yet started).
- **B7. Difficulty → Threat levels at the Berth.** The player picks Threat
  1–5 before launching: more enemy HP and damage, more elites and affixes,
  better loot. Threat never raises the kill target above 100 (TAKOAI-128).

## Build order

_From TAKOAI-109. Each step is one or more Engineer / Art issues under it;
numbers refer to § Prioritised change list._

0. **Record the decisions** (this doc, TAKOAI-110).
1. **Batch 1: feel.** #1 Hit and kill feedback (sim events with stable
   entity IDs first, then client flash, knock-back, death bursts, damage
   numbers, shake), #2 Packs and one-hit chaff, #3 Telegraphs on the
   Spitter, #4 Placeholder sound. Art: the battle feedback pack (hit sparks,
   death bursts, Spitter telegraph, damage-number style).
2. **Batch 2: monsters.** #5 Lancer, Bloater, Brood and Warden (via
   TAKOAI-5's roster), #6 elite packs with affixes, #7 Fissure Guardian
   ending Elimination (B5).
3. **Batch 3: building and loot.** #8 Ship modules (B3), #9 loot rarity,
   module drops and salvage (B4), #10 Threat levels at the Berth (B7),
   #11 E and R skills.
4. **Later:** #12 more objectives (close the fissures first), #13 Pilot
   Combat Operation tree, #14 enemy families and story bosses.

Each batch is played before the next starts; a playtest can reorder the
later batches or reopen a provisional decision.

## Enemy roles

_For TAKOAI-5 (first enemies and mission level design) to adopt or change.
Numbers are starting points._

| Role | Enemy | Behaviour | Telegraph | Answer |
|---|---|---|---|---|
| Rusher | Void Swarmer (exists) | Chases in packs, contact damage | none needed | AoE (Scatter, Shockwave), one-hit kills |
| Ranged | Void Spitter (exists) | Holds 200–320 px, strafes, slow shot | glow + aim line, 0.4 s | dodge the shot, close in |
| Charger | **Lancer** | Stops, aims, dashes ~400 px in a line | line drawn on the ground, 0.7 s | sidestep |
| Death-bomber | **Bloater** | Slow; bursts 1 s after death (or on contact), AoE ~90 px | swelling ring, 1 s | kill it at range; chain it into packs |
| Summoner | **Brood** | Hangs back near its fissure, releases 3 Swarmers every ~6 s | sac pulses before release | dive it first |
| Buffer | **Warden** | Shields allies within ~120 px (absorbs half their damage) | visible tether to each ally | kill it first, or push the pack apart |
| Boss | **Fissure Guardian** | Phase 1: rings of slow shots with gaps to fly through, plus Swarmer packs. Phase 2 (below 50% HP): faster rings, a Lancer-style charge, and it spawns Bloaters. | rings flash before firing; charge line | read the gaps, use skills on cooldown |

A tank role (high HP, slow, blocks shots for allies behind it) is left out
of the first pass: against an auto-firing basic attack it mostly adds time,
not decisions.

## Elite affixes

Elites roll 1 affix (2 at higher Threat) from:

- **Swift:** +40% speed.
- **Volatile:** bursts on death like a Bloater.
- **Warding:** pulses a shield on itself and nearby allies every few seconds.
- **Blinking:** teleports next to the ship after a visible 0.5 s mark.
- **Splitting:** splits into two smaller copies on death.
- **Scorching:** leaves short-lived burning patches behind it.
- **Chilling:** its hits slow the ship briefly.

**Left out on purpose:** _Reflect_ (the basic attack fires on its own, so
the player can't stop shooting into a reflector), and invulnerable phases
that only make the player wait.

## Ship modules

_Examples to make B3 (§ Decisions) concrete. Not a catalogue._

| Slot | Module | Effect |
|---|---|---|
| Kite Q, Afterburn | Ion Wake | the dash leaves a damaging trail |
| | Twin Burn | 2 charges |
| Kite W, Scatter | Flechettes | bolts pierce 2 enemies |
| | Focus Barrel | 3 bolts in a narrow cone, double damage |
| Bulwark Q, Bastion | Reactive Plating | the shield bursts for damage when it breaks |
| | Overcharge | damage absorbed raises basic-attack damage for 4 s |
| Bulwark W, Shockwave | Implosion | pulls enemies in instead of pushing them away (pairs with Bloaters) |
| | Echo | a second, weaker pulse 0.5 s later |
| Basic attack (any ship) | Chain | each bolt bounces to 2 more enemies |
| | Incendiary | hits burn for a few seconds |
| | Splinter | kills fire 3 small bolts outward |

## Engineering notes

- **Gameplay vs. cosmetic.** Anything that changes the outcome (knock-back,
  affixes, modules, telegraph timings, Threat) lives in `sim` and stays
  deterministic. Flash, numbers, shake, particles and sound live in
  `client`, driven by sim events.
- **Events under rollback.** Rollback re-simulates frames, so the client
  must not play a sound or spawn a burst twice for the same hit. Key events
  by (frame, entity ID) and drop duplicates; this needs stable entity IDs,
  which the sim doesn't have yet.
- **Hit-stop.** The sim can't pause for one player in a shared mission, so
  hit-stop is a render-only freeze of a few frames, reserved for elite and
  boss kills.
- **Loadout.** Modules, E/R picks and Threat reach the sim through
  `Loadout` / the mission start state, the way Workshop upgrades do today,
  so every peer starts identical.
