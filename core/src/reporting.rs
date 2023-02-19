use serde::{Deserialize, Serialize};

use crate::board::{Coordinate, Square};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BoardChangeAction {
    Added,
    Swapped,
    Defeated,
    Truncated,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HandChange {
    pub player: usize,
    pub removed: Vec<char>,
    pub added: Vec<char>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Change {
    Board(BoardChange),
    Hand(HandChange),
}
