//! Battle visuals, camera and HUD. Runs in `Update`, outside the rollback
//! schedule, and only reads `SimWorld`.
//!
//! Ships, enemies, fissures, loot and enemy shots are sprites of the shipped
//! art, looked up by stable content ID (`ids`) through the manifest, over
//! the procedural nebula sky (`crate::sky`). Player bolts use a generated
//! glow tinted per player; shields, the shockwave, hull bars, the move
//! marker and the off-screen fissure pointers are gizmo effects.
//!
//! Elite leaders (TAKOAI-117) are drawn bigger (their sim radius), ringed by
//! Art's aura, with their name, affix label and icon above them; their
//! affixes' telegraphs (Warding shimmer, Blinking mark, Scorching patches,
//! a chilled ship's tint) and the light beam on elite loot all read sim state.
//!
//! The Fissure Guardian (TAKOAI-118) is drawn from Art's phase 1 / phase 2
//! poses; its warnings (the ring flash with the coming shots' slots and
//! gaps, the charge line, the core throbbing before a summon) read its
//! wind-up. A boss bar with its name sits bottom centre while it is out, the
//! banner announces it, its phase change and its death, and the fissures
//! dim once they have gone quiet.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_ggrs::{LocalPlayers, Session};
use sim::state::warden_of;
use sim::tuning::{
    BLOATER_BLAST_RADIUS, BLOATER_FUSE, BROOD_WINDUP, FISSURE_RADIUS, GUARDIAN_CHARGE_DISTANCE,
    GUARDIAN_CHARGE_WINDUP, GUARDIAN_SHOT_RADIUS, GUARDIAN_SUMMON_WINDUP, LANCER_DASH_DISTANCE,
    LANCER_WINDUP, SHOCKWAVE_RADIUS, SPITTER_FIRE_RANGE, SPITTER_WINDUP, TICKS_PER_SEC,
};
use sim::{
    elite_name, Affix, Boss, BossAttack, Enemy, EnemyKind, FxVec2, LootKind, MissionStage,
    MissionStatus, Ship, ShipKind, SimState, SUB,
};

use crate::art::ContentImages;
use crate::flow::PauseMenu;
use crate::fx::{EnemyFlash, HitStop, ScreenShake};
use crate::rollback::{GameConfig, SimWorld};
use crate::stats::Stats;
use crate::zoom::{self, Zoom};
use crate::{ContentStatus, NetStatus};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .insert_resource(ClearColor(Color::BLACK))
            .init_resource::<CameraFollow>()
            .init_resource::<ContentImages>()
            .init_resource::<BattleSprites>()
            .add_plugins(crate::sky::SkyPlugin)
            .add_systems(Startup, setup_scene)
            .add_systems(
                Update,
                (
                    draw_world,
                    draw_sprites,
                    draw_fissure_pointers,
                    follow_camera,
                    update_hud,
                    update_banner,
                    update_boss_bar,
                    elite_labels,
                ),
            );
    }
}

#[derive(Component)]
struct Hud;

#[derive(Component)]
struct Banner;

/// The Guardian's boss bar (bottom centre), its name and its fill.
#[derive(Component)]
struct BossBar;
#[derive(Component)]
struct BossName;
#[derive(Component)]
struct BossFill;

/// Stable content IDs of the battle sprites (`assets/manifest.json`).
pub mod ids {
    pub const SHIP_KITE: &str = "core.battle.ship.kite";
    pub const SHIP_BULWARK: &str = "core.battle.ship.bulwark";
    pub const ENEMY_SWARMER: &str = "core.battle.enemy.void_swarmer";
    pub const ENEMY_SPITTER: &str = "core.battle.enemy.void_spitter";
    pub const ENEMY_LANCER: &str = "core.battle.enemy.void_lancer";
    pub const ENEMY_LANCER_WINDUP: &str = "core.battle.pose.void_lancer_windup";
    pub const ENEMY_LANCER_DASH: &str = "core.battle.pose.void_lancer_dash";
    pub const ENEMY_BLOATER: &str = "core.battle.enemy.void_bloater";
    pub const ENEMY_BLOATER_SWELLING: &str = "core.battle.pose.void_bloater_swelling";
    pub const ENEMY_BROOD: &str = "core.battle.enemy.void_brood";
    pub const ENEMY_BROOD_WINDUP: &str = "core.battle.pose.void_brood_windup";
    pub const ENEMY_WARDEN: &str = "core.battle.enemy.void_warden";
    pub const ENEMY_GUARDIAN: &str = "core.battle.enemy.fissure_guardian";
    pub const ENEMY_GUARDIAN_PHASE2: &str = "core.battle.pose.fissure_guardian_phase2";
    pub const FX_GUARDIAN_RING_FLASH: &str = "core.battle.fx.guardian_ring_flash";
    pub const FX_GUARDIAN_CHARGE_LINE: &str = "core.battle.fx.guardian_charge_line";
    pub const FX_LANCER_CHARGE_LINE: &str = "core.battle.fx.lancer_charge_line";
    pub const FX_BLOATER_WARNING_RING: &str = "core.battle.fx.bloater_warning_ring";
    pub const FX_WARDEN_TETHER: &str = "core.battle.fx.warden_tether";
    pub const FX_WARDEN_SHIELD: &str = "core.battle.fx.warden_shield";
    pub const ELITE_AURA: &str = "core.battle.elite.aura";
    pub const ELITE_BURNING_PATCH: &str = "core.battle.elite.burning_patch";
    pub const ELITE_TELEPORT_MARK: &str = "core.battle.elite.teleport_mark";
    pub const AFFIX_SWIFT: &str = "core.battle.elite.affix_swift";
    pub const AFFIX_VOLATILE: &str = "core.battle.elite.affix_volatile";
    pub const AFFIX_WARDING: &str = "core.battle.elite.affix_warding";
    pub const AFFIX_BLINKING: &str = "core.battle.elite.affix_blinking";
    pub const AFFIX_SPLITTING: &str = "core.battle.elite.affix_splitting";
    pub const AFFIX_SCORCHING: &str = "core.battle.elite.affix_scorching";
    pub const AFFIX_CHILLING: &str = "core.battle.elite.affix_chilling";
    pub const LOOT_CREDITS: &str = crate::art::ids::ICON_CREDITS;
    pub const LOOT_VOID_CRYSTAL: &str = crate::art::ids::ICON_VOID_CRYSTAL;
    pub const FX_SPIT: &str = "core.battle.fx.spit";
    pub const FX_SPITTER_WINDUP: &str = "core.battle.fx.spitter_windup";
    pub const FX_SPITTER_AIM_LINE: &str = "core.battle.fx.spitter_aim_line";
    pub const FISSURE: &str = "core.battle.env.void_fissure";
}

/// Default sprite size: the longest side is this many times the sim hit
/// circle's diameter, so what you see is close to what gets hit (TAKOAI-50).
const SPRITE_SCALE: f32 = 1.2;
/// The TAKOAI-114 role art sits on padded square canvases (horns, tails
/// and sacs reach the edges), so it is drawn a little larger to match.
const ROLE_SCALE: f32 = 1.6;

const fn role(id: &'static str) -> SpriteArt {
    SpriteArt {
        id,
        scale: ROLE_SCALE,
        spin: 0.0,
        pulse: 0.0,
    }
}

/// How big each sprite is drawn: its longest side is `scale` times the sim
/// hit circle's diameter.
struct SpriteArt {
    id: &'static str,
    scale: f32,
    /// Visual spin in radians/second.
    spin: f32,
    /// Visual pulse: the size swings by this fraction, about once per 2.5 s.
    pulse: f32,
}

const SPRITES: &[SpriteArt] = &[
    SpriteArt {
        id: ids::SHIP_KITE,
        scale: SPRITE_SCALE,
        spin: 0.0,
        pulse: 0.0,
    },
    SpriteArt {
        id: ids::SHIP_BULWARK,
        scale: SPRITE_SCALE,
        spin: 0.0,
        pulse: 0.0,
    },
    SpriteArt {
        id: ids::ENEMY_SWARMER,
        scale: SPRITE_SCALE,
        spin: 1.5,
        pulse: 0.0,
    },
    SpriteArt {
        id: ids::ENEMY_SPITTER,
        scale: SPRITE_SCALE,
        spin: 0.0,
        pulse: 0.0,
    },
    SpriteArt {
        id: ids::LOOT_CREDITS,
        scale: SPRITE_SCALE,
        spin: 0.0,
        pulse: 0.0,
    },
    SpriteArt {
        id: ids::LOOT_VOID_CRYSTAL,
        scale: SPRITE_SCALE,
        spin: 0.0,
        pulse: 0.0,
    },
    SpriteArt {
        id: ids::FX_SPIT,
        scale: SPRITE_SCALE,
        spin: 0.0,
        pulse: 0.0,
    },
    role(ids::ENEMY_LANCER),
    role(ids::ENEMY_LANCER_WINDUP),
    role(ids::ENEMY_LANCER_DASH),
    role(ids::ENEMY_BLOATER),
    role(ids::ENEMY_BLOATER_SWELLING),
    role(ids::ENEMY_BROOD),
    role(ids::ENEMY_BROOD_WINDUP),
    role(ids::ENEMY_WARDEN),
    role(ids::ENEMY_GUARDIAN),
    role(ids::ENEMY_GUARDIAN_PHASE2),
    // Fissures have no hit circle; drawn at about their spawn ring's size.
    // Static (no spin, TAKOAI-58), with a gentle pulse.
    SpriteArt {
        id: ids::FISSURE,
        scale: 1.0,
        spin: 0.0,
        pulse: 0.06,
    },
];

