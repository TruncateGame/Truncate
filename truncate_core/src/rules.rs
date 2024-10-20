// TODO: Maximum consecutive swaps / stalemate rule

use serde::{Deserialize, Serialize};

use crate::{
    board::Board,
    generation::{
        BoardElements, BoardNoiseParams, BoardParams, BoardSeed, DockType, Symmetry, WaterLayer,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TownDefense {
    BeatenByContact,
    BeatenByValidity,
    BeatenWithDefenseStrength(usize),
}

/// Conditions which, when hit, end the game and mark a winner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WinCondition {
    Destination { town_defense: TownDefense },
    Elimination, // TODO: Implement
}

/// Metrics to used to assign a winner when no condition was hit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WinMetric {
    TownProximity,
    ObeliskProximity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Standard,
    TileFog,
    LandFog,
    OnlyHouseFog,
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
        turn_delay: usize,
        total_time_allowance: usize,
    },
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TileBagBehaviour {
    Standard,
    Infinite, // TODO: Implement
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleRules {
    pub length_delta: isize,
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
pub enum BoardGenesis {
    Passthrough,
    SpecificBoard(Board),
    Classic(usize, usize),
    Random(BoardParams),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRules {
    pub generation: Option<u32>,
    pub win_condition: WinCondition,
    pub win_metric: WinMetric,
    pub visibility: Visibility,
    pub truncation: Truncation,
    pub timing: Timing,
    pub hand_size: usize,
    pub tile_generation: u32,
    pub tile_bag_behaviour: TileBagBehaviour,
    pub battle_rules: BattleRules,
    pub swapping: Swapping,
    pub battle_delay: u64,
    pub max_turns: Option<u64>,
    pub board_genesis: BoardGenesis,
}

const RULE_GENERATIONS: [GameRules; 2] = [
    GameRules {
        generation: None, // hydrated on fetch
        win_condition: WinCondition::Destination {
            town_defense: TownDefense::BeatenWithDefenseStrength(0),
        },
        win_metric: WinMetric::TownProximity,
        visibility: Visibility::Standard,
        truncation: Truncation::Root,
        timing: Timing::None,
        hand_size: 7,
        tile_generation: 0,
        tile_bag_behaviour: TileBagBehaviour::Standard,
        battle_rules: BattleRules { length_delta: 2 },
        swapping: Swapping::Contiguous(SwapPenalty::Disallowed { allowed_swaps: 1 }),
        battle_delay: 2,
        max_turns: None,
        board_genesis: BoardGenesis::Passthrough,
    },
    GameRules {
        generation: None, // hydrated on fetch
        win_condition: WinCondition::Destination {
            town_defense: TownDefense::BeatenWithDefenseStrength(0),
        },
        win_metric: WinMetric::TownProximity,
        visibility: Visibility::Standard,
        truncation: Truncation::Root,
        timing: Timing::None,
        hand_size: 7,
        tile_generation: 1,
        tile_bag_behaviour: TileBagBehaviour::Standard,
        battle_rules: BattleRules { length_delta: 2 },
        swapping: Swapping::Contiguous(SwapPenalty::Disallowed { allowed_swaps: 1 }),
        battle_delay: 2,
        max_turns: None,
        board_genesis: BoardGenesis::Passthrough,
    },
];

impl GameRules {
    pub fn generation(gen: u32) -> Self {
        let mut rules = RULE_GENERATIONS
            .get(gen as usize)
            .expect("rule generation should exist")
            .clone();
        rules.generation = Some(gen);
        rules
    }

    pub fn latest() -> (u32, Self) {
        assert!(!RULE_GENERATIONS.is_empty());
        let generation = (RULE_GENERATIONS.len() - 1) as u32;
        (generation, GameRules::generation(generation))
    }

    pub fn tuesday() -> Self {
        Self {
            generation: None, // hydrated on fetch
            win_condition: WinCondition::Destination {
                town_defense: TownDefense::BeatenWithDefenseStrength(0),
            },
            win_metric: WinMetric::ObeliskProximity,
            visibility: Visibility::LandFog,
            truncation: Truncation::None,
            timing: Timing::PerPlayer {
                time_allowance: 75 * 60,
                overtime_rule: OvertimeRule::Elimination,
            },
            hand_size: 7,
            tile_generation: 1,
            tile_bag_behaviour: TileBagBehaviour::Standard,
            battle_rules: BattleRules { length_delta: 1 },
            swapping: Swapping::Contiguous(SwapPenalty::Disallowed { allowed_swaps: 1 }),
            battle_delay: 2,
            max_turns: Some(1050),
            board_genesis: BoardGenesis::Random(BoardParams {
                land_layer: BoardNoiseParams {
                    dispersion: [3.0, 3.0],
                    symmetric: Symmetry::SmoothTwoFoldRotational,
                    island_influence: 0.48,
                },
                water_layer: Some(WaterLayer {
                    params: BoardNoiseParams {
                        dispersion: [10.0, 10.0],
                        island_influence: 0.0,
                        symmetric: Symmetry::TwoFoldRotational,
                    },
                    density: 0.42,
                }),
                land_dimensions: [103, 103],
                canvas_dimensions: [196, 196],
                maximum_town_density: 0.2,
                maximum_town_distance: 0.1,
                minimum_choke: 3,
                dock_type: DockType::Continental,
                ideal_dock_extremity: 0.0,
                elements: BoardElements {
                    docks: true,
                    towns: false,
                    obelisk: true,
                },
            }),
        }
    }
}
