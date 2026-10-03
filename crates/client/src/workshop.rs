//! Workshop upgrade shop rules (design/READINESS.md § Demo Spec › Workshop
//! upgrades). Client-only economy: buying writes a purchase ID into the save;
//! the battle reads those IDs through `mission::upgrade_levels` into the sim
//! `Loadout`, so the effect itself lives in `sim` and stays deterministic.

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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

pub fn levels(save: &SaveGame) -> Upgrades {
    crate::mission::upgrade_levels(&save.purchased_upgrades)
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
    save.credits -= cost.credits;
    *save
        .resources
        .entry(VOID_CRYSTAL_ID.to_string())
        .or_default() -= cost.void_crystal;
    save.purchased_upgrades.insert(upgrade.purchase_id(level));
    Ok(level)
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
}