/// Every manifest image the battle draws (the transition screen loads
/// them all before the battle starts).
pub fn battle_image_ids() -> Vec<&'static str> {
    SPRITES
        .iter()
        .map(|s| s.id)
        .chain(crate::fx::FX.iter().map(|f| f.id))
        .chain([
            ids::FX_SPITTER_WINDUP,
            ids::FX_SPITTER_AIM_LINE,
            ids::FX_LANCER_CHARGE_LINE,
            ids::FX_BLOATER_WARNING_RING,
            ids::FX_WARDEN_TETHER,
            ids::FX_WARDEN_SHIELD,
            ids::ELITE_AURA,
            ids::ELITE_BURNING_PATCH,
            ids::ELITE_TELEPORT_MARK,
            ids::FX_GUARDIAN_RING_FLASH,
            ids::FX_GUARDIAN_CHARGE_LINE,
        ])
        .chain(Affix::ALL.map(affix_icon))
        .collect()
}

/// An affix's icon (Art's TAKOAI-114 elite sheet).
pub fn affix_icon(affix: Affix) -> &'static str {
    match affix {
        Affix::Swift => ids::AFFIX_SWIFT,
        Affix::Volatile => ids::AFFIX_VOLATILE,
        Affix::Warding => ids::AFFIX_WARDING,
        Affix::Blinking => ids::AFFIX_BLINKING,
        Affix::Splitting => ids::AFFIX_SPLITTING,
        Affix::Scorching => ids::AFFIX_SCORCHING,
        Affix::Chilling => ids::AFFIX_CHILLING,
    }
}

/// The text over an elite: its name, then its affixes.
pub fn elite_label(e: &Enemy) -> Option<(String, String)> {
    let el = e.elite?;
    let affixes: Vec<_> = Affix::in_mask(el.affixes).map(Affix::name).collect();
    Some((elite_name(el.name), affixes.join(" · ")))
}

/// The Spitter's wind-up telegraph (TAKOAI-113, Art's
/// `design/art/battle/feedback/README-v1.md`): a 4-frame glow over the
/// Spitter, one frame per 100 ms of the sim's 0.4 s wind-up, and an aim
/// line from it along the locked shot direction. Both are picked from the
/// sim's `Enemy::windup`, so they always match when the shot leaves.
const WINDUP_FRAMES: u32 = 4;
const WINDUP_SIZE: f32 = 96.0;
/// The aim line texture's stretchable inner region (x, y, w, h in px).
const AIM_LINE_UV: (f32, f32, f32, f32) = (16.0, 120.0, 224.0, 16.0);
const AIM_LINE_LENGTH: f32 = SPITTER_FIRE_RANGE as f32;
const AIM_LINE_THICKNESS: f32 = 6.0;
/// The strip's frame size (256 x 256 cells, one row).
const FX_CELL: f32 = 256.0;

/// Which wind-up frame to show with `windup` ticks left, and how far
/// through the wind-up it is (0..1).
pub fn windup_frame(windup: u32) -> (u32, f32) {
    let done = SPITTER_WINDUP.saturating_sub(windup);
    let frame = (done * WINDUP_FRAMES / SPITTER_WINDUP).min(WINDUP_FRAMES - 1);
    (frame, done as f32 / SPITTER_WINDUP as f32)
}

/// Loot is drawn as if its hit circle had this radius (pickups have none;
/// collection uses `PICKUP_COLLECT_RANGE`).
const LOOT_RADIUS: f32 = 5.0;
/// Player bolts are a soft glow whose bright core is about half its size, so
/// it is drawn larger than `SPRITE_SCALE` to read at ~1.2x the hit circle.
const BOLT_GLOW_SCALE: f32 = 2.0;

/// Battle camera zoom (orthographic scale): world px per screen px. 1.265 ≈
/// √1.6 shows 1.6x the field area of the old 1:1 view. HUD text is UI and
/// keeps its screen size.
pub const BATTLE_ZOOM: f32 = 1.265;

/// Per-player colour: bolts, hull-bar outline, move marker.
const PLAYER_COLORS: [Color; 4] = [
    Color::srgb(0.3, 0.8, 1.0),
    Color::srgb(1.0, 0.6, 0.2),
    Color::srgb(0.5, 1.0, 0.4),
    Color::srgb(1.0, 0.4, 0.9),
];
/// Off-screen fissure pointer, in the v2 fissure art's rim colours
/// (TAKOAI-64): bright violet-magenta with a teal inner accent.
const POINTER_COLOR: Color = Color::srgb(0.78, 0.36, 1.0);
const POINTER_ACCENT: Color = Color::srgb(0.25, 0.92, 0.88);

/// Fixed-point sim position -> Bevy world units (1 unit = 1 px). The only
/// place floats meet sim state, and it is one-way.
fn to_world(p: FxVec2) -> Vec2 {
    Vec2::new(p.x as f32 / SUB as f32, p.y as f32 / SUB as f32)
}

use crate::fx::local_handle;

fn ship_id(kind: ShipKind) -> &'static str {
    match kind {
        ShipKind::Kite => ids::SHIP_KITE,
        ShipKind::Bulwark => ids::SHIP_BULWARK,
    }
}

/// An enemy kind's resting sprite (codex, tests).
pub fn enemy_id(kind: EnemyKind) -> &'static str {
    match kind {
        EnemyKind::Swarmer => ids::ENEMY_SWARMER,
        EnemyKind::Spitter => ids::ENEMY_SPITTER,
        EnemyKind::Lancer => ids::ENEMY_LANCER,
        EnemyKind::Bloater => ids::ENEMY_BLOATER,
        EnemyKind::Brood => ids::ENEMY_BROOD,
        EnemyKind::Warden => ids::ENEMY_WARDEN,
        EnemyKind::Guardian => ids::ENEMY_GUARDIAN,
    }
}

/// The pose an enemy is drawn in right now: the Lancer crouches through its
/// wind-up and smears through its dash, the Bloater swells once armed, the
/// Brood's sac bulges before a release, the Guardian cracks open in phase 2.
pub fn enemy_pose(e: &Enemy) -> &'static str {
    match e.kind {
        EnemyKind::Guardian if e.boss.is_some_and(|b| b.phase >= 2) => ids::ENEMY_GUARDIAN_PHASE2,
        EnemyKind::Lancer if e.dash > 0 => ids::ENEMY_LANCER_DASH,
        EnemyKind::Lancer if e.windup > 0 => ids::ENEMY_LANCER_WINDUP,
        EnemyKind::Bloater if e.struck != 0 => ids::ENEMY_BLOATER_SWELLING,
        EnemyKind::Brood if e.windup > 0 => ids::ENEMY_BROOD_WINDUP,
        kind => enemy_id(kind),
    }
}

/// How far through a telegraph `left` ticks of `total` is (0..1).
pub fn telegraph_done(left: u32, total: u32) -> f32 {
    total.saturating_sub(left) as f32 / total.max(1) as f32
}

/// Bloater warning ring diameter (px) with `fuse` ticks left: it grows to
/// the burst's real size as the fuse runs out, so the edge at the end is
/// where the damage reaches.
pub fn warning_ring_size(fuse: u32) -> f32 {
    let d = 2.0 * BLOATER_BLAST_RADIUS as f32;
    d * (0.45 + 0.55 * telegraph_done(fuse, BLOATER_FUSE))
}

fn loot_id(kind: LootKind) -> &'static str {
    match kind {
        LootKind::Credits => ids::LOOT_CREDITS,
        LootKind::VoidCrystal => ids::LOOT_VOID_CRYSTAL,
    }
}

fn skill_names(kind: ShipKind) -> (&'static str, &'static str) {
    match kind {
        ShipKind::Kite => ("Afterburn", "Scatter"),
        ShipKind::Bulwark => ("Bastion", "Shockwave"),
    }
}

