//! The mission transition screen (`GameScreen::Loading`, TAKOAI-103):
//! between the Dock's launch and the Battle. It shows the mission name, a
//! progress bar and one tip (`crate::tips`) while it decodes every image the
//! battle draws (`render::battle_image_ids`: ships, enemies, fissures, loot,
//! shots), one per frame so the bar moves. Battle starts the frame they are
//! all in `Assets<Image>`, with no minimum time. A missing or broken image
//! stops on an error panel whose button (or Enter) returns to the Carrier.
//!
//! UI and assets only; the sim world is created on entering Battle. The demo
//! flow is single-player and offline. Networked missions will need every
//! peer to report its load done before Battle (there is no ready handshake
//! yet: synctest / p2p runs skip this flow).

use bevy::prelude::*;

use crate::art::ContentImages;
use crate::flow::{transition, FlowEvent, GameScreen, ScreenEntity};
use crate::mission::MissionType;
use crate::tips::TipDeck;

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MissionLoad>()
            .init_resource::<TipDeck>()
            .add_systems(OnEnter(GameScreen::Loading), enter_loading)
            .add_systems(
                Update,
                (load_step, back_to_carrier)
                    .chain()
                    .run_if(in_state(GameScreen::Loading)),
            );
    }
}

/// The current transition's load: images still to decode, the decoded
/// handles, and the IDs that failed.
#[derive(Resource, Default)]
pub struct MissionLoad {
    pending: Vec<&'static str>,
    loaded: Vec<(&'static str, Handle<Image>)>,
    pub failed: Vec<&'static str>,
    total: usize,
    /// The tip on screen.
    pub tip: &'static str,
    error_shown: bool,
}

impl MissionLoad {
    fn new(ids: Vec<&'static str>, tip: &'static str) -> Self {
        let mut pending = ids;
        // Popped from the back: load in list order.
        pending.reverse();
        Self {
            total: pending.len(),
            pending,
            tip,
            ..default()
        }
    }

    /// Share of the images decoded or given up on, 0..=1.
    pub fn progress(&self) -> f32 {
        if self.total == 0 {
            return 1.0;
        }
        (self.total - self.pending.len()) as f32 / self.total as f32
    }

    /// Every image is decoded and still in `images`.
    pub fn ready(&self, images: &Assets<Image>) -> bool {
        self.pending.is_empty()
            && self.failed.is_empty()
            && self.loaded.iter().all(|(_, h)| images.contains(h))
    }
}

#[derive(Component)]
struct ProgressFill;

#[derive(Component)]
struct LoadStatus;

#[derive(Component)]
struct BackToCarrier;

const PANEL: Color = Color::srgba(0.03, 0.05, 0.08, 0.9);
const ACCENT: Color = Color::srgb(0.35, 0.52, 0.58);
const ERROR: Color = Color::srgb(1.0, 0.4, 0.35);

fn text(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

pub fn mission_name(mission: &MissionType) -> &'static str {
    match mission {
        MissionType::Elimination => "Elimination",
    }
}

fn enter_loading(mut commands: Commands, mut load: ResMut<MissionLoad>, mut deck: ResMut<TipDeck>) {
    *load = MissionLoad::new(crate::render::battle_image_ids(), deck.next_tip());
    commands
        .spawn((
            ScreenEntity,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                BackgroundColor(PANEL),
                BorderColor::all(ACCENT),
                Node {
                    width: px(560),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(px(24)),
                    row_gap: px(14),
                    border: UiRect::all(px(1)),
                    ..default()
                },
            ))
            .with_children(|panel| {
                panel.spawn(text("MISSION", 14.0, Color::srgb(0.6, 0.7, 0.75)));
                panel.spawn(text(
                    mission_name(&MissionType::Elimination),
                    34.0,
                    Color::srgb(1.0, 0.86, 0.55),
                ));
                panel
                    .spawn((
                        BackgroundColor(Color::srgb(0.08, 0.1, 0.12)),
                        BorderColor::all(ACCENT),
                        Node {
                            width: percent(100),
                            height: px(12),
                            border: UiRect::all(px(1)),
                            ..default()
                        },
                    ))
                    .with_child((
                        ProgressFill,
                        BackgroundColor(Color::srgb(0.3, 0.8, 1.0)),
                        Node {
                            width: percent(0),
                            height: percent(100),
                            ..default()
                        },
                    ));
                panel.spawn((
                    LoadStatus,
                    text("Loading...", 15.0, Color::srgb(0.7, 0.75, 0.8)),
                ));
                panel.spawn(text("TIP", 14.0, Color::srgb(0.95, 0.8, 0.35)));
                panel.spawn(text(load.tip, 18.0, Color::srgb(1.0, 0.93, 0.7)));
            });
        });
}

