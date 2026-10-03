//! Versioned client save data. This stays outside rollback; missions receive
//! only deterministic config derived from the loaded save.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const SAVE_VERSION: u32 = 1;
const SAVE_FILE_NAME: &str = "save.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveGame {
    pub version: u32,
    pub credits: u32,
    pub resources: BTreeMap<String, u32>,
    pub purchased_upgrades: BTreeSet<String>,
    pub selected_battleship: String,
    pub tutorial_seen: BTreeSet<String>,
    pub mission_count: u32,
    /// Outcome of the most recent mission; drives the crew's reaction lines.
    /// Added after v1 shipped, so older v1 saves load it as `None`.
    #[serde(default)]
    pub last_result: LastResult,
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
        resources.insert("salvage".to_string(), 0);
        Self {
            version: SAVE_VERSION,
            credits: 100,
            resources,
            purchased_upgrades: BTreeSet::new(),
            selected_battleship: "kite".to_string(),
            tutorial_seen: BTreeSet::new(),
            mission_count: 0,
            last_result: LastResult::None,
        }
    }
}

impl SaveGame {
    pub fn record_mission_return(&mut self, credits: u32, resources: &BTreeMap<String, u32>) {
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
    let save: SaveGame = serde_json::from_slice(&fs::read(path)?)?;
    if save.version != SAVE_VERSION {
        return Err(SaveError::VersionMismatch {
            found: save.version,
            expected: SAVE_VERSION,
        });
    }
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
    fn v1_save_without_last_result_loads() {
        let mut json = serde_json::to_value(SaveGame::default()).unwrap();
        json.as_object_mut().unwrap().remove("last_result");
        let save: SaveGame = serde_json::from_value(json).unwrap();
        assert_eq!(save.last_result, LastResult::None);
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