/// The battle HUD text's offset from the top-left corner.
const HUD_MARGIN: f32 = 10.0;

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera2d);
    spawn_boss_bar(&mut commands);
    commands.spawn((
        Hud,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgb(0.75, 1.0, 0.75)),
        Node {
            position_type: PositionType::Absolute,
            left: px(HUD_MARGIN),
            top: px(HUD_MARGIN),
            ..default()
        },
    ));
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            top: percent(30),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_child((
            Banner,
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(44.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
}

fn bar(gizmos: &mut Gizmos, c: Vec2, width: f32, frac: f32, color: Color) {
    let left = c - Vec2::new(width / 2.0, 0.0);
    gizmos.line_2d(
        left,
        left + Vec2::new(width, 0.0),
        Color::srgb(0.25, 0.25, 0.25),
    );
    gizmos.line_2d(
        left,
        left + Vec2::new(width * frac.clamp(0.0, 1.0), 0.0),
        color,
    );
}

/// Gizmo effects for one ship: shield, shockwave, dash trail, hull bar and
/// (our own ship) the move marker. The ship itself is a sprite.
fn draw_ship_fx(gizmos: &mut Gizmos, ship: &Ship, me: usize) {
    if !ship.alive() {
        return;
    }
    let color = PLAYER_COLORS[ship.handle % PLAYER_COLORS.len()];
    let p = to_world(ship.pos);
    let r = ship.stats.radius as f32;
    if ship.shield > 0 {
        gizmos.circle_2d(p, r + 6.0, Color::srgb(0.5, 0.8, 1.0));
        gizmos.circle_2d(p, r + 8.0, Color::srgba(0.5, 0.8, 1.0, 0.4));
    }
    if ship.kind == ShipKind::Bulwark && ship.w_cooldown + 12 > ship.w_cooldown_max() {
        // Shockwave: a ring that expands over the first 12 ticks.
        let age = (ship.w_cooldown_max() - ship.w_cooldown) as f32 / 12.0;
        gizmos.circle_2d(
            p,
            SHOCKWAVE_RADIUS as f32 * age.min(1.0),
            Color::srgb(0.6, 0.8, 1.0),
        );
    }
    if ship.dash_ticks > 0 {
        let back = to_world(ship.pos - ship.dash_vel) - p;
        gizmos.line_2d(p, p + back * 3.0, color);
    }
    bar(
        gizmos,
        p + Vec2::new(0.0, r * SPRITE_SCALE + 6.0),
        2.0 * r + 8.0,
        ship.hull as f32 / ship.stats.max_hull as f32,
        Color::srgb(0.3, 1.0, 0.4),
    );
    // Click-to-move destination marker for our own ship.
    if ship.handle == me && ship.target != ship.pos {
        move_marker(gizmos, to_world(ship.target), color);
    }
}

/// The click-to-move destination marker (battle and Carrier): a small X.
pub fn move_marker(gizmos: &mut Gizmos, t: Vec2, color: Color) {
    gizmos.line_2d(t - Vec2::splat(4.0), t + Vec2::splat(4.0), color);
    gizmos.line_2d(t + Vec2::new(-4.0, 4.0), t + Vec2::new(4.0, -4.0), color);
}

fn draw_world(mut gizmos: Gizmos, world: Option<Res<SimWorld>>, local: Option<Res<LocalPlayers>>) {
    let Some(world) = world else { return };
    for e in &world.enemies {
        // An elite always shows its bar, in gold; the Guardian has its own
        // at the top of the screen.
        if e.boss.is_none() && (e.hp < e.max_hp() || e.is_elite()) {
            let c = to_world(e.pos);
            let r = e.radius() as f32;
            let color = if e.is_elite() {
                ELITE_GOLD
            } else {
                Color::srgb(1.0, 0.4, 0.4)
            };
            bar(
                &mut gizmos,
                c + Vec2::new(0.0, r * SPRITE_SCALE + 6.0),
                2.0 * r,
                e.hp as f32 / e.max_hp() as f32,
                color,
            );
        }
    }
    let me = local_handle(local.as_deref());
    for ship in &world.ships {
        draw_ship_fx(&mut gizmos, ship, me);
    }
}

/// One sprite to draw this frame.
struct Draw {
    image: Handle<Image>,
    pos: Vec2,
    /// Longest side in px.
    size: f32,
    angle: f32,
    color: Color,
    z: f32,
    /// Part of the image to draw (px), and then its exact size: for strip
    /// frames and the stretched aim line. `None` draws the whole image at
    /// `size`.
    rect: Option<(Rect, Vec2)>,
    /// Mirror the image (art drawn facing right, moving left).
    flip: bool,
}

/// Reused sprite entities (hidden when unused) plus per-ship facing, which
/// is purely visual and so lives here rather than in the sim.
#[derive(Resource, Default)]
struct BattleSprites {
    pool: Vec<Entity>,
    glow: Option<Handle<Image>>,
    facing: Vec<f32>,
}

#[derive(Component)]
struct BattleSprite;

/// Soft round glow for player bolts (white, tinted per player).
fn glow_image() -> Image {
    const N: u32 = 32;
    let mut data = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5).distance(Vec2::splat(N as f32 / 2.0))
                / (N as f32 / 2.0);
            let a = (1.0 - d).clamp(0.0, 1.0).powf(1.5);
            data.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: N,
            height: N,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    )
}

/// An enemy's sprite tint: plain, or over-bright toward white for `flash`
/// (0..=1) just after a hit. Values above 1 saturate the art to white.
pub fn hit_tint(flash: f32) -> Color {
    let k = 1.0 + HIT_FLASH_GAIN * flash;
    Color::LinearRgba(LinearRgba::new(k, k, k, 1.0))
}

const HIT_FLASH_GAIN: f32 = 6.0;

/// Direction `v` as a sprite rotation, for art drawn pointing up.
fn heading(v: Vec2) -> f32 {
    v.y.atan2(v.x) - std::f32::consts::FRAC_PI_2
}

