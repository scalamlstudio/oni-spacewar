//! Workshop upgrade shop rules (design/READINESS.md § Demo Spec › Workshop
//! upgrades). Client-only economy: buying writes a purchase ID into the save;
//! the battle reads those IDs through `mission::upgrade_levels` into the sim
//! `Loadout`, so the effect itself lives in `sim` and stays deterministic.
//! Also the wallet side of Build mode: building pays up front, removing
//! refunds in full (design/READINESS.md § Carrier › Cost, refunds and
//! unlocks); the layout rules themselves are `crate::layout`'s.

use crate::layout::{ItemId, Reason, RoomSize, Tile};
use crate::mission::VOID_CRYSTAL_ID;
use crate::save::SaveGame;
use sim::tuning::MAX_UPGRADE_LEVEL;
use sim::Upgrades;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Upgrade {
    HullPlating,
    WeaponTuning,
    ThrusterTuning,
}

/// Credits and Void Crystal.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Cost {
    pub credits: u32,
    pub void_crystal: u32,
}

pub const UPGRADES: [Upgrade; 3] = [
    Upgrade::HullPlating,
    Upgrade::WeaponTuning,
    Upgrade::ThrusterTuning,
];

/// Level 1 and level 2 prices. Every upgrade costs the same in the spec.
pub const COSTS: [Cost; MAX_UPGRADE_LEVEL as usize] = [
    Cost {
        credits: 150,
        void_crystal: 4,
    },
    Cost {
        credits: 250,
        void_crystal: 6,
    },
];

impl Upgrade {
    pub fn name(self) -> &'static str {
        match self {
            Upgrade::HullPlating => "Hull Plating",
            Upgrade::WeaponTuning => "Weapon Tuning",
            Upgrade::ThrusterTuning => "Thruster Tuning",
        }
    }

    pub fn effect(self) -> &'static str {
        match self {
            Upgrade::HullPlating => "+25% hull per level",
            Upgrade::WeaponTuning => "+25% gun and skill damage per level",
            Upgrade::ThrusterTuning => "+15% move speed per level",
        }
    }

    /// Stable ID prefix; a purchase is saved as `<prefix>_<level>`, the
    /// format `mission::upgrade_levels` reads.
    pub fn id_prefix(self) -> &'static str {
        match self {
            Upgrade::HullPlating => "hull_plating",
            Upgrade::WeaponTuning => "weapon_tuning",
            Upgrade::ThrusterTuning => "thruster_tuning",
        }
    }

    /// Workshop row icon (`assets/source/core/ui/icon/`).
    pub fn icon_id(self) -> &'static str {
        match self {
            Upgrade::HullPlating => "core.ui.icon.hull_plating",
            Upgrade::WeaponTuning => "core.ui.icon.weapon_tuning",
            Upgrade::ThrusterTuning => "core.ui.icon.thruster_tuning",
        }
    }

    pub fn purchase_id(self, level: u8) -> String {
        format!("{}_{level}", self.id_prefix())
    }

    pub fn level_in(self, up: Upgrades) -> u8 {
        match self {
            Upgrade::HullPlating => up.hull,
            Upgrade::WeaponTuning => up.weapon,
            Upgrade::ThrusterTuning => up.thruster,
        }
    }
}

/// Every battleship modifier in the save: Workshop levels and the Combat
/// Simulator.
pub fn levels(save: &SaveGame) -> Upgrades {
    crate::mission::upgrade_levels(&crate::mission::upgrade_ids(save))
}

/// The next level of `upgrade` and its price, or `None` when it's maxed.
pub fn next_level(save: &SaveGame, upgrade: Upgrade) -> Option<(u8, Cost)> {
    let level = upgrade.level_in(levels(save));
    (level < MAX_UPGRADE_LEVEL).then(|| (level + 1, COSTS[level as usize]))
}

pub fn void_crystal(save: &SaveGame) -> u32 {
    save.resources.get(VOID_CRYSTAL_ID).copied().unwrap_or(0)
}

