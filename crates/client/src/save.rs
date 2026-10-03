//! Versioned client save data. This stays outside rollback; missions receive
//! only deterministic config derived from the loaded save.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// v2 (TAKOAI-42): adds `last_result` and `missions_won`. v1 saves load
/// through [`migrate`]; both fields default.
pub const SAVE_VERSION: u32 = 2;
const OLDEST_SUPPORTED_VERSION: u32 = 1;
const SAVE_FILE_NAME: &str = "save.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveGame {
    pub version: u32,
    pub credits: u32,
    pub resources: BTreeMap<String, u32>,
    pub purchased_upgrades: BTreeSet<String>,
    pub selected_battleship: String,
    /// Tutorial hint IDs already shown (`hints::Hint::id`).
    pub tutorial_seen: BTreeSet<String>,
    /// Missions played (also seeds the next mission).
    pub mission_count: u32,
    /// Outcome of the most recent mission; drives the crew's reaction lines.
    #[serde(default)]
    pub last_result: LastResult,
    #[serde(default)]
    pub missions_won: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LastResult {
    #[default]
    None,
    Success,
    Failed,
}

impl Default for SaveGame {
    fn default() -> Self {
        let mut resources = BTreeMap::new();
        resources.insert(crate::mission::VOID_CRYSTAL_ID.to_string(), 0);
        Self {
            version: SAVE_VERSION,
            // Demo Spec § Workshop upgrades: a new game starts with nothing.
            credits: 0,
            resources,
            purchased_upgrades: BTreeSet::new(),
            selected_battleship: crate::mission::KITE_ID.to_string(),
            tutorial_seen: BTreeSet::new(),
            mission_count: 0,
            last_result: LastResult::None,
            missions_won: 0,
        }
    }
}

impl SaveGame {
    /// Books a finished mission: adds what the player keeps and counts it.
    pub fn record_mission_return(
        &mut self,
        success: bool,
        credits: u32,
        resources: &BTreeMap<String, u32>,
    ) {
        self.credits = self.credits.saturating_add(credits);
        for (id, amount) in resources {
            *self.resources.entry(id.clone()).or_default() = self
                .resources
                .get(id)
                .copied()
                .unwrap_or(0)
                .saturating_add(*amount);
        }
        self.mission_count = self.mission_count.saturating_add(1);
        if success {
            self.missions_won = self.missions_won.saturating_add(1);
        }
        self.last_result = if success {
            LastResult::Success
        } else {
            LastResult::Failed
        };
    }
}

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    Json(serde_json::Error),
    VersionMismatch { found: u32, expected: u32 },
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "{e}"),
            SaveError::Json(e) => write!(f, "{e}"),
            SaveError::VersionMismatch { found, expected } => {
                write!(
                    f,
                    "save version {found} is not supported (expected {expected})"
                )
            }
        }
    }
}

impl std::error::Error for SaveError {}

impl From<io::Error> for SaveError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for SaveError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub fn save_path() -> PathBuf {
    if let Ok(dir) = std::env::var("ONI_SAVE_DIR") {
        return PathBuf::from(dir).join(SAVE_FILE_NAME);
    }
    data_dir().join("oni-spacewar").join(SAVE_FILE_NAME)
}

fn data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(dir) = std::env::var("APPDATA") {
            return PathBuf::from(dir);
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support");
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
            return PathBuf::from(dir);
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".local").join("share");
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn load(path: &Path) -> Result<SaveGame, SaveError> {
    migrate(serde_json::from_slice(&fs::read(path)?)?)
}

/// Brings an older save up to [`SAVE_VERSION`]. Every field added since v1
/// has a serde default, so upgrading is just stamping the new version; the
/// next store writes it back in the current format. Newer saves are refused.
pub fn migrate(mut save: SaveGame) -> Result<SaveGame, SaveError> {
    if !(OLDEST_SUPPORTED_VERSION..=SAVE_VERSION).contains(&save.version) {
        return Err(SaveError::VersionMismatch {
            found: save.version,
            expected: SAVE_VERSION,
        });
    }
    save.version = SAVE_VERSION;
    Ok(save)
}

pub fn store(path: &Path, save: &SaveGame) -> Result<(), SaveError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(save)?;
    fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(name: &str) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("oni-spacewar-{name}-{id}.json"))
    }

    #[test]
    fn save_round_trip() {
        let path = temp_file("round-trip");
        let mut save = SaveGame {
            credits: 275,
            last_result: LastResult::Failed,
            ..Default::default()
        };
        save.resources.insert("alloy".to_string(), 4);
        save.purchased_upgrades.insert("demo-drive".to_string());
        store(&path, &save).unwrap();
        assert_eq!(load(&path).unwrap(), save);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn v1_save_loads_and_migrates() {
        // A save written by the TAKOAI-40/41 builds (v1, no `missions_won`;
        // the first v1 builds had no `last_result` either).
        let path = temp_file("v1");
        let v1 = r#"{"version":1,"credits":120,"resources":{"void_crystal":3},
            "purchased_upgrades":["weapon_tuning_1"],"selected_battleship":"bulwark",
            "tutorial_seen":[],"mission_count":2}"#;
        fs::write(&path, v1).unwrap();
        let save = load(&path).unwrap();
        assert_eq!(save.version, SAVE_VERSION);
        assert_eq!((save.credits, save.mission_count), (120, 2));
        assert_eq!(save.last_result, LastResult::None);
        assert_eq!(save.missions_won, 0);
        assert!(save.purchased_upgrades.contains("weapon_tuning_1"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn mission_return_books_rewards_and_counts() {
        let mut save = SaveGame::default();
        let mut res = BTreeMap::new();
        res.insert("void_crystal".to_string(), 5);
        save.record_mission_return(true, 170, &res);
        save.record_mission_return(false, 0, &BTreeMap::new());
        assert_eq!((save.credits, save.resources["void_crystal"]), (170, 5));
        assert_eq!((save.mission_count, save.missions_won), (2, 1));
        assert_eq!(save.last_result, LastResult::Failed);
    }

    #[test]
    fn version_mismatch_is_rejected() {
        let path = temp_file("version");
        let save = SaveGame {
            version: SAVE_VERSION + 1,
            ..Default::default()
        };
        fs::write(&path, serde_json::to_vec(&save).unwrap()).unwrap();
        assert!(matches!(
            load(&path),
            Err(SaveError::VersionMismatch {
                found,
                expected: SAVE_VERSION
            }) if found == SAVE_VERSION + 1
        ));
        let _ = fs::remove_file(path);
    }
}
