//! The procedural "magical night sky with nebula" background (TAKOAI-58):
//! one `Material2d` (`sky.wgsl`) on a quad that always covers the camera's
//! view. One sky for every scene (TAKOAI-83): behind the battle, the
//! Carrier's hull, and the Title and Result art (whose skies are
//! transparent). A new scene gets it without any code. Runs outside rollback
//! and never touches the sim: it reads only the camera and wall-clock time,
//! which is fine for visuals.

use bevy::asset::{embedded_asset, embedded_path, AssetPath};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{Material2d, Material2dPlugin};
use bevy::window::PrimaryWindow;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "sky.wgsl");
        app.add_plugins(Material2dPlugin::<SkyMaterial>::default())
            .add_systems(Startup, spawn_sky)
            .add_systems(PostUpdate, follow_view.before(TransformSystems::Propagate));
    }
}

/// Behind everything: the Carrier's hull starts at z -30, the battle's
/// sprites at z >= 0, and UI (Title / Result art) draws over all of it.
const Z_SKY: f32 = -500.0;
/// The quad covers the view plus this fraction, so it never shows an edge
/// while the camera moves within a frame.
const OVERSCAN: f32 = 1.1;

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct SkyParams {
    /// Camera centre in world px.
    pub camera: Vec2,
    /// Seconds since start (visual only).
    pub time: f32,
    pub brightness: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct SkyMaterial {
    #[uniform(0)]
    pub params: SkyParams,
}

impl Material2d for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("sky.wgsl")).with_source("embedded"),
        )
    }
}

#[derive(Component)]
struct Sky;

/// The 2D camera, read while the sky's own `Transform` is written.
type CameraView<'w, 's> =
    Query<'w, 's, (&'static Transform, &'static Projection), (With<Camera2d>, Without<Sky>)>;

fn spawn_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SkyMaterial>>,
) {
    commands.spawn((
        Sky,
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial2d(materials.add(SkyMaterial {
            params: SkyParams {
                brightness: 1.0,
                ..default()
            },
        })),
        Transform::from_xyz(0.0, 0.0, Z_SKY),
    ));
}

/// Keep the quad over the camera's view (at any window size, also while
/// resizing) and feed the shader the camera position (for parallax) and the
/// time (for drift and twinkle). Menu screens never move the camera, so
/// there the sky only drifts.
fn follow_view(
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: CameraView,
    mut sky: Query<(&mut Transform, &MeshMaterial2d<SkyMaterial>), With<Sky>>,
    mut materials: ResMut<Assets<SkyMaterial>>,
) {
    let (Ok((mut tf, material)), Ok(window), Ok((cam, projection))) =
        (sky.single_mut(), windows.single(), camera.single())
    else {
        return;
    };
    let scale = match projection {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    let centre = cam.translation.truncate();
    tf.translation = centre.extend(Z_SKY);
    tf.scale = (window.size() * scale * OVERSCAN).extend(1.0);
    if let Some(mut m) = materials.get_mut(&material.0) {
        m.params.camera = centre;
        m.params.time = time.elapsed_secs();
    }
}
