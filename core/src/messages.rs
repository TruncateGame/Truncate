use serde::{Deserialize, Serialize};

use crate::{
    board::{Board, Coordinate},
    hand::Hand,
};

pub type RoomCode = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerMessage {
    NewGame, // TODO: Add player Name
    StartGame,
    JoinGame(RoomCode),
    Place(Coordinate, char),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameMessage {
    JoinedGame(RoomCode),
    StartedGame(RoomCode, Board, Hand), // TODO: All other events. GameStart(Board, Hand) next
}