#[allow(clippy::too_many_arguments)]
fn draw_sprites(
    mut commands: Commands,
    world: Option<Res<SimWorld>>,
    time: Res<Time>,
    flash: Res<EnemyFlash>,
    hit_stop: Res<HitStop>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<BattleSprites>,
    mut sprites: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<BattleSprite>>,
) {
    // Hit-stop: the sprites hold their last pose while the sim runs on.
    if hit_stop.active(time.elapsed_secs()) && world.is_some() {
        return;
    }
    let mut draws = Vec::new();
    if let Some(world) = world.as_deref() {
        let t = time.elapsed_secs(); // visual only
                                     // Returns the new draw's index, if the art is there.
        let mut push = |id: &str, pos: Vec2, radius: f32, angle: f32, color: Color, z: f32| {
            let spec = SPRITES.iter().find(|s| s.id == id)?;
            let image = art.get(&mut images, id)?;
            {
                let pulse = 1.0 + spec.pulse * (t * 2.5).sin();
                draws.push(Draw {
                    image,
                    pos,
                    size: 2.0 * radius * spec.scale * pulse,
                    angle: angle + spec.spin * t,
                    color,
                    z,
                    rect: None,
                    flip: false,
                });
            }
            Some(draws.len() - 1)
        };
        // Quiet fissures (the Guardian is out) dim.
        let fissure_tint = if world.mission.stage == MissionStage::Kills {
            Color::WHITE
        } else {
            Color::srgba(0.55, 0.5, 0.6, 0.55)
        };
        for f in &world.fissures {
            push(
                ids::FISSURE,
                to_world(f.pos),
                FISSURE_RADIUS as f32,
                0.0,
                fissure_tint,
                0.05,
            );
        }
        for p in &world.pickups {
            push(
                loot_id(p.kind),
                to_world(p.pos),
                LOOT_RADIUS,
                0.0,
                Color::WHITE,
                0.1,
            );
        }
        let mut poses = Vec::new();
        for e in &world.enemies {
            let drawn = push(
                enemy_pose(e),
                to_world(e.pos),
                e.radius() as f32,
                0.0,
                hit_tint(flash.amount(e.id, t)),
                0.2,
            );
            poses.extend(drawn.map(|i| (i, e)));
        }
        for p in &world.enemy_projectiles {
            let v = to_world(p.vel);
            push(
                ids::FX_SPIT,
                to_world(p.pos),
                p.radius as f32,
                v.y.atan2(v.x),
                Color::WHITE,
                0.3,
            );
        }
        state.facing.resize(world.ships.len(), 0.0);
        for (i, ship) in world.ships.iter().enumerate() {
            let p = to_world(ship.pos);
            let moving = if ship.dash_ticks > 0 {
                to_world(ship.dash_vel)
            } else {
                to_world(ship.target) - p
            };
            if moving.length_squared() > 1.0 {
                state.facing[i] = heading(moving);
            }
            let color = if ship.alive() && ship.chill > 0 {
                CHILL_TINT
            } else if ship.alive() {
                Color::WHITE
            } else {
                Color::srgba(0.35, 0.35, 0.35, 0.6)
            };
            push(
                ship_id(ship.kind),
                p,
                ship.stats.radius as f32,
                state.facing[i],
                color,
                0.4,
            );
        }
        // Role art faces right: mirror it toward the ship or along the dash.
        // A Brood about to release throbs.
        for (i, e) in poses {
            let d = &mut draws[i];
            let toward = match world
                .ships
                .iter()
                .filter(|s| s.alive())
                .min_by_key(|s| (s.pos - e.pos).length_squared())
            {
                _ if e.kind == EnemyKind::Lancer && (e.dash > 0 || e.windup > 0) => e.aim.x,
                Some(s) => (s.pos - e.pos).x,
                None => 1,
            };
            // The Guardian faces the camera: never mirrored.
            d.flip = !matches!(
                e.kind,
                EnemyKind::Swarmer | EnemyKind::Spitter | EnemyKind::Guardian
            ) && toward < 0;
            if e.kind == EnemyKind::Brood && e.windup > 0 {
                let k = telegraph_done(e.windup, BROOD_WINDUP);
                d.size *= 1.0 + 0.12 * k * (t * 18.0).sin().abs();
            }
            if e.boss.is_some_and(|b| b.attack == BossAttack::Summon) && e.windup > 0 {
                let k = telegraph_done(e.windup, GUARDIAN_SUMMON_WINDUP);
                d.size *= 1.0 + 0.08 * k * (t * 14.0).sin().abs();
                d.color = hit_tint(0.15 * k);
            }
        }
        let glow = state
            .glow
            .get_or_insert_with(|| images.add(glow_image()))
            .clone();
        for p in world.pickups.iter().filter(|p| p.elite) {
            // A light beam over elite loot (rarity colours come with Step 7).
            let at = to_world(p.pos);
            let flicker = 0.75 + 0.25 * (t * 5.0 + p.id as f32).sin();
            for (w, a) in [(30.0, 0.35), (10.0, 0.9)] {
                draws.push(Draw {
                    image: glow.clone(),
                    pos: at + Vec2::Y * BEAM_HEIGHT / 2.0,
                    size: 0.0,
                    angle: 0.0,
                    color: ELITE_GOLD.with_alpha(a * flicker),
                    z: 0.08,
                    rect: Some((Rect::new(0.0, 0.0, 0.0, 0.0), Vec2::new(w, BEAM_HEIGHT))),
                    flip: false,
                });
            }
        }
        for p in &world.projectiles {
            draws.push(Draw {
                image: glow.clone(),
                pos: to_world(p.pos),
                size: 2.0 * p.radius as f32 * BOLT_GLOW_SCALE,
                angle: 0.0,
                color: PLAYER_COLORS[p.owner % PLAYER_COLORS.len()],
                z: 0.5,
                rect: None,
                flip: false,
            });
        }
        let windup = art.get(&mut images, ids::FX_SPITTER_WINDUP);
        let aim_line = art.get(&mut images, ids::FX_SPITTER_AIM_LINE);
        for e in world
            .enemies
            .iter()
            .filter(|e| e.kind == EnemyKind::Spitter && e.windup > 0)
        {
            let at = to_world(e.pos);
            let (frame, done) = windup_frame(e.windup);
            if let Some(image) = aim_line.clone() {
                let dir = to_world(e.aim).normalize_or_zero();
                let (x, y, w, h) = AIM_LINE_UV;
                draws.push(Draw {
                    image,
                    pos: at + dir * AIM_LINE_LENGTH / 2.0,
                    size: 0.0,
                    angle: dir.y.atan2(dir.x),
                    color: Color::srgba(1.0, 1.0, 1.0, 0.45 + 0.55 * done),
                    z: 0.15,
                    rect: Some((
                        Rect::new(x, y, x + w, y + h),
                        Vec2::new(AIM_LINE_LENGTH, AIM_LINE_THICKNESS),
                    )),
                    flip: false,
                });
            }
            if let Some(image) = windup.clone() {
                let x = frame as f32 * FX_CELL;
                draws.push(Draw {
                    image,
                    pos: at,
                    size: 0.0,
                    angle: 0.0,
                    color: Color::WHITE,
                    z: 0.25,
                    rect: Some((
                        Rect::new(x, 0.0, x + FX_CELL, FX_CELL),
                        Vec2::splat(WINDUP_SIZE),
                    )),
                    flip: false,
                });
            }
        }
    }

    if let Some(world) = world.as_deref() {
        elite_visuals(
            world,
            time.elapsed_secs(),
            &mut art,
            &mut images,
            &mut draws,
        );
        role_telegraphs(
            world,
            time.elapsed_secs(),
            &mut art,
            &mut images,
            &mut draws,
        );
        guardian_telegraphs(world, &mut art, &mut images, &mut draws);
    }

    for (i, d) in draws.iter().enumerate() {
        let (rect, size) = match d.rect {
            // An empty rect: the whole image at exactly `size`.
            Some((rect, size)) => ((!rect.is_empty()).then_some(rect), size),
            None => {
                let px = ContentImages::size(&images, &d.image);
                (None, px * (d.size / px.max_element()))
            }
        };
        let tf = Transform::from_translation(d.pos.extend(d.z))
            .with_rotation(Quat::from_rotation_z(d.angle));
        let sprite = Sprite {
            image: d.image.clone(),
            color: d.color,
            custom_size: Some(size),
            rect,
            flip_x: d.flip,
            ..default()
        };
        match state.pool.get(i).copied() {
            Some(entity) => {
                if let Ok((mut s, mut t, mut v)) = sprites.get_mut(entity) {
                    *s = sprite;
                    *t = tf;
                    *v = Visibility::Visible;
                }
            }
            None => {
                let entity = commands.spawn((BattleSprite, sprite, tf)).id();
                state.pool.push(entity);
            }
        }
    }
    for entity in state.pool.iter().skip(draws.len()) {
        if let Ok((_, _, mut v)) = sprites.get_mut(*entity) {
            if *v != Visibility::Hidden {
                *v = Visibility::Hidden;
            }
        }
    }
}

/// The new roles' warnings (TAKOAI-116, Art's TAKOAI-114 sheets), all read
/// from sim state so they match what is about to happen:
/// - Lancer: the charge line along the locked dash, `LANCER_DASH_DISTANCE`
///   long, brightening over the 0.7 s wind-up.
/// - Bloater: the dashed warning ring round an armed Bloater or a pending
///   burst, growing to the burst's reach as the fuse runs out.
/// - Warden: a tether to every ally it shields and a shimmer round each.
fn role_telegraphs(
    world: &SimState,
    t: f32,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    draws: &mut Vec<Draw>,
) {
    let line = art.get(images, ids::FX_LANCER_CHARGE_LINE);
    let ring = art.get(images, ids::FX_BLOATER_WARNING_RING);
    let tether = art.get(images, ids::FX_WARDEN_TETHER);
    let shield = art.get(images, ids::FX_WARDEN_SHIELD);
    let exact = |image: Handle<Image>, pos, size: Vec2, angle, color, z| Draw {
        image,
        pos,
        size: 0.0,
        angle,
        color,
        z,
        rect: Some((Rect::new(0.0, 0.0, 0.0, 0.0), size)),
        flip: false,
    };
    let mut rings = Vec::new();
    for (i, e) in world.enemies.iter().enumerate() {
        let at = to_world(e.pos);
        match e.kind {
            EnemyKind::Lancer if e.windup > 0 => {
                if let Some(image) = line.clone() {
                    let dir = to_world(e.aim).normalize_or_zero();
                    let done = telegraph_done(e.windup, LANCER_WINDUP);
                    let len = LANCER_DASH_DISTANCE as f32;
                    draws.push(exact(
                        image,
                        at + dir * len / 2.0,
                        Vec2::new(len, LANCER_LINE_THICKNESS),
                        dir.y.atan2(dir.x),
                        Color::srgba(1.0, 1.0, 1.0, 0.35 + 0.65 * done),
                        0.15,
                    ));
                }
            }
            EnemyKind::Bloater if e.struck != 0 => rings.push((at, e.windup)),
            _ => {}
        }
        if let Some(w) = warden_of(&world.enemies, i) {
            let from = to_world(world.enemies[w].pos);
            let d = at - from;
            if let Some(image) = tether.clone() {
                draws.push(exact(
                    image,
                    from + d / 2.0,
                    Vec2::new(d.length(), TETHER_THICKNESS),
                    d.y.atan2(d.x),
                    Color::srgba(1.0, 1.0, 1.0, 0.8),
                    0.12,
                ));
            }
            if let Some(image) = shield.clone() {
                let size = 2.0 * e.radius() as f32 * SHIELD_SCALE * (1.0 + 0.05 * (t * 4.0).sin());
                draws.push(exact(
                    image,
                    at,
                    Vec2::splat(size),
                    0.0,
                    Color::srgba(1.0, 1.0, 1.0, 0.6),
                    0.21,
                ));
            }
        }
    }
    rings.extend(world.blasts.iter().map(|b| (to_world(b.pos), b.fuse)));
    if let Some(image) = ring {
        for (at, fuse) in rings {
            let done = telegraph_done(fuse, BLOATER_FUSE);
            // Blinks faster as it gets close.
            let blink = 0.65 + 0.35 * (t * (8.0 + 16.0 * done)).sin();
            draws.push(exact(
                image.clone(),
                at,
                Vec2::splat(warning_ring_size(fuse)),
                0.0,
                Color::srgba(1.0, 1.0, 1.0, blink),
                0.16,
            ));
        }
    }
}

