//! Player settings (TAKOAI-96): a local `settings.json` beside the save,
//! the Settings screen, and the carrier's floor and wall colours.
//!
//! Settings are per player and per machine. They are not part of the save
//! game and never reach `sim`, so nothing here can change a battle or
//! multiplayer sync. Floats and `powf` are fine here: all of it is visual.
//!
//! The Settings screen opens from its gear button (Title, Carrier HUD),
//! from Esc on the Title and from the Carrier's Esc (pause) menu, and
//! closes with Esc or Close. It is built from sections (`spawn_panel`):
//! Display › Carrier colours, and Codex, which opens the full-screen
//! Codex (`crate::codex`) on top.
//!
//! **Recolouring.** Art's carrier tiles (`core.carrier.tile.*`) are flat
//! scripted shapes in a handful of colours: the floor (`FLOOR_REF`, with
//! darker seams), the wall body (`WALL_REF`, with a darker trim) and the
//! cyan light line. [`CarrierPalette`] keeps a copy of each tile's pixels
//! and draws the tiles from its own images, made by [`recolour`]: every
//! pixel keeps its offset from the reference of its family, moved onto the
//! chosen floor or wall colour, so seams and trim stay as visible on any
//! colour; the cyan line is left alone. A colour change rewrites
//! those images in place, so the deck updates live without a redraw.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::art::ContentImages;
use crate::codex::CodexScreen;
use crate::flow::{GameScreen, PauseMenu};

/// Version of `settings.json`.
pub const SETTINGS_VERSION: u32 = 1;
const SETTINGS_FILE_NAME: &str = "settings.json";

/// An sRGB colour, stored as `"#RRGGBB"`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb(pub [u8; 3]);

impl Rgb {
    pub const fn hex(v: u32) -> Self {
        Self([(v >> 16) as u8, (v >> 8) as u8, v as u8])
    }

    pub fn to_hex(self) -> String {
        let [r, g, b] = self.0;
        format!("#{r:02X}{g:02X}{b:02X}")
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#')?;
        if s.len() != 6 {
            return None;
        }
        u32::from_str_radix(s, 16).ok().map(Self::hex)
    }

    pub fn color(self) -> Color {
        let [r, g, b] = self.0;
        Color::srgb_u8(r, g, b)
    }

    /// WCAG relative luminance, 0 (black) to 1 (white).
    pub fn luminance(self) -> f32 {
        let lin = |c: u8| {
            let c = c as f32 / 255.0;
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        let [r, g, b] = self.0;
        0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
    }
}

impl Serialize for Rgb {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("bad colour {s:?}")))
    }
}

/// WCAG contrast ratio of two colours' luminance, 1 to 21, either order.
pub fn contrast(a: Rgb, b: Rgb) -> f32 {
    let (la, lb) = (a.luminance(), b.luminance());
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Below this floor / wall contrast the panel warns (it never blocks).
pub const MIN_CONTRAST: f32 = 3.0;

/// A named swatch.
#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub name: &'static str,
    pub rgb: Rgb,
}

/// Floor swatches; the first is the default. "Office" is Art's v3 floor.
pub const FLOOR_PRESETS: [Preset; 6] = [
    Preset {
        name: "Slate",
        rgb: Rgb::hex(0x3A4350),
    },
    Preset {
        name: "Graphite",
        rgb: Rgb::hex(0x2A2E35),
    },
    Preset {
        name: "Deep navy",
        rgb: Rgb::hex(0x253246),
    },
    Preset {
        name: "Olive",
        rgb: Rgb::hex(0x3B4234),
    },
    Preset {
        name: "Rust",
        rgb: Rgb::hex(0x4A3B35),
    },
    Preset {
        name: "Office",
        rgb: FLOOR_REF,
    },
];

