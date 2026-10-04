//! Battle visuals, camera and HUD. Runs in `Update`, outside the rollback
//! schedule, and only reads `SimWorld`.
//!
//! Ships, enemies, fissures, loot and enemy shots are sprites of the shipped
//! art, looked up by stable content ID (`ids`) through the manifest, over a
//! tiled background. Player bolts use a generated glow tinted per player;
//! shields, the shockwave, hull bars, the move marker and the off-screen
//! fissure pointer are gizmo effects.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_ggrs::{LocalPlayers, Session};
use sim::tuning::{
    ARENA_HALF_H, ARENA_HALF_W, FISSURE_RADIUS, KILL_TARGET, SHOCKWAVE_COOLDOWN, SHOCKWAVE_RADIUS,
    TICKS_PER_SEC,
};
use sim::{EnemyKind, FxVec2, LootKind, MissionStatus, Ship, ShipKind, SimState, SUB};

use crate::art::ContentImages;
use crate::rollback::{GameConfig, SimWorld};
use crate::stats::Stats;
use crate::{ContentStatus, NetStatus};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .insert_resource(ClearColor(Color::BLACK))
            .init_resource::<CameraFollow>()
            .init_resource::<ContentImages>()
            .init_resource::<BattleSprites>()
            .add_systems(Startup, setup_scene)
            .add_systems(
                Update,
                (
                    draw_world,
                    draw_sprites,
                    battle_background,
                    draw_fissure_pointer,
                    battle_zoom,
                    follow_camera,
                    update_hud,
                    update_banner,
                ),
            );
    }
}

#[derive(Component)]
struct Hud;

#[derive(Component)]
struct Banner;

/// The tiled space background behind the battle.
#[derive(Component)]
struct Background;

/// Stable content IDs of the battle sprites (`assets/manifest.json`).
pub mod ids {
    pub const SHIP_KITE: &str = "core.battle.ship.kite";
    pub const SHIP_BULWARK: &str = "core.battle.ship.bulwark";
    pub const ENEMY_SWARMER: &str = "core.battle.enemy.void_swarmer";
    pub const ENEMY_SPITTER: &str = "core.battle.enemy.void_spitter";
    pub const LOOT_CREDITS: &str = crate::art::ids::ICON_CREDITS;
    pub const LOOT_VOID_CRYSTAL: &str = crate::art::ids::ICON_VOID_CRYSTAL;
    pub const FX_SPIT: &str = "core.battle.fx.spit";
    pub const FISSURE: &str = "core.battle.env.void_fissure";
    pub const BACKGROUND: &str = "core.battle.env.background_tile";
}

/// Default sprite size: the longest side is this many times the sim hit
/// circle's diameter, so what you see is close to what gets hit (TAKOAI-50).
const SPRITE_SCALE: f32 = 1.2;

/// How big each sprite is drawn: its longest side is `scale` times the sim
/// hit circle's diameter.
struct SpriteArt {
    id: &'static str,
    scale: f32,
    /// Visual spin in radians/second.
    spin: f32,
}

const SPRITES: &[SpriteArt] = &[
    SpriteArt {
        id: ids::SHIP_KITE,
        scale: SPRITE_SCALE,
        spin: 0.0,
    },
    SpriteArt {
        id: ids::SHIP_BULWARK,
        scale: SPRITE_SCALE,
        spin: 0.0,
    },
    SpriteArt {
        id: ids::ENEMY_SWARMER,
        scale: SPRITE_SCALE,
        spin: 1.5,
    },
    SpriteArt {
        id: ids::ENEMY_SPITTER,
        scale: SPRITE_SCALE,
        spin: 0.0,
    },
    SpriteArt {
        id: ids::LOOT_CREDITS,
        scale: SPRITE_SCALE,
        spin: 0.0,
    },
    SpriteArt {
        id: ids::LOOT_VOID_CRYSTAL,
        scale: SPRITE_SCALE,
        spin: 0.0,
    },
    SpriteArt {
        id: ids::FX_SPIT,
        scale: SPRITE_SCALE,
        spin: 0.0,
    },
    // Fissures have no hit circle; drawn at about their spawn ring's size.
    SpriteArt {
        id: ids::FISSURE,
        scale: 1.0,
        spin: 0.2,
    },
];

/// Every manifest image the battle draws.
#[cfg(test)]
pub fn sprite_ids() -> Vec<&'static str> {
    SPRITES
        .iter()
        .map(|s| s.id)
        .chain([ids::BACKGROUND])
        .collect()
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
const ARENA_EDGE: Color = Color::srgb(0.25, 0.3, 0.4);
/// Off-screen fissure pointer (the fissure art's magenta).
const POINTER_COLOR: Color = Color::srgb(0.95, 0.24, 0.88);
/// Background tiles cover the arena plus this much beyond each edge, so the
/// camera never sees past them.
const BACKGROUND_MARGIN: f32 = 800.0;