/// Where the Guardian's coming ring will fly (unit directions), gaps left
/// out: the same slots the sim fires (`Boss::in_gap`).
pub fn ring_preview(b: &Boss) -> Vec<Vec2> {
    let (n, _, _) = b.ring();
    let steps = sim::trig::ANGLE_STEPS;
    (0..n)
        .filter(|&k| !Boss::in_gap(k, n))
        .map(|k| {
            // The sim's angle units -> radians, visual only.
            let a = ((b.gap + k * steps / n) % steps) as f32 / steps as f32 * std::f32::consts::TAU;
            Vec2::new(a.cos(), a.sin())
        })
        .collect()
}

/// The Guardian's warnings, from its wind-up:
/// - Ring: Art's ring flash grows and brightens round it, and a faint shot
///   marks each slot of the coming ring just outside it, so the two gaps
///   show before a shot leaves.
/// - Charge: its charge line along the locked dash, brightening.
///
/// (The summon throbs the body, in `draw_sprites`.)
fn guardian_telegraphs(
    world: &SimState,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    draws: &mut Vec<Draw>,
) {
    let Some(g) = world.guardian() else {
        return;
    };
    let Some(b) = g.boss.filter(|_| g.windup > 0) else {
        return;
    };
    let at = to_world(g.pos);
    let r = g.radius() as f32;
    match b.attack {
        BossAttack::Ring => {
            let done = telegraph_done(g.windup, b.ring().2);
            if let Some(image) = art.get(images, ids::FX_GUARDIAN_RING_FLASH) {
                let size = crate::fx::GUARDIAN_RING_FLASH_SIZE * (0.6 + 0.4 * done);
                draws.push(exact_draw(
                    image,
                    at,
                    Vec2::splat(size),
                    0.0,
                    Color::srgba(1.0, 1.0, 1.0, 0.4 + 0.6 * done),
                    0.26,
                ));
            }
            if let Some(image) = art.get(images, ids::FX_SPIT) {
                let out = r * 1.9;
                let d = 2.0 * GUARDIAN_SHOT_RADIUS as f32 * SPRITE_SCALE;
                for dir in ring_preview(&b) {
                    draws.push(exact_draw(
                        image.clone(),
                        at + dir * out,
                        Vec2::splat(d),
                        dir.y.atan2(dir.x),
                        Color::srgba(1.0, 1.0, 1.0, 0.15 + 0.45 * done),
                        0.27,
                    ));
                }
            }
        }
        BossAttack::Charge => {
            if let Some(image) = art.get(images, ids::FX_GUARDIAN_CHARGE_LINE) {
                let dir = to_world(g.aim).normalize_or_zero();
                let done = telegraph_done(g.windup, GUARDIAN_CHARGE_WINDUP);
                let len = GUARDIAN_CHARGE_DISTANCE as f32;
                draws.push(exact_draw(
                    image,
                    at + dir * len / 2.0,
                    Vec2::new(len, 2.0 * r * 0.7),
                    dir.y.atan2(dir.x),
                    Color::srgba(1.0, 1.0, 1.0, 0.35 + 0.65 * done),
                    0.15,
                ));
            }
        }
        BossAttack::Summon => {}
    }
}

/// A whole image drawn at exactly `size`.
fn exact_draw(
    image: Handle<Image>,
    pos: Vec2,
    size: Vec2,
    angle: f32,
    color: Color,
    z: f32,
) -> Draw {
    Draw {
        image,
        pos,
        size: 0.0,
        angle,
        color,
        z,
        rect: Some((Rect::new(0.0, 0.0, 0.0, 0.0), size)),
        flip: false,
    }
}

/// Elite colour: health bar, loot beam.
const ELITE_GOLD: Color = Color::srgb(1.0, 0.82, 0.35);
/// A ship slowed by a Chilling elite.
const CHILL_TINT: Color = Color::srgb(0.55, 0.8, 1.0);
/// Elite loot beam height (px).
const BEAM_HEIGHT: f32 = 140.0;
/// The aura ring is this many times the elite's diameter.
const AURA_SCALE: f32 = 2.1;
/// Affix icon size (px; Art suggests ~24, a little bigger reads better).
const AFFIX_ICON: f32 = 30.0;
/// Where the label's bottom sits above the elite's top edge (px).
const LABEL_GAP: f32 = 14.0;
/// The label is two lines; the icons sit above it.
const LABEL_HEIGHT: f32 = 30.0;

/// Where an elite's label starts (bottom centre), above its health bar.
fn label_pos(e: &Enemy) -> Vec2 {
    to_world(e.pos) + Vec2::Y * (e.radius() as f32 * SPRITE_SCALE + LABEL_GAP)
}

/// Elite leaders and their affixes (TAKOAI-117), from sim state:
/// - every elite: Art's aura ring under it (slowly turning) and its affix
///   icons above its label;
/// - Warding: the Warden's shimmer on each enemy it shields (`Enemy::ward`);
/// - Blinking: the teleport mark where it will land, closing in over the
///   0.5 s mark;
/// - Scorching: the burning patches, fading as they burn out.
fn elite_visuals(
    world: &SimState,
    t: f32,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
    draws: &mut Vec<Draw>,
) {
    use sim::tuning::{BLINK_MARK, SCORCH_LIFETIME, SCORCH_RADIUS};
    if let Some(image) = art.get(images, ids::ELITE_BURNING_PATCH) {
        for p in &world.patches {
            let left = p.ttl as f32 / SCORCH_LIFETIME as f32;
            let flicker = 0.85 + 0.15 * (t * 9.0 + p.id as f32).sin();
            draws.push(exact_draw(
                image.clone(),
                to_world(p.pos),
                Vec2::splat(2.0 * SCORCH_RADIUS as f32 * 1.3),
                0.0,
                Color::srgba(1.0, 1.0, 1.0, (left * 3.0).min(1.0) * flicker),
                0.04,
            ));
        }
    }
    let aura = art.get(images, ids::ELITE_AURA);
    let mark = art.get(images, ids::ELITE_TELEPORT_MARK);
    let shield = art.get(images, ids::FX_WARDEN_SHIELD);
    for e in &world.enemies {
        let at = to_world(e.pos);
        let d = 2.0 * e.radius() as f32;
        if e.ward > 0 {
            if let Some(image) = shield.clone() {
                let size = d * SHIELD_SCALE * (1.0 + 0.06 * (t * 6.0).sin());
                draws.push(exact_draw(
                    image,
                    at,
                    Vec2::splat(size),
                    0.0,
                    Color::srgba(1.0, 0.95, 0.7, 0.7),
                    0.21,
                ));
            }
        }
        let Some(el) = e.elite else { continue };
        if let Some(image) = aura.clone() {
            let pulse = 1.0 + 0.04 * (t * 3.0).sin();
            draws.push(exact_draw(
                image,
                at,
                Vec2::splat(d * AURA_SCALE * pulse),
                t * 0.6,
                Color::WHITE,
                0.18,
            ));
        }
        if el.blink > 0 {
            if let Some(image) = mark.clone() {
                let done = telegraph_done(el.blink, BLINK_MARK);
                draws.push(exact_draw(
                    image,
                    to_world(el.mark),
                    Vec2::splat(d * (2.4 - 0.9 * done)),
                    -t * 2.0,
                    Color::srgba(1.0, 1.0, 1.0, 0.5 + 0.5 * done),
                    0.17,
                ));
            }
        }
        let icons: Vec<_> = Affix::in_mask(el.affixes).collect();
        let top = label_pos(e) + Vec2::Y * (LABEL_HEIGHT + AFFIX_ICON / 2.0);
        let n = icons.len() as f32;
        for (k, a) in icons.into_iter().enumerate() {
            let Some(image) = art.get(images, affix_icon(a)) else {
                continue;
            };
            let x = (k as f32 - (n - 1.0) / 2.0) * (AFFIX_ICON + 4.0);
            draws.push(exact_draw(
                image,
                top + Vec2::X * x,
                Vec2::splat(AFFIX_ICON),
                0.0,
                Color::WHITE,
                0.85,
            ));
        }
    }
}

/// An elite's name and affix label, following it (`Text2d`, world space).
#[derive(Component)]
struct EliteLabel(u32);

