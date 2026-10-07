//! Elimination mission bookkeeping: spawn director timer, kill count, loot totals
//! and the win/lose result. Part of [`SimState`](crate::SimState), so it is
//! rolled back and checksummed like everything else.

use serde::{Deserialize, Serialize};

use crate::entity::Loot;
use crate::tuning::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum MissionStatus {
    #[default]
    InProgress,
    /// Kill target reached.
    Success,
    /// Every battleship in the mission was destroyed.
    Failed,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Mission {
    /// Ticks until the spawn director's next spawn (it waits at 0 while
    /// `MAX_LIVE_ENEMIES` are alive).
    pub spawn_timer: u32,
    /// Kills that win the mission. Part of the start state, so identical on
    /// every peer; `KILL_TARGET` unless the launch sets another value.
    pub kill_target: u32,
    pub kills: u32,
    pub spawned: u32,
    /// Loot picked up so far (shared by every ship in the mission).
    pub collected: Loot,
    pub status: MissionStatus,
    /// Frame on which the mission ended (meaningful once `status` isn't
    /// `InProgress`).
    pub end_frame: u32,
}

impl Default for Mission {
    fn default() -> Self {
        Self {
            spawn_timer: SPAWN_FIRST,
            kill_target: KILL_TARGET,
            kills: 0,
            spawned: 0,
            collected: Loot::default(),
            status: MissionStatus::InProgress,
            end_frame: 0,
        }
    }
}

impl Mission {
    /// The finished mission's result, or `None` while it is still running.
    pub fn outcome(&self) -> Option<MissionOutcome> {
        let success = match self.status {
            MissionStatus::InProgress => return None,
            MissionStatus::Success => true,
            MissionStatus::Failed => false,
        };
        Some(MissionOutcome {
            success,
            kills: self.kills,
            collected: self.collected,
            bonus: if success {
                Loot {
                    credits: SUCCESS_BONUS_CREDITS,
                    void_crystal: SUCCESS_BONUS_CRYSTAL,
                }
            } else {
                Loot::default()
            },
        })
    }
}

/// What the sim reports when a mission ends. Which loot the player keeps
/// (the "survives failure" flags) is decided by the client.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct MissionOutcome {
    pub success: bool,
    pub kills: u32,
    /// Loot collected during the mission.
    pub collected: Loot,
    /// Mission success bonus (zero on failure).
    pub bonus: Loot,
}
