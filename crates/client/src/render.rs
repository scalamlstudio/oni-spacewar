//! Placeholder visuals, camera and HUD. Runs in `Update`, outside the rollback
//! schedule, and only reads `SimWorld`. Shapes and colours follow the LÖVE2D
//! prototype (`src/class/shape/*.lua`); real art comes later from `design/art/`.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_ggrs::LocalPlayers;
use sim::content::{
    ObstacleKind, ProjectileShape, IMMORTAL_HP, PORTAL_RADIUS, SHIP_HP, SHIP_RADIUS,
};
use sim::{FxVec2, Side, SUB};

use crate::rollback::SimWorld;
use crate::stats::Stats;
use crate::NetStatus;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .insert_resource(ClearColor(Color::BLACK))
            .init_resource::<CameraFollow>()
            .add_systems(Startup, setup_scene)
            .add_systems(Update, (draw_world, follow_camera, update_hud));
    }
}

#[derive(Component)]
struct Hud;

const SHIP_COLORS: [Color; 4] = [
    Color::srgb(0.3, 0.8, 1.0),
    Color::srgb(1.0, 0.6, 0.2),
    Color::srgb(0.5, 1.0, 0.4),
    Color::srgb(1.0, 0.4, 0.9),
];
const WHITE: Color = Color::srgb(0.9, 0.9, 0.9);
const MAGENTA: Color = Color::srgb(0.9, 0.0, 0.9);
const ORANGE: Color = Color::srgb(1.0, 0.7, 0.2);
const ENEMY_SHOT: Color = Color::srgb(1.0, 0.35, 0.35);

/// Fixed-point sim position -> Bevy world units (1 unit = 1 px). The only
/// place floats meet sim state, and it is one-way.
fn to_world(p: FxVec2) -> Vec2 {
    Vec2::new(p.x as f32 / SUB as f32, p.y as f32 / SUB as f32)
}

/// The handle whose ship the camera and HUD follow (first local player).
fn local_handle(local: Option<&LocalPlayers>) -> usize {
    local.and_then(|l| l.0.first().copied()).unwrap_or(0)
}

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Hud,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.0, 1.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            left: px(10),
            top: px(10),
            ..default()
        },
    ));
}

/// Closed regular polygon outline, spinning with `angle` (visual only).
fn polygon(gizmos: &mut Gizmos, c: Vec2, r: f32, sides: usize, angle: f32, color: Color) {
    let pts = (0..=sides).map(|i| {
        let a = angle + i as f32 * std::f32::consts::TAU / sides as f32;
        c + Vec2::new(a.cos(), a.sin()) * r
    });
    gizmos.linestrip_2d(pts, color);
}

fn draw_world(
    mut gizmos: Gizmos,
    world: Option<Res<SimWorld>>,
    local: Option<Res<LocalPlayers>>,
    time: Res<Time>,
) {
    let Some(world) = world else { return };
    // The prototype spins triangles 5°/frame and hexagons 0.5°/frame.
    let t = time.elapsed_secs();
    let (tri_spin, hex_spin) = (t * 5.0, t * 0.5);

    for o in &world.obstacles {
        let size = o.kind.def().size as f32;
        let color = if o.hp > IMMORTAL_HP / 10 {
            ORANGE
        } else {
            let f = 0.9 * o.hp as f32 / o.kind.def().hp as f32;
            Color::srgb(0.9, f, f)
        };
        gizmos.rect_2d(to_world(o.pos), Vec2::splat(size), color);
        if o.kind == ObstacleKind::O1 {
            gizmos.rect_2d(to_world(o.pos), Vec2::splat(size - 8.0), color);
        }
    }
    for p in &world.portals {
        polygon(
            &mut gizmos,
            to_world(p.pos),
            PORTAL_RADIUS as f32,
            6,
            hex_spin,
            MAGENTA,
        );
    }
    for i in &world.items {
        polygon(
            &mut gizmos,
            to_world(i.pos),
            PORTAL_RADIUS as f32 / 2.0,
            6,
            hex_spin,
            MAGENTA,
        );
    }
    for e in &world.enemies {
        polygon(
            &mut gizmos,
            to_world(e.pos),
            e.kind.def().radius as f32,
            3,
            tri_spin,
            WHITE,
        );
    }
    for p in &world.projectiles {
        let def = p.kind.def();
        let color = if p.side == Side::Enemy {
            ENEMY_SHOT
        } else {
            WHITE
        };
        let r = def.radius as f32;
        match def.shape {
            ProjectileShape::Circle => {
                gizmos.circle_2d(to_world(p.pos), r, color);
            }
            ProjectileShape::Triangle => {
                polygon(&mut gizmos, to_world(p.pos), r, 3, tri_spin, color)
            }
        }
    }
    let me = local_handle(local.as_deref());
    for ship in world.ships.iter().filter(|s| s.alive) {
        let color = SHIP_COLORS[ship.handle % SHIP_COLORS.len()];
        let p = to_world(ship.pos);
        gizmos.circle_2d(p, SHIP_RADIUS as f32, color);
        gizmos.circle_2d(p, SHIP_RADIUS as f32 - 3.0, color);
        // Click-to-move destination marker for our own ship.
        if ship.handle == me && ship.target != ship.pos {
            let t = to_world(ship.target);
            gizmos.line_2d(t - Vec2::splat(4.0), t + Vec2::splat(4.0), color);
            gizmos.line_2d(t + Vec2::new(-4.0, 4.0), t + Vec2::new(4.0, -4.0), color);
        }
    }
}

/// Camera state from `src/general/camera.lua` (follow with a "sticky" spring).
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
    // Same per-frame update as `Camera:follow` (no vibration: nothing sets it).
    let pos = cam.translation.truncate();
    let next = pos + follow.vel;
    follow.vel = follow.vel * CAMERA_STICKY + (to_world(ship.pos) - next) * (1.0 - CAMERA_STICKY);
    cam.translation = next.extend(cam.translation.z);
}

fn update_hud(
    mut hud: Query<&mut Text, With<Hud>>,
    diagnostics: Res<DiagnosticsStore>,
    stats: Res<Stats>,
    status: Res<NetStatus>,
    world: Option<Res<SimWorld>>,
    local: Option<Res<LocalPlayers>>,
) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let mut s = format!(
        "{} | fps {:.0} | frame {} | rollbacks {} (max depth {}) | desyncs {} | mismatches {}",
        status.0,
        fps,
        stats.max_frame,
        stats.rollbacks,
        stats.max_rollback,
        stats.desyncs,
        stats.synctest_mismatches,
    );
    // The prototype's panel: level, own ship, object count.
    if let Some(world) = world {
        let me = local_handle(local.as_deref());
        if let Some(ship) = world.ships.get(me) {
            let p = to_world(ship.pos);
            s += &format!(
                "\nLevel {} | P{} X:{:.0} Y:{:.0} HP:{}/{}{}",
                world.level,
                me + 1,
                p.x,
                p.y,
                ship.hp.max(0),
                SHIP_HP,
                if ship.alive { "" } else { " (destroyed)" },
            );
        }
        s += &format!(
            "\nEnemies {} | Projectiles {} | Obstacles {}",
            world.enemies.len(),
            world.projectiles.len(),
            world.obstacles.len()
        );
    }
    text.0 = s;
}
