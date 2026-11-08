//! Battle hit and kill feedback (TAKOAI-112, design/BATTLE.md #1): enemy hit
//! flash, Art's hit sparks / death bursts / skill impacts, floating damage
//! numbers, screen shake and a render-only hit-stop. All of it is cosmetic
//! client state in `Update`, driven by the sim's [`SimEvent`]s; nothing here
//! reaches the sim.
//!
//! **Events under rollback.** The sim only keeps the events of its last
//! step, so whoever steps it (`rollback::step_sim`, `flow::step_demo_battle`)
//! hands each step's events to [`BattleEvents::record`] straight away.
//! A re-simulated frame reports the same `(frame, SimEvent::key)` again,
//! which `record` drops, so a rollback never flashes, bursts or plays a
//! sound twice. (A mispredicted event that the corrected frame doesn't
//! repeat has already shown; that is cosmetic and accepted.) Consumers read
//! [`BattleEvents::fresh`] during `Update`; it is emptied in `Last`.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy::sprite::Text2dShadow;
use bevy_ggrs::LocalPlayers;
use sim::tuning::{BLOATER_BLAST_RADIUS, GUARDIAN_RADIUS, SHOCKWAVE_RADIUS};
use sim::{Attack, EnemyKind, FxVec2, SimEvent, SimState, Skill, SUB};

use crate::art::ContentImages;
use crate::rollback::SimWorld;
use crate::settings::SettingsFile;

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BattleEvents>()
            .init_resource::<EnemyFlash>()
            .init_resource::<ScreenShake>()
            .init_resource::<HitStop>()
            .init_resource::<FxLayouts>()
            .init_resource::<ScreenFlash>()
            .add_systems(Startup, spawn_flash_overlay)
            .add_systems(
                Update,
                (
                    spawn_feedback,
                    animate_fx,
                    float_numbers,
                    decay_shake,
                    show_flash,
                    clear_when_battle_ends,
                )
                    .chain(),
            )
            .add_systems(Last, |mut events: ResMut<BattleEvents>| {
                events.fresh.clear()
            });
    }
}

/// How many frames back a seen event key is remembered. Rollback (GGRS
/// prediction, SyncTest's check distance) re-simulates far fewer.
const SEEN_FRAMES: u32 = 240;

/// The sim's events, each once, in the order they happened.
#[derive(Resource, Default)]
pub struct BattleEvents {
    seen: HashSet<(u32, (u8, u32, u32))>,
    fresh: Vec<(u32, SimEvent)>,
}

impl BattleEvents {
    /// Takes the events of the step that just ran (`world.frame` is then
    /// the step's number + 1), skipping any already recorded for that frame.
    pub fn record(&mut self, world: &SimState) {
        let frame = world.frame;
        for ev in &world.events {
            if self.seen.insert((frame, ev.key())) {
                self.fresh.push((frame, *ev));
            }
        }
        if self.seen.len() > 4096 {
            self.seen
                .retain(|(f, _)| f.saturating_add(SEEN_FRAMES) > frame);
        }
    }

    /// Events recorded since the last `Last` schedule, oldest first.
    pub fn fresh(&self) -> impl Iterator<Item = &SimEvent> {
        self.fresh.iter().map(|(_, e)| e)
    }

    /// A new battle: frame numbers and entity IDs start over.
    pub fn reset(&mut self) {
        self.seen.clear();
        self.fresh.clear();
    }
}

/// The handle whose ship the camera follows (first local player).
pub fn local_handle(local: Option<&LocalPlayers>) -> usize {
    local.and_then(|l| l.0.first().copied()).unwrap_or(0)
}

/// Sim position -> world units (1 unit = 1 px).
fn to_world(p: FxVec2) -> Vec2 {
    Vec2::new(p.x as f32 / SUB as f32, p.y as f32 / SUB as f32)
}

// ---- Hit flash -----------------------------------------------------------

/// How long an enemy stays white after a hit.
pub const FLASH_SECS: f32 = 0.09;