/// One label per living elite: spawned when it appears, moved with it,
/// despawned when it dies (after any hit-stop) or the battle ends.
fn elite_labels(
    mut commands: Commands,
    world: Option<Res<SimWorld>>,
    time: Res<Time>,
    hit_stop: Res<HitStop>,
    mut art: ResMut<ContentImages>,
    mut fonts: ResMut<Assets<Font>>,
    mut labels: Query<(Entity, &EliteLabel, &mut Transform)>,
) {
    // Hold with the sprites through a hit-stop (an elite kill's freeze).
    if hit_stop.active(time.elapsed_secs()) && world.is_some() {
        return;
    }
    let elites: Vec<&Enemy> = world
        .as_deref()
        .map(|w| w.enemies.iter().filter(|e| e.is_elite()).collect())
        .unwrap_or_default();
    let mut shown = Vec::new();
    for (entity, label, mut tf) in &mut labels {
        match elites.iter().find(|e| e.id == label.0) {
            Some(e) => {
                tf.translation = label_pos(e).extend(0.85);
                shown.push(label.0);
            }
            None => commands.entity(entity).despawn(),
        }
    }
    let font = art.font(&mut fonts, crate::art::ids::FONT_DISPLAY);
    let font = |size: f32| TextFont {
        font: font.clone().map(Into::into).unwrap_or_default(),
        font_size: FontSize::Px(size),
        ..default()
    };
    for e in elites.into_iter().filter(|e| !shown.contains(&e.id)) {
        let Some((name, affixes)) = elite_label(e) else {
            continue;
        };
        commands
            .spawn((
                EliteLabel(e.id),
                Text2d::new(name),
                font(15.0),
                TextColor(ELITE_GOLD),
                TextLayout::justify(Justify::Center),
                bevy::sprite::Anchor::BOTTOM_CENTER,
                bevy::sprite::Text2dShadow {
                    offset: Vec2::new(1.5, -1.5),
                    color: Color::srgb_u8(0x24, 0x10, 0x32),
                },
                Transform::from_translation(label_pos(e).extend(0.85)),
            ))
            .with_child((
                TextSpan::new(format!("\n{affixes}")),
                font(12.0),
                TextColor(Color::srgb(0.9, 0.85, 1.0)),
            ));
    }
}

/// Lancer charge line thickness, Warden tether thickness (px).
const LANCER_LINE_THICKNESS: f32 = 22.0;
const TETHER_THICKNESS: f32 = 14.0;
/// Warden shield shimmer: this many times the shielded enemy's diameter.
const SHIELD_SCALE: f32 = 1.6;

/// One edge arrow per fissure that is off screen: a point just inside the
/// screen edge toward that fissure and the unit direction to it, in
/// fissure order. A fissure (even partly) on screen gets none. `half` is
/// half the visible world area, `inset` how far inside the edge the point
/// sits.
pub fn fissure_pointers(
    centre: Vec2,
    half: Vec2,
    inset: f32,
    fissures: &[Vec2],
) -> Vec<(Vec2, Vec2)> {
    let r = FISSURE_RADIUS as f32;
    let edge = (half - Vec2::splat(inset)).max(Vec2::ONE);
    fissures
        .iter()
        .filter_map(|f| {
            let d = (*f - centre).abs();
            if d.x <= half.x + r && d.y <= half.y + r {
                return None;
            }
            let dir = (*f - centre).normalize_or_zero();
            if dir == Vec2::ZERO {
                return None;
            }
            let t = (edge.x / dir.x.abs()).min(edge.y / dir.y.abs());
            Some((centre + dir * t, dir))
        })
        .collect()
}

/// Slides an edge pointer at `tip` along its screen edge until it is clear
/// of `keep_out` (the HUD's corner, grown by the arrow's size), the way the
/// Carrier labels keep clear of its panels. It slides toward the screen
/// `centre` (away from the corner); `on_top_or_bottom` says which edge the
/// pointer sits on, so which way it may slide.
pub fn clear_of(tip: Vec2, keep_out: Rect, centre: Vec2, on_top_or_bottom: bool) -> Vec2 {
    if !keep_out.contains(tip) {
        return tip;
    }
    let toward = |at: f32, c: f32, min: f32, max: f32| if c > at { max } else { min };
    if on_top_or_bottom {
        let x = toward(tip.x, centre.x, keep_out.min.x, keep_out.max.x);
        Vec2::new(x, tip.y)
    } else {
        let y = toward(tip.y, centre.y, keep_out.min.y, keep_out.max.y);
        Vec2::new(tip.x, y)
    }
}

/// Every off-screen fissure (or, once it is out, the Guardian) gets an
/// arrow at the screen edge pointing at it, kept off the HUD text.
/// Client-only.
fn draw_fissure_pointers(
    mut gizmos: Gizmos,
    world: Option<Res<SimWorld>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Transform, &Projection), With<Camera2d>>,
    hud: Query<&ComputedNode, With<Hud>>,
) {
    let (Some(world), Ok(window), Ok((cam, projection))) =
        (world, windows.single(), camera.single())
    else {
        return;
    };
    if world.mission.status != MissionStatus::InProgress {
        return;
    }
    let scale = match projection {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    let half = window.size() / 2.0 * scale;
    // While the Guardian is out the arrow points at it instead.
    let fissures: Vec<Vec2> = match world.guardian() {
        Some(g) => vec![to_world(g.pos)],
        None => world.fissures.iter().map(|f| to_world(f.pos)).collect(),
    };
    let centre = cam.translation.truncate();
    let size = 22.0 * scale;
    let inset = 24.0 * scale;
    // The HUD text block (top left, `HUD_MARGIN` in) in world space, grown
    // by an arrow's length so no part of an arrow reaches the text.
    let keep_out = hud.single().ok().map(|node| {
        let hud = node.size() * node.inverse_scale_factor() * scale;
        let top_left = centre + Vec2::new(-half.x, half.y);
        let margin = HUD_MARGIN * scale + size;
        Rect::from_corners(
            top_left - Vec2::splat(margin),
            top_left + Vec2::new(hud.x, -hud.y) + Vec2::new(margin, -margin),
        )
    });
    for (tip, dir) in fissure_pointers(centre, half, inset, &fissures) {
        let on_top_or_bottom = (tip.y - centre.y).abs() >= half.y - inset - 0.5;
        let tip = keep_out.map_or(tip, |r| clear_of(tip, r, centre, on_top_or_bottom));
        // A solid-looking chevron: nested outlines shrinking toward the tip,
        // the innermost ones teal.
        let side = dir.perp();
        for k in 0..6 {
            let s = size * (1.0 - k as f32 * 0.15);
            let base = tip - dir * s;
            let color = if k >= 4 {
                POINTER_ACCENT
            } else {
                POINTER_COLOR
            };
            gizmos.linestrip_2d(
                [tip, base + side * s * 0.6, base - side * s * 0.6, tip],
                color,
            );
        }
    }
}

/// Battle camera: follows our ship with a damped spring, zoomed with the
/// mouse wheel about the cursor (`crate::zoom`) on top of `BATTLE_ZOOM`.
/// Each battle starts at zoom 1; when the battle world goes, the camera
/// goes back to 1:1 (the Carrier sets its own zoom on entry).
#[derive(Resource, Default)]
struct CameraFollow {
    vel: Vec2,
    zoom: Zoom,
    active: bool,
    /// The shake offset added this frame.
    shake: Vec2,
}

const CAMERA_STICKY: f32 = 0.3;

#[allow(clippy::too_many_arguments)]
fn follow_camera(
    time: Res<Time>,
    world: Option<Res<SimWorld>>,
    local: Option<Res<LocalPlayers>>,
    scroll: Res<AccumulatedMouseScroll>,
    pause: Option<Res<PauseMenu>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    shake: Res<ScreenShake>,
    mut follow: ResMut<CameraFollow>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let Ok((mut cam, mut projection)) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(ortho) = &mut *projection else {
        return;
    };
    let Some(world) = world else {
        if follow.active {
            *follow = CameraFollow::default();
            ortho.scale = 1.0;
        }
        return;
    };
    if world.is_added() || !follow.active {
        *follow = CameraFollow {
            active: true,
            ..default()
        };
    }
    if !pause.is_some_and(|p| p.open) {
        follow.zoom.scroll(zoom::notches(&scroll));
    }
    let window = windows.single().ok();
    let size = window.map_or(crate::DEFAULT_WINDOW.as_vec2(), Window::size);
    let cursor = window.and_then(Window::cursor_position);
    let base = 1.0 / BATTLE_ZOOM;
    // Last frame's shake is not part of where the camera is.
    let pos = cam.translation.truncate() - follow.shake;
    let pos = follow
        .zoom
        .update(time.delta_secs(), base, pos, cursor, size);
    let scale = BATTLE_ZOOM / follow.zoom.shown;
    if ortho.scale != scale {
        ortho.scale = scale;
    }
    // No arena edge, so the clamp is our ship staying on screen.
    follow.zoom.keep_in_view(size / 2.0 * scale);
    // Screen shake (`crate::fx`), in screen px so it reads the same at
    // every zoom; only ever added on top of the follow position.
    follow.shake = shake.offset(time.elapsed_secs()) * scale;
    let Some(ship) = world.ships.get(local_handle(local.as_deref())) else {
        cam.translation = (pos + follow.shake).extend(cam.translation.z);
        return;
    };
    let target = to_world(ship.pos) + follow.zoom.offset;
    let next = pos + follow.vel;
    follow.vel = follow.vel * CAMERA_STICKY + (target - next) * (1.0 - CAMERA_STICKY);
    cam.translation = (next + follow.shake).extend(cam.translation.z);
}

fn cooldown_text(ticks: u32) -> String {
    if ticks == 0 {
        "ready".to_string()
    } else {
        format!("{:.1}s", ticks as f32 / TICKS_PER_SEC as f32)
    }
}

/// Battle HUD: hull, kills/objective, mission time and live enemies, Q/W
/// cooldowns, loot so far.
pub fn battle_hud(world: &SimState, me: usize) -> String {
    let m = &world.mission;
    let mut s = String::new();
    if let Some(ship) = world.ships.get(me) {
        let frac = ship.hull.max(0) as f32 / ship.stats.max_hull as f32;
        let filled = (frac * 20.0).ceil() as usize;
        s += &format!(
            "Hull [{}{}] {}/{}",
            "#".repeat(filled),
            "-".repeat(20 - filled),
            ship.hull.max(0),
            ship.stats.max_hull
        );
        if ship.shield > 0 {
            s += &format!("   Shield {}", ship.shield);
        }
        let (q, w) = skill_names(ship.kind);
        s += &format!(
            "\nQ {q}: {}    W {w}: {}",
            cooldown_text(ship.q_cooldown),
            cooldown_text(ship.w_cooldown)
        );
    }
    let secs = world.frame / TICKS_PER_SEC as u32;
    let target = m.kill_target;
    let objective = match m.stage {
        MissionStage::Kills => format!(
            "Kills {}/{target} - Objective: destroy {target} void monsters",
            m.kills
        ),
        MissionStage::Guardian => {
            format!(
                "Kills {} - Objective: destroy the Fissure Guardian",
                m.kills
            )
        }
        MissionStage::Outro => format!("Kills {} - Fissure Guardian destroyed", m.kills),
    };
    s += &format!(
        "\n{objective}\nTime {}:{:02}   Enemies {}\nLoot: {} credits, {} Void Crystal",
        secs / 60,
        secs % 60,
        world.enemies.len(),
        m.collected.credits,
        m.collected.void_crystal
    );
    s
}

#[allow(clippy::too_many_arguments)]
fn update_hud(
    mut hud: Query<&mut Text, With<Hud>>,
    diagnostics: Res<DiagnosticsStore>,
    stats: Res<Stats>,
    status: Res<NetStatus>,
    content: Res<ContentStatus>,
    session: Option<Res<Session<GameConfig>>>,
    world: Option<Res<SimWorld>>,
    local: Option<Res<LocalPlayers>>,
) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let mut s = String::new();
    // Netcode diagnostics only in synctest / p2p runs.
    if session.is_some() {
        let fps = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.smoothed())
            .unwrap_or(0.0);
        s = format!(
            "{} | {} | fps {:.0} | frame {} | rollbacks {} (max depth {}) | desyncs {} | mismatches {}\n",
            status.0,
            content.0,
            fps,
            stats.max_frame,
            stats.rollbacks,
            stats.max_rollback,
            stats.desyncs,
            stats.synctest_mismatches,
        );
    }
    if let Some(world) = world {
        s += &battle_hud(&world, local_handle(local.as_deref()));
    }
    text.0 = s;
}

