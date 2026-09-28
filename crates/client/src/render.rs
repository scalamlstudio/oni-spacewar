//! Placeholder visuals and HUD. Runs in `Update`, outside the rollback
//! schedule, and only reads `SimWorld`. Real art comes later from `design/art/`.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use sim::state::{ARENA_HALF_H, ARENA_HALF_W};
use sim::{FxVec2, SUB};

use crate::rollback::SimWorld;
use crate::stats::Stats;
use crate::NetStatus;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.05)))
            .add_systems(Startup, setup_scene)
            .add_systems(Update, (draw_ships, update_hud));
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

/// Fixed-point sim position -> Bevy world units (1 unit = 1 px). The only
/// place floats meet sim state, and it is one-way.
fn to_world(p: FxVec2) -> Vec2 {
    Vec2::new(p.x as f32 / SUB as f32, p.y as f32 / SUB as f32)
}

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.06, 0.06, 0.12),
            Vec2::new(ARENA_HALF_W as f32 * 2.0, ARENA_HALF_H as f32 * 2.0),
        ),
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));
    commands.spawn((
        Hud,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: px(6),
            top: px(4),
            ..default()
        },
    ));
}

fn draw_ships(mut gizmos: Gizmos, world: Option<Res<SimWorld>>) {
    let Some(world) = world else { return };
    for ship in &world.ships {
        let p = to_world(ship.pos);
        let color = SHIP_COLORS[ship.handle % SHIP_COLORS.len()];
        // Placeholder battleship: a triangle.
        gizmos.linestrip_2d(
            [
                p + Vec2::new(0.0, 16.0),
                p + Vec2::new(-11.0, -10.0),
                p + Vec2::new(11.0, -10.0),
                p + Vec2::new(0.0, 16.0),
            ],
            color,
        );
    }
}

fn update_hud(
    mut hud: Query<&mut Text, With<Hud>>,
    diagnostics: Res<DiagnosticsStore>,
    stats: Res<Stats>,
    status: Res<NetStatus>,
) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    text.0 = format!(
        "{} | fps {:.0} | frame {} | rollbacks {} (max depth {}) | desyncs {} | mismatches {}",
        status.0,
        fps,
        stats.max_frame,
        stats.rollbacks,
        stats.max_rollback,
        stats.desyncs,
        stats.synctest_mismatches,
    );
}
