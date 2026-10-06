//! One-time tutorial hints (design/READINESS.md § Demo Spec › Tutorial
//! hints). Each hint shows once, the first time its trigger fires, and its ID
//! goes into the save's `tutorial_seen` right away. Client UI only: battle
//! triggers read `SimWorld` and never write to it.

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::flow::{GameScreen, PauseMenu, SaveSlot};
use crate::rollback::SimWorld;

/// Seconds a hint stays on screen.
const SHOW_SECS: f32 = 6.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hint {
    /// First Carrier load.
    CarrierWalk,
    /// First time the Pilot stands at something they can use.
    CarrierInteract,
    /// First time the Pilot walks into the Dock.
    Dock,
    /// First time the Workshop upgrade panel opens.
    Workshop,
    /// First return from a mission with loot.
    ReturnWithLoot,
    /// First Carrier arrival that can afford a room, with none built yet.
    CarrierCanBuild,
    /// First time in Build mode.
    CarrierBuild,
    /// First battle start.
    BattleMove,
    /// First enemy in basic-attack range.
    BattleSkills,
    /// First loot drop.
    BattleLoot,
}

impl Hint {
    /// Stable ID stored in `SaveGame::tutorial_seen`.
    pub fn id(self) -> &'static str {
        match self {
            Hint::CarrierWalk => "carrier_walk",
            Hint::CarrierInteract => "carrier_interact",
            Hint::Dock => "carrier_dock",
            Hint::Workshop => "carrier_workshop",
            Hint::ReturnWithLoot => "carrier_return_with_loot",
            Hint::CarrierCanBuild => "carrier_can_build",
            Hint::CarrierBuild => "carrier_build",
            Hint::BattleMove => "battle_move",
            Hint::BattleSkills => "battle_skills",
            Hint::BattleLoot => "battle_loot",
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Hint::CarrierWalk => {
                "Click to walk. Click a crew member, ship or the Workshop bench to use it."
            }
            Hint::CarrierInteract => {
                "Click what the label points at to use it. E works too when you stand next to it."
            }
            Hint::Dock => "Click a battleship here to board it and start a mission.",
            Hint::Workshop => {
                "W / S pick an upgrade, E buys it. Tab switches to Build for new rooms."
            }
            Hint::ReturnWithLoot => "Spend credits and Void Crystal at the Workshop.",
            Hint::CarrierCanBuild => "You can afford a new room. Build it at the Workshop bench.",
            Hint::CarrierBuild => {
                "Click a glowing slot to build. Rooms need a corridor at one of their doors."
            }
            Hint::BattleMove => "Click to move. Your guns fire on their own.",
            Hint::BattleSkills => "Q / W use skills. Watch the cooldowns.",
            Hint::BattleLoot => "Fly over drops to collect them.",
        }
    }

    /// The scene the hint belongs to; it is dropped if the player has left.
    pub fn screen(self) -> GameScreen {
        match self {
            Hint::BattleMove | Hint::BattleSkills | Hint::BattleLoot => GameScreen::Battle,
            _ => GameScreen::Carrier,
        }
    }
}

/// Hints waiting to be shown, and the one on screen with its seconds left.
#[derive(Resource, Default)]
pub struct Hints {
    queue: VecDeque<Hint>,
    showing: Option<(Hint, f32)>,
}

impl Hints {
    /// Fire `hint`'s trigger: queue it and record it as seen (and save) the
    /// first time; later triggers do nothing. Returns whether it was new.
    pub fn trigger(&mut self, save: &mut SaveSlot, hint: Hint) -> bool {
        let Some(game) = &mut save.game else {
            return false;
        };
        if !game.tutorial_seen.insert(hint.id().to_string()) {
            return false;
        }
        save.store();
        self.queue.push_back(hint);
        true
    }

    /// Advance the display by `dt` seconds on `screen`; returns the hint to
    /// show, if any.
    fn tick(&mut self, screen: GameScreen, dt: f32) -> Option<Hint> {
        if let Some((hint, left)) = &mut self.showing {
            *left -= dt;
            if *left <= 0.0 || hint.screen() != screen {
                self.showing = None;
            }
        }
        self.queue.retain(|h| h.screen() == screen);
        if self.showing.is_none() {
            self.showing = self.queue.pop_front().map(|h| (h, SHOW_SECS));
        }
        self.showing.map(|(h, _)| h)
    }
}

#[derive(Component)]
struct HintUi;

pub struct HintsPlugin;

impl Plugin for HintsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hints>()
            .add_systems(Startup, spawn_hint_ui)
            .add_systems(OnEnter(GameScreen::Battle), battle_start_hint)
            .add_systems(Update, battle_hints.run_if(in_state(GameScreen::Battle)))
            .add_systems(PostUpdate, show_hint);
    }
}