/// How long the Guardian's banners stay up (seconds).
const GUARDIAN_BANNER_SECS: u32 = 3;

/// The battle banner: the Guardian coming out, its phase change and its
/// death (each for `GUARDIAN_BANNER_SECS`), then the mission's end.
pub fn banner_text(world: &SimState) -> &'static str {
    let m = &world.mission;
    let recent = |frame: u32| world.frame.saturating_sub(frame) < GUARDIAN_BANNER_SECS * 60;
    match m.status {
        MissionStatus::Success => return "MISSION SUCCESS",
        MissionStatus::Failed => return "MISSION FAILED",
        MissionStatus::InProgress => {}
    }
    match m.stage {
        MissionStage::Outro => "FISSURE GUARDIAN DESTROYED",
        MissionStage::Guardian => {
            let phase = world.guardian().and_then(|g| g.boss).map(|b| b.phase_frame);
            match phase {
                Some(f) if f > 0 && recent(f) => "THE GUARDIAN CRACKS OPEN",
                _ if recent(m.guardian_frame) => "THE FISSURE GUARDIAN AWAKENS",
                _ => "",
            }
        }
        MissionStage::Kills => "",
    }
}

fn update_banner(mut banner: Query<&mut Text, With<Banner>>, world: Option<Res<SimWorld>>) {
    let Ok(mut text) = banner.single_mut() else {
        return;
    };
    let s = world.as_deref().map_or("", |w| banner_text(w)).to_string();
    if text.0 != s {
        text.0 = s;
    }
}

/// Boss bar colours: phase 1, phase 2.
const BOSS_FILL: Color = Color::srgb(0.78, 0.36, 1.0);
const BOSS_FILL_ENRAGED: Color = Color::srgb(1.0, 0.3, 0.55);
const BOSS_BAR_WIDTH: f32 = 420.0;

/// The Guardian's bar: its name over a wide bar at the bottom centre,
/// shown while it is out.
fn spawn_boss_bar(commands: &mut Commands) {
    commands
        .spawn((
            BossBar,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                // Bottom centre, above the controls line: the top is the
                // HUD's.
                bottom: px(40),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                BossName,
                Text::new("FISSURE GUARDIAN"),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.85, 1.0)),
            ));
            root.spawn((
                Node {
                    width: px(BOSS_BAR_WIDTH),
                    height: px(14),
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.05, 0.15, 0.85)),
                BorderColor::all(Color::srgb(0.95, 0.85, 1.0)),
            ))
            .with_child((
                BossFill,
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(BOSS_FILL),
            ));
        });
}

/// The boss bar's name line, fill fraction and colour, while the Guardian
/// is out.
pub fn boss_bar(world: &SimState) -> Option<(&'static str, f32, Color)> {
    let g = world.guardian()?;
    let b = g.boss?;
    let frac = g.hp.max(0) as f32 / b.max_hp.max(1) as f32;
    Some(if b.phase >= 2 {
        ("FISSURE GUARDIAN  -  ENRAGED", frac, BOSS_FILL_ENRAGED)
    } else {
        ("FISSURE GUARDIAN", frac, BOSS_FILL)
    })
}

