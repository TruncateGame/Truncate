use serde::{Deserialize, Serialize};

use crate::{
    board::{Board, Coordinate},
    hand::Hand,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerMessage {
    NewGame, // TODO: Add player Name
    StartGame,
    Place(Coordinate, char),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameMessage {
    JoinedGame(String),
    StartedGame(String, Board, Hand), // TODO: All other events. GameStart(Board, Hand) next
}
