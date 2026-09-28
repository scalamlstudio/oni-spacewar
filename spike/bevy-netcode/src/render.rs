//! Placeholder visuals and HUD. Runs in `Update`/`PostUpdate`, never in the
//! rollback schedule: it only reads simulation state and converts fixed-point
//! positions to `Transform`s.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::sim::{
    swarmling_pos, Bullets, FxPos, Ship, ShipHealth, SimFrame, ARENA_HALF_H, ARENA_HALF_W,
    BULLET_KIND_RING, BULLET_RADIUS, SHIP_HITBOX, SUB, SWARMLINGS,
};
use crate::stats::Stats;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.05)))
            .add_systems(Startup, setup_scene)
            .add_systems(Update, (sync_bullet_sprites, draw_ships_and_emitters, update_hud));
    }
}

#[derive(Component)]
struct BulletSprite(usize);

#[derive(Component)]
struct Hud;

const SHIP_COLORS: [Color; 4] = [
    Color::srgb(0.3, 0.8, 1.0),
    Color::srgb(1.0, 0.6, 0.2),
    Color::srgb(0.5, 1.0, 0.4),
    Color::srgb(1.0, 0.4, 0.9),
];

fn to_world(x: i32, y: i32) -> Vec2 {
    Vec2::new(x as f32 / SUB as f32, y as f32 / SUB as f32)
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
            font_size: 14.0,
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

/// Keeps a pool of sprite entities, one per live bullet.
fn sync_bullet_sprites(
    mut commands: Commands,
    bullets: Option<Res<Bullets>>,
    mut sprites: Query<(&BulletSprite, &mut Transform, &mut Visibility, &mut Sprite)>,
) {
    let Some(bullets) = bullets else { return };
    let mut pool_len = 0;
    for (BulletSprite(i), mut tf, mut vis, mut sprite) in &mut sprites {
        pool_len += 1;
        if let Some(b) = bullets.0.get(*i) {
            tf.translation = to_world(b.x, b.y).extend(0.0);
            *vis = Visibility::Visible;
            sprite.color = bullet_color(b.kind);
        } else {
            *vis = Visibility::Hidden;
        }
    }
    let size = Vec2::splat(BULLET_RADIUS as f32 * 2.0);
    for i in pool_len..bullets.0.len() {
        let b = bullets.0[i];
        commands.spawn((
            BulletSprite(i),
            Sprite::from_color(bullet_color(b.kind), size),
            Transform::from_translation(to_world(b.x, b.y).extend(0.0)),
        ));
    }
}

fn bullet_color(kind: u8) -> Color {
    if kind == BULLET_KIND_RING {
        Color::srgb(1.0, 0.3, 0.35)
    } else {
        Color::srgb(1.0, 0.9, 0.3)
    }
}

fn draw_ships_and_emitters(
    mut gizmos: Gizmos,
    ships: Query<(&Ship, &FxPos, &ShipHealth)>,
    frame: Option<Res<SimFrame>>,
) {
    for (ship, pos, health) in &ships {
        let p = to_world(pos.x, pos.y);
        let color = SHIP_COLORS[ship.handle % SHIP_COLORS.len()];
        let body = if health.invuln > 0 && health.invuln % 4 < 2 {
            Color::WHITE
        } else {
            color
        };
        // Placeholder battleship: a triangle, with the real (tiny) hitbox as a dot.
        gizmos.linestrip_2d(
            [
                p + Vec2::new(0.0, 16.0),
                p + Vec2::new(-11.0, -10.0),
                p + Vec2::new(11.0, -10.0),
                p + Vec2::new(0.0, 16.0),
            ],
            body,
        );
        gizmos.circle_2d(p, SHIP_HITBOX as f32, Color::WHITE);
    }
    if let Some(frame) = frame {
        gizmos.circle_2d(to_world(0, 120 * SUB), 18.0, Color::srgb(0.8, 0.2, 0.3));
        for i in 0..SWARMLINGS {
            let (x, y) = swarmling_pos(i, frame.0);
            gizmos.circle_2d(to_world(x, y), 9.0, Color::srgb(0.9, 0.8, 0.2));
        }
    }
}

fn update_hud(
    mut hud: Query<&mut Text, With<Hud>>,
    diagnostics: Res<DiagnosticsStore>,
    stats: Res<Stats>,
    bullets: Option<Res<Bullets>>,
    ships: Query<(&Ship, &ShipHealth)>,
    status: Res<crate::NetStatus>,
) {
    let Ok(mut text) = hud.single_mut() else { return };
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let mut hits: Vec<_> = ships.iter().map(|(s, h)| (s.handle, h.hits_taken)).collect();
    hits.sort_unstable();
    let hits: Vec<String> = hits.iter().map(|(h, n)| format!("P{}:{}", h + 1, n)).collect();
    text.0 = format!(
        "{} | fps {:.0} | frame {} | bullets {} | rollbacks {} (max depth {}) | desyncs {} | mismatches {} | hits {}",
        status.0,
        fps,
        stats.max_frame,
        bullets.map_or(0, |b| b.0.len()),
        stats.rollbacks,
        stats.max_rollback,
        stats.desyncs,
        stats.synctest_mismatches,
        hits.join(" "),
    );
}