fn update_boss_bar(
    world: Option<Res<SimWorld>>,
    mut bar: Query<&mut Visibility, With<BossBar>>,
    mut name: Query<&mut Text, With<BossName>>,
    mut fill: Query<(&mut Node, &mut BackgroundColor), With<BossFill>>,
) {
    let shown = world.as_deref().and_then(|w| boss_bar(w));
    for mut v in &mut bar {
        let want = if shown.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    }
    let Some((label, frac, color)) = shown else {
        return;
    };
    for mut text in &mut name {
        if text.0 != label {
            text.0 = label.to_string();
        }
    }
    for (mut node, mut bg) in &mut fill {
        node.width = percent(100.0 * frac);
        if bg.0 != color {
            bg.0 = color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::{Loadout, ShipKind};

    #[test]
    fn every_battle_visual_has_sprite_art() {
        for id in [
            ship_id(ShipKind::Kite),
            ship_id(ShipKind::Bulwark),
            loot_id(LootKind::Credits),
            loot_id(LootKind::VoidCrystal),
            ids::FX_SPIT,
            ids::FISSURE,
        ] {
            assert!(SPRITES.iter().any(|p| p.id == id), "{id}");
        }
        // Every enemy in every pose.
        for kind in EnemyKind::ALL {
            let mut e = sim::Enemy::new(kind, FxVec2::ZERO);
            for (windup, dash, struck) in [(0, 0, 0), (5, 0, 0), (0, 5, 0), (5, 0, 1)] {
                (e.windup, e.dash, e.struck) = (windup, dash, struck);
                let id = enemy_pose(&e);
                assert!(SPRITES.iter().any(|p| p.id == id), "{id}");
            }
        }
        let mut art = ContentImages::repo();
        let mut images = Assets::<Image>::default();
        for id in battle_image_ids() {
            assert!(art.get(&mut images, id).is_some(), "{id} not shipped");
        }
    }

    #[test]
    fn elites_show_their_name_affixes_and_icons() {
        let mut e = sim::Enemy::new(EnemyKind::Lancer, FxVec2::ZERO);
        assert_eq!(elite_label(&e), None);
        e.make_elite(Affix::Scorching.bit(), 0);
        assert_eq!(
            elite_label(&e),
            Some(("Gloomwhisker the Hollow".into(), "Scorching".into()))
        );
        e.make_elite(Affix::Swift.bit() | Affix::Chilling.bit(), 0);
        assert_eq!(elite_label(&e).unwrap().1, "Swift · Chilling");
        let ids = battle_image_ids();
        for a in Affix::ALL {
            assert!(ids.contains(&affix_icon(a)), "{a:?}");
        }
        assert!(ids.contains(&ids::ELITE_AURA));
    }

    #[test]
    fn role_poses_follow_the_sim() {
        let mut e = sim::Enemy::new(EnemyKind::Lancer, FxVec2::ZERO);
        assert_eq!(enemy_pose(&e), ids::ENEMY_LANCER);
        e.windup = 3;
        assert_eq!(enemy_pose(&e), ids::ENEMY_LANCER_WINDUP);
        e.dash = 3;
        assert_eq!(enemy_pose(&e), ids::ENEMY_LANCER_DASH);
        let mut b = sim::Enemy::new(EnemyKind::Bloater, FxVec2::ZERO);
        b.struck = 1;
        assert_eq!(enemy_pose(&b), ids::ENEMY_BLOATER_SWELLING);
        // The warning ring ends exactly at the burst's reach.
        assert_eq!(warning_ring_size(0), 2.0 * BLOATER_BLAST_RADIUS as f32);
        assert!(warning_ring_size(BLOATER_FUSE) < warning_ring_size(BLOATER_FUSE / 2));
        assert_eq!(telegraph_done(LANCER_WINDUP, LANCER_WINDUP), 0.0);
        assert_eq!(telegraph_done(0, LANCER_WINDUP), 1.0);
    }

    #[test]
    fn sim_starting_view_matches_the_battle_camera() {
        // The sim keeps fissures off the starting screen using its own copy
        // of the view size; it must match the default window at battle zoom.
        use sim::tuning::{INITIAL_VIEW_HALF_H, INITIAL_VIEW_HALF_W};
        let half = crate::DEFAULT_WINDOW.as_vec2() * BATTLE_ZOOM / 2.0;
        assert_eq!(half.x.round() as i32, INITIAL_VIEW_HALF_W);
        assert_eq!(half.y.round() as i32, INITIAL_VIEW_HALF_H);
    }

    #[test]
    fn one_pointer_per_off_screen_fissure() {
        let half = Vec2::new(600.0, 350.0);
        let far_right = Vec2::new(900.0, 0.0);
        let far_up_left = Vec2::new(-700.0, 700.0);
        // On screen (even just the edge of its sprite): no pointer.
        assert!(fissure_pointers(Vec2::ZERO, half, 20.0, &[Vec2::new(620.0, 0.0)]).is_empty());
        assert!(fissure_pointers(Vec2::ZERO, half, 20.0, &[]).is_empty());
        // One on screen, one off: only the off-screen one gets an arrow.
        let p = fissure_pointers(Vec2::ZERO, half, 20.0, &[far_right, Vec2::new(0.0, 300.0)]);
        assert_eq!(p, vec![(Vec2::new(580.0, 0.0), Vec2::X)]);
        // Both off screen: one arrow each, in fissure order.
        let p = fissure_pointers(Vec2::ZERO, half, 20.0, &[far_up_left, far_right]);
        assert_eq!(p.len(), 2);
        assert!(p[0].1.x < 0.0 && p[0].1.y > 0.0, "{p:?}");
        assert!((p[0].0.y - 330.0).abs() < 1e-3, "{p:?}");
        assert_eq!(p[1], (Vec2::new(580.0, 0.0), Vec2::X));
        let p = fissure_pointers(Vec2::new(-650.0, 0.0), half, 20.0, &[far_up_left]);
        let (tip, dir) = p[0];
        assert!(dir.y > 0.99 && (tip.y - 330.0).abs() < 1e-3, "{tip} {dir}");
    }

    #[test]
    fn pointers_slide_off_the_hud() {
        let hud = Rect::new(-600.0, 200.0, -100.0, 350.0);
        // On the top edge under the HUD: slides right, past its far side.
        let c = Vec2::ZERO;
        let tip = Vec2::new(-200.0, 330.0);
        assert_eq!(clear_of(tip, hud, c, true), Vec2::new(-100.0, 330.0));
        // Even right by the corner it slides toward the middle, not off screen.
        let tip = Vec2::new(-580.0, 330.0);
        assert_eq!(clear_of(tip, hud, c, true), Vec2::new(-100.0, 330.0));
        // On the left edge beside it: slides down.
        let tip = Vec2::new(-580.0, 340.0);
        assert_eq!(clear_of(tip, hud, c, false), Vec2::new(-580.0, 200.0));
        // Clear of it already: untouched.
        let tip = Vec2::new(300.0, 330.0);
        assert_eq!(clear_of(tip, hud, c, true), tip);
    }

    #[test]
    fn fissures_do_not_spin() {
        let f = SPRITES.iter().find(|s| s.id == ids::FISSURE).unwrap();
        assert_eq!(f.spin, 0.0);
        assert!(f.pulse > 0.0 && f.pulse <= 0.1);
    }

    #[test]
    fn windup_glow_steps_through_its_frames_with_the_sim() {
        assert_eq!(windup_frame(SPITTER_WINDUP), (0, 0.0));
        let frames: Vec<_> = (1..=SPITTER_WINDUP)
            .rev()
            .map(|w| windup_frame(w).0)
            .collect();
        // One frame per 100 ms: four equal steps, last frame at the end.
        for f in 0..WINDUP_FRAMES {
            let n = frames.iter().filter(|&&x| x == f).count() as u32;
            assert_eq!(n, SPITTER_WINDUP / WINDUP_FRAMES, "frame {f}");
        }
        assert_eq!(windup_frame(1).0, WINDUP_FRAMES - 1);
    }

    #[test]
    fn heading_points_art_along_movement() {
        assert!(heading(Vec2::Y).abs() < 1e-6);
        assert!((heading(Vec2::X) + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn hud_shows_hull_kills_time_enemies_cooldowns_and_loot() {
        let mut w = SimState::with_loadouts(
            1,
            &[Loadout {
                ship: ShipKind::Bulwark,
                ..Default::default()
            }],
        );
        w.mission.kills = 7;
        w.mission.collected.credits = 35;
        w.ships[0].w_cooldown = 90;
        w.frame = 75 * 60;
        w.enemies.clear();
        w.enemies
            .push(sim::Enemy::new(EnemyKind::Swarmer, FxVec2::ZERO));
        let hud = battle_hud(&w, 0);
        assert!(hud.contains("140/140"), "{hud}");
        let kills = format!("Kills 7/{}", w.mission.kill_target);
        assert!(hud.contains(&kills), "{hud}");
        assert!(hud.contains("Time 1:15   Enemies 1"), "{hud}");
        assert!(!hud.contains("Wave"), "{hud}");
        assert!(
            hud.contains("Q Bastion: ready") && hud.contains("W Shockwave: 1.5s"),
            "{hud}"
        );
        assert!(hud.contains("35 credits"), "{hud}");
    }

    #[test]
    fn guardian_bar_banners_and_ring_preview_follow_the_sim() {
        use sim::input::NetInput;
        let mut w = SimState::with_loadouts(5, &[Loadout::default()]).with_kill_target(1);
        w.mission.spawn_timer = u32::MAX;
        assert!(boss_bar(&w).is_none());
        let mut e = sim::Enemy::new(EnemyKind::Swarmer, FxVec2::from_px(500, 500));
        e.hp = 0;
        w.enemies.push(e);
        w.step(&[NetInput::default()]);
        assert_eq!(banner_text(&w), "THE FISSURE GUARDIAN AWAKENS");
        let (name, frac, _) = boss_bar(&w).expect("bar");
        assert_eq!((name, frac), ("FISSURE GUARDIAN", 1.0));
        let g = w.guardian().unwrap();
        assert_eq!(enemy_pose(g), ids::ENEMY_GUARDIAN);
        // The preview shows every slot the ring will fire, gaps left out.
        let b = g.boss.unwrap();
        let (n, _, _) = b.ring();
        assert_eq!(
            ring_preview(&b).len() as i32,
            n - 2 * sim::tuning::GUARDIAN_RING_GAP
        );
        // Phase 2: the cracked pose, the enraged bar and banner.
        let id = w.mission.guardian;
        let g = w.enemies.iter_mut().find(|e| e.id == id).unwrap();
        g.hp = g.max_hp() / 2;
        w.step(&[NetInput::default()]);
        assert_eq!(banner_text(&w), "THE GUARDIAN CRACKS OPEN");
        assert_eq!(
            enemy_pose(w.guardian().unwrap()),
            ids::ENEMY_GUARDIAN_PHASE2
        );
        assert!(boss_bar(&w).unwrap().0.contains("ENRAGED"));
        for _ in 0..GUARDIAN_BANNER_SECS * 60 {
            w.step(&[NetInput::default()]);
        }
        assert_eq!(banner_text(&w), "");
        // Dead: the bar goes, the banner says so, then the mission ends.
        let g = w.enemies.iter_mut().find(|e| e.id == id).unwrap();
        g.hp = 0;
        w.step(&[NetInput::default()]);
        assert!(boss_bar(&w).is_none());
        assert_eq!(banner_text(&w), "FISSURE GUARDIAN DESTROYED");
        assert!(battle_hud(&w, 0).contains("Fissure Guardian destroyed"));
        for _ in 0..sim::tuning::GUARDIAN_OUTRO {
            w.step(&[NetInput::default()]);
        }
        assert_eq!(banner_text(&w), "MISSION SUCCESS");
    }
}
