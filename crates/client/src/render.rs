//! Placeholder visuals, camera and HUD. Runs in `Update`, outside the rollback
//! schedule, and only reads `SimWorld`.
//!
//! Every battle visual is drawn through a stable content ID (`PLACEHOLDERS`),
//! so the art import (TAKOAI-49) swaps a gizmo shape for a sprite with the
//! same ID instead of reworking the drawing code.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_ggrs::{LocalPlayers, Session};
use sim::tuning::{
    ARENA_HALF_H, ARENA_HALF_W, KILL_TARGET, SHOCKWAVE_COOLDOWN, SHOCKWAVE_RADIUS, TICKS_PER_SEC,
    WAVE_COUNT,
};
use sim::{EnemyKind, FxVec2, LootKind, MissionStatus, Ship, ShipKind, SimState, SUB};

use crate::rollback::{GameConfig, SimWorld};
use crate::stats::Stats;
use crate::{ContentStatus, NetStatus};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .insert_resource(ClearColor(Color::BLACK))
            .init_resource::<CameraFollow>()
            .add_systems(Startup, setup_scene)
            .add_systems(
                Update,
                (draw_world, follow_camera, update_hud, update_banner),
            );
    }
}

#[derive(Component)]
struct Hud;

#[derive(Component)]
struct Banner;

/// Stable content IDs of the battle visuals. The art set will ship assets
/// under these IDs (`assets/manifest.json`); until then each is a shape.
pub mod ids {
    pub const SHIP_KITE: &str = "core.battle.ship.kite";
    pub const SHIP_BULWARK: &str = "core.battle.ship.bulwark";
    pub const ENEMY_SWARMER: &str = "core.battle.enemy.void_swarmer";
    pub const ENEMY_SPITTER: &str = "core.battle.enemy.void_spitter";
    pub const LOOT_CREDITS: &str = "core.battle.loot.credits";
    pub const LOOT_VOID_CRYSTAL: &str = "core.battle.loot.void_crystal";
    pub const FX_BOLT: &str = "core.battle.fx.bolt";
    pub const FX_SPIT: &str = "core.battle.fx.spit";
    pub const FX_SHIELD: &str = "core.battle.fx.bastion_shield";
    pub const FX_SHOCKWAVE: &str = "core.battle.fx.shockwave";
}

/// How a content ID is drawn until its art exists.
struct Placeholder {
    id: &'static str,
    /// Polygon sides (0 = circle).
    sides: usize,
    /// Rotation speed in radians/second (visual only).
    spin: f32,
    color: Color,
}

const PLACEHOLDERS: &[Placeholder] = &[
    Placeholder {
        id: ids::SHIP_KITE,
        sides: 3,
        spin: 0.0,
        color: Color::srgb(0.3, 0.8, 1.0),
    },
    Placeholder {
        id: ids::SHIP_BULWARK,
        sides: 6,
        spin: 0.0,
        color: Color::srgb(0.4, 0.6, 1.0),
    },
    Placeholder {
        id: ids::ENEMY_SWARMER,
        sides: 3,
        spin: 4.0,
        color: Color::srgb(1.0, 0.35, 0.35),
    },
    Placeholder {
        id: ids::ENEMY_SPITTER,
        sides: 5,
        spin: 1.0,
        color: Color::srgb(0.85, 0.4, 1.0),
    },
    Placeholder {
        id: ids::LOOT_CREDITS,
        sides: 0,
        spin: 0.0,
        color: Color::srgb(1.0, 0.85, 0.2),
    },
    Placeholder {
        id: ids::LOOT_VOID_CRYSTAL,
        sides: 4,
        spin: 2.0,
        color: Color::srgb(0.4, 1.0, 0.9),
    },
    Placeholder {
        id: ids::FX_BOLT,
        sides: 0,
        spin: 0.0,
        color: Color::srgb(0.9, 0.95, 1.0),
    },
    Placeholder {
        id: ids::FX_SPIT,
        sides: 0,
        spin: 0.0,
        color: Color::srgb(0.7, 1.0, 0.2),
    },
    Placeholder {
        id: ids::FX_SHIELD,
        sides: 0,
        spin: 0.0,
        color: Color::srgb(0.5, 0.8, 1.0),
    },
    Placeholder {
        id: ids::FX_SHOCKWAVE,
        sides: 0,
        spin: 0.0,
        color: Color::srgb(0.6, 0.8, 1.0),
    },
];

/// Per-player tint on top of the ship shape.
const PLAYER_COLORS: [Color; 4] = [
    Color::srgb(0.3, 0.8, 1.0),
    Color::srgb(1.0, 0.6, 0.2),
    Color::srgb(0.5, 1.0, 0.4),
    Color::srgb(1.0, 0.4, 0.9),
];
const ARENA_EDGE: Color = Color::srgb(0.25, 0.3, 0.4);

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

/// Closed regular polygon outline (or circle for 0 sides).
fn shape(gizmos: &mut Gizmos, c: Vec2, r: f32, sides: usize, angle: f32, color: Color) {
    if sides == 0 {
        gizmos.circle_2d(c, r, color);
        return;
    }
    let pts = (0..=sides).map(|i| {
        let a = angle + i as f32 * std::f32::consts::TAU / sides as f32;
        c + Vec2::new(a.cos(), a.sin()) * r
    });
    gizmos.linestrip_2d(pts, color);
}

