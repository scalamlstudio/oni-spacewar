//! Versioned client save data. This stays outside rollback; missions receive
//! only deterministic config derived from the loaded save.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::layout::Carrier;
use crate::layout_v1::{CarrierLayout, RoomId};
use crate::workshop::Cost;

/// v2 (TAKOAI-42): adds `last_result` and `missions_won`. v3 (TAKOAI-56):
/// adds `carrier`, the 2.5D carrier layout. v4 (TAKOAI-60): the Dock is a
/// one-berth room and the carrier has two. v5 (TAKOAI-90): `carrier` is the
/// Revision 3 interior (rooms and items); a v1–v4 layout is refunded and
/// replaced by the starting one. Older saves load through [`migrate`];
/// every added field defaults.
pub const SAVE_VERSION: u32 = 5;
/// The last version whose `carrier` field is a Revision 1 layout.
const LAST_REV1_VERSION: u32 = 4;
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
    /// The carrier's rooms and items (design/READINESS.md § Carrier › Save
    /// (version 5)). Checked on load; see [`migrate`].
    #[serde(default)]
    pub carrier: Carrier,
    /// Transitional: the Revision 1 layout the Carrier scene still runs on
    /// until the Revision 3 scene lands (TAKOAI-88, Engineer B), which
    /// drops this field. Checked on load like the old `carrier` was.
    #[serde(default)]
    pub carrier_rev1: CarrierLayout,
    /// A v1–v4 save was migrated and the `carrier_refit` hint hasn't been
    /// shown yet (design/READINESS.md § Tutorial hints).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub carrier_refit: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LastResult {
    #[default]
    None,
    Success,
    Failed,
}

/// What a new game starts with. Loading a save keeps that save's balances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartingResources {
    pub credits: u32,
    pub void_crystal: u32,
}

/// Demo Spec § Workshop upgrades: a new game starts with nothing.
#[cfg_attr(not(test), allow(dead_code))] // Unused while the test values are live.
pub const RELEASE_STARTING_RESOURCES: StartingResources = StartingResources {
    credits: 0,
    void_crystal: 0,
};

/// TEST VALUES (TAKOAI-97): lets Wei try the Workshop and carrier without
/// grinding missions. Lower these before a real release.
pub const TEST_BUILD_STARTING_RESOURCES: StartingResources = StartingResources {
    credits: 10_000,
    void_crystal: 1_000,
};

/// The balances [`SaveGame::default`] (every new game) starts with. To
/// restore the release values, point this at [`RELEASE_STARTING_RESOURCES`].
pub const STARTING_RESOURCES: StartingResources = TEST_BUILD_STARTING_RESOURCES;

impl Default for SaveGame {
    fn default() -> Self {
        let mut resources = BTreeMap::new();
        resources.insert(
            crate::mission::VOID_CRYSTAL_ID.to_string(),
            STARTING_RESOURCES.void_crystal,
        );
        Self {
            version: SAVE_VERSION,
            credits: STARTING_RESOURCES.credits,
            resources,
            purchased_upgrades: BTreeSet::new(),
            selected_battleship: crate::mission::KITE_ID.to_string(),
            tutorial_seen: BTreeSet::new(),
            mission_count: 0,
            last_result: LastResult::None,
            missions_won: 0,
            carrier: Carrier::starting(),
            carrier_rev1: CarrierLayout::starting(),
            carrier_refit: false,
        }
    }
}

/// Salvage Bay: +25% of what a successful mission keeps (success bonus
/// included), rounded down.
pub const SALVAGE_BONUS_PCT: u32 = 25;

impl SaveGame {
    /// A new game at [`RELEASE_STARTING_RESOURCES`], for tests whose
    /// arithmetic assumes the release start.
    #[cfg(test)]
    pub(crate) fn release_start() -> Self {
        let mut save = Self {
            credits: RELEASE_STARTING_RESOURCES.credits,
            ..Self::default()
        };
        save.resources.insert(
            crate::mission::VOID_CRYSTAL_ID.to_string(),
            RELEASE_STARTING_RESOURCES.void_crystal,
        );
        save
    }

