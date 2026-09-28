//! Placeholder visuals, camera and HUD. Runs in `Update`, outside the rollback
//! schedule, and only reads `SimWorld`. Real art comes later from `design/art/`.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_ggrs::LocalPlayers;
use sim::tuning::{ENEMY_RADIUS, SHIP_RADIUS, SHOT_RADIUS};
use sim::{FxVec2, SUB};

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
const ENEMY: Color = Color::srgb(1.0, 0.35, 0.35);

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
    let spin = time.elapsed_secs() * 3.0; // visual only
    for e in &world.enemies {
        polygon(
            &mut gizmos,
            to_world(e.pos),
            ENEMY_RADIUS as f32,
            3,
            spin,
            ENEMY,
        );
    }
    for p in &world.projectiles {
        let color = SHIP_COLORS[p.owner % SHIP_COLORS.len()];
        gizmos.circle_2d(to_world(p.pos), SHOT_RADIUS as f32, color);
    }
    let me = local_handle(local.as_deref());
    for ship in &world.ships {
        let color = SHIP_COLORS[ship.handle % SHIP_COLORS.len()];
        let p = to_world(ship.pos);
        gizmos.circle_2d(p, SHIP_RADIUS as f32, color);
        gizmos.circle_2d(p, SHIP_RADIUS as f32 - 4.0, color);
        // Click-to-move destination marker for our own ship.
        if ship.handle == me && ship.target != ship.pos {
            let t = to_world(ship.target);
            gizmos.line_2d(t - Vec2::splat(4.0), t + Vec2::splat(4.0), color);
            gizmos.line_2d(t + Vec2::new(-4.0, 4.0), t + Vec2::new(4.0, -4.0), color);
        }
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
    if let Some(world) = world {
        let me = local_handle(local.as_deref());
        if let Some(ship) = world.ships.get(me) {
            let p = to_world(ship.pos);
            s += &format!("\nP{} X:{:.0} Y:{:.0}", me + 1, p.x, p.y);
        }
        s += &format!(
            " | wave {} | enemies {} | shots {}",
            world.wave,
            world.enemies.len(),
            world.projectiles.len()
        );
    }
    text.0 = s;
}
