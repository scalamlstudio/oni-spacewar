//! The Codex (TAKOAI-100): a full-screen view, opened from the Settings
//! screen, to browse every piece of game content with its image, name and
//! description.
//!
//! **Data-driven.** The entries come from one content asset, `core.codex`
//! (`assets/source/core/codex.json`, shipped through the manifest like any
//! other asset): its `categories` give the tabs and their order, and each
//! entry names its category, its image (a content ID), a name and a
//! description. A new enemy, item or ship shows up by adding its image and
//! an entry there; a category the file doesn't declare still gets a tab,
//! after the declared ones. Every description written so far is marked
//! `"placeholder": true` in that one file, for Wei to review.
//!
//! The few stats shown beside an entry are read from the registries that
//! already hold them ([`stats`]): the carrier catalogue for stations and
//! furniture, `sim`'s tuning for enemies and battleships. Nothing new is
//! invented, and nothing here reaches `sim`'s state.
//!
//! **Discovery.** Everything is visible in this version; [`discovered`] is
//! the one hook a later unlock system replaces.
//!
//! Controls: click a tab or a thumbnail; mouse wheel scrolls the grid;
//! arrow keys move the selection, Tab / Shift+Tab switch category, Esc or
//! Close returns to Settings.

use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use serde::Deserialize;

use crate::art::ContentImages;
use crate::layout::{Category, CATALOGUE};
use crate::settings::{self, text, SettingsScreen, BUTTON_BG, MUTED, PANEL_BG, PANEL_BORDER, TEXT};

/// The content ID of the Codex data.
pub const CODEX_ID: &str = "core.codex";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CategoryDef {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CodexEntry {
    /// Stable entry ID (for a later discovery record).
    pub id: String,
    pub category: String,
    /// Content ID of the image; a missing one shows a placeholder.
    #[serde(default)]
    pub image: Option<String>,
    pub name: String,
    pub description: String,
    /// The description is placeholder text awaiting review.
    #[serde(default)]
    pub placeholder: bool,
}

/// `core.codex`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct CodexData {
    #[serde(default)]
    pub categories: Vec<CategoryDef>,
    pub entries: Vec<CodexEntry>,
}

/// One tab: a category and its visible entries, in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub id: String,
    pub name: String,
    pub entries: Vec<CodexEntry>,
}

impl CodexData {
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// The tabs: declared categories in order, then undeclared ones in order
    /// of first use (named from their ID); only entries `visible` keeps, and
    /// no empty tabs.
    pub fn sections(&self, visible: impl Fn(&CodexEntry) -> bool) -> Vec<Section> {
        let mut sections: Vec<Section> = self
            .categories
            .iter()
            .map(|c| Section {
                id: c.id.clone(),
                name: c.name.clone(),
                entries: Vec::new(),
            })
            .collect();
        for entry in self.entries.iter().filter(|e| visible(e)) {
            let i = match sections.iter().position(|s| s.id == entry.category) {
                Some(i) => i,
                None => {
                    sections.push(Section {
                        id: entry.category.clone(),
                        name: category_name(&entry.category),
                        entries: Vec::new(),
                    });
                    sections.len() - 1
                }
            };
            sections[i].entries.push(entry.clone());
        }
        sections.retain(|s| !s.entries.is_empty());
        sections
    }
}