/// Wall swatches; the first is the default. "Navy" is Art's v3 wall.
pub const WALL_PRESETS: [Preset; 6] = [
    Preset {
        name: "Steel",
        rgb: Rgb::hex(0xA3B1C0),
    },
    Preset {
        name: "Bone",
        rgb: Rgb::hex(0xD6DAD2),
    },
    Preset {
        name: "Sand",
        rgb: Rgb::hex(0xC4AE88),
    },
    Preset {
        name: "Teal",
        rgb: Rgb::hex(0x5F9497),
    },
    Preset {
        name: "Navy",
        rgb: WALL_REF,
    },
    Preset {
        name: "Charcoal",
        rgb: Rgb::hex(0x1E2329),
    },
];

pub const DEFAULT_FLOOR: Rgb = FLOOR_PRESETS[0].rgb;
pub const DEFAULT_WALL: Rgb = WALL_PRESETS[0].rgb;

/// `settings.json`. New options go in new sections with `serde(default)`,
/// so older files keep loading.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub version: u32,
    #[serde(default)]
    pub display: Display,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Display {
    pub carrier_floor: Rgb,
    pub carrier_wall: Rgb,
}

impl Default for Display {
    fn default() -> Self {
        Self {
            carrier_floor: DEFAULT_FLOOR,
            carrier_wall: DEFAULT_WALL,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            display: Display::default(),
        }
    }
}

#[derive(Debug)]
pub enum SettingsError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Version(u32),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Json(e) => write!(f, "{e}"),
            Self::Version(v) => write!(f, "settings version {v} is newer than this build"),
        }
    }
}

impl From<std::io::Error> for SettingsError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for SettingsError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// `settings.json` beside the save (so `ONI_SAVE_DIR` moves both).
pub fn settings_path() -> PathBuf {
    crate::save::save_path().with_file_name(SETTINGS_FILE_NAME)
}

pub fn from_json(bytes: &[u8]) -> Result<Settings, SettingsError> {
    let settings: Settings = serde_json::from_slice(bytes)?;
    if settings.version > SETTINGS_VERSION {
        return Err(SettingsError::Version(settings.version));
    }
    Ok(settings)
}

pub fn load(path: &Path) -> Result<Settings, SettingsError> {
    from_json(&std::fs::read(path)?)
}

pub fn store(path: &Path, settings: &Settings) -> Result<(), SettingsError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(settings)?)?;
    Ok(())
}

/// The settings in use and where they are stored. A missing or unreadable
/// file means the defaults (logged), and is rewritten on the first change.
#[derive(Resource)]
pub struct SettingsFile {
    path: PathBuf,
    pub settings: Settings,
}

impl SettingsFile {
    pub fn load() -> Self {
        let path = settings_path();
        let settings = match load(&path) {
            Ok(s) => s,
            Err(SettingsError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                Settings::default()
            }
            Err(e) => {
                warn!(
                    "settings {} unreadable, using defaults: {e}",
                    path.display()
                );
                Settings::default()
            }
        };
        Self { path, settings }
    }

    fn store(&self) {
        if let Err(e) = store(&self.path, &self.settings) {
            warn!("settings not saved: {e}");
        }
    }
}

// ---- Recolouring -------------------------------------------------------

/// Art's v3 floor and wall body colours, which the tiles are recoloured from.
pub const FLOOR_REF: Rgb = Rgb::hex(0xDFE5E8);
pub const WALL_REF: Rgb = Rgb::hex(0x394654);

/// Content IDs drawn through the palette: floor, wall and doorway tiles.
/// Stations, scaffold, marker and characters keep their own colours.
pub fn recolourable(id: &str) -> bool {
    id.starts_with("core.carrier.tile.")
}

fn luma([r, g, b]: [u8; 3]) -> f32 {
    0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32
}

/// Art's cyan light line (and its blends with the wall in smaller mips):
/// green and blue well above red. Floor and wall greys are close to neutral.
fn is_accent([r, g, b]: [u8; 3]) -> bool {
    g as i32 - r as i32 > 60 && b as i32 - r as i32 > 60
}