/// Draw content `id` at `c` with radius `r`. `tint` overrides its color.
fn draw_id(gizmos: &mut Gizmos, id: &str, c: Vec2, r: f32, t: f32, tint: Option<Color>) {
    let Some(p) = PLACEHOLDERS.iter().find(|p| p.id == id) else {
        gizmos.circle_2d(c, r, Color::WHITE);
        return;
    };
    let angle = std::f32::consts::FRAC_PI_2 + p.spin * t;
    shape(gizmos, c, r, p.sides, angle, tint.unwrap_or(p.color));
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

fn draw_ship(gizmos: &mut Gizmos, ship: &Ship, me: usize, t: f32) {
    let color = PLAYER_COLORS[ship.handle % PLAYER_COLORS.len()];
    let p = to_world(ship.pos);
    let r = ship.stats.radius as f32;
    if !ship.alive() {
        let grey = Color::srgb(0.4, 0.4, 0.4);
        gizmos.line_2d(p - Vec2::splat(r), p + Vec2::splat(r), grey);
        gizmos.line_2d(p + Vec2::new(-r, r), p + Vec2::new(r, -r), grey);
        return;
    }
    draw_id(gizmos, ship_id(ship.kind), p, r, t, Some(color));
    draw_id(gizmos, ship_id(ship.kind), p, r - 4.0, t, Some(color));
    if ship.shield > 0 {
        draw_id(gizmos, ids::FX_SHIELD, p, r + 6.0, t, None);
    }
    if ship.kind == ShipKind::Bulwark && ship.w_cooldown + 12 > SHOCKWAVE_COOLDOWN {
        // Shockwave: a ring that expands over the first 12 ticks.
        let age = (SHOCKWAVE_COOLDOWN - ship.w_cooldown) as f32 / 12.0;
        draw_id(
            gizmos,
            ids::FX_SHOCKWAVE,
            p,
            SHOCKWAVE_RADIUS as f32 * age.min(1.0),
            t,
            None,
        );
    }
    if ship.dash_ticks > 0 {
        let back = to_world(ship.pos - ship.dash_vel) - p;
        gizmos.line_2d(p, p + back * 3.0, color);
    }
    bar(
        gizmos,
        p + Vec2::new(0.0, r + 10.0),
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

fn draw_world(
    mut gizmos: Gizmos,
    world: Option<Res<SimWorld>>,
    local: Option<Res<LocalPlayers>>,
    time: Res<Time>,
) {
    let Some(world) = world else { return };
    let t = time.elapsed_secs(); // visual only
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
    for p in &world.pickups {
        draw_id(&mut gizmos, loot_id(p.kind), to_world(p.pos), 5.0, t, None);
    }
    for e in &world.enemies {
        let c = to_world(e.pos);
        let r = e.kind.radius() as f32;
        draw_id(&mut gizmos, enemy_id(e.kind), c, r, t, None);
        if e.hp < e.kind.hp() {
            bar(
                &mut gizmos,
                c + Vec2::new(0.0, r + 5.0),
                2.0 * r,
                e.hp as f32 / e.kind.hp() as f32,
                Color::srgb(1.0, 0.4, 0.4),
            );
        }
    }
    for p in &world.projectiles {
        let tint = PLAYER_COLORS[p.owner % PLAYER_COLORS.len()];
        draw_id(
            &mut gizmos,
            ids::FX_BOLT,
            to_world(p.pos),
            p.radius as f32,
            t,
            Some(tint),
        );
    }
    for p in &world.enemy_projectiles {
        draw_id(
            &mut gizmos,
            ids::FX_SPIT,
            to_world(p.pos),
            p.radius as f32,
            t,
            None,
        );
    }
    let me = local_handle(local.as_deref());
    for ship in &world.ships {
        draw_ship(&mut gizmos, ship, me, t);
    }
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

/// Battle HUD: hull, kills/objective, wave, Q/W cooldowns, loot so far.
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
    s += &format!(
        "\nKills {}/{KILL_TARGET} - Objective: destroy {KILL_TARGET} void monsters\nWave {}/{WAVE_COUNT}\nLoot: {} credits, {} Void Crystal",
        m.kills, m.wave, m.collected.credits, m.collected.void_crystal
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
        Some(m) if m.banner_up() => format!("Wave {}", m.wave),
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
    fn every_battle_visual_has_a_placeholder() {
        for id in [
            ship_id(ShipKind::Kite),
            ship_id(ShipKind::Bulwark),
            enemy_id(EnemyKind::Swarmer),
            enemy_id(EnemyKind::Spitter),
            loot_id(LootKind::Credits),
            loot_id(LootKind::VoidCrystal),
            ids::FX_BOLT,
            ids::FX_SPIT,
            ids::FX_SHIELD,
            ids::FX_SHOCKWAVE,
        ] {
            assert!(PLACEHOLDERS.iter().any(|p| p.id == id), "{id}");
        }
    }

    #[test]
    fn hud_shows_hull_kills_wave_cooldowns_and_loot() {
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
        let hud = battle_hud(&w, 0);
        assert!(hud.contains("140/140"), "{hud}");
        assert!(hud.contains("Kills 7/20"), "{hud}");
        assert!(hud.contains("Wave 1/3"), "{hud}");
        assert!(
            hud.contains("Q Bastion: ready") && hud.contains("W Shockwave: 1.5s"),
            "{hud}"
        );
        assert!(hud.contains("35 credits"), "{hud}");
    }
}
