//! Gameplay tips for the mission transition screen (`crate::loading`), the
//! only place tips show (TAKOAI-98). To add a tip, add a line to `TIPS`.
//! The texts are the retired tutorial hints (`hints.rs` before TAKOAI-98),
//! reworded where they pointed at something on screen. Client UI only: the
//! pick uses its own RNG, never the sim's.

use bevy::prelude::*;

pub const TIPS: &[&str] = &[
    "Click or hold to walk. Click a crew member, ship or the Upgrade Bench to use it.",
    "On the Carrier, click what a label points at to use it. E works too when you stand next to it.",
    "Board a battleship at its Dock to pick it and start a mission.",
    "At the Upgrade Bench, W / S pick an upgrade and E buys it.",
    "Spend credits and Void Crystal at the Upgrade Bench.",
    "Press B on the Carrier to build rooms, stations like the Salvage Sorter, and furniture.",
    "In Build mode, pick a room or an item, then click to build. Rooms open into their neighbours. Keep the front of a station clear.",
    "Click or hold to move. Your guns fire on their own.",
    "Q / W use skills. Watch the cooldowns.",
    "Fly over drops to collect them.",
];

/// Picks the transition screen's tip: random, never the one shown last.
#[derive(Resource)]
pub struct TipDeck {
    rng: u64,
    last: Option<usize>,
}

impl Default for TipDeck {
    /// Seeded from the clock: tips are UI, outside the deterministic sim.
    fn default() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        Self::seeded(nanos)
    }
}

impl TipDeck {
    pub fn seeded(seed: u64) -> Self {
        Self {
            // xorshift never leaves 0.
            rng: seed | 1,
            last: None,
        }
    }

    /// The next tip's index into `TIPS`.
    pub fn draw(&mut self) -> usize {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let pick = match self.last {
            // Draw from the others: skip over the last one.
            Some(last) if TIPS.len() > 1 => {
                let i = (self.rng % (TIPS.len() as u64 - 1)) as usize;
                if i >= last {
                    i + 1
                } else {
                    i
                }
            }
            _ => (self.rng % TIPS.len() as u64) as usize,
        };
        self.last = Some(pick);
        pick
    }

    pub fn next_tip(&mut self) -> &'static str {
        TIPS[self.draw()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tips_never_repeat_back_to_back_and_all_show_up() {
        let mut deck = TipDeck::seeded(7);
        let mut seen = vec![false; TIPS.len()];
        let mut prev = None;
        for _ in 0..1000 {
            let i = deck.draw();
            assert!(i < TIPS.len());
            assert_ne!(Some(i), prev, "tip {i} shown twice in a row");
            seen[i] = true;
            prev = Some(i);
        }
        assert!(seen.iter().all(|&s| s), "{seen:?}");
    }

    #[test]
    fn tips_are_short_single_lines() {
        assert!(TIPS.len() >= 2);
        for tip in TIPS {
            assert!(!tip.is_empty() && !tip.contains('\n'), "{tip:?}");
            assert!(tip.len() <= 160, "{tip:?}");
        }
    }
}
