//! Versioned JSON save data for the first playable shell.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const SAVE_SCHEMA_VERSION: u32 = 1;

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct SaveData {
    pub schema_version: u32,
    pub credits: u32,
    pub resources: BTreeMap<String, u32>,
    pub upgrade_levels: BTreeMap<String, u32>,
    pub unlocked_ships: Vec<String>,
    pub tutorial_flags_seen: Vec<String>,
}

impl Default for SaveData {
    fn default() -> Self {
        let mut resources = BTreeMap::new();
        resources.insert("alloy".to_string(), 0);
        Self {
            schema_version: SAVE_SCHEMA_VERSION,
            credits: 100,
            resources,
            upgrade_levels: BTreeMap::new(),
            unlocked_ships: vec!["starter_battleship".to_string()],
            tutorial_flags_seen: Vec::new(),
        }
    }
}

#[derive(Resource, Clone, Debug)]
pub struct SaveSlot {
    pub path: PathBuf,
}

impl SaveSlot {
    pub fn new() -> Self {
        Self { path: save_path() }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

pub fn load(slot: &SaveSlot) -> Result<SaveData, String> {
    let raw = std::fs::read_to_string(&slot.path)
        .map_err(|e| format!("read {}: {e}", slot.path.display()))?;
    let save: SaveData = serde_json::from_str(&raw).map_err(|e| format!("parse save: {e}"))?;
    if save.schema_version != SAVE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported save schema {} (expected {})",
            save.schema_version, SAVE_SCHEMA_VERSION
        ));
    }
    Ok(save)
}

pub fn store(slot: &SaveSlot, save: &SaveData) -> Result<(), String> {
    if let Some(parent) = slot.path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(save).map_err(|e| format!("encode save: {e}"))?;
    std::fs::write(&slot.path, raw).map_err(|e| format!("write {}: {e}", slot.path.display()))
}

fn save_path() -> PathBuf {
    user_data_dir().join("Oni Spacewar").join("save.json")
}

fn user_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(dir) = std::env::var_os("APPDATA") {
            return PathBuf::from(dir);
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support");
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(dir);
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".local").join("share");
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