/// When each enemy (by stable ID) was last hit; `render` tints it white.
#[derive(Resource, Default)]
pub struct EnemyFlash(HashMap<u32, f32>);

impl EnemyFlash {
    /// 1 right after a hit, fading to 0 over `FLASH_SECS`.
    pub fn amount(&self, enemy: u32, now: f32) -> f32 {
        self.0
            .get(&enemy)
            .map_or(0.0, |t| (1.0 - (now - t) / FLASH_SECS).clamp(0.0, 1.0))
    }
}

// ---- Animated effects ----------------------------------------------------

/// One of Art's one-shot strips (`design/art/battle/feedback/`): content
/// ID, frame count, ms per frame, longest side in world px.
pub struct FxArt {
    pub id: &'static str,
    pub frames: u32,
    pub frame_ms: u32,
    pub size: f32,
    /// Frame size in the image (px, square): 256, or 512 for the Guardian.
    pub cell: u32,
}

pub const HIT_ENEMY: FxArt = FxArt {
    id: "core.battle.fx.hit_enemy",
    frames: 4,
    frame_ms: 35,
    size: 40.0,
    cell: FX_CELL,
};
pub const HIT_SHIP: FxArt = FxArt {
    id: "core.battle.fx.hit_ship",
    frames: 4,
    frame_ms: 35,
    size: 48.0,
    cell: FX_CELL,
};
pub const DEATH_SWARMER: FxArt = FxArt {
    id: "core.battle.fx.death_swarmer",
    frames: 6,
    frame_ms: 45,
    size: 80.0,
    cell: FX_CELL,
};
pub const DEATH_SPITTER: FxArt = FxArt {
    id: "core.battle.fx.death_spitter",
    frames: 6,
    frame_ms: 45,
    size: 112.0,
    cell: FX_CELL,
};
/// Centred on the Bulwark, sized to the Shockwave's reach.
pub const SHOCKWAVE_IMPACT: FxArt = FxArt {
    id: "core.battle.fx.shockwave_impact",
    frames: 4,
    frame_ms: 45,
    size: 2.0 * SHOCKWAVE_RADIUS as f32,
    cell: FX_CELL,
};
pub const SCATTER_IMPACT: FxArt = FxArt {
    id: "core.battle.fx.scatter_impact",
    frames: 4,
    frame_ms: 35,
    size: 48.0,
    cell: FX_CELL,
};

/// The TAKOAI-114 role death bursts and the Bloater's explosion are single
/// key poses, not strips: one 256 px frame that is shown and faded out.
const fn key_pose(id: &'static str, frame_ms: u32, size: f32) -> FxArt {
    FxArt {
        id,
        frames: 1,
        frame_ms,
        size,
        cell: FX_CELL,
    }
}
pub const DEATH_LANCER: FxArt = key_pose("core.battle.fx.death_lancer", 320, 96.0);
pub const DEATH_BLOATER: FxArt = key_pose("core.battle.fx.death_bloater", 320, 104.0);
pub const DEATH_BROOD: FxArt = key_pose("core.battle.fx.death_brood", 360, 128.0);
pub const DEATH_WARDEN: FxArt = key_pose("core.battle.fx.death_warden", 320, 112.0);
/// Sized to the burst's real reach.
pub const BLOATER_EXPLOSION: FxArt = key_pose(
    "core.battle.fx.bloater_explosion",
    260,
    2.0 * BLOATER_BLAST_RADIUS as f32,
);

/// The Fissure Guardian's death (TAKOAI-118): Art's 512 px key pose, held
/// longer and drawn big.
pub const DEATH_GUARDIAN: FxArt = FxArt {
    cell: 512,
    ..key_pose("core.battle.fx.death_guardian", 900, 420.0)
};
/// The Guardian's shot ring leaving: its flash, popped once at full size.
pub const GUARDIAN_RING_BURST: FxArt = key_pose(
    "core.battle.fx.guardian_ring_flash",
    220,
    GUARDIAN_RING_FLASH_SIZE,
);
/// The ring flash's full size (px): about the Guardian's body.
pub const GUARDIAN_RING_FLASH_SIZE: f32 = 3.0 * GUARDIAN_RADIUS as f32;