/// Fixed-point sim position -> Bevy world units (1 unit = 1 px). The only
/// place floats meet sim state, and it is one-way.
fn to_world(p: FxVec2) -> Vec2 {
    Vec2::new(p.x as f32 / SUB as f32, p.y as f32 / SUB as f32)
}

/// The handle whose ship the camera and HUD follow (first local player).
fn local_handle(local: Option<&LocalPlayers>) -> usize {
    local.and_then(|l| l.0.first().copied()).unwrap_or(0)
}

fn ship_id(kind: ShipKind) -> &'static str {
    match kind {
        ShipKind::Kite => ids::SHIP_KITE,
        ShipKind::Bulwark => ids::SHIP_BULWARK,
    }
}

fn enemy_id(kind: EnemyKind) -> &'static str {
    match kind {
        EnemyKind::Swarmer => ids::ENEMY_SWARMER,
        EnemyKind::Spitter => ids::ENEMY_SPITTER,
    }
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

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Background,
        Sprite::default(),
        Transform::from_xyz(0.0, 0.0, -10.0),
        Visibility::Hidden,
    ));
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
            left: px(10),
            top: px(10),
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
    if ship.kind == ShipKind::Bulwark && ship.w_cooldown + 12 > SHOCKWAVE_COOLDOWN {
        // Shockwave: a ring that expands over the first 12 ticks.
        let age = (SHOCKWAVE_COOLDOWN - ship.w_cooldown) as f32 / 12.0;
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
        let t = to_world(ship.target);
        gizmos.line_2d(t - Vec2::splat(4.0), t + Vec2::splat(4.0), color);
        gizmos.line_2d(t + Vec2::new(-4.0, 4.0), t + Vec2::new(4.0, -4.0), color);
    }
}