/// `void_beasts` -> `Void beasts`.
fn category_name(id: &str) -> String {
    let words = id.replace(['_', '-'], " ");
    let mut chars = words.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Whether the player has discovered `entry`. Everything is unlocked for
/// now; a discovery system replaces this.
pub fn discovered(_entry: &CodexEntry) -> bool {
    true
}

/// Stats already in the game's data for the content under `image`, as
/// label / value rows; none for content without any.
pub fn stats(image: &str) -> Vec<(&'static str, String)> {
    let mut rows = Vec::new();
    if let Some(key) = image.strip_prefix("core.carrier.item.") {
        if let Some(def) = CATALOGUE.iter().find(|d| d.key == key) {
            rows.push((
                "Footprint",
                format!("{} x {} tiles", def.size.0, def.size.1),
            ));
            rows.push(("Placement", def.placement.label().to_string()));
            if def.count.fixed() {
                rows.push(("Cost", "Starter (comes with the carrier)".to_string()));
            } else {
                rows.push(("Cost", crate::carrier::cost_text(def.cost)));
            }
            if def.category == Category::Station && !def.effect.is_empty() {
                rows.push(("Effect", def.effect.to_string()));
            }
        }
    } else if let Some(key) = image.strip_prefix("core.battle.enemy.") {
        use sim::tuning::*;
        let (hp, speed) = match key {
            "void_swarmer" => (SWARMER_HP, SWARMER_SPEED),
            "void_spitter" => (SPITTER_HP, SPITTER_SPEED),
            _ => return rows,
        };
        rows.push(("HP", hp.to_string()));
        rows.push(("Speed", format!("{speed} px/s")));
    } else if let Some(key) = image.strip_prefix("core.battle.ship.") {
        let ship = match key {
            "kite" => sim::ShipKind::Kite,
            "bulwark" => sim::ShipKind::Bulwark,
            _ => return rows,
        };
        let sheet = sim::ShipSheet::new(sim::Loadout { ship, ..default() });
        rows.push(("Hull", sheet.hull.to_string()));
        rows.push(("Speed", format!("{} px/s", sheet.speed)));
        rows.push((
            "Basic attack",
            format!("{} dmg, range {} px", sheet.basic_damage, sheet.basic_range),
        ));
    } else if let Some(rest) = image.strip_prefix("core.portraits.") {
        use crate::layout::Crew;
        let post = match rest.split('.').next() {
            Some("gunner") => Crew::Gunner.post(),
            Some("engineer") => Crew::Engineer.post(),
            Some("researcher") => Crew::Researcher.post(),
            _ => return rows,
        };
        rows.push(("Post", post.name().to_string()));
    }
    rows
}

/// `size` scaled to fit in `bounds`, keeping its aspect ratio.
pub fn fit(size: Vec2, bounds: Vec2) -> Vec2 {
    if size.x <= 0.0 || size.y <= 0.0 {
        return bounds;
    }
    size * (bounds.x / size.x).min(bounds.y / size.y)
}

// ---- Screen ------------------------------------------------------------

/// What needs redrawing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Dirty {
    #[default]
    None,
    /// Thumbnail outlines and the detail panel.
    Selection,
    /// Everything (opened, tab changed).
    All,
}

/// Whether the Codex is open, which tab and entry are selected, and the
/// Codex data (read once, on first open).
#[derive(Resource, Default)]
pub struct CodexScreen {
    pub open: bool,
    tab: usize,
    selected: usize,
    dirty: Dirty,
    /// Keep the selection scrolled into view (keyboard moves).
    reveal: bool,
    data: Option<CodexData>,
}

impl CodexScreen {
    pub fn open(&mut self) {
        self.open = true;
        self.tab = 0;
        self.selected = 0;
        self.dirty = Dirty::All;
    }

    /// Opens on `tab` with its `entry` selected (the autoplay's shots).
    pub fn show(&mut self, tab: usize, entry: usize) {
        self.open();
        self.tab = tab;
        self.selected = entry;
        self.reveal = true;
    }

    pub fn close(&mut self) {
        if self.open {
            self.open = false;
            self.dirty = Dirty::All;
        }
    }

    fn sections(&self) -> Vec<Section> {
        self.data
            .as_ref()
            .map(|d| d.sections(discovered))
            .unwrap_or_default()
    }
}

#[derive(Component)]
struct CodexEntity;

/// The scrolling thumbnail grid.
#[derive(Component)]
struct CodexGrid;

/// The detail panel, rebuilt when the selection changes.
#[derive(Component)]
struct CodexDetail;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum CodexClick {
    Close,
    Tab(usize),
    Entry(usize),
}

/// Thumbnails per grid row (the keyboard moves by this much up and down).
const COLUMNS: usize = 4;
const CELL: f32 = 112.0;
const CELL_H: f32 = 150.0;
const GAP: f32 = 8.0;
/// Pixels per wheel line.
const LINE_PX: f32 = 40.0;
const SELECTED: Color = Color::srgb(1.0, 0.85, 0.4);

pub struct CodexPlugin;

impl Plugin for CodexPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CodexScreen>().add_systems(
            Update,
            (
                codex_keys,
                codex_clicks,
                sync_codex,
                codex_wheel,
                reveal_selected,
            )
                .chain()
                .before(settings::settings_keys),
        );
    }
}