fn spawn_hint_ui(mut commands: Commands) {
    commands
        .spawn((
            HintUi,
            Visibility::Hidden,
            // Opaque: Carrier room labels scroll underneath.
            BackgroundColor(Color::srgb(0.05, 0.12, 0.16)),
            BorderColor::all(Color::srgb(0.95, 0.8, 0.35)),
            Node {
                position_type: PositionType::Absolute,
                max_width: px(600),
                padding: UiRect::axes(px(14), px(8)),
                border: UiRect::all(px(1)),
                ..default()
            },
            GlobalZIndex(10),
        ))
        .with_child((
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(17.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.93, 0.7)),
        ));
}

fn show_hint(
    time: Res<Time>,
    state: Res<State<GameScreen>>,
    pause: Res<PauseMenu>,
    mut hints: ResMut<Hints>,
    mut ui: Query<(&mut Visibility, &mut Node, &Children), With<HintUi>>,
    mut texts: Query<&mut Text>,
) {
    let Ok((mut vis, mut node, children)) = ui.single_mut() else {
        return;
    };
    // Clear of each scene's HUD: the Carrier's strip above the deck (left
    // of the wallet), the battle's bottom edge (above the controls line).
    let (top, bottom) = if *state.get() == GameScreen::Battle {
        (Val::Auto, px(44))
    } else {
        (px(8), Val::Auto)
    };
    if node.top != top || node.bottom != bottom {
        node.top = top;
        node.bottom = bottom;
        node.left = px(16);
    }
    let dt = if pause.open { 0.0 } else { time.delta_secs() };
    match hints.tick(*state.get(), dt) {
        Some(hint) if !pause.open => {
            if let Some(mut text) = children.first().and_then(|c| texts.get_mut(*c).ok()) {
                if text.0 != format!("Tip: {}", hint.text()) {
                    text.0 = format!("Tip: {}", hint.text());
                }
            }
            vis.set_if_neq(Visibility::Inherited);
        }
        _ => {
            vis.set_if_neq(Visibility::Hidden);
        }
    }
}

fn battle_start_hint(mut hints: ResMut<Hints>, mut save: ResMut<SaveSlot>) {
    hints.trigger(&mut save, Hint::BattleMove);
}

fn battle_hints(
    world: Option<Res<SimWorld>>,
    mut hints: ResMut<Hints>,
    mut save: ResMut<SaveSlot>,
) {
    let Some(world) = world else {
        return;
    };
    // Single-player demo: the local battleship is handle 0.
    let Some(ship) = world.ships.first() else {
        return;
    };
    let range = (ship.stats.basic_range + ship.stats.radius) as i64 * sim::SUB as i64;
    if world
        .enemies
        .iter()
        .any(|e| (e.pos - ship.pos).length() <= range)
    {
        hints.trigger(&mut save, Hint::BattleSkills);
    }
    if !world.pickups.is_empty() {
        hints.trigger(&mut save, Hint::BattleLoot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_queue_per_screen_and_expire() {
        let mut h = Hints::default();
        h.queue
            .extend([Hint::CarrierWalk, Hint::BattleMove, Hint::Dock]);
        assert_eq!(h.tick(GameScreen::Carrier, 0.0), Some(Hint::CarrierWalk));
        assert_eq!(h.tick(GameScreen::Carrier, SHOW_SECS), Some(Hint::Dock));
        // The battle hint was dropped: the player was on the Carrier.
        assert_eq!(h.tick(GameScreen::Carrier, SHOW_SECS), None);
        h.queue.push_back(Hint::BattleLoot);
        assert_eq!(h.tick(GameScreen::Carrier, 0.0), None);
    }

    #[test]
    fn a_hint_triggers_once_and_is_recorded() {
        let mut save = SaveSlot::scratch("hints", crate::save::SaveGame::default());
        let mut h = Hints::default();
        assert!(h.trigger(&mut save, Hint::Dock));
        assert!(!h.trigger(&mut save, Hint::Dock));
        assert!(save
            .game
            .as_ref()
            .unwrap()
            .tutorial_seen
            .contains("carrier_dock"));
        assert_eq!(h.queue.len(), 1);
    }

    #[test]
    fn hint_ids_are_unique() {
        let all = [
            Hint::CarrierWalk,
            Hint::CarrierInteract,
            Hint::Dock,
            Hint::Workshop,
            Hint::ReturnWithLoot,
            Hint::CarrierCanBuild,
            Hint::CarrierBuild,
            Hint::BattleMove,
            Hint::BattleSkills,
            Hint::BattleLoot,
        ];
        let ids: std::collections::BTreeSet<_> = all.iter().map(|h| h.id()).collect();
        assert_eq!(ids.len(), all.len());
    }

    #[test]
    fn carrier_walk_hint_leads_with_click_to_move() {
        // TAKOAI-62: click-to-move is the main Carrier control; WASD is a
        // secondary input and no longer the headline.
        let text = Hint::CarrierWalk.text();
        assert!(text.starts_with("Click to walk."));
        assert!(!text.contains("WASD"));
    }
}