    /// What the Salvage Bay adds to a mission that kept `credits` and
    /// `resources`: nothing on failure or without the room.
    pub fn salvage_bonus(
        &self,
        success: bool,
        credits: u32,
        resources: &BTreeMap<String, u32>,
    ) -> (u32, BTreeMap<String, u32>) {
        if !success || !self.carrier_rev1.has_room(RoomId::SalvageBay) {
            return (0, BTreeMap::new());
        }
        let pct = |v: u32| v.saturating_mul(SALVAGE_BONUS_PCT) / 100;
        let extra = resources
            .iter()
            .map(|(id, v)| (id.clone(), pct(*v)))
            .filter(|(_, v)| *v > 0)
            .collect();
        (pct(credits), extra)
    }

    /// Books a finished mission: adds what the player keeps (plus the
    /// Salvage Bay's share) and counts it.
    pub fn record_mission_return(
        &mut self,
        success: bool,
        credits: u32,
        resources: &BTreeMap<String, u32>,
    ) {
        let (bonus_credits, bonus) = self.salvage_bonus(success, credits, resources);
        self.credits = self
            .credits
            .saturating_add(credits)
            .saturating_add(bonus_credits);
        for (id, amount) in resources.iter().chain(&bonus) {
            let v = self.resources.entry(id.clone()).or_default();
            *v = v.saturating_add(*amount);
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
    from_json(&fs::read(path)?)
}

/// Parses a save of any supported version and brings it up to
/// [`SAVE_VERSION`] ([`migrate`]). Up to v4 the `carrier` field holds a
/// Revision 1 layout; it is taken out and handed to the migration.
pub fn from_json(bytes: &[u8]) -> Result<SaveGame, SaveError> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)?;
    let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
    let mut old_carrier = None;
    if version <= LAST_REV1_VERSION as u64 {
        if let Some(obj) = value.as_object_mut() {
            old_carrier = obj.remove("carrier");
            obj.remove("carrier_rev1");
        }
    }
    let save: SaveGame = serde_json::from_value(value)?;
    let old_carrier = match old_carrier {
        Some(v) => Some(serde_json::from_value(v)?),
        None => None,
    };
    migrate(save, old_carrier)
}

/// Brings an older save up to [`SAVE_VERSION`]. Every field added since v1
/// has a serde default, so upgrading is mostly stamping the new version;
/// the next store writes it back in the current format. Newer saves are
/// refused.
///
/// v1–v4 (§ Carrier › Save › Migration): `old_carrier` is the save's
/// Revision 1 layout, if it had one. The full cost of every room and
/// corridor the player built in it is refunded at Revision 1's prices
/// (a v3 Dock is split first, as v4 did, so its corridors count right),
/// the carrier becomes the starting layout and the `carrier_refit` hint is
/// queued. Credits, Void Crystal, upgrades, hints, stats and the last ship
/// are kept.
///
/// Then the layout is checked: a broken one resets to the starting layout
/// and refunds every non-starter room and item (unknown IDs refund
/// nothing), with a warning.
pub fn migrate(
    mut save: SaveGame,
    old_carrier: Option<CarrierLayout>,
) -> Result<SaveGame, SaveError> {
    if !(OLDEST_SUPPORTED_VERSION..=SAVE_VERSION).contains(&save.version) {
        return Err(SaveError::VersionMismatch {
            found: save.version,
            expected: SAVE_VERSION,
        });
    }
    if save.version <= LAST_REV1_VERSION {
        if let Some(mut old) = old_carrier {
            if save.version == 3 {
                let refund = old.split_v3_dock();
                add_refund(&mut save, refund);
            }
            let refund = old.refund();
            add_refund(&mut save, refund);
        }
        save.carrier = Carrier::starting();
        save.carrier_rev1 = CarrierLayout::starting();
        save.carrier_refit = true;
    }
    save.version = SAVE_VERSION;
    save.carrier.normalize();
    if let Err(e) = save.carrier.validate() {
        let refund = save.carrier.refund();
        eprintln!(
            "warning: saved carrier layout is invalid ({e:?}); reset to the starting layout, refunded {} cr + {} VC",
            refund.credits, refund.void_crystal
        );
        add_refund(&mut save, refund);
        save.carrier = Carrier::starting();
    }
    // Transitional (see `SaveGame::carrier_rev1`).
    save.carrier_rev1.normalize();
    if let Err(e) = save.carrier_rev1.validate() {
        let refund = save.carrier_rev1.refund();
        eprintln!(
            "warning: saved Revision 1 carrier layout is invalid ({e:?}); reset, refunded {} cr + {} VC",
            refund.credits, refund.void_crystal
        );
        add_refund(&mut save, refund);
        save.carrier_rev1 = CarrierLayout::starting();
    }
    Ok(save)
}