/// Esc closes the Codex back to Settings (and is used up); arrows move the
/// selection; Tab / Shift+Tab switch category.
fn codex_keys(mut keys: ResMut<ButtonInput<KeyCode>>, mut codex: ResMut<CodexScreen>) {
    if !codex.open {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        codex.close();
        keys.clear_just_pressed(KeyCode::Escape);
        return;
    }
    let sections = codex.sections();
    let Some(section) = sections.get(codex.tab) else {
        return;
    };
    if keys.just_pressed(KeyCode::Tab) {
        let n = sections.len();
        let back = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        codex.tab = if back {
            (codex.tab + n - 1) % n
        } else {
            (codex.tab + 1) % n
        };
        codex.selected = 0;
        codex.dirty = Dirty::All;
        return;
    }
    let n = section.entries.len();
    let step = |k: KeyCode, d: isize| if keys.just_pressed(k) { d } else { 0 };
    let delta = step(KeyCode::ArrowLeft, -1)
        + step(KeyCode::ArrowRight, 1)
        + step(KeyCode::ArrowUp, -(COLUMNS as isize))
        + step(KeyCode::ArrowDown, COLUMNS as isize);
    if delta != 0 {
        let to = (codex.selected as isize + delta).clamp(0, n as isize - 1) as usize;
        if to != codex.selected {
            codex.selected = to;
            codex.dirty = Dirty::Selection;
            codex.reveal = true;
        }
    }
}

fn codex_clicks(
    clicks: Query<(&Interaction, &CodexClick), Changed<Interaction>>,
    mut codex: ResMut<CodexScreen>,
) {
    for (_, click) in clicks.iter().filter(|(i, _)| **i == Interaction::Pressed) {
        match *click {
            CodexClick::Close => codex.close(),
            CodexClick::Tab(i) if i != codex.tab => {
                codex.tab = i;
                codex.selected = 0;
                codex.dirty = Dirty::All;
            }
            CodexClick::Entry(i) if i != codex.selected => {
                codex.selected = i;
                codex.dirty = Dirty::Selection;
            }
            _ => {}
        }
    }
}

/// The wheel scrolls the grid (the Settings screen holds the Carrier's
/// pause, so the wheel doesn't also zoom the deck).
fn codex_wheel(
    scroll: Res<AccumulatedMouseScroll>,
    codex: Res<CodexScreen>,
    mut grid: Query<&mut ScrollPosition, With<CodexGrid>>,
) {
    if !codex.open || scroll.delta.y == 0.0 {
        return;
    }
    let dy = match scroll.unit {
        bevy::input::mouse::MouseScrollUnit::Line => scroll.delta.y * LINE_PX,
        bevy::input::mouse::MouseScrollUnit::Pixel => scroll.delta.y,
    };
    for mut pos in &mut grid {
        pos.0.y = (pos.0.y - dy).max(0.0);
    }
}

/// After a keyboard move, scrolls the grid just enough to show the
/// selected thumbnail.
fn reveal_selected(
    mut codex: ResMut<CodexScreen>,
    mut grid: Query<(&mut ScrollPosition, &ComputedNode), With<CodexGrid>>,
) {
    if !codex.reveal {
        return;
    }
    for (mut pos, node) in &mut grid {
        // A grid spawned this frame has no layout yet: wait for it.
        let view = node.size().y * node.inverse_scale_factor() - 2.0;
        if view <= 0.0 {
            return;
        }
        // The cell with the padding / gap around it.
        let top = (codex.selected / COLUMNS) as f32 * (CELL_H + GAP);
        pos.0.y = reveal(pos.0.y, top, CELL_H + 2.0 * GAP, view);
    }
    codex.reveal = false;
}

