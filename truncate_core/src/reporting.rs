use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{
    board::{Board, Coordinate, Square},
    judge::Outcome,
    rules,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BoardChangeAction {
    Added,
    Swapped,
    Victorious,
    Defeated,
    Truncated,
    Exploded,
}

impl fmt::Display for BoardChangeAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BoardChangeAction::Added => write!(f, "Added"),
            BoardChangeAction::Swapped => write!(f, "Swapped"),
            BoardChangeAction::Victorious => write!(f, "Victorious"),
            BoardChangeAction::Defeated => write!(f, "Defeated"),
            BoardChangeAction::Truncated => write!(f, "Truncated"),
            BoardChangeAction::Exploded => write!(f, "Exploded"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardChangeDetail {
    pub square: Square,
    pub coordinate: Coordinate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardChange {
    pub detail: BoardChangeDetail,
    pub action: BoardChangeAction,
}

impl fmt::Display for BoardChange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "The square {} at {} was {}",
            self.detail.square, self.detail.coordinate, self.action
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HandChange {
    pub player: usize,
    pub removed: Vec<char>,
    pub added: Vec<char>,
}

impl fmt::Display for HandChange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Player {} used tiles {} and gained tiles {}",
            self.player,
            self.removed.iter().collect::<String>(),
            self.added.iter().collect::<String>()
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WordMeaning {
    pub pos: String,
    pub defs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BattleWord {
    pub original_word: String,
    pub resolved_word: String,
    pub meanings: Option<Vec<WordMeaning>>,
    pub valid: Option<bool>,
}

impl fmt::Display for BattleWord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} ({})",
            self.resolved_word,
            match self.valid {
                Some(true) => "Valid",
                Some(false) => "Invalid",
                None => "Unknown",
            }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BattleReport {
    pub battle_number: Option<u32>,
    pub attackers: Vec<BattleWord>,
    pub defenders: Vec<BattleWord>,
    pub outcome: Outcome,
}

impl fmt::Display for BattleReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Battle Report\nAttackers: {}\nDefenders: {}\nOutcome: {}",
            self.attackers
                .iter()
                .map(|w| format!("{w}"))
                .collect::<Vec<_>>()
                .join(", "),
            self.defenders
                .iter()
                .map(|w| format!("{w}"))
                .collect::<Vec<_>>()
                .join(", "),
            self.outcome
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimeChange {
    pub player: usize,
    pub time_change: isize,
    pub reason: String,
}

impl fmt::Display for TimeChange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.time_change {
            0..=isize::MAX => write!(f, "Player gained {} seconds", self.time_change),
            _ => write!(f, "Player lost {} seconds", self.time_change),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Change {
    Board(BoardChange),
    Hand(HandChange),
    Battle(BattleReport),
    Time(TimeChange),
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Change::Board(c) => write!(f, "{c}"),
            Change::Hand(c) => write!(f, "{c}"),
            Change::Battle(c) => write!(f, "{c}"),
            Change::Time(c) => write!(f, "{c}"),
        }
    }
}

pub(crate) fn filter_to_player(
    changes: &Vec<Change>,
    full_board: &Board,
    visible_board: &Board,
    player_index: usize,
    visibility: &rules::Visibility,
    winner: &Option<usize>,
) -> Vec<Change> {
    changes
        .iter()
        .filter_map(|change| match change {
            Change::Hand(HandChange {
                player: changed_player,
                removed: _,
                added: _,
            }) => {
                if *changed_player == player_index {
                    Some(change.clone())
                } else {
                    None
                }
            }
            Change::Board(BoardChange {
                detail: BoardChangeDetail { coordinate, square },
                action,
            }) => {
                let Some(relative_coord) =
                    full_board.map_game_coord_to_player(player_index, *coordinate, visibility)
                else {
                    return None;
                };
                let relative_change = Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: square.clone(),
                        coordinate: relative_coord,
                    },
                    action: action.clone(),
                });

                // All board visibility is restored when the game ends
                if winner.is_some() {
                    return Some(relative_change);
                }

                if action == &BoardChangeAction::Victorious
                    || action == &BoardChangeAction::Defeated
                    || action == &BoardChangeAction::Truncated
                    || action == &BoardChangeAction::Exploded
                {
                    return Some(relative_change);
                }
                match visibility {
                    rules::Visibility::Standard => Some(relative_change),
                    rules::Visibility::TileFog
                    | rules::Visibility::LandFog
                    | rules::Visibility::OnlyHouseFog => match visible_board.get(relative_coord) {
                        Ok(Square::Occupied(_, _)) => Some(relative_change),
                        _ => None,
                    },
                }
            }
            Change::Battle(_) => Some(change.clone()),
            Change::Time(_) => Some(change.clone()),
        })
        .collect::<Vec<_>>()
}