pub fn can_afford(save: &SaveGame, cost: Cost) -> bool {
    save.credits >= cost.credits && void_crystal(save) >= cost.void_crystal
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuyError {
    Maxed,
    TooExpensive(Cost),
}

/// Buy the next level of `upgrade`: spends the cost and records the
/// purchase. The caller saves the game.
pub fn buy(save: &mut SaveGame, upgrade: Upgrade) -> Result<u8, BuyError> {
    let (level, cost) = next_level(save, upgrade).ok_or(BuyError::Maxed)?;
    if !can_afford(save, cost) {
        return Err(BuyError::TooExpensive(cost));
    }
    spend(save, cost);
    save.purchased_upgrades.insert(upgrade.purchase_id(level));
    Ok(level)
}

fn spend(save: &mut SaveGame, cost: Cost) {
    save.credits -= cost.credits;
    *save
        .resources
        .entry(VOID_CRYSTAL_ID.to_string())
        .or_default() -= cost.void_crystal;
}

fn refund(save: &mut SaveGame, cost: Cost) {
    save.credits = save.credits.saturating_add(cost.credits);
    let vc = save
        .resources
        .entry(VOID_CRYSTAL_ID.to_string())
        .or_default();
    *vc = vc.saturating_add(cost.void_crystal);
}

/// What the save's wallet holds, as a [`Cost`].
pub fn wallet(save: &SaveGame) -> Cost {
    Cost {
        credits: save.credits,
        void_crystal: void_crystal(save),
    }
}

/// Builds a room of `size` anchored at (x, y): checks the layout and the
/// wallet, pays, and starts the construction site. Returns its index. The
/// caller saves.
pub fn build_room(save: &mut SaveGame, size: RoomSize, x: i32, y: i32) -> Result<usize, Reason> {
    save.carrier.can_place_room(size, x, y, wallet(save))?;
    spend(save, size.cost());
    Ok(save.carrier.place_room(size, x, y))
}

/// Builds `id` anchored at (x, y) (`pilot`: the tile under the Pilot's
/// feet): checks, pays, and starts the site. Returns its index. The caller
/// saves.
pub fn build_item(
    save: &mut SaveGame,
    id: ItemId,
    x: i32,
    y: i32,
    pilot: Option<Tile>,
) -> Result<usize, Reason> {
    save.carrier.can_place(id, x, y, wallet(save), pilot)?;
    let cost = id.def().ok_or(Reason::Blocked)?.cost;
    spend(save, cost);
    Ok(save.carrier.place(id, x, y, None))
}

/// Removes room `i` (an empty one the player built, or its site) for a
/// full refund. The caller saves.
pub fn remove_room(save: &mut SaveGame, i: usize) -> Result<Cost, Reason> {
    save.carrier.can_remove_room(i)?;
    let cost = save.carrier.remove_room(i);
    refund(save, cost);
    Ok(cost)
}

/// Removes item `i` (or cancels its site) for a full refund; starter
/// pieces refund nothing. The caller saves.
pub fn remove_item(save: &mut SaveGame, i: usize) -> Result<Cost, Reason> {
    save.carrier.can_remove(i)?;
    let cost = save.carrier.remove(i);
    refund(save, cost);
    Ok(cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rich(credits: u32, crystal: u32) -> SaveGame {
        let mut save = SaveGame {
            credits,
            ..Default::default()
        };
        save.resources.insert(VOID_CRYSTAL_ID.to_string(), crystal);
        save
    }

    #[test]
    fn buying_spends_the_cost_and_writes_the_battle_id() {
        let mut save = rich(500, 12);
        assert_eq!(buy(&mut save, Upgrade::WeaponTuning), Ok(1));
        assert_eq!((save.credits, void_crystal(&save)), (500 - 150, 12 - 4));
        assert!(save.purchased_upgrades.contains("weapon_tuning_1"));
        assert_eq!(buy(&mut save, Upgrade::WeaponTuning), Ok(2));
        assert_eq!((save.credits, void_crystal(&save)), (100, 2));
        assert_eq!(levels(&save).weapon, 2);
        assert_eq!(buy(&mut save, Upgrade::WeaponTuning), Err(BuyError::Maxed));
    }

    #[test]
    fn cannot_buy_without_both_currencies() {
        let mut save = rich(1000, 3);
        assert_eq!(
            buy(&mut save, Upgrade::HullPlating),
            Err(BuyError::TooExpensive(COSTS[0]))
        );
        let mut save = rich(149, 40);
        assert!(buy(&mut save, Upgrade::HullPlating).is_err());
        assert!(save.purchased_upgrades.is_empty());
        assert_eq!(save.credits, 149);
    }

    #[test]
    fn all_six_levels_cost_the_spec_total() {
        let mut save = rich(1200, 30);
        for u in UPGRADES {
            buy(&mut save, u).unwrap();
            buy(&mut save, u).unwrap();
        }
        assert_eq!((save.credits, void_crystal(&save)), (0, 0));
        let up = levels(&save);
        assert_eq!((up.hull, up.weapon, up.thruster), (2, 2, 2));
    }

    #[test]
    fn building_pays_and_removing_refunds_in_full() {
        // The spec's example edits: a Small Room south of the Workshop and a
        // Salvage Sorter in it.
        let mut save = rich(179, 4);
        let room = build_room(&mut save, RoomSize::Small, 14, 10).unwrap();
        assert_eq!((save.credits, void_crystal(&save)), (119, 3));
        assert!(!save.carrier.rooms[room].finished());
        // Nothing goes into a room still being built.
        assert_eq!(
            build_item(&mut save, ItemId::SalvageSorter, 19, 11, None),
            Err(Reason::StillBeingBuilt)
        );
        save.carrier.tick(5000);
        // Layout reasons come before the wallet.
        assert_eq!(
            build_item(&mut save, ItemId::SalvageSorter, 30, 18, None),
            Err(Reason::NeedsFloor)
        );
        assert_eq!(
            build_item(&mut save, ItemId::SalvageSorter, 19, 11, None),
            Err(Reason::Need(Cost {
                credits: 120,
                void_crystal: 3
            }))
        );
        save.credits += 1;
        let sorter = build_item(&mut save, ItemId::SalvageSorter, 19, 11, None).unwrap();
        assert_eq!((save.credits, void_crystal(&save)), (0, 0));
        assert_eq!(
            build_item(&mut save, ItemId::SalvageSorter, 15, 12, None),
            Err(Reason::AlreadyBuilt)
        );
        // The room isn't empty until the sorter (a site, cancelled) goes.
        let room = save.carrier.room_at((15, 12)).unwrap();
        assert_eq!(remove_room(&mut save, room), Err(Reason::RoomNotEmpty));
        assert_eq!(remove_item(&mut save, sorter).map(|c| c.credits), Ok(120));
        assert_eq!(remove_room(&mut save, room).map(|c| c.credits), Ok(60));
        assert_eq!((save.credits, void_crystal(&save)), (180, 4));
        assert_eq!(save.carrier, crate::layout::Carrier::starting());
        // Starter pieces: rooms can't go, fixed stations move instead,
        // furniture goes for nothing back.
        let bridge = save.carrier.room_at((2, 2)).unwrap();
        assert_eq!(remove_room(&mut save, bridge), Err(Reason::StarterPiece));
        let bench = save.carrier.find(ItemId::UpgradeBench).unwrap();
        assert_eq!(remove_item(&mut save, bench), Err(Reason::MoveItInstead));
        let lamp = save.carrier.find(ItemId::FloorLamp).unwrap();
        assert_eq!(remove_item(&mut save, lamp).map(|c| c.credits), Ok(0));
        assert_eq!(save.credits, 180);
    }
}
