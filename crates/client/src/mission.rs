//! Typed client-side mission handoff. The demo is single-player, but the
//! mission config is already a list of ship loadouts so later multiplayer
//! launch code does not need a shape change.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sim::{Loadout, Loot, ShipKind, SimState, Upgrades};

use crate::save::SaveGame;

/// Stable battleship IDs (save file, Dock selection, content IDs).
pub const KITE_ID: &str = "kite";
pub const BULWARK_ID: &str = "bulwark";

/// Stable resource ID for Void Crystal in the save file.
pub const VOID_CRYSTAL_ID: &str = "void_crystal";

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
    /// Quit Mission from the pause menu: counts as Failed.
    Abandoned,
}

impl MissionOutcome {
    pub fn success(self) -> bool {
        matches!(self, Self::Victory)
    }
}

/// A reward type and whether a failed mission keeps it (Demo Spec § Loot:
/// every flag is off in the demo).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RewardRule {
    pub id: &'static str,
    pub survives_failure: bool,
}

pub const CREDITS_REWARD: RewardRule = RewardRule {
    id: "credits",
    survives_failure: false,
};
pub const VOID_CRYSTAL_REWARD: RewardRule = RewardRule {
    id: VOID_CRYSTAL_ID,
    survives_failure: false,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionResult {
    pub outcome: MissionOutcome,
    pub kills: u32,
    pub wave: u32,
    /// Mission length in sim ticks (60 per second), to the end or the quit.
    pub ticks: u32,
    /// Loot picked up during the mission.
    pub collected: Loot,
    /// Success bonus (zero unless Victory).
    pub bonus: Loot,
    /// What the player keeps: credits and resources by ID.
    pub credits: u32,
    pub resources: BTreeMap<String, u32>,
    /// Collected loot the player does not keep (its reward doesn't survive
    /// failure).
    pub lost: Loot,
}

pub fn ship_kind(battleship_id: &str) -> ShipKind {
    match battleship_id {
        BULWARK_ID => ShipKind::Bulwark,
        _ => ShipKind::Kite,
    }
}

/// Upgrade levels from purchased upgrade IDs `<upgrade>_<level>`, e.g.
/// `hull_plating_2`. The highest level bought wins.
pub fn upgrade_levels<'a>(ids: impl IntoIterator<Item = &'a String>) -> Upgrades {
    let mut up = Upgrades::default();
    for id in ids {
        let Some((name, level)) = id.rsplit_once('_') else {
            continue;
        };
        let Ok(level) = level.parse::<u8>() else {
            continue;
        };
        let level = level.min(sim::tuning::MAX_UPGRADE_LEVEL);
        let slot = match name {
            "hull_plating" => &mut up.hull,
            "weapon_tuning" => &mut up.weapon,
            "thruster_tuning" => &mut up.thruster,
            _ => continue,
        };
        *slot = (*slot).max(level);
    }
    up
}