fn draw_world(mut gizmos: Gizmos, world: Option<Res<SimWorld>>, local: Option<Res<LocalPlayers>>) {
    let Some(world) = world else { return };
    let (hw, hh) = (ARENA_HALF_W as f32, ARENA_HALF_H as f32);
    gizmos.linestrip_2d(
        [
            Vec2::new(-hw, -hh),
            Vec2::new(hw, -hh),
            Vec2::new(hw, hh),
            Vec2::new(-hw, hh),
            Vec2::new(-hw, -hh),
        ],
        ARENA_EDGE,
    );
    for e in &world.enemies {
        if e.hp < e.kind.hp() {
            let c = to_world(e.pos);
            let r = e.kind.radius() as f32;
            bar(
                &mut gizmos,
                c + Vec2::new(0.0, r * SPRITE_SCALE + 6.0),
                2.0 * r,
                e.hp as f32 / e.kind.hp() as f32,
                Color::srgb(1.0, 0.4, 0.4),
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

/// Direction `v` as a sprite rotation, for art drawn pointing up.
fn heading(v: Vec2) -> f32 {
    v.y.atan2(v.x) - std::f32::consts::FRAC_PI_2
}

#[allow(clippy::too_many_arguments)]
fn draw_sprites(
    mut commands: Commands,
    world: Option<Res<SimWorld>>,
    time: Res<Time>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<BattleSprites>,
    mut sprites: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<BattleSprite>>,
) {
    let mut draws = Vec::new();
    if let Some(world) = world.as_deref() {
        let t = time.elapsed_secs(); // visual only
        let mut push = |id: &str, pos: Vec2, radius: f32, angle: f32, color: Color, z: f32| {
            let Some(spec) = SPRITES.iter().find(|s| s.id == id) else {
                return;
            };
            if let Some(image) = art.get(&mut images, id) {
                draws.push(Draw {
                    image,
                    pos,
                    size: 2.0 * radius * spec.scale,
                    angle: angle + spec.spin * t,
                    color,
                    z,
                });
            }
        };
        for f in &world.fissures {
            push(
                ids::FISSURE,
                to_world(f.pos),
                FISSURE_RADIUS as f32,
                0.0,
                Color::WHITE,
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
        for e in &world.enemies {
            push(
                enemy_id(e.kind),
                to_world(e.pos),
                e.kind.radius() as f32,
                0.0,
                Color::WHITE,
                0.2,
            );
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
            let color = if ship.alive() {
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
        let glow = state
            .glow
            .get_or_insert_with(|| images.add(glow_image()))
            .clone();
        for p in &world.projectiles {
            draws.push(Draw {
                image: glow.clone(),
                pos: to_world(p.pos),
                size: 2.0 * p.radius as f32 * BOLT_GLOW_SCALE,
                angle: 0.0,
                color: PLAYER_COLORS[p.owner % PLAYER_COLORS.len()],
                z: 0.5,
            });
        }
    }

    for (i, d) in draws.iter().enumerate() {
        let px = ContentImages::size(&images, &d.image);
        let size = px * (d.size / px.max_element());
        let tf = Transform::from_translation(d.pos.extend(d.z))
            .with_rotation(Quat::from_rotation_z(d.angle));
        let sprite = Sprite {
            image: d.image.clone(),
            color: d.color,
            custom_size: Some(size),
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

/// Show the tiled background while a battle world exists.
fn battle_background(
    world: Option<Res<SimWorld>>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    mut bg: Query<(&mut Sprite, &mut Visibility), With<Background>>,
) {
    let Ok((mut sprite, mut vis)) = bg.single_mut() else {
        return;
    };
    let want = if world.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *vis != want {
        *vis = want;
    }
    if world.is_none() || sprite.custom_size.is_some() {
        return;
    }
    let Some(image) = art.get(&mut images, ids::BACKGROUND) else {
        return;
    };
    *sprite = Sprite {
        image,
        custom_size: Some(Vec2::new(
            2.0 * (ARENA_HALF_W as f32 + BACKGROUND_MARGIN),
            2.0 * (ARENA_HALF_H as f32 + BACKGROUND_MARGIN),
        )),
        image_mode: SpriteImageMode::Tiled {
            tile_x: true,
            tile_y: true,
            stretch_value: 1.0,
        },
        ..default()
    };
}

/// Where the off-screen fissure pointer goes: `None` while any fissure is
/// (even partly) on screen, else a point just inside the screen edge toward
/// the nearest fissure and the unit direction to it. `half` is half the
/// visible world area, `inset` how far inside the edge the point sits.
pub fn fissure_pointer(
    centre: Vec2,
    half: Vec2,
    inset: f32,
    fissures: &[Vec2],
) -> Option<(Vec2, Vec2)> {
    let r = FISSURE_RADIUS as f32;
    let on_screen = |f: &Vec2| {
        let d = (*f - centre).abs();
        d.x <= half.x + r && d.y <= half.y + r
    };
    if fissures.is_empty() || fissures.iter().any(on_screen) {
        return None;
    }
    let nearest = fissures.iter().min_by(|a, b| {
        a.distance_squared(centre)
            .total_cmp(&b.distance_squared(centre))
    })?;
    let dir = (*nearest - centre).normalize_or_zero();
    if dir == Vec2::ZERO {
        return None;
    }
    let edge = (half - Vec2::splat(inset)).max(Vec2::ONE);
    let t = (edge.x / dir.x.abs()).min(edge.y / dir.y.abs());
    Some((centre + dir * t, dir))
}

/// While no fissure is on screen, an arrow at the screen edge points to the
/// nearest one. Client-only.
fn draw_fissure_pointer(
    mut gizmos: Gizmos,
    world: Option<Res<SimWorld>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Transform, &Projection), With<Camera2d>>,
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
    let fissures: Vec<Vec2> = world.fissures.iter().map(|f| to_world(f.pos)).collect();
    let Some((tip, dir)) =
        fissure_pointer(cam.translation.truncate(), half, 24.0 * scale, &fissures)
    else {
        return;
    };
    // A solid-looking chevron: nested outlines shrinking toward the tip.
    let side = dir.perp();
    let size = 22.0 * scale;
    for k in 0..6 {
        let s = size * (1.0 - k as f32 * 0.15);
        let base = tip - dir * s;
        gizmos.linestrip_2d(
            [tip, base + side * s * 0.6, base - side * s * 0.6, tip],
            POINTER_COLOR,
        );
    }
}

/// Zoom out while a battle world exists, back to 1:1 when it goes (the
/// Carrier sets its own zoom on entry).
fn battle_zoom(world: Option<Res<SimWorld>>, mut camera: Query<&mut Projection, With<Camera2d>>) {
    let Ok(mut projection) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(ortho) = &mut *projection else {
        return;
    };
    let want = match world {
        Some(w) if w.is_added() => BATTLE_ZOOM,
        None if ortho.scale == BATTLE_ZOOM => 1.0,
        _ => return,
    };
    ortho.scale = want;
}

/// Camera follows our ship with a damped spring.
#[derive(Resource, Default)]
struct CameraFollow {
    vel: Vec2,
}

const CAMERA_STICKY: f32 = 0.3;

fn follow_camera(
    world: Option<Res<SimWorld>>,
    local: Option<Res<LocalPlayers>>,
    mut follow: ResMut<CameraFollow>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
) {
    let (Some(world), Ok(mut cam)) = (world, camera.single_mut()) else {
        return;
    };
    let Some(ship) = world.ships.get(local_handle(local.as_deref())) else {
        return;
    };
    let pos = cam.translation.truncate();
    let next = pos + follow.vel;
    follow.vel = follow.vel * CAMERA_STICKY + (to_world(ship.pos) - next) * (1.0 - CAMERA_STICKY);
    cam.translation = next.extend(cam.translation.z);
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
    s += &format!(
        "\nKills {}/{KILL_TARGET} - Objective: destroy {KILL_TARGET} void monsters\nTime {}:{:02}   Enemies {}\nLoot: {} credits, {} Void Crystal",
        m.kills,
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

fn update_banner(mut banner: Query<&mut Text, With<Banner>>, world: Option<Res<SimWorld>>) {
    let Ok(mut text) = banner.single_mut() else {
        return;
    };
    let s = match world.as_deref().map(|w| &w.mission) {
        Some(m) if m.status == MissionStatus::Success => "MISSION SUCCESS".to_string(),
        Some(m) if m.status == MissionStatus::Failed => "MISSION FAILED".to_string(),
        _ => String::new(),
    };
    if text.0 != s {
        text.0 = s;
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
            enemy_id(EnemyKind::Swarmer),
            enemy_id(EnemyKind::Spitter),
            loot_id(LootKind::Credits),
            loot_id(LootKind::VoidCrystal),
            ids::FX_SPIT,
            ids::FISSURE,
        ] {
            assert!(SPRITES.iter().any(|p| p.id == id), "{id}");
        }
    }

    #[test]
    fn sim_starting_view_matches_the_battle_camera() {
        // The sim keeps fissures off the starting screen using its own copy
        // of the view size; it must match the default window at battle zoom.
        use sim::tuning::{INITIAL_VIEW_HALF_H, INITIAL_VIEW_HALF_W};
        let half = crate::DEFAULT_WINDOW.as_vec2() * BATTLE_ZOOM / 2.0;
        assert_eq!(half.x.round() as i32, INITIAL_VIEW_HALF_W);
        assert_eq!(half.y.round() as i32, INITIAL_VIEW_HALF_H);
        // ... and that view is about 40% of the arena.
        let frac = half.x * half.y / (ARENA_HALF_W * ARENA_HALF_H) as f32;
        assert!((0.38..=0.42).contains(&frac), "{frac}");
    }

    #[test]
    fn pointer_shows_only_while_every_fissure_is_off_screen() {
        let half = Vec2::new(600.0, 350.0);
        let far_right = Vec2::new(900.0, 0.0);
        let far_up_left = Vec2::new(-700.0, 700.0);
        // On screen (even just the edge of its sprite): no pointer.
        assert_eq!(
            fissure_pointer(Vec2::ZERO, half, 20.0, &[Vec2::new(620.0, 0.0)]),
            None
        );
        assert_eq!(
            fissure_pointer(Vec2::ZERO, half, 20.0, &[far_right, Vec2::new(0.0, 300.0)]),
            None
        );
        assert_eq!(fissure_pointer(Vec2::ZERO, half, 20.0, &[]), None);
        // Off screen: the arrow sits on the edge toward the nearest one.
        let (tip, dir) =
            fissure_pointer(Vec2::ZERO, half, 20.0, &[far_up_left, far_right]).unwrap();
        assert_eq!(dir, Vec2::X);
        assert_eq!(tip, Vec2::new(580.0, 0.0));
        let (tip, dir) =
            fissure_pointer(Vec2::new(-650.0, 0.0), half, 20.0, &[far_up_left]).unwrap();
        assert!(dir.y > 0.99 && (tip.y - 330.0).abs() < 1e-3, "{tip} {dir}");
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
        w.enemies
            .push(sim::Enemy::new(EnemyKind::Swarmer, FxVec2::ZERO));
        let hud = battle_hud(&w, 0);
        assert!(hud.contains("140/140"), "{hud}");
        assert!(hud.contains("Kills 7/20"), "{hud}");
        assert!(hud.contains("Time 1:15   Enemies 1"), "{hud}");
        assert!(!hud.contains("Wave"), "{hud}");
        assert!(
            hud.contains("Q Bastion: ready") && hud.contains("W Shockwave: 1.5s"),
            "{hud}"
        );
        assert!(hud.contains("35 credits"), "{hud}");
    }
}
