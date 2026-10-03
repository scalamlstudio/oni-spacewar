//! Typed client-side mission handoff. The demo is single-player, but the
//! mission config is already a list of ship loadouts so later multiplayer
//! launch code does not need a shape change.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sim::{SimParams, SimState};

use crate::save::SaveGame;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionType {
    Elimination,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShipLoadout {
    pub player_handle: usize,
    pub battleship_id: String,
    pub upgrade_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionConfig {
    pub mission_type: MissionType,
    pub ship_loadouts: Vec<ShipLoadout>,
    pub seed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionOutcome {
    Victory,
    Defeat,
    Abandoned,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionResult {
    pub outcome: MissionOutcome,
    pub credits: u32,
    pub resources: BTreeMap<String, u32>,
    pub waves_cleared: u32,
}

pub fn config_from_save(save: &SaveGame) -> MissionConfig {
    MissionConfig {
        mission_type: MissionType::Elimination,
        ship_loadouts: vec![ShipLoadout {
            player_handle: 0,
            battleship_id: save.selected_battleship.clone(),
            upgrade_ids: save.purchased_upgrades.iter().cloned().collect(),
        }],
        seed: 10_000 + save.mission_count as u64,
    }
}

pub fn launch_sim(config: &MissionConfig) -> SimState {
    SimState::new(SimParams {
        num_players: config.ship_loadouts.len(),
        seed: config.seed,
    })
}

pub fn result_from_sim(sim: &SimState, outcome: MissionOutcome) -> MissionResult {
    let mut resources = BTreeMap::new();
    let waves = sim.wave.saturating_sub(1);
    if matches!(outcome, MissionOutcome::Victory) {
        resources.insert("salvage".to_string(), 3 + waves);
    }
    MissionResult {
        outcome,
        credits: match outcome {
            MissionOutcome::Victory => 75 + waves * 10,
            MissionOutcome::Defeat | MissionOutcome::Abandoned => 0,
        },
        resources,
        waves_cleared: waves,
    }
}
