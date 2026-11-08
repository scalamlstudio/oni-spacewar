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
| 10 | **Threat levels.** The mission-selection station offers Threat 1–5: more enemy HP and damage, more elites and affixes, better loot. | 5 | S–M | Design + Eng |
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
- **B3. Build depth → modules first.** Originally module sockets that change Q, W and
  the basic attack (§ Ship modules), built so random-affix hull / weapon /
  thruster parts, E/R picks and the Pilot's Combat Operation tree can be
  added later. Random-affix parts only once module drops prove fun.
  Playtest #2 supersedes the socket framing with six replaceable component
  categories (§ Ship modules); the concrete mapping and balance remain proposed.
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
- **B7. Difficulty → Threat levels at the mission-selection station.** The player picks Threat
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
   module drops and salvage (B4), #10 Threat levels at the mission-selection station (B7),
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

**Accepted interaction direction — Wei, playtest #2, 2026-11-08:** replace
engine, armor, main gun, automatic side gun, targeting system and extra equipment
at the existing battleship interaction point (Berth). Mission selection and
launch belong at a separate station (TAKOAI-130). This supersedes B3's original
skill-sockets-only framing and B7's Berth location. The boss fight is accepted.

**TAKOAI-120 proposal, awaiting Wei's confirmation:** all catalogue values,
slot mapping, unlocks, rarity, crafting and Threat numbers below are starting
balance values, not accepted decisions or descriptions of implemented gameplay.
Confirmation lives only in [OPEN_QUESTIONS.md § Component and loot proposal](OPEN_QUESTIONS.md#component-and-loot-proposal).

### Component sockets and progression

Both Kite and Bulwark have **six component sockets**, one of each requested
type. One component occupies each socket; replacing it returns the old one to
inventory. Engine carries the Q modifier, extra equipment carries the W
modifier, main gun carries the basic-attack modifier. These are complete
replacement components, not three additional rune sockets. Armor, side gun
and targeting are independent components. Q/W names and inputs stay intact;
E/R remain deferred. Carrier Ship Modules (Docking, Lab, etc.) are a different
system.

Proposed demo unlock: all six sockets open immediately on both ships, with
free stock components. No Workshop socket tax or hidden progression gate.
One item instance can be equipped on only one ship at once; shared types can
be transferred at the Berth. Engine and extra equipment are hull-specific;
the other four types fit either hull. Selection previews old/new effective
stats, behavior and trade-offs before confirmation. Refit only in the carrier;
mission start snapshots the selected hull and all six components.

Stock engine/equipment preserve the current Q/W; stock armor/targeting preserve
hull and nearest-enemy targeting; stock main gun preserves current basic fire.
Stock side gun is inactive, so migrating an old save preserves its balance.
Stock items cannot be sold or salvaged and can always be restored for free.
Existing Workshop upgrades and Combat Simulator remain applicable.

### Demo catalogue

Numbers use current unupgraded stats (READINESS.md § Battleships): Kite basic
4 damage / 0.35 s, Bulwark 8 / 0.6 s; Q Afterburn 160 px / 4 s or Bastion
40 shield / 5 s duration / 12 s cooldown; W Scatter 5 × 6 damage / 6 s or
Shockwave 15 damage / 150 px / 100 px push / 9 s. Below, D is the equipped
hull's base basic damage, before Workshop bonuses. Unmentioned parameters stay
at stock. Percentages are multiplicative; cooldown reduction from Simulator
applies after component changes. Damage receives Weapon Tuning once, including
secondary damage. Stock is additional to these **28 lootable designs**.

| Socket / hull / skill | Component | Effect and trade-off (common baseline) |
|---|---|---|
| Engine / Kite Q | Ion Wake | Dash leaves a 24 px wide trail for 2 s; enemies touching it take 4 damage/s (no stacking trails). Dash cooldown 5 s. |
| Engine / Kite Q | Twin Burn | Two dash charges, one restored every 5 s sequentially; each dash travels 120 px. |
| Engine / Kite Q | Long Burn | Dash 240 px; cooldown 6 s. No invulnerability added. |
| Extra / Kite W | Flechettes | Each Scatter bolt can hit 3 distinct enemies once each; bolt damage 4 instead of 6. |
| Extra / Kite W | Focus Barrel | Three bolts in a 12° cone, 12 damage each; cooldown 7 s. |
| Extra / Kite W | Wide Sweep | Seven bolts over 70°, 5 damage each; cooldown 7 s. |
| Engine / Bulwark Q | Reactive Plating | Shield 30 instead of 40; on damage breaking it, burst for 20 damage in 100 px. Expiry gives no burst. |
| Engine / Bulwark Q | Overcharge | Shield 30; each absorbed damage adds 1% basic damage, capped at 30%, for 4 s from the latest absorption. |
| Engine / Bulwark Q | Deep Bastion | Shield 60 for 6 s; cooldown 16 s. |
| Extra / Bulwark W | Implosion | Pull enemies 80 px toward the ship instead of pushing; damage 12. Boss displacement immunity remains. |
| Extra / Bulwark W | Echo | Initial pulse 10 damage and 100 px push; second pulse after 0.5 s, centered on current ship position, 8 damage, no push, same 150 px radius. |
| Extra / Bulwark W | Repulsor | Radius 200 px and push 140 px; damage 10, cooldown 11 s. |
| Main gun / either / basic | Chain | Primary retains D; bounces to two distinct targets within 100 px of last hit, for 0.5D then 0.25D. Fire interval ×1.2. |
| Main gun / either / basic | Incendiary | Hit D plus burn 0.25D/s for 3 s; interval ×1.2. Rehits refresh, do not stack burns from this ship. |
| Main gun / either / basic | Splinter | Primary D; direct primary kills launch three outward bolts 120° apart, 0.5D each, range 100 px; interval ×1.2. |
| Main gun / either / basic | Rail | Primary 1.5D, pierces up to 3 distinct enemies; interval ×1.6, range ×0.85. |
| Armor / either | Reinforced | Maximum hull ×1.25; movement speed ×0.90. |
| Armor / either | Lightweight | Maximum hull ×0.85; movement speed ×1.15. |
| Armor / either | Ablative | First incoming hit every 8 s reduced by 50%; maximum hull ×0.90. Recharge starts on that hit, no charges. |
| Side gun / either | Point Defense | Every 1 s removes one hostile projectile within 90 px, nearest first; deals no enemy damage and cannot erase beams, contact or explosions. |
| Side gun / either | Flak | Automatically fires at nearest enemy within 120 px every 1.5 s; impact deals 0.5D in 40 px radius. Short reach requires approaching danger. |
| Side gun / either | Needle | Automatically fires at nearest enemy within 260 px every 1 s for 0.25D; single target, no control effect. |
| Targeting / either | Long Lens | Main-gun range ×1.25; main fire interval ×1.10. |
| Targeting / either | Priority Lock | Main gun prefers Guardian, then elites, then nearest; main range ×0.90. |
| Targeting / either | Close Tracker | Main interval ×0.90; main range ×0.80. |
| Extra / either / passive | Recovery Rig | Replaces the W modifier with stock W; after 6 s without hull damage, heal 1% maximum hull/s, capped at 20% maximum hull per mission. |
| Engine / either / passive | Cruise Drive | Replaces Q modifier with stock Q; move speed ×1.10, Q cooldown ×1.15. |
| Armor / either | Emergency Bulkhead (unique only) | Once per mission, fatal hull damage instead leaves 1 HP and grants 1 s damage immunity; maximum hull ×0.75. |

Recovery Rig and Cruise Drive are shared alternatives, not extra sockets.
Unique-only Emergency Bulkhead has no common/magic/rare version. The other
27 designs have those three rarities. Secondary hits never trigger Chain,
Splinter, burns or side guns recursively; side guns do not inherit main-gun
modifiers or targeting. Closest-target ties resolve by stable entity ID.

### Rarity and drops

| Tier | Color (label also shown) | Numeric quality and effects |
|---|---|---|
| Common | white #D9DEE8 | Catalogue baseline; no random bonus. |
| Magic | blue #559DFF | One roll: +5–10% maximum hull. |
| Rare | yellow #FFD45A | Two rolls: +10–15% maximum hull and +3–6% movement speed. |
| Unique | orange #FF953D | Fixed build-defining design; no random bonuses and cannot be quality-rerolled. |

Rolls use whole integer percentage points, uniformly inclusive. Random bonuses
are deliberately restricted to hull/speed in this demo to avoid changing the
catalogue's one-hit chaff breakpoints. Sum hull bonuses across equipped items,
and speed bonuses separately, then multiply Workshop stats, component-specific
multipliers, and the summed bonus factor. Numeric quality never changes the
negative trade-off. Unique pool: Emergency Bulkhead, **Storm Relay** (main gun:
Chain with four bounces at 0.5D each instead of two, interval ×1.5), **Furnace
Core** (Kite engine: Ion Wake deals 8/s for 3 s, cooldown 6 s), **Gravity Heart**
(Bulwark extra: Implosion radius 200 px, damage 8, cooldown 12 s). These last
three are unique variants of catalogue designs, bringing total item templates
to 31. There is no random unique conversion.

Threat 1 table: roll module presence first, then rarity conditional on a drop.
Existing Credits/VC remain separate. Non-elite non-chaff roles use the normal
row; Guardian supersedes elite; summoned enemies and splitting copies give no
module drops. Eligible designs are uniform within tier, restricted to the
mission's selected hull plus shared components. Each roll gives one item.

| Enemy | Module chance per kill | Common | Magic | Rare | Unique |
|---|---:|---:|---:|---:|---:|
| Chaff (Swarmer) | 1% | 80% | 18% | 2% | 0% |
| Normal role | 3% | 70% | 25% | 5% | 0% |
| Elite leader | 30% | 20% | 55% | 24% | 1% |
| Guardian | 100% | 0% | 55% | 40% | 5% |

Modules are mission escrow: collected pickups persist only on success; failure
or quit loses newly collected items, preserving pre-mission inventory/equipment.
No pickup expiry for modules; victory collects all remaining drops before the
Result screen. Persistent inventory is unbounded in the demo (filter/sort by
socket, hull, rarity). Salvaging requires an explicit carrier action; equipped
items must be unequipped first. Unique salvage shows a confirmation.

Art handoff: socket icons for six categories, distinct silhouettes for each
design, hull/skill tags, and rarity label + border + beam (never color alone).
Common: small white sparkle; magic: blue short beam; rare: taller yellow beam
and rare sound; unique: orange pulsing beam and distinct unique sound. Compare
panel shows every effect, penalty, rolled bonus and salvage yield.

### Salvage and craft (Workshop / Upgrade Bench)

Use existing **Void Crystal**, not a new currency. Common → 1 VC, magic → 2,
rare → 5, unique → 12. Salvage Sorter's existing mission-haul bonus applies to
Credits/VC pickups only, not module quantity, rarity or salvage yields.

| Action | Cost | Result |
|---|---|---|
| Craft known non-unique design | 50 credits + 3 VC | Common item, player chooses hull/socket/design; all 27 recipes available immediately. |
| Upgrade common → magic | 75 credits + 4 VC | Same design, new magic roll. |
| Upgrade magic → rare | 150 credits + 8 VC | Same design, new rare rolls. Preview warns that existing roll is replaced. |
| Reroll magic | 25 credits + 2 VC | Rerolls its hull bonus, retains design/tier; equal result possible. |
| Reroll rare | 50 credits + 4 VC | Rerolls both bonuses, retains design/tier; equal result possible. |

Upgrades consume the previous item and retain its identity/equip binding;
no rare→unique upgrade, salvage→craft loop profit, or mission-time crafting.
Crafting a common for 3 VC and salvaging it for 1 always loses resources.
These costs compete with existing Workshop upgrades and need playtest tuning.

### Threat 1–5

All levels available from the **separate mission-selection station**, with
loadout summary, risk/loot preview and explicit launch. Default Threat 1;
remember last choice but never silently raise it. Threat multiplies current
role HP/damage after elite base scaling; Guardian uses the same multipliers.
**Non-elite Swarmers stay 4 HP at every Threat**, preserving one-hit chaff;
other enemies scale. Telegraph duration and projectile speed stay unchanged.

| Threat | HP × | Damage × | Elite leaders / eligible packs | Affixes per leader | Loot bonus |
|---|---:|---:|---:|---:|---|
| 1 | 1.00 | 1.00 | 10% | 1 | baseline |
| 2 | 1.15 | 1.10 | 15% | 1 | +15% currency, module chance ×1.15 |
| 3 | 1.35 | 1.20 | 20% | 2 | +30% currency, module chance ×1.30 |
| 4 | 1.60 | 1.35 | 25% | 2 | +50% currency, module chance ×1.50 |
| 5 | 1.90 | 1.50 | 30% | 2 | +75% currency, module chance ×1.75 |

Round HP/damage to nearest integer (minimum 1). Eligible packs are ordinary
mission packs with at least one non-summoned enemy; one leader maximum,
selected by seeded RNG before spawning. Boss adds never roll elite. No random
elite affixes on Guardian; its authored phases remain unchanged. Affixes are
chosen without replacement from implemented, compatible affixes; if fewer
than required exist, use those available. Module chance caps at 100%; Guardian
always gives exactly one module. Currency bonus multiplies pickup amounts,
rounding down per pickup, before Salvage Sorter bonus. Rarity weights stay as
above: higher Threat improves quantity, not a hidden second rarity multiplier.
Elimination N remains seeded **1–100**, unaffected by Threat. No extra mandatory
waves or minimum elites can lengthen a short-N mission.

### Implementation handoff and verification

TAKOAI-130 owns the mission-selection station and walking/spawn fixes; its
station launches the selected hull with mission seed and later Threat, while
the existing Berth opens component refit. TAKOAI-122 owns six-slot equipment,
loadout validation, stock-save migration and the catalogue; TAKOAI-121 owns
icons, compare panels and loot effects; TAKOAI-123 owns escrow, drops, inventory,
salvage and craft; TAKOAI-124 owns Threat scaling/preview. These are scope
boundaries, not permission to implement unconfirmed design (see canonical
confirmation entry). Existing saves receive stock components, preserving
credits, VC, upgrades and hull unlocks. Stable item IDs, template IDs, rolls,
per-hull equipment and selected Threat must be persisted/versioned; mission
seed controls drops, rarity, rolls, affixes and all target tie-breaking in sim.

Acceptance examples for downstream work: reject wrong-hull/duplicate-instance
equipment; verify one-hit 4-HP chaff with every main gun; compare Twin Burn
recharge and Echo timing; ensure secondary damage cannot recurse; same seed +
loadout gives identical drops; failure retains old gear and loses escrow;
Guardian gives one module at every Threat; rarity rows sum to 100%; salvage
and reroll never mutate equipped gear silently; old saves load stock stats;
Berth refits and the separate station launches. Balance is to be playtested,
not claimed as validated by these document checks.

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