/// Every strip, for the transition screen's preload.
pub const FX: [&FxArt; 13] = [
    &HIT_ENEMY,
    &HIT_SHIP,
    &DEATH_SWARMER,
    &DEATH_SPITTER,
    &SHOCKWAVE_IMPACT,
    &SCATTER_IMPACT,
    &DEATH_LANCER,
    &DEATH_BLOATER,
    &DEATH_BROOD,
    &DEATH_WARDEN,
    &BLOATER_EXPLOSION,
    &DEATH_GUARDIAN,
    &GUARDIAN_RING_BURST,
];

/// The frame size of every strip (README-v1: 256 x 256 cells, one row).
const FX_CELL: u32 = 256;

pub fn death_burst(kind: EnemyKind) -> &'static FxArt {
    match kind {
        EnemyKind::Swarmer => &DEATH_SWARMER,
        EnemyKind::Spitter => &DEATH_SPITTER,
        EnemyKind::Lancer => &DEATH_LANCER,
        EnemyKind::Bloater => &DEATH_BLOATER,
        EnemyKind::Brood => &DEATH_BROOD,
        EnemyKind::Warden => &DEATH_WARDEN,
        EnemyKind::Guardian => &DEATH_GUARDIAN,
    }
}

/// Atlas layouts by frame count and cell size.
#[derive(Resource, Default)]
struct FxLayouts(HashMap<(u32, u32), Handle<TextureAtlasLayout>>);

/// A playing one-shot strip; despawned after its last frame, which fades.
#[derive(Component)]
struct FxAnim {
    started: f32,
    frame_secs: f32,
    frames: u32,
}

/// Anything spawned here, despawned when the battle ends.
#[derive(Component)]
struct BattleFx;

// ---- Damage numbers -----------------------------------------------------

/// Damage number styles (Art's TAKOAI-111 colours and world sizes).
const NUMBER_NORMAL: (Color, f32) = (Color::srgb_u8(0xFF, 0xF3, 0xCC), 18.0);
const NUMBER_BIG: (Color, f32) = (Color::srgb_u8(0xFF, 0xD1, 0x5C), 25.0);
const NUMBER_SHIP: (Color, f32) = (Color::srgb_u8(0xFF, 0x78, 0x9E), 20.0);
/// Damage the shield took instead of the hull.
const NUMBER_SHIELD: (Color, f32) = (Color::srgb_u8(0x8C, 0xCB, 0xFF), 16.0);
const NUMBER_OUTLINE: Color = Color::srgb_u8(0x24, 0x10, 0x32);
/// A hit this big (or any Shockwave hit) uses the big style.
const BIG_HIT: i32 = 10;
const NUMBER_SECS: f32 = 0.7;
const NUMBER_RISE: f32 = 32.0;
/// Most damage numbers on screen; the oldest go first.
const MAX_NUMBERS: usize = 60;

#[derive(Component)]
struct DamageNumber {
    started: f32,
    from: Vec2,
}

/// What a damage number says and how it looks: `None` for no number.
pub fn damage_number(ev: &SimEvent, me: usize) -> Option<(String, Color, f32)> {
    match *ev {
        SimEvent::Hit { amount, attack, .. } => {
            let big = matches!(attack, Attack::Shockwave | Attack::Blast);
            let (c, s) = if big || amount >= BIG_HIT {
                NUMBER_BIG
            } else {
                NUMBER_NORMAL
            };
            Some((amount.to_string(), c, s))
        }
        SimEvent::ShipDamaged {
            ship,
            amount,
            hull_lost,
            ..
        } if ship == me => Some(if hull_lost > 0 {
            (format!("-{hull_lost}"), NUMBER_SHIP.0, NUMBER_SHIP.1)
        } else {
            (amount.to_string(), NUMBER_SHIELD.0, NUMBER_SHIELD.1)
        }),
        _ => None,
    }
}

// ---- Screen shake and hit-stop -------------------------------------------