/// One pixel: kept if it is the light line, else its offset from the
/// nearer (by luma) of `FLOOR_REF` / `WALL_REF`, channel by channel, added
/// to `floor` / `wall`.
pub fn recolour_pixel(p: [u8; 3], floor: Rgb, wall: Rgb) -> [u8; 3] {
    if is_accent(p) {
        return p;
    }
    let (lf, lw) = (luma(FLOOR_REF.0), luma(WALL_REF.0));
    let (from, to) = if luma(p) >= (lf + lw) / 2.0 {
        (FLOOR_REF, floor)
    } else {
        (WALL_REF, wall)
    };
    std::array::from_fn(|c| (to.0[c] as i32 + p[c] as i32 - from.0[c] as i32).clamp(0, 255) as u8)
}

/// Recolours RGBA8 `src` into `out` (same length; mip levels included,
/// alpha kept).
pub fn recolour(src: &[u8], out: &mut [u8], floor: Rgb, wall: Rgb) {
    for (s, o) in src.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
        let rgb = recolour_pixel([s[0], s[1], s[2]], floor, wall);
        o[..3].copy_from_slice(&rgb);
        o[3] = s[3];
    }
}

/// The carrier's recoloured tile images, by content ID, with the colours
/// they are drawn in. `apply_palette` keeps those in step with the settings.
#[derive(Resource)]
pub struct CarrierPalette {
    floor: Rgb,
    wall: Rgb,
    tiles: std::collections::HashMap<String, (Vec<u8>, Handle<Image>)>,
}

impl Default for CarrierPalette {
    fn default() -> Self {
        Self {
            floor: DEFAULT_FLOOR,
            wall: DEFAULT_WALL,
            tiles: default(),
        }
    }
}

impl CarrierPalette {
    /// The image to draw `id` with: its recoloured copy for a floor, wall
    /// or doorway tile, else the shipped image.
    pub fn image(
        &mut self,
        art: &mut ContentImages,
        images: &mut Assets<Image>,
        id: &str,
    ) -> Option<Handle<Image>> {
        let source = art.get(images, id)?;
        if !recolourable(id) {
            return Some(source);
        }
        if let Some((_, handle)) = self.tiles.get(id) {
            return Some(handle.clone());
        }
        let mut image = images.get(&source)?.clone();
        let pixels = image.data.clone()?;
        if let Some(data) = image.data.as_mut() {
            recolour(&pixels, data, self.floor, self.wall);
        }
        let handle = images.add(image);
        self.tiles.insert(id.to_string(), (pixels, handle.clone()));
        Some(handle)
    }

    /// Recolours every tile made so far, in place.
    fn set(&mut self, images: &mut Assets<Image>, floor: Rgb, wall: Rgb) {
        if (self.floor, self.wall) == (floor, wall) {
            return;
        }
        (self.floor, self.wall) = (floor, wall);
        for (pixels, handle) in self.tiles.values() {
            let Some(mut image) = images.get_mut(handle) else {
                continue;
            };
            if let Some(data) = image.data.as_mut() {
                recolour(pixels, data, floor, wall);
            }
        }
    }
}

// ---- Settings screen ----------------------------------------------------

/// Whether the Settings screen is open, and whether closing it goes back
/// to the pause menu it was opened from. While it is open on the Carrier
/// the pause menu stays `open` (hidden) so the scene stays paused.
#[derive(Resource, Default)]
pub struct SettingsScreen {
    pub open: bool,
    back_to_pause: bool,
    dirty: bool,
}

impl SettingsScreen {
    /// Opens the screen; `from_pause`: from the pause menu, which it
    /// returns to. Pauses the Carrier while open.
    pub fn open(&mut self, pause: &mut PauseMenu, from_pause: bool) {
        self.open = true;
        self.back_to_pause = from_pause;
        self.dirty = true;
        pause.hold(true);
    }

    pub fn close(&mut self, pause: &mut PauseMenu) {
        if !self.open {
            return;
        }
        self.open = false;
        self.dirty = true;
        pause.hold(self.back_to_pause);
    }
}

#[derive(Component)]
struct SettingsEntity;

