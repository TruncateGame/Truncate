// TODO: Maximum consecutive swaps / stalemate rule

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TownDefense {
    BeatenByContact,
    BeatenByValidity,
    BeatenWithDefenseStrength(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WinCondition {
    Destination { town_defense: TownDefense },
    Elimination, // TODO: Implement
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Standard,
    TileFog,
    LandFog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Truncation {
    Root,
    Larger, // TODO: Implement
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OvertimeRule {
    FreeWildcard { period: usize },
    Bomb { period: usize },
    RemoveTiles { period: usize, phase_time: usize }, // TODO: Implement
    Elimination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Timing {
    PerPlayer {
        time_allowance: usize,
        overtime_rule: OvertimeRule,
    },
    PerTurn {
        // TODO: Implement
        time_allowance: usize,
    },
    Periodic {
        // TODO: Implement
        turn_delay: usize,
    },
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TileDistribution {
    Standard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TileBagBehaviour {
    Standard,
    Infinite, // TODO: Implement
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Champion {
    Weakest,
    Strongest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleRules {
    pub length_delta: isize,
    pub attacker_champion: Champion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Swapping {
    Contiguous(SwapPenalty),
    Universal(SwapPenalty),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwapPenalty {
    Time {
        swap_threshold: usize,
        penalties: Vec<usize>,
    },
    Disallowed {
        allowed_swaps: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRules {
    pub win_condition: WinCondition,
    pub visibility: Visibility,
    pub truncation: Truncation,
    pub timing: Timing,
    pub hand_size: usize,
    pub tile_distribution: TileDistribution,
    pub tile_bag_behaviour: TileBagBehaviour,
    pub battle_rules: BattleRules,
    pub swapping: Swapping,
    pub battle_delay: u64,
}

impl Default for GameRules {
    fn default() -> Self {
        Self {
            win_condition: WinCondition::Destination {
                town_defense: TownDefense::BeatenWithDefenseStrength(0),
            },
            visibility: Visibility::Standard,
            truncation: Truncation::Root,
            timing: Timing::None,
            hand_size: 7,
            tile_distribution: TileDistribution::Standard,
            tile_bag_behaviour: TileBagBehaviour::Standard,
            battle_rules: BattleRules { length_delta: 2, attacker_champion: Champion::Weakest },
            swapping: Swapping::Contiguous(SwapPenalty::Disallowed { allowed_swaps: 1 }),
            battle_delay: 2,
        }
    }
}