/// Camera shake "trauma", 0..=1; the offset grows with its square.
#[derive(Resource, Default)]
pub struct ScreenShake {
    pub trauma: f32,
}

/// Largest shake offset in screen px (at trauma 1).
const SHAKE_PX: f32 = 10.0;
const SHAKE_DECAY_PER_SEC: f32 = 1.8;

impl ScreenShake {
    pub fn add(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }

    /// The camera offset at time `t`, in screen px.
    pub fn offset(&self, t: f32) -> Vec2 {
        let k = self.trauma * self.trauma * SHAKE_PX;
        // Visual only, so platform float functions are fine here.
        Vec2::new((t * 53.0).sin(), (t * 41.0 + 1.3).sin()) * k
    }
}

/// How much shake an event gives our ship's camera.
pub fn shake_for(ev: &SimEvent, me: usize) -> f32 {
    match *ev {
        SimEvent::ShipDamaged {
            ship, hull_lost, ..
        } if ship == me => 0.25 + hull_lost as f32 / 40.0,
        SimEvent::SkillUsed { ship, skill, .. } if ship == me => match skill {
            Skill::Shockwave => 0.45,
            Skill::Scatter => 0.15,
            Skill::Afterburn | Skill::Bastion => 0.0,
        },
        // A burst on screen rattles the camera a little even when it misses.
        SimEvent::Burst { .. } => 0.2,
        SimEvent::Killed {
            kind: EnemyKind::Guardian,
            ..
        } => 1.0,
        SimEvent::Killed { elite: true, .. } => 0.4,
        SimEvent::GuardianEnraged { .. } => 0.8,
        SimEvent::GuardianEmerged { .. } => 0.5,
        SimEvent::GuardianRing { .. } => 0.12,
        _ => 0.0,
    }
}

/// A render-only freeze: the battle sprites hold still until `until`
/// (`Time` seconds) while the sim runs on. The sim can't pause for one
/// player in a shared mission (design/BATTLE.md § Engineering notes).
#[derive(Resource, Default)]
pub struct HitStop {
    pub until: f32,
}

impl HitStop {
    pub fn active(&self, now: f32) -> bool {
        now < self.until
    }
}

/// Seconds of hit-stop a kill gives: elite kills (TAKOAI-117) get a short
/// freeze, the Fissure Guardian (TAKOAI-118) a long one; ordinary enemies
/// none.
pub fn hit_stop_secs(kind: EnemyKind, elite: bool) -> Option<f32> {
    match kind {
        EnemyKind::Guardian => Some(GUARDIAN_HIT_STOP),
        _ if elite => Some(ELITE_HIT_STOP),
        EnemyKind::Swarmer
        | EnemyKind::Spitter
        | EnemyKind::Lancer
        | EnemyKind::Bloater
        | EnemyKind::Brood
        | EnemyKind::Warden => None,
    }
}

/// An elite kill: freeze this long, and draw its death burst this many
/// times bigger.
pub const ELITE_HIT_STOP: f32 = 0.12;
pub const ELITE_BURST_SCALE: f32 = 2.2;
/// The Guardian's kill: a long freeze.
pub const GUARDIAN_HIT_STOP: f32 = 0.4;

// ---- Screen flash ------------------------------------------------------------

/// A full-screen flash over the battle (the Guardian's phase change and
/// death), fading out over `secs` from `at` (`Time` seconds).
#[derive(Resource, Default)]
pub struct ScreenFlash {
    pub at: f32,
    pub secs: f32,
    pub strength: f32,
}

impl ScreenFlash {
    pub fn start(&mut self, now: f32, secs: f32, strength: f32) {
        *self = Self {
            at: now,
            secs,
            strength,
        };
    }

    /// The overlay's alpha at `now`.
    pub fn alpha(&self, now: f32) -> f32 {
        if self.secs <= 0.0 {
            return 0.0;
        }
        let t = (now - self.at) / self.secs;
        if !(0.0..1.0).contains(&t) {
            return 0.0;
        }
        self.strength * (1.0 - t) * (1.0 - t)
    }
}

