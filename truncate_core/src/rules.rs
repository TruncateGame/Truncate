// TODO: Maximum consecutive swaps / stalemate rule

#[derive(Debug, Clone)]
pub enum TownDefense {
    BeatenByContact,
    BeatenByValidity,
    BeatenWithDefenseStrength(usize),
}

#[derive(Debug, Clone)]
pub enum WinCondition {
    Destination { town_defense: TownDefense }, // TODO: Implement
    Elimination,                               // TODO: Implement
}

#[derive(Debug, Clone)]
pub enum Visibility {
    Standard,
    FogOfWar,
}

#[derive(Debug, Clone)]
pub enum Truncation {
    Root,
    Larger, // TODO: Implement
    None,
}

#[derive(Debug, Clone)]
pub enum OvertimeRule {
    FreeWildcard { period: usize },                   // TODO: Implement
    Bomb { period: usize },                           // TODO: Implement
    RemoveTiles { period: usize, phase_time: usize }, // TODO: Implement
    Elimination,                                      // TODO: Implement
}

#[derive(Debug, Clone)]
pub enum Timing {
    PerPlayer {
        time_allowance: usize,
        overtime_rule: OvertimeRule, // TODO: Implement
    },
    PerTurn {
        // TODO: Implement
        time_allowance: usize,
    },
    Periodic {
        // TODO: Implement
        turn_delay: usize,
    },
    None, // TODO: Implement
}

#[derive(Debug, Clone)]
pub enum TileDistribution {
    Standard,
}

#[derive(Debug, Clone)]
pub enum TileBagBehaviour {
    Standard, // TODO: Implement
    Infinite, // TODO: Implement
}

#[derive(Debug, Clone)]
pub struct BattleRules {
    pub length_delta: isize,
}

#[derive(Debug, Clone)]
pub enum Swapping {
    Contiguous(SwapPenalty),
    Universal(SwapPenalty),
    None,
}

#[derive(Debug, Clone)]
pub enum SwapPenalty {
    Time {
        swap_threshold: usize,
        penalties: Vec<usize>,
    },
    Disallowed {
        allowed_swaps: usize,
    },
}

#[derive(Debug, Clone)]
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
            battle_rules: BattleRules { length_delta: 2 },
            swapping: Swapping::Contiguous(SwapPenalty::Disallowed { allowed_swaps: 1 }),
            battle_delay: 2,
        }
    }
}