/// The scroll offset that shows `[top, top + height)` in a `view` px tall
/// window, moving as little as possible from `scroll`.
fn reveal(scroll: f32, top: f32, height: f32, view: f32) -> f32 {
    if top < scroll {
        top
    } else if top + height > scroll + view {
        top + height - view
    } else {
        scroll
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_codex(
    mut commands: Commands,
    mut codex: ResMut<CodexScreen>,
    settings_screen: Res<SettingsScreen>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    entities: Query<Entity, With<CodexEntity>>,
    detail: Query<Entity, With<CodexDetail>>,
    mut cells: Query<(&CodexClick, &mut BorderColor)>,
) {
    // Closing Settings (or leaving the screen) closes the Codex too.
    if codex.open && !settings_screen.open {
        codex.close();
    }
    let dirty = std::mem::take(&mut codex.dirty);
    if dirty == Dirty::None {
        return;
    }
    if codex.open && codex.data.is_none() {
        codex.data = Some(load(&mut art));
    }
    let sections = codex.sections();
    codex.tab = codex.tab.min(sections.len().saturating_sub(1));
    let section = sections.get(codex.tab);
    let selected = section.and_then(|s| s.entries.get(codex.selected));
    if dirty == Dirty::Selection {
        for (click, mut border) in &mut cells {
            if let CodexClick::Entry(i) = *click {
                *border = BorderColor::all(if i == codex.selected {
                    SELECTED
                } else {
                    PANEL_BORDER
                });
            }
        }
        for panel in &detail {
            commands.entity(panel).despawn_children();
            commands.entity(panel).with_children(|p| {
                spawn_detail(p, selected, &mut art, &mut images);
            });
        }
        return;
    }
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    if codex.open {
        spawn_codex(
            &mut commands,
            &sections,
            codex.tab,
            codex.selected,
            &mut art,
            &mut images,
        );
    }
}

/// `core.codex`, or no entries (logged) if it is missing or malformed.
fn load(art: &mut ContentImages) -> CodexData {
    let Some(json) = art.text(CODEX_ID) else {
        warn!("codex {CODEX_ID} unavailable");
        return CodexData::default();
    };
    CodexData::parse(&json).unwrap_or_else(|e| {
        warn!("codex {CODEX_ID} unreadable: {e}");
        CodexData::default()
    })
}

fn spawn_codex(
    commands: &mut Commands,
    sections: &[Section],
    tab: usize,
    selected: usize,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
) {
    commands
        .spawn((
            CodexEntity,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                padding: UiRect::all(px(16)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            FocusPolicy::Block,
            GlobalZIndex(60),
        ))
        .with_children(|p| {
            p.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                row.spawn(text("Codex", 24.0, Color::WHITE));
                row.spawn(text(
                    "Arrows select   Tab category   Wheel scroll",
                    13.0,
                    MUTED,
                ));
                button(row, CodexClick::Close, "[Esc] Back to Settings", false);
            });
            // Category tabs.
            p.spawn(Node {
                column_gap: px(6),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|row| {
                for (i, s) in sections.iter().enumerate() {
                    let label = format!("{} ({})", s.name, s.entries.len());
                    button(row, CodexClick::Tab(i), &label, i == tab);
                }
            });
            let Some(section) = sections.get(tab) else {
                p.spawn(text("No codex entries.", 15.0, MUTED));
                return;
            };
            p.spawn(Node {
                flex_grow: 1.0,
                min_height: px(0),
                column_gap: px(16),
                ..default()
            })
            .with_children(|body| {
                // Thumbnail grid, scrolling.
                body.spawn((
                    CodexGrid,
                    Node {
                        width: px(COLUMNS as f32 * (CELL + GAP) + GAP + 4.0),
                        flex_shrink: 0.0,
                        flex_wrap: FlexWrap::Wrap,
                        align_content: AlignContent::FlexStart,
                        column_gap: px(GAP),
                        row_gap: px(GAP),
                        padding: UiRect::all(px(GAP)),
                        overflow: Overflow::scroll_y(),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    ScrollPosition::default(),
                    BorderColor::all(PANEL_BORDER),
                ))
                .with_children(|grid| {
                    for (i, entry) in section.entries.iter().enumerate() {
                        spawn_cell(grid, i, entry, i == selected, art, images);
                    }
                });
                body.spawn((
                    CodexDetail,
                    Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        padding: UiRect::all(px(14)),
                        border: UiRect::all(px(1)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BorderColor::all(PANEL_BORDER),
                ))
                .with_children(|d| {
                    spawn_detail(d, section.entries.get(selected), art, images);
                });
            });
        });
}

fn button(p: &mut ChildSpawnerCommands, click: CodexClick, label: &str, on: bool) {
    p.spawn((
        Button,
        click,
        BackgroundColor(if on { PANEL_BORDER } else { BUTTON_BG }),
        BorderColor::all(if on { Color::WHITE } else { PANEL_BORDER }),
        Node {
            padding: UiRect::axes(px(10), px(4)),
            border: UiRect::all(px(1)),
            ..default()
        },
    ))
    .with_child(text(label, 14.0, if on { Color::WHITE } else { TEXT }));
}

/// An entry's image fitted in `bounds`, or a "no art yet" box.
fn spawn_image(
    p: &mut ChildSpawnerCommands,
    image: Option<&str>,
    bounds: Vec2,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
) {
    p.spawn(Node {
        width: px(bounds.x),
        height: px(bounds.y),
        flex_shrink: 0.0,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|b| match image.and_then(|id| art.get(images, id)) {
        Some(handle) => {
            let size = fit(ContentImages::size(images, &handle), bounds);
            b.spawn((
                ImageNode::new(handle),
                Node {
                    width: px(size.x),
                    height: px(size.y),
                    ..default()
                },
            ));
        }
        None => {
            b.spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(MUTED),
            ))
            .with_children(|q| {
                q.spawn(text("?", bounds.y * 0.35, MUTED));
                q.spawn(text("No art yet", 12.0, MUTED));
            });
        }
    });
}

fn spawn_cell(
    grid: &mut ChildSpawnerCommands,
    i: usize,
    entry: &CodexEntry,
    on: bool,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
) {
    grid.spawn((
        Button,
        CodexClick::Entry(i),
        BackgroundColor(BUTTON_BG),
        BorderColor::all(if on { SELECTED } else { PANEL_BORDER }),
        Node {
            width: px(CELL),
            height: px(CELL_H),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(4),
            padding: UiRect::all(px(6)),
            border: UiRect::all(px(2)),
            ..default()
        },
    ))
    .with_children(|c| {
        spawn_image(
            c,
            entry.image.as_deref(),
            Vec2::splat(CELL - 24.0),
            art,
            images,
        );
        c.spawn((
            text(&entry.name, 12.0, TEXT),
            TextLayout::justify(Justify::Center),
        ));
    });
}

fn spawn_detail(
    d: &mut ChildSpawnerCommands,
    entry: Option<&CodexEntry>,
    art: &mut ContentImages,
    images: &mut Assets<Image>,
) {
    let Some(entry) = entry else {
        return;
    };
    d.spawn(Node {
        justify_content: JustifyContent::Center,
        ..default()
    })
    .with_children(|r| spawn_image(r, entry.image.as_deref(), Vec2::splat(220.0), art, images));
    d.spawn(text(&entry.name, 22.0, Color::WHITE));
    d.spawn(text(&entry.description, 15.0, TEXT));
    for (label, value) in stats(entry.image.as_deref().unwrap_or("")) {
        d.spawn(text(format!("{label}: {value}"), 14.0, MUTED));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> CodexData {
        let mut art = ContentImages::repo();
        CodexData::parse(&art.text(CODEX_ID).expect("core.codex shipped")).unwrap()
    }

    fn manifest() -> content::ContentManifest {
        content::ContentManifest::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/manifest.json"
        ))
        .unwrap()
    }

    /// Every entry has a name, a description and shipped art, and the
    /// shipped file declares the five tabs in order, each non-empty.
    #[test]
    fn every_entry_has_a_name_description_and_image() {
        let codex = shipped();
        let manifest = manifest();
        for e in &codex.entries {
            assert!(!e.name.trim().is_empty(), "{} has no name", e.id);
            assert!(
                !e.description.trim().is_empty(),
                "{} has no description",
                e.id
            );
            let image = e
                .image
                .as_deref()
                .unwrap_or_else(|| panic!("{} has no image", e.id));
            assert!(
                manifest.asset(image).is_ok(),
                "{} image {image} not shipped",
                e.id
            );
        }
        let mut ids: Vec<_> = codex.entries.iter().map(|e| &e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), codex.entries.len(), "duplicate entry IDs");
        let tabs: Vec<_> = codex
            .sections(discovered)
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(
            tabs,
            [
                "Enemies",
                "Work Stations",
                "Furniture",
                "Characters",
                "Battleships"
            ]
        );
    }

    /// Content in the codex categories can't ship without an entry: every
    /// enemy, battleship and carrier item image in the manifest, every
    /// catalogue item (same name) and every crew member has one.
    #[test]
    fn all_content_in_codex_categories_has_an_entry() {
        let codex = shipped();
        let by_image = |id: &str| {
            codex
                .entries
                .iter()
                .find(|e| e.image.as_deref() == Some(id))
        };
        for asset in &manifest().packs[0].assets {
            for (prefix, category) in [
                ("core.battle.enemy.", "enemies"),
                ("core.battle.ship.", "battleships"),
                ("core.carrier.item.", ""),
            ] {
                if asset.id.starts_with(prefix) {
                    let e = by_image(&asset.id)
                        .unwrap_or_else(|| panic!("{} has no codex entry", asset.id));
                    if !category.is_empty() {
                        assert_eq!(e.category, category, "{}", asset.id);
                    }
                }
            }
        }
        for def in &CATALOGUE {
            let e = by_image(&def.id.content_id())
                .unwrap_or_else(|| panic!("{} has no codex entry", def.key));
            assert_eq!(e.name, def.name);
            let category = if def.station() {
                "stations"
            } else {
                "furniture"
            };
            assert_eq!(e.category, category, "{}", def.key);
        }
        for crew in ["pilot", "gunner", "researcher", "engineer"] {
            let e = by_image(&format!("core.portraits.{crew}.normal")).expect(crew);
            assert_eq!(e.category, "characters");
        }
        for e in &codex.entries {
            assert!(
                e.placeholder,
                "{}: descriptions are placeholders until reviewed",
                e.id
            );
        }
    }

    /// A new entry shows up with no UI change, in a new tab if its
    /// category is new; one without art still lists (with a placeholder).
    #[test]
    fn adding_an_entry_makes_it_appear() {
        let mut codex = shipped();
        let before = codex.sections(discovered);
        codex.entries.push(CodexEntry {
            id: "enemy.void_brute".into(),
            category: "enemies".into(),
            image: Some("core.battle.enemy.void_brute".into()),
            name: "Void Brute".into(),
            description: "Big.".into(),
            placeholder: true,
        });
        codex.entries.push(CodexEntry {
            id: "drone.scout".into(),
            category: "support_drones".into(),
            image: None,
            name: "Scout drone".into(),
            description: "Small.".into(),
            placeholder: true,
        });
        let after = codex.sections(discovered);
        assert_eq!(after[0].entries.len(), before[0].entries.len() + 1);
        assert_eq!(after[0].entries.last().unwrap().name, "Void Brute");
        let last = after.last().unwrap();
        assert_eq!(
            (last.name.as_str(), last.entries.len()),
            ("Support drones", 1)
        );
        assert_eq!(after.len(), before.len() + 1);
        // Art the manifest doesn't have: no image, no panic.
        let mut art = ContentImages::repo();
        let mut images = Assets::<Image>::default();
        assert!(art
            .get(&mut images, "core.battle.enemy.void_brute")
            .is_none());
    }

    #[test]
    fn hidden_entries_and_empty_categories_are_left_out() {
        let codex = CodexData::parse(
            r#"{"categories":[{"id":"a","name":"A"},{"id":"b","name":"B"}],
                "entries":[{"id":"x","category":"b","name":"X","description":"d"},
                           {"id":"y","category":"b","name":"Y","description":"d"}]}"#,
        )
        .unwrap();
        let tabs = codex.sections(|e| e.id != "y");
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].name, "B");
        assert_eq!(tabs[0].entries.len(), 1);
        assert!(codex.sections(|_| false).is_empty());
    }

    #[test]
    fn stats_come_from_the_registries() {
        let stats = |id| {
            stats(id)
                .into_iter()
                .collect::<std::collections::HashMap<_, _>>()
        };
        let berth = stats("core.carrier.item.berth");
        assert_eq!(berth["Footprint"], "4 x 4 tiles");
        assert!(berth["Cost"].starts_with("Starter"));
        assert_eq!(stats("core.carrier.item.sofa")["Cost"], "40 cr");
        assert_eq!(stats("core.battle.enemy.void_spitter")["HP"], "36");
        assert_eq!(stats("core.battle.ship.bulwark")["Hull"], "140");
        assert_eq!(
            stats("core.portraits.gunner.normal")["Post"],
            "Command Console"
        );
        assert!(stats("core.portraits.pilot.normal").is_empty());
        assert!(stats("core.unknown").is_empty());
    }

    #[test]
    fn images_fit_and_reveal_scrolls_minimally() {
        assert_eq!(
            fit(Vec2::new(200.0, 100.0), Vec2::splat(100.0)),
            Vec2::new(100.0, 50.0)
        );
        assert_eq!(
            fit(Vec2::new(10.0, 40.0), Vec2::splat(80.0)),
            Vec2::new(20.0, 80.0)
        );
        assert_eq!(reveal(0.0, 0.0, 100.0, 300.0), 0.0);
        assert_eq!(reveal(0.0, 400.0, 100.0, 300.0), 200.0);
        assert_eq!(reveal(500.0, 100.0, 100.0, 300.0), 100.0);
        assert_eq!(category_name("void_beasts"), "Void beasts");
    }
}
