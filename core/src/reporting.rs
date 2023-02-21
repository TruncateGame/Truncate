use serde::{Deserialize, Serialize};
use std::fmt;

use crate::board::{Coordinate, Square};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BoardChangeAction {
    Added,
    Swapped,
    Defeated,
    Truncated,
}

impl fmt::Display for BoardChangeAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BoardChangeAction::Added => write!(f, "Added"),
            BoardChangeAction::Swapped => write!(f, "Swapped"),
            BoardChangeAction::Defeated => write!(f, "Defeated"),
            BoardChangeAction::Truncated => write!(f, "Truncated"),
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
pub enum Change {
    Board(BoardChange),
    Hand(HandChange),
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Change::Board(c) => write!(f, "{c}"),
            Change::Hand(c) => write!(f, "{c}"),
        }
    }
}