/// Sent by the Carrier's Dock console on Launch. The flow stores the pick in
/// the save and enters Battle; `config_from_save` turns it into a config.
#[derive(bevy::prelude::Message, Clone, Debug, PartialEq, Eq)]
pub struct MissionRequest {
    pub battleship_id: String,
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

pub fn sim_loadouts(config: &MissionConfig) -> Vec<Loadout> {
    let mut loadouts = config.ship_loadouts.clone();
    loadouts.sort_by_key(|l| l.player_handle);
    loadouts
        .iter()
        .map(|l| Loadout {
            ship: ship_kind(&l.battleship_id),
            upgrades: upgrade_levels(&l.upgrade_ids),
        })
        .collect()
}

pub fn launch_sim(config: &MissionConfig) -> SimState {
    SimState::with_loadouts(config.seed, &sim_loadouts(config))
}

/// The mission's result. `outcome` comes from the sim when it ended there
/// (Victory/Defeat) or from the client for Quit Mission (Abandoned).
pub fn result_from_sim(sim: &SimState, outcome: MissionOutcome) -> MissionResult {
    let m = &sim.mission;
    let success = outcome.success();
    let bonus = sim
        .outcome()
        .map(|o| o.bonus)
        .filter(|_| success)
        .unwrap_or_default();
    let keep = |rule: RewardRule, amount: u32| {
        if success || rule.survives_failure {
            amount
        } else {
            0
        }
    };
    let credits = keep(CREDITS_REWARD, m.collected.credits + bonus.credits);
    let crystal = keep(
        VOID_CRYSTAL_REWARD,
        m.collected.void_crystal + bonus.void_crystal,
    );
    let mut resources = BTreeMap::new();
    if crystal > 0 {
        resources.insert(VOID_CRYSTAL_ID.to_string(), crystal);
    }
    let ticks = if m.status == sim::MissionStatus::InProgress {
        sim.frame
    } else {
        m.end_frame
    };
    MissionResult {
        outcome,
        kills: m.kills,
        wave: m.wave,
        ticks,
        collected: m.collected,
        bonus,
        credits,
        resources,
        lost: Loot {
            credits: (m.collected.credits + bonus.credits).saturating_sub(credits),
            void_crystal: (m.collected.void_crystal + bonus.void_crystal).saturating_sub(crystal),
        },
    }
}

/// The sim's own outcome, once its mission has ended.
pub fn sim_outcome(sim: &SimState) -> Option<MissionOutcome> {
    sim.outcome().map(|o| {
        if o.success {
            MissionOutcome::Victory
        } else {
            MissionOutcome::Defeat
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::MissionStatus;

    fn config(ship: &str, upgrades: &[&str]) -> MissionConfig {
        MissionConfig {
            mission_type: MissionType::Elimination,
            ship_loadouts: vec![ShipLoadout {
                player_handle: 0,
                battleship_id: ship.to_string(),
                upgrade_ids: upgrades.iter().map(|s| s.to_string()).collect(),
            }],
            seed: 1,
        }
    }

    #[test]
    fn loadouts_map_ship_and_upgrades() {
        let l = sim_loadouts(&config(
            BULWARK_ID,
            &[
                "hull_plating_1",
                "hull_plating_2",
                "weapon_tuning_1",
                "junk",
            ],
        ));
        assert_eq!(l[0].ship, ShipKind::Bulwark);
        assert_eq!(
            l[0].upgrades,
            Upgrades {
                hull: 2,
                weapon: 1,
                thruster: 0
            }
        );
        assert_eq!(
            sim_loadouts(&config("starter-frigate", &[]))[0].ship,
            ShipKind::Kite
        );
        let sim = launch_sim(&config(BULWARK_ID, &["hull_plating_1"]));
        assert_eq!(sim.ships[0].hull, 175);
    }

    #[test]
    fn victory_keeps_loot_plus_bonus() {
        let mut sim = launch_sim(&config(KITE_ID, &[]));
        sim.mission.collected = Loot {
            credits: 120,
            void_crystal: 3,
        };
        sim.mission.kills = 20;
        sim.mission.status = MissionStatus::Success;
        let outcome = sim_outcome(&sim).unwrap();
        assert_eq!(outcome, MissionOutcome::Victory);
        let r = result_from_sim(&sim, outcome);
        assert_eq!(r.credits, 120 + sim::tuning::SUCCESS_BONUS_CREDITS);
        assert_eq!(
            r.resources.get(VOID_CRYSTAL_ID),
            Some(&(3 + sim::tuning::SUCCESS_BONUS_CRYSTAL))
        );
        assert_eq!(r.kills, 20);
        assert_eq!(r.lost, Loot::default());
    }

    #[test]
    fn failure_and_quitting_lose_the_loot() {
        let mut sim = launch_sim(&config(KITE_ID, &[]));
        sim.mission.collected = Loot {
            credits: 60,
            void_crystal: 2,
        };
        let r = result_from_sim(&sim, MissionOutcome::Abandoned);
        assert_eq!((r.credits, r.resources.len()), (0, 0));
        assert_eq!(r.collected.credits, 60);
        assert_eq!(
            r.lost,
            Loot {
                credits: 60,
                void_crystal: 2
            }
        );
        sim.mission.status = MissionStatus::Failed;
        let r = result_from_sim(&sim, sim_outcome(&sim).unwrap());
        assert_eq!(r.outcome, MissionOutcome::Defeat);
        assert_eq!((r.credits, r.bonus), (0, Loot::default()));
    }
}