/// Decodes the next image, moves the bar, and starts the Battle once every
/// image is in, or shows the error panel if any failed.
#[allow(clippy::too_many_arguments)]
fn load_step(
    mut commands: Commands,
    mut load: ResMut<MissionLoad>,
    mut art: ResMut<ContentImages>,
    mut images: ResMut<Assets<Image>>,
    state: Res<State<GameScreen>>,
    mut next: ResMut<NextState<GameScreen>>,
    mut fill: Query<&mut Node, With<ProgressFill>>,
    mut status: Query<(&mut Text, &mut TextColor), With<LoadStatus>>,
) {
    if let Some(id) = load.pending.pop() {
        match art.get(&mut images, id) {
            Some(handle) => load.loaded.push((id, handle)),
            None => load.failed.push(id),
        }
    }
    // A decoded image that is gone from `Assets` again is a failure too,
    // so the screen can never wait forever.
    if load.pending.is_empty() {
        let gone: Vec<_> = load
            .loaded
            .iter()
            .filter(|(_, h)| !images.contains(h))
            .map(|(id, _)| *id)
            .collect();
        load.failed.extend(gone);
    }
    let progress = load.progress();
    for mut node in &mut fill {
        node.width = percent(progress * 100.0);
    }
    if load.ready(&images) {
        next.set(transition(*state.get(), FlowEvent::AssetsLoaded, true));
        return;
    }
    if load.pending.is_empty() && !load.error_shown {
        load.error_shown = true;
        error!("mission assets failed to load: {:?}", load.failed);
        for (mut t, mut color) in &mut status {
            t.0 = format!(
                "Couldn't load the mission's assets: {}",
                load.failed.join(", ")
            );
            color.0 = ERROR;
        }
        commands
            .spawn((
                ScreenEntity,
                BackToCarrier,
                Button,
                BackgroundColor(Color::srgb(0.12, 0.22, 0.28)),
                BorderColor::all(ACCENT),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(34),
                    bottom: px(34),
                    width: px(240),
                    height: px(42),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(px(1)),
                    ..default()
                },
            ))
            .with_child(text("Back to Carrier  [Enter]", 18.0, Color::WHITE));
    }
}

fn back_to_carrier(
    load: Res<MissionLoad>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<BackToCarrier>)>,
    state: Res<State<GameScreen>>,
    mut next: ResMut<NextState<GameScreen>>,
) {
    if !load.error_shown {
        return;
    }
    let enter = keys.is_some_and(|k| k.just_pressed(KeyCode::Enter));
    if enter || buttons.iter().any(|i| *i == Interaction::Pressed) {
        next.set(transition(*state.get(), FlowEvent::ReturnToCarrier, true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    /// Whether every battle image was in `Assets<Image>` when Battle began.
    #[derive(Resource, Default)]
    struct LoadedAtBattle(Option<bool>);

    fn record_battle_start(
        mut seen: ResMut<LoadedAtBattle>,
        mut art: ResMut<ContentImages>,
        mut images: ResMut<Assets<Image>>,
    ) {
        let all = crate::render::battle_image_ids().iter().all(|id| {
            art.get(&mut images, id)
                .is_some_and(|h| images.contains(&h))
        });
        seen.0 = Some(all);
    }

    fn app(art: ContentImages) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin))
            .init_asset::<Image>()
            .insert_resource(art)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<LoadedAtBattle>()
            .insert_state(GameScreen::Carrier)
            .add_plugins(LoadingPlugin)
            .add_systems(OnEnter(GameScreen::Battle), record_battle_start);
        app.update();
        app
    }

    fn launch(app: &mut App) {
        let next = transition(GameScreen::Carrier, FlowEvent::LaunchMission, true);
        app.world_mut()
            .resource_mut::<NextState<GameScreen>>()
            .set(next);
    }

    fn screen(app: &App) -> GameScreen {
        *app.world().resource::<State<GameScreen>>().get()
    }

    #[test]
    fn launch_enters_loading_then_battle_with_every_image_loaded() {
        let mut app = app(ContentImages::repo());
        for mission in 0..2 {
            app.world_mut().resource_mut::<LoadedAtBattle>().0 = None;
            launch(&mut app);
            app.update();
            assert_eq!(screen(&app), GameScreen::Loading, "mission {mission}");
            let tip = app.world().resource::<MissionLoad>().tip;
            assert!(crate::tips::TIPS.contains(&tip));
            let mut frames = 0;
            while screen(&app) == GameScreen::Loading {
                app.update();
                frames += 1;
                assert!(frames < 100, "still loading after {frames} frames");
            }
            assert_eq!(screen(&app), GameScreen::Battle);
            assert_eq!(app.world().resource::<LoadedAtBattle>().0, Some(true));
            app.world_mut()
                .resource_mut::<NextState<GameScreen>>()
                .set(GameScreen::Carrier);
            app.update();
        }
    }

    #[test]
    fn missing_assets_show_an_error_with_a_way_back() {
        let mut app = app(ContentImages::at("/nonexistent/oni-assets"));
        launch(&mut app);
        for _ in 0..100 {
            app.update();
        }
        assert_eq!(screen(&app), GameScreen::Loading, "never starts the battle");
        let load = app.world().resource::<MissionLoad>();
        assert_eq!(load.failed.len(), crate::render::battle_image_ids().len());
        assert!(load.error_shown);
        let mut buttons = app
            .world_mut()
            .query_filtered::<Entity, With<BackToCarrier>>();
        assert_eq!(buttons.iter(app.world()).count(), 1);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        app.update();
        assert_eq!(screen(&app), GameScreen::Carrier);
        assert_eq!(app.world().resource::<LoadedAtBattle>().0, None);
    }

    #[test]
    fn consecutive_transitions_show_different_tips() {
        let mut app = app(ContentImages::repo());
        let mut last = None;
        for _ in 0..5 {
            launch(&mut app);
            app.update();
            let tip = app.world().resource::<MissionLoad>().tip;
            assert_ne!(Some(tip), last);
            last = Some(tip);
            app.world_mut()
                .resource_mut::<NextState<GameScreen>>()
                .set(GameScreen::Carrier);
            app.update();
        }
    }
}
