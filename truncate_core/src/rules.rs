// TODO: Maximum consecutive swaps / stalemate rule

pub enum WinCondition {
    Destination,
    Elimination,
}

pub enum Visibility {
    Standard,
    FogOfWar,
}

pub enum Truncation {
    Root,
    Larger,
    None,
}

pub enum OvertimeRule {
    FreeWildcard { period: usize },
    RemoveTiles { period: usize, phase_time: usize },
    Elimination,
}

pub enum Timing {
    PerPlayer {
        time_allowance: usize,
        overtime_rule: OvertimeRule,
    },
    PerTurn {
        time_allowance: usize,
    },
    Periodic {
        turn_delay: usize,
    },
    None,
}

pub enum TileDistribution {
    Standard,
}

pub enum TileBagBehaviour {
    Standard,
    Infinite,
}

pub struct BattleRules {
    pub length_delta: isize,
}

pub enum Swapping {
    Contiguous,
    Universal,
    None,
}

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
}

impl Default for GameRules {
    fn default() -> Self {
        Self {
            win_condition: WinCondition::Destination,
            visibility: Visibility::Standard,
            truncation: Truncation::Root,
            timing: Timing::PerPlayer {
                time_allowance: 600,
                overtime_rule: OvertimeRule::FreeWildcard { period: 60 },
            },
            hand_size: 7,
            tile_distribution: TileDistribution::Standard,
            tile_bag_behaviour: TileBagBehaviour::Standard,
            battle_rules: BattleRules { length_delta: 2 },
            swapping: Swapping::Contiguous,
        }
    }
}