/// The gear button (Title, Carrier HUD).
#[derive(Component)]
pub struct GearButton;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum SettingsClick {
    Close,
    Floor(usize),
    Wall(usize),
    ResetFloor,
    ResetWall,
    OpenCodex,
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SettingsFile::load())
            .init_resource::<SettingsScreen>()
            .init_resource::<CarrierPalette>()
            .add_systems(
                Update,
                (
                    close_on_screen_change.run_if(state_changed::<GameScreen>),
                    settings_keys,
                    settings_clicks,
                    sync_settings_screen,
                    apply_palette,
                )
                    .chain()
                    .before(crate::flow::toggle_pause_menu),
            );
    }
}

fn close_on_screen_change(mut screen: ResMut<SettingsScreen>) {
    if screen.open {
        screen.open = false;
        screen.dirty = true;
    }
}

/// Esc closes the screen (and is used up, so it doesn't also pause or
/// unpause); on the Title it also opens it.
pub(crate) fn settings_keys(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    state: Res<State<GameScreen>>,
    mut screen: ResMut<SettingsScreen>,
    mut pause: ResMut<PauseMenu>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if screen.open {
        screen.close(&mut pause);
        keys.clear_just_pressed(KeyCode::Escape);
    } else if *state.get() == GameScreen::Title {
        screen.open(&mut pause, false);
        keys.clear_just_pressed(KeyCode::Escape);
    }
}

#[allow(clippy::type_complexity)]
fn settings_clicks(
    gears: Query<&Interaction, (Changed<Interaction>, With<GearButton>)>,
    clicks: Query<(&Interaction, &SettingsClick), Changed<Interaction>>,
    mut screen: ResMut<SettingsScreen>,
    mut pause: ResMut<PauseMenu>,
    mut file: ResMut<SettingsFile>,
    mut codex: ResMut<CodexScreen>,
) {
    if codex.open {
        return;
    }
    if gears.iter().any(|i| *i == Interaction::Pressed) && !screen.open && !pause.open {
        screen.open(&mut pause, false);
    }
    for (_, click) in clicks.iter().filter(|(i, _)| **i == Interaction::Pressed) {
        let display = &mut file.settings.display;
        match *click {
            SettingsClick::Close => {
                screen.close(&mut pause);
                continue;
            }
            SettingsClick::OpenCodex => {
                codex.open();
                continue;
            }
            SettingsClick::Floor(i) => display.carrier_floor = FLOOR_PRESETS[i].rgb,
            SettingsClick::Wall(i) => display.carrier_wall = WALL_PRESETS[i].rgb,
            SettingsClick::ResetFloor => display.carrier_floor = DEFAULT_FLOOR,
            SettingsClick::ResetWall => display.carrier_wall = DEFAULT_WALL,
        }
        file.store();
        screen.dirty = true;
    }
}

fn apply_palette(
    file: Res<SettingsFile>,
    mut palette: ResMut<CarrierPalette>,
    mut images: ResMut<Assets<Image>>,
) {
    if file.is_changed() {
        let d = &file.settings.display;
        palette.set(&mut images, d.carrier_floor, d.carrier_wall);
    }
}

fn sync_settings_screen(
    mut commands: Commands,
    mut screen: ResMut<SettingsScreen>,
    file: Res<SettingsFile>,
    entities: Query<Entity, With<SettingsEntity>>,
) {
    if !screen.dirty {
        return;
    }
    screen.dirty = false;
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    if screen.open {
        spawn_panel(&mut commands, &file.settings);
    }
}

pub(crate) const PANEL_BG: Color = Color::srgb(0.02, 0.04, 0.06);
pub(crate) const PANEL_BORDER: Color = Color::srgb(0.35, 0.52, 0.58);
pub(crate) const BUTTON_BG: Color = Color::srgb(0.08, 0.16, 0.2);
pub(crate) const TEXT: Color = Color::srgb(0.85, 0.93, 0.96);
pub(crate) const MUTED: Color = Color::srgb(0.55, 0.66, 0.72);
const WARN: Color = Color::srgb(1.0, 0.75, 0.35);

