//! Elimination mission bookkeeping: wave schedule, kill count, loot totals
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
    /// Current wave number, 1..=`WAVE_COUNT` (its banner may still be up).
    pub wave: u32,
    /// Ticks left on the "Wave N" banner; the wave spawns when it reaches 0.
    pub banner_ticks: u32,
    /// Ticks since the current wave started spawning.
    pub wave_ticks: u32,
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
            wave: 1,
            banner_ticks: WAVE_BANNER,
            wave_ticks: 0,
            kills: 0,
            spawned: 0,
            collected: Loot::default(),
            status: MissionStatus::InProgress,
            end_frame: 0,
        }
    }
}

impl Mission {
    pub fn banner_up(&self) -> bool {
        self.status == MissionStatus::InProgress && self.banner_ticks > 0
    }

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
            wave: self.wave,
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
    /// Wave reached.
    pub wave: u32,
    /// Loot collected during the mission.
    pub collected: Loot,
    /// Mission success bonus (zero on failure).
    pub bonus: Loot,
}