fn add_refund(save: &mut SaveGame, refund: Cost) {
    save.credits = save.credits.saturating_add(refund.credits);
    let vc = save
        .resources
        .entry(crate::mission::VOID_CRYSTAL_ID.to_string())
        .or_default();
    *vc = vc.saturating_add(refund.void_crystal);
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

    fn vc(save: &SaveGame) -> u32 {
        save.resources["void_crystal"]
    }

    #[test]
    fn v2_save_migrates_to_the_starting_carrier() {
        let path = temp_file("v2");
        let v2 = r#"{"version":2,"credits":300,"resources":{"void_crystal":7},
            "purchased_upgrades":[],"selected_battleship":"kite",
            "tutorial_seen":["carrier_walk"],"mission_count":3,
            "last_result":"Success","missions_won":2}"#;
        fs::write(&path, v2).unwrap();
        let save = load(&path).unwrap();
        assert_eq!(save.version, SAVE_VERSION);
        assert_eq!(save.carrier, Carrier::starting());
        assert_eq!((save.credits, save.missions_won), (300, 2));
        assert!(save.carrier_refit);
        assert!(save.tutorial_seen.contains("carrier_walk"));
        // Written back as v5 with the new layout.
        store(&path, &save).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(r#""version": 5"#), "{text}");
        assert!(text.contains(r#""id": "command_console""#), "{text}");
        assert!(text.contains(r#""carrier_refit": true"#), "{text}");
        assert_eq!(load(&path).unwrap(), save);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn v3_save_refunds_every_player_piece_at_revision_1_prices() {
        // A v3 save with the spec's Salvage Bay off the old Dock's E door,
        // a player corridor (5,7) where v4's Dock link went and one more at
        // (8,6): Salvage Bay 120 cr + 3 VC and two corridors at 10 cr.
        let v3 = br#"{"version":3,"credits":5,"resources":{"void_crystal":1},
            "purchased_upgrades":["hull_plating_1"],"selected_battleship":"bulwark",
            "tutorial_seen":[],"mission_count":4,"last_result":"Success","missions_won":3,
            "carrier":{"rooms":[{"id":"bridge","x":0,"y":1},{"id":"crew_quarters","x":4,"y":1},
              {"id":"dock","x":4,"y":4},{"id":"salvage_bay","x":9,"y":5},
              {"id":"workshop","x":0,"y":4}],
              "corridors":[[1,3],[2,3],[3,3],[3,4],[3,5],[3,6],[4,3],[5,7],[8,6]]}}"#;
        let save = from_json(v3).unwrap();
        assert_eq!(save.version, SAVE_VERSION);
        assert_eq!(save.carrier, Carrier::starting());
        assert_eq!(save.carrier_rev1, CarrierLayout::starting());
        assert_eq!((save.credits, vc(&save)), (5 + 120 + 10 + 10, 1 + 3));
        assert_eq!(save.selected_battleship, "bulwark");
        assert!(save.purchased_upgrades.contains("hull_plating_1"));
        assert!(save.carrier_refit);
    }

    #[test]
    fn v3_dock_that_cant_split_refunds_only_player_pieces() {
        // The old Dock moved off its starting spot (a broken v3 layout):
        // only the player's own corridor (8,3) is refunded.
        let v3 = br#"{"version":3,"credits":0,"resources":{"void_crystal":0},
            "purchased_upgrades":[],"selected_battleship":"kite",
            "tutorial_seen":[],"mission_count":0,
            "carrier":{"rooms":[{"id":"bridge","x":0,"y":1},{"id":"dock","x":8,"y":0}],
              "corridors":[[1,3],[2,3],[8,3]]}}"#;
        let save = from_json(v3).unwrap();
        assert_eq!(save.carrier, Carrier::starting());
        assert_eq!((save.credits, vc(&save)), (10, 0));
    }

    #[test]
    fn v4_save_refunds_rooms_and_corridors() {
        use crate::layout_v1::Piece;
        let mut old = CarrierLayout::starting();
        old.place(Piece::Corridor, 8, 6);
        old.place(Piece::Room(RoomId::SalvageBay), 9, 5);
        old.place(Piece::Corridor, 6, 1);
        old.place(Piece::Room(RoomId::TrainingRoom), 7, 0);
        assert_eq!(old.validate(), Ok(()));
        let mut v4 = serde_json::to_value(SaveGame {
            version: 4,
            credits: 7,
            ..SaveGame::release_start()
        })
        .unwrap();
        let obj = v4.as_object_mut().unwrap();
        obj.insert("carrier".into(), serde_json::to_value(&old).unwrap());
        obj.remove("carrier_rev1");
        let save = from_json(&serde_json::to_vec(&v4).unwrap()).unwrap();
        assert_eq!(save.carrier, Carrier::starting());
        assert_eq!(save.carrier_rev1, CarrierLayout::starting());
        assert_eq!((save.credits, vc(&save)), (7 + 120 + 200 + 20, 3 + 6));
        assert!(save.carrier_refit);
    }

    #[test]
    fn broken_v4_layout_refunds_known_pieces_only() {
        // A Salvage Bay floating with no corridor (disconnected), an extra
        // corridor, and a room this build doesn't know.
        let v4 = br#"{"version":4,"credits":5,"resources":{"void_crystal":1},
            "purchased_upgrades":[],"selected_battleship":"kite",
            "tutorial_seen":[],"mission_count":1,"last_result":"None","missions_won":0,
            "carrier":{"rooms":[{"id":"bridge","x":0,"y":1},{"id":"crew_quarters","x":4,"y":1},
              {"id":"workshop","x":0,"y":4},{"id":"dock","x":4,"y":4},
              {"id":"salvage_bay","x":9,"y":0},{"id":"hangar_bay","x":9,"y":4}],
              "corridors":[[1,3],[2,3],[3,3],[4,3],[3,4],[3,5],[3,6],[8,6]]}}"#;
        let save = from_json(v4).unwrap();
        assert_eq!(save.carrier, Carrier::starting());
        assert_eq!((save.credits, vc(&save)), (5 + 120 + 10, 1 + 3));
    }

    /// The spec's example edits, finished, plus one site still building.
    fn built_carrier() -> Carrier {
        use crate::layout::{ItemId, RoomSize};
        let mut c = Carrier::starting();
        c.place_room(RoomSize::Small, 14, 10);
        c.tick(5000);
        c.place(ItemId::SalvageSorter, 19, 11, None);
        c.place(ItemId::CombatSimulator, 12, 9, None);
        c.tick(1000);
        c.place(ItemId::Stool, 16, 12, None);
        c
    }

    #[test]
    fn built_v5_layout_survives_a_round_trip() {
        let path = temp_file("layout");
        let save = SaveGame {
            carrier: built_carrier(),
            ..Default::default()
        };
        assert_eq!(save.carrier.validate(), Ok(()));
        store(&path, &save).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(r#""size": "small""#), "{text}");
        assert!(text.contains(r#""build_left_ms": 3000"#), "{text}");
        assert!(text.contains(r#""ship": "kite""#), "{text}");
        assert!(!text.contains("carrier_refit"), "{text}");
        let loaded = load(&path).unwrap();
        assert_eq!(loaded, save);
        assert!(!loaded.carrier_refit);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn v5_layout_loads_in_canonical_order() {
        let mut shuffled = built_carrier();
        shuffled.rooms.reverse();
        shuffled.items.reverse();
        let save = SaveGame {
            carrier: shuffled,
            ..Default::default()
        };
        let a = from_json(&serde_json::to_vec(&save).unwrap()).unwrap();
        assert_eq!(a.carrier, built_carrier());
        let again = serde_json::to_vec(&a).unwrap();
        assert_eq!(
            serde_json::to_vec(&from_json(&again).unwrap()).unwrap(),
            again
        );
    }

    #[test]
    fn broken_v5_layout_resets_with_a_refund() {
        // A Salvage Sorter (120 cr + 3 VC) out on bare hull, a Small Room
        // site (60 cr + 1 VC) floating free and an item this build doesn't
        // know (refunds nothing).
        let mut save = SaveGame {
            credits: 5,
            ..SaveGame::release_start()
        };
        let mut v = serde_json::to_value(&save).unwrap();
        let carrier = v["carrier"].as_object_mut().unwrap();
        let items = carrier["items"].as_array_mut().unwrap();
        items.push(serde_json::json!({"id": "salvage_sorter", "x": 30, "y": 18}));
        items.push(serde_json::json!({"id": "jacuzzi", "x": 2, "y": 2}));
        let rooms = carrier["rooms"].as_array_mut().unwrap();
        rooms.push(serde_json::json!({"size": "small", "x": 26, "y": 14, "build_left_ms": 100}));
        save = from_json(&serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!(save.carrier, Carrier::starting());
        assert_eq!((save.credits, vc(&save)), (5 + 120 + 60, 3 + 1));
        assert!(!save.carrier_refit);
    }

    #[test]
    fn salvage_bay_adds_a_quarter_on_success_only() {
        use crate::layout_v1::Piece;
        let mut save = SaveGame::release_start();
        let res = BTreeMap::from([("void_crystal".to_string(), 6)]);
        save.record_mission_return(true, 180, &res);
        assert_eq!((save.credits, save.resources["void_crystal"]), (180, 6));
        save.carrier_rev1.place(Piece::Corridor, 8, 6);
        save.carrier_rev1
            .place(Piece::Room(RoomId::SalvageBay), 9, 5);
        assert_eq!(
            save.salvage_bonus(true, 180, &res),
            (45, BTreeMap::from([("void_crystal".to_string(), 1)]))
        );
        save.record_mission_return(true, 180, &res);
        assert_eq!(
            (save.credits, save.resources["void_crystal"]),
            (180 + 225, 6 + 7)
        );
        save.record_mission_return(false, 0, &BTreeMap::new());
        assert_eq!(save.salvage_bonus(false, 100, &res), (0, BTreeMap::new()));
        assert_eq!(save.credits, 405);
    }

    #[test]
    fn new_game_starts_with_the_test_build_balances() {
        let save = SaveGame::default();
        assert_eq!(STARTING_RESOURCES, TEST_BUILD_STARTING_RESOURCES);
        assert_eq!((save.credits, vc(&save)), (10_000, 1_000));
        // Loading a save keeps its own balances.
        let mut v = serde_json::to_value(&save).unwrap();
        v["credits"] = 42.into();
        v["resources"]["void_crystal"] = 3.into();
        let loaded = from_json(&serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!((loaded.credits, vc(&loaded)), (42, 3));
    }

    #[test]
    fn mission_return_books_rewards_and_counts() {
        let mut save = SaveGame::release_start();
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