/// The screen flash for an event: (seconds, peak alpha).
pub fn flash_for(ev: &SimEvent) -> Option<(f32, f32)> {
    match *ev {
        SimEvent::GuardianEnraged { .. } => Some((0.45, 0.7)),
        SimEvent::Killed {
            kind: EnemyKind::Guardian,
            ..
        } => Some((0.8, 0.9)),
        _ => None,
    }
}

#[derive(Component)]
struct FlashOverlay;

fn spawn_flash_overlay(mut commands: Commands) {
    commands.spawn((
        FlashOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(Color::NONE),
        GlobalZIndex(30),
    ));
}

fn show_flash(
    time: Res<Time>,
    flash: Res<ScreenFlash>,
    mut overlay: Query<&mut BackgroundColor, With<FlashOverlay>>,
) {
    let a = flash.alpha(time.elapsed_secs());
    for mut bg in &mut overlay {
        let c = Color::srgba(1.0, 0.85, 1.0, a);
        if bg.0 != c {
            bg.0 = c;
        }
    }
}

// ---- Systems ---------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn spawn_feedback(
    mut commands: Commands,
    events: Res<BattleEvents>,
    time: Res<Time>,
    local: Option<Res<LocalPlayers>>,
    settings: Option<Res<SettingsFile>>,
    mut flash: ResMut<EnemyFlash>,
    mut shake: ResMut<ScreenShake>,
    mut hit_stop: ResMut<HitStop>,
    mut screen_flash: ResMut<ScreenFlash>,
    mut layouts: ResMut<FxLayouts>,
    mut atlas: ResMut<Assets<TextureAtlasLayout>>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    numbers: Query<(Entity, &DamageNumber)>,
) {
    let now = time.elapsed_secs();
    flash.0.retain(|_, t| now - *t < FLASH_SECS);
    let battle = settings
        .map(|s| s.settings.battle.clone())
        .unwrap_or_default();
    let me = local_handle(local.as_deref());
    let mut bursts: Vec<(&FxArt, Vec2, f32)> = Vec::new();
    let font = art.font(&mut fonts, crate::art::ids::FONT_DISPLAY);
    let mut new_numbers = 0;
    for ev in events.fresh() {
        match *ev {
            SimEvent::Hit {
                enemy, attack, pos, ..
            } => {
                flash.0.insert(enemy, now);
                match attack {
                    Attack::Scatter => bursts.push((&SCATTER_IMPACT, to_world(pos), 1.0)),
                    _ => bursts.push((&HIT_ENEMY, to_world(pos), 1.0)),
                }
            }
            SimEvent::Killed {
                kind, elite, pos, ..
            } => {
                let scale = if elite { ELITE_BURST_SCALE } else { 1.0 };
                bursts.push((death_burst(kind), to_world(pos), scale));
                if let Some(secs) = hit_stop_secs(kind, elite) {
                    hit_stop.until = hit_stop.until.max(now + secs);
                }
            }
            SimEvent::GuardianEnraged { enemy, .. } => {
                flash.0.insert(enemy, now);
            }
            // Drawn back into the fissures: a small teleport-like puff.
            SimEvent::Recalled { pos, .. } => bursts.push((&HIT_ENEMY, to_world(pos), 1.5)),
            SimEvent::GuardianRing { pos, .. } => {
                bursts.push((&GUARDIAN_RING_BURST, to_world(pos), 1.0))
            }
            SimEvent::ShipDamaged { pos, .. } => bursts.push((&HIT_SHIP, to_world(pos), 1.0)),
            SimEvent::Burst { pos, .. } => bursts.push((&BLOATER_EXPLOSION, to_world(pos), 1.0)),
            SimEvent::SkillUsed {
                skill: Skill::Shockwave,
                pos,
                ..
            } => bursts.push((&SHOCKWAVE_IMPACT, to_world(pos), 1.0)),
            _ => {}
        }
        if battle.screen_shake {
            shake.add(shake_for(ev, me));
        }
        if let Some((secs, strength)) = flash_for(ev) {
            screen_flash.start(now, secs, strength);
        }
        if !battle.damage_numbers {
            continue;
        }
        let (Some((text, color, size)), Some(pos)) = (damage_number(ev, me), number_pos(ev)) else {
            continue;
        };
        // Spread numbers that land on the same spot a little.
        let (a, b, _) = ev.key();
        let jitter = ((a as u32 * 7 + b.wrapping_mul(37)) % 21) as f32 - 10.0;
        let from = pos + Vec2::new(jitter, 12.0);
        commands.spawn((
            BattleFx,
            DamageNumber { started: now, from },
            Text2d::new(text),
            TextFont {
                font: font.clone().map(Into::into).unwrap_or_default(),
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(color),
            Text2dShadow {
                offset: Vec2::new(1.5, -1.5),
                color: NUMBER_OUTLINE,
            },
            Transform::from_translation(from.extend(0.9)),
        ));
        new_numbers += 1;
    }
    for (fx, at, scale) in bursts {
        let Some(image) = art.get(&mut images, fx.id) else {
            continue;
        };
        let layout = layouts
            .0
            .entry((fx.frames, fx.cell))
            .or_insert_with(|| {
                atlas.add(TextureAtlasLayout::from_grid(
                    UVec2::splat(fx.cell),
                    fx.frames,
                    1,
                    None,
                    None,
                ))
            })
            .clone();
        commands.spawn((
            BattleFx,
            FxAnim {
                started: now,
                frame_secs: fx.frame_ms as f32 / 1000.0,
                frames: fx.frames,
            },
            Sprite {
                custom_size: Some(Vec2::splat(fx.size * scale)),
                ..Sprite::from_atlas_image(image, TextureAtlas { layout, index: 0 })
            },
            Transform::from_translation(at.extend(0.6)),
        ));
    }
    // Keep the screen readable: drop the oldest numbers past the cap.
    let live = numbers.iter().count() + new_numbers;
    if live > MAX_NUMBERS {
        let mut old: Vec<_> = numbers.iter().collect();
        old.sort_by(|a, b| a.1.started.total_cmp(&b.1.started));
        for (entity, _) in old.into_iter().take(live - MAX_NUMBERS) {
            commands.entity(entity).despawn();
        }
    }
}

/// Where an event's damage number starts.
fn number_pos(ev: &SimEvent) -> Option<Vec2> {
    match *ev {
        SimEvent::Hit { pos, .. } | SimEvent::ShipDamaged { pos, .. } => Some(to_world(pos)),
        _ => None,
    }
}

fn animate_fx(
    mut commands: Commands,
    time: Res<Time>,
    hit_stop: Res<HitStop>,
    mut anims: Query<(Entity, &FxAnim, &mut Sprite)>,
) {
    let now = time.elapsed_secs();
    if hit_stop.active(now) {
        return;
    }
    for (entity, anim, mut sprite) in &mut anims {
        let t = (now - anim.started) / anim.frame_secs;
        let frame = t as u32;
        if frame >= anim.frames {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(atlas) = &mut sprite.texture_atlas {
            if atlas.index != frame as usize {
                atlas.index = frame as usize;
            }
        }
        // Fade the last frame out rather than holding it (README-v1).
        let alpha = if frame + 1 == anim.frames {
            1.0 - t.fract()
        } else {
            1.0
        };
        sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
    }
}

fn float_numbers(
    mut commands: Commands,
    time: Res<Time>,
    mut numbers: Query<(Entity, &DamageNumber, &mut Transform, &mut TextColor)>,
) {
    let now = time.elapsed_secs();
    for (entity, n, mut tf, mut color) in &mut numbers {
        let t = (now - n.started) / NUMBER_SECS;
        if t >= 1.0 {
            commands.entity(entity).despawn();
            continue;
        }
        // Fast pop up, then drift; fade over the last 40 %.
        let rise = 1.0 - (1.0 - t) * (1.0 - t);
        tf.translation = (n.from + Vec2::Y * NUMBER_RISE * rise).extend(0.9);
        let alpha = ((1.0 - t) / 0.4).min(1.0);
        color.0.set_alpha(alpha);
    }
}

fn decay_shake(
    time: Res<Time>,
    settings: Option<Res<SettingsFile>>,
    mut shake: ResMut<ScreenShake>,
) {
    let on = settings.is_none_or(|s| s.settings.battle.screen_shake);
    shake.trauma = if on {
        (shake.trauma - SHAKE_DECAY_PER_SEC * time.delta_secs()).max(0.0)
    } else {
        0.0
    };
}

/// No battle world: drop every effect and the shake.
fn clear_when_battle_ends(
    mut commands: Commands,
    world: Option<Res<SimWorld>>,
    fx: Query<Entity, With<BattleFx>>,
    mut shake: ResMut<ScreenShake>,
    mut flash: ResMut<EnemyFlash>,
    mut hit_stop: ResMut<HitStop>,
    mut screen_flash: ResMut<ScreenFlash>,
) {
    if world.is_some() {
        return;
    }
    *screen_flash = ScreenFlash::default();
    for entity in &fx {
        commands.entity(entity).despawn();
    }
    shake.trauma = 0.0;
    flash.0.clear();
    hit_stop.until = 0.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::input::{NetInput, INPUT_SKILL_W};
    use sim::{Loadout, ShipKind};

    fn bulwark_battle() -> SimState {
        SimState::with_loadouts(
            3,
            &[Loadout {
                ship: ShipKind::Bulwark,
                ..Default::default()
            }],
        )
    }

    #[test]
    fn a_replayed_frame_records_nothing_new() {
        let mut world = bulwark_battle();
        world.mission.spawn_timer = u32::MAX;
        world
            .enemies
            .push(sim::Enemy::new(EnemyKind::Swarmer, FxVec2::from_px(100, 0)));
        let before = world.clone();
        let w = NetInput {
            buttons: INPUT_SKILL_W,
            ..Default::default()
        };
        let mut events = BattleEvents::default();
        world.step(&[w]);
        events.record(&world);
        let first: Vec<_> = events.fresh().copied().collect();
        assert!(
            first
                .iter()
                .any(|e| matches!(e, SimEvent::SkillUsed { .. })),
            "{first:?}"
        );
        events.fresh.clear();
        // Rollback: load the old state and simulate the same frame again.
        let mut world = before;
        world.step(&[w]);
        events.record(&world);
        assert_eq!(events.fresh().count(), 0);
        // The next frame is new.
        world.events = vec![SimEvent::MissionEnded { success: true }];
        world.frame += 1;
        events.record(&world);
        assert_eq!(events.fresh().count(), 1);
        // A new battle starts over.
        events.reset();
        let mut world = bulwark_battle();
        world.step(&[w]);
        events.record(&world);
        assert_eq!(
            events.fresh().count(),
            first
                .iter()
                .filter(|e| matches!(e, SimEvent::SkillUsed { .. }))
                .count()
        );
    }

    #[test]
    fn damage_numbers_style_hits_and_our_own_damage() {
        let hit = |attack, amount| SimEvent::Hit {
            enemy: 1,
            ship: 0,
            attack,
            source: 5,
            amount,
            pos: FxVec2::ZERO,
        };
        let (text, color, _) = damage_number(&hit(Attack::Bolt, 4), 0).unwrap();
        assert_eq!((text.as_str(), color), ("4", NUMBER_NORMAL.0));
        assert_eq!(
            damage_number(&hit(Attack::Bolt, 12), 0).unwrap().1,
            NUMBER_BIG.0
        );
        assert_eq!(
            damage_number(&hit(Attack::Shockwave, 6), 0).unwrap().1,
            NUMBER_BIG.0
        );
        let damaged = |ship, hull_lost| SimEvent::ShipDamaged {
            ship,
            source: 9,
            amount: 8,
            hull_lost,
            pos: FxVec2::ZERO,
        };
        let (text, color, _) = damage_number(&damaged(0, 8), 0).unwrap();
        assert_eq!((text.as_str(), color), ("-8", NUMBER_SHIP.0));
        // All absorbed by the shield: the amount, in the shield colour.
        assert_eq!(damage_number(&damaged(0, 0), 0).unwrap().1, NUMBER_SHIELD.0);
        // Another player's ship: not on our screen as a number.
        assert!(damage_number(&damaged(1, 8), 0).is_none());
    }

    #[test]
    fn only_our_ship_shakes_the_camera() {
        let damaged = |ship| SimEvent::ShipDamaged {
            ship,
            source: 9,
            amount: 8,
            hull_lost: 8,
            pos: FxVec2::ZERO,
        };
        assert!(shake_for(&damaged(0), 0) > 0.0);
        assert_eq!(shake_for(&damaged(1), 0), 0.0);
        let skill = |skill| SimEvent::SkillUsed {
            ship: 0,
            skill,
            pos: FxVec2::ZERO,
        };
        assert!(shake_for(&skill(Skill::Shockwave), 0) > shake_for(&skill(Skill::Scatter), 0));
        assert_eq!(shake_for(&skill(Skill::Bastion), 0), 0.0);
        let mut shake = ScreenShake::default();
        assert_eq!(shake.offset(1.0), Vec2::ZERO);
        shake.add(5.0);
        assert_eq!(shake.trauma, 1.0);
        assert!(shake.offset(1.0).length() <= SHAKE_PX * 1.5);
    }

    #[test]
    fn hit_stop_is_for_elite_and_guardian_kills_only() {
        for kind in EnemyKind::ALL {
            if kind == EnemyKind::Guardian {
                assert_eq!(hit_stop_secs(kind, false), Some(GUARDIAN_HIT_STOP));
                continue;
            }
            assert_eq!(hit_stop_secs(kind, false), None);
            assert_eq!(hit_stop_secs(kind, true), Some(ELITE_HIT_STOP));
        }
        const { assert!(GUARDIAN_HIT_STOP > ELITE_HIT_STOP) };
        let stop = HitStop { until: 2.0 };
        assert!(stop.active(1.9) && !stop.active(2.0));
    }

    #[test]
    fn every_effect_is_shipped_as_a_strip_of_its_frames() {
        let mut art = ContentImages::repo();
        let mut images = Assets::<Image>::default();
        for fx in FX {
            let handle = art.get(&mut images, fx.id).expect(fx.id);
            let size = images.get(&handle).unwrap().size();
            assert_eq!(size, UVec2::new(fx.cell * fx.frames, fx.cell), "{}", fx.id);
        }
    }

    #[test]
    fn the_guardian_flashes_the_screen_when_it_enrages_and_dies() {
        let pos = FxVec2::ZERO;
        assert!(flash_for(&SimEvent::GuardianEnraged { enemy: 1, pos }).is_some());
        let killed = |kind| SimEvent::Killed {
            enemy: 1,
            kind,
            elite: false,
            pos,
        };
        assert!(flash_for(&killed(EnemyKind::Guardian)).is_some());
        assert!(flash_for(&killed(EnemyKind::Spitter)).is_none());
        assert_eq!(shake_for(&killed(EnemyKind::Guardian), 0), 1.0);
        let mut flash = ScreenFlash::default();
        assert_eq!(flash.alpha(1.0), 0.0);
        flash.start(1.0, 0.5, 0.8);
        assert_eq!(flash.alpha(1.0), 0.8);
        assert!(flash.alpha(1.25) < 0.8 && flash.alpha(1.25) > 0.0);
        assert_eq!(flash.alpha(1.5), 0.0);
    }

    #[test]
    fn flash_fades_out() {
        let mut flash = EnemyFlash::default();
        flash.0.insert(3, 1.0);
        assert_eq!(flash.amount(3, 1.0), 1.0);
        assert!(flash.amount(3, 1.0 + FLASH_SECS / 2.0) > 0.0);
        assert_eq!(flash.amount(3, 1.0 + FLASH_SECS), 0.0);
        assert_eq!(flash.amount(4, 1.0), 0.0);
    }
}