pub(crate) fn text(s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn text_button(p: &mut ChildSpawnerCommands, click: SettingsClick, label: &str) {
    p.spawn((
        Button,
        click,
        BackgroundColor(BUTTON_BG),
        BorderColor::all(PANEL_BORDER),
        Node {
            padding: UiRect::axes(px(10), px(4)),
            border: UiRect::all(px(1)),
            ..default()
        },
    ))
    .with_child(text(label, 14.0, TEXT));
}

/// The panel, left of centre so the carrier shows beside it. One block
/// per section; a later option adds a section or a row.
fn spawn_panel(commands: &mut Commands, settings: &Settings) {
    commands
        .spawn((
            SettingsEntity,
            Node {
                position_type: PositionType::Absolute,
                left: px(24),
                top: px(24),
                width: px(430),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                padding: UiRect::all(px(16)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            BorderColor::all(PANEL_BORDER),
            GlobalZIndex(50),
        ))
        .with_children(|p| {
            p.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                row.spawn(text("Settings", 24.0, Color::WHITE));
                text_button(row, SettingsClick::Close, "[Esc] Close");
            });
            p.spawn(text("Display", 18.0, TEXT));
            p.spawn(text("Carrier colours", 15.0, MUTED));
            let d = &settings.display;
            colour_row(p, "Floor colour", &FLOOR_PRESETS, d.carrier_floor, SettingsClick::Floor, SettingsClick::ResetFloor);
            colour_row(p, "Wall colour", &WALL_PRESETS, d.carrier_wall, SettingsClick::Wall, SettingsClick::ResetWall);
            let ratio = contrast(d.carrier_floor, d.carrier_wall);
            p.spawn(text(format!("Floor / wall contrast {ratio:.1}:1"), 14.0, MUTED));
            if ratio < MIN_CONTRAST {
                p.spawn(text(
                    format!(
                        "Warning: floor and walls are close in brightness (below {MIN_CONTRAST:.0}:1), so walls may be hard to see."
                    ),
                    14.0,
                    WARN,
                ));
            }
            p.spawn(text("Codex", 18.0, TEXT));
            p.spawn(Node {
                column_gap: px(10),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                text_button(row, SettingsClick::OpenCodex, "Open Codex");
                row.spawn(text("Enemies, stations, furniture, crew and ships", 14.0, MUTED));
            });
        });
}

/// A label with the chosen preset's name (or the hex of a custom colour),
/// the swatches (the chosen one outlined), and Reset to default.
fn colour_row(
    p: &mut ChildSpawnerCommands,
    label: &str,
    presets: &[Preset],
    chosen: Rgb,
    pick: fn(usize) -> SettingsClick,
    reset: SettingsClick,
) {
    let name = presets
        .iter()
        .position(|s| s.rgb == chosen)
        .map(|i| {
            let name = presets[i].name;
            if i == 0 {
                format!("{name} (default)")
            } else {
                name.to_string()
            }
        })
        .unwrap_or_else(|| chosen.to_hex());
    p.spawn(text(format!("{label}: {name}"), 15.0, TEXT));
    p.spawn(Node {
        column_gap: px(8),
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|row| {
        for (i, s) in presets.iter().enumerate() {
            let on = s.rgb == chosen;
            row.spawn((
                Button,
                pick(i),
                BackgroundColor(s.rgb.color()),
                BorderColor::all(if on { Color::WHITE } else { PANEL_BORDER }),
                Node {
                    width: px(34),
                    height: px(34),
                    border: UiRect::all(px(if on { 3 } else { 1 })),
                    ..default()
                },
            ));
        }
        text_button(row, reset, "Reset to default");
    });
}

/// A gear-shaped placeholder icon (there is no UI art for one yet): a
/// ring with eight teeth, white on transparent, `size` px.
pub fn gear_image(size: u32) -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let mut data = vec![0u8; (size * size * 4) as usize];
    let c = (size as f32 - 1.0) / 2.0;
    let s = size as f32 / 32.0;
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = (x as f32 - c, y as f32 - c);
            let r = (dx * dx + dy * dy).sqrt();
            let a = dy.atan2(dx);
            // Teeth: the outer radius steps out for half of each 45°.
            let tooth = ((a / std::f32::consts::TAU * 8.0).rem_euclid(1.0)) < 0.5;
            let outer = if tooth { 15.0 } else { 11.5 } * s;
            if r <= outer && r >= 5.0 * s {
                let i = ((y * size + x) * 4) as usize;
                data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// The gear button as a UI node bundle (the caller positions it).
pub fn gear_button(images: &mut Assets<Image>, node: Node) -> impl Bundle {
    (
        Button,
        GearButton,
        node,
        BackgroundColor(BUTTON_BG),
        BorderColor::all(PANEL_BORDER),
        ImageNode::new(images.add(gear_image(32))).with_color(TEXT),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        let c = Rgb::hex(0x3A4350);
        assert_eq!(c.0, [0x3A, 0x43, 0x50]);
        assert_eq!(c.to_hex(), "#3A4350");
        assert_eq!(Rgb::parse("#3a4350"), Some(c));
        assert_eq!(Rgb::parse("3A4350"), None);
        assert_eq!(Rgb::parse("#3A435"), None);
    }

    #[test]
    fn contrast_matches_wcag() {
        let (black, white) = (Rgb::hex(0x000000), Rgb::hex(0xFFFFFF));
        assert!((contrast(black, white) - 21.0).abs() < 0.01);
        assert!((contrast(white, black) - 21.0).abs() < 0.01);
        assert!((contrast(white, white) - 1.0).abs() < 1e-6);
        // WCAG's own example: #777777 on white is about 4.48:1.
        assert!((contrast(Rgb::hex(0x777777), white) - 4.48).abs() < 0.01);
    }

    #[test]
    fn defaults_are_dark_floor_and_clearly_separate_walls() {
        assert_eq!(DEFAULT_FLOOR, Rgb::hex(0x3A4350));
        assert!(DEFAULT_FLOOR.luminance() < 0.1, "default floor is dark");
        assert!(contrast(DEFAULT_FLOOR, DEFAULT_WALL) >= MIN_CONTRAST);
        // The old light floor is a preset, and Art's pair is high contrast.
        assert!(FLOOR_PRESETS.iter().any(|p| p.rgb == FLOOR_REF));
        assert!(WALL_PRESETS.iter().any(|p| p.rgb == WALL_REF));
        assert!(contrast(FLOOR_REF, WALL_REF) >= MIN_CONTRAST);
        // A dark wall on the dark default floor warns.
        assert!(contrast(DEFAULT_FLOOR, WALL_REF) < MIN_CONTRAST);
    }

    #[test]
    fn settings_save_and_load() {
        let dir = std::env::temp_dir().join(format!("oni-settings-{}", std::process::id()));
        let path = dir.join(SETTINGS_FILE_NAME);
        let s = Settings {
            display: Display {
                carrier_floor: FLOOR_PRESETS[5].rgb,
                carrier_wall: WALL_PRESETS[2].rgb,
            },
            ..Settings::default()
        };
        store(&path, &s).unwrap();
        assert_eq!(load(&path).unwrap(), s);
        let json = std::fs::read_to_string(&path).unwrap();
        assert!(json.contains("\"carrier_floor\": \"#DFE5E8\""), "{json}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn settings_files_fill_in_defaults_and_refuse_newer_versions() {
        assert_eq!(from_json(br#"{"version":1}"#).unwrap(), Settings::default());
        let s = from_json(br##"{"version":1,"display":{"carrier_wall":"#123456"}}"##).unwrap();
        assert_eq!(s.display.carrier_floor, DEFAULT_FLOOR);
        assert_eq!(s.display.carrier_wall, Rgb::hex(0x123456));
        assert!(matches!(
            from_json(br#"{"version":2}"#),
            Err(SettingsError::Version(2))
        ));
        assert!(from_json(br##"{"version":1,"display":{"carrier_wall":"red"}}"##).is_err());
    }

    #[test]
    fn recolour_moves_floor_and_wall_and_keeps_the_light_line() {
        let (floor, wall) = (DEFAULT_FLOOR, DEFAULT_WALL);
        // Art's references map onto the chosen colours exactly.
        assert_eq!(recolour_pixel(FLOOR_REF.0, floor, wall), floor.0);
        assert_eq!(recolour_pixel(WALL_REF.0, floor, wall), wall.0);
        // The cyan light line is kept.
        assert_eq!(
            recolour_pixel([100, 213, 224], floor, wall),
            [100, 213, 224]
        );
        // Floor seams and wall trim stay darker than their base.
        let seam = recolour_pixel([207, 215, 220], floor, wall);
        assert!(luma(seam) < luma(floor.0) - 10.0, "{seam:?}");
        let trim = recolour_pixel([36, 46, 58], floor, wall);
        assert!(luma(trim) < luma(wall.0) - 15.0, "{trim:?}");
        // Art's own pair reproduces the shipped tiles.
        for p in [[222, 228, 231], [224, 230, 233], [57, 70, 84], [36, 46, 58]] {
            assert_eq!(recolour_pixel(p, FLOOR_REF, WALL_REF), p);
        }
        // RGBA: alpha is kept.
        let src = [207, 215, 220, 128, 100, 213, 224, 255];
        let mut out = [0u8; 8];
        recolour(&src, &mut out, floor, wall);
        assert_eq!(out[3], 128);
        assert_eq!(&out[4..], &src[4..]);
    }

    /// TAKOAI-102: the hull-edge tiles are drawn in Art's v3 floor, wall,
    /// trim and cyan, so the same recolour moves the floor inside them
    /// with the floor colour and the shell with the wall colour.
    #[test]
    fn hull_tiles_recolour_their_floor_and_shell_apart() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");
        let manifest = content::ContentManifest::load(format!("{root}/manifest.json")).unwrap();
        let (floor, wall) = (Rgb::hex(0x203040), Rgb::hex(0xC08060));
        let mut tiles = 0;
        for a in &manifest.packs[0].assets {
            if !a.id.starts_with("core.carrier.tile.hull_wall_") {
                continue;
            }
            assert!(recolourable(&a.id));
            let image = image::open(format!("{root}/{}", a.processed))
                .unwrap()
                .to_rgba8();
            let (mut floors, mut shells) = (0, 0);
            for p in image.pixels() {
                let rgb = [p[0], p[1], p[2]];
                let out = recolour_pixel(rgb, floor, wall);
                if is_accent(rgb) {
                    assert_eq!(out, rgb);
                } else if rgb == FLOOR_REF.0 {
                    assert_eq!(out, floor.0, "{}", a.id);
                    floors += 1;
                } else {
                    // Shell, trim or rim: moved with the wall colour.
                    let expect: [u8; 3] = std::array::from_fn(|c| {
                        (wall.0[c] as i32 + rgb[c] as i32 - WALL_REF.0[c] as i32).clamp(0, 255)
                            as u8
                    });
                    assert_eq!(out, expect, "{} {rgb:?}", a.id);
                    shells += 1;
                }
            }
            assert!(floors > 0 && shells > 0, "{}", a.id);
            tiles += 1;
        }
        assert_eq!(tiles, 38);
    }

    #[test]
    fn only_floor_wall_and_doorway_tiles_recolour() {
        assert!(recolourable("core.carrier.tile.floor.0"));
        assert!(recolourable("core.carrier.tile.wall.1010"));
        assert!(recolourable("core.carrier.tile.doorway.ns"));
        assert!(!recolourable("core.carrier.scaffold"));
        assert!(!recolourable("core.carrier.front_marker"));
        assert!(!recolourable("core.carrier.item.command_console"));
    }
}
