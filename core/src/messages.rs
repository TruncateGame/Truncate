use serde::{Deserialize, Serialize};

use crate::{
    board::{Board, Coordinate},
    player::Hand,
};

pub type RoomCode = String;
pub type PlayerNumber = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerMessage {
    NewGame, // TODO: Add player Name
    StartGame,
    JoinGame(RoomCode),
    Place(Coordinate, char),
    Swap(Coordinate, Coordinate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateMessage {
    pub room_code: RoomCode,
    pub player_number: PlayerNumber,
    pub next_player_number: PlayerNumber,
    pub board: Board,
    pub hand: Hand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameMessage {
    JoinedGame(RoomCode),
    StartedGame(GameStateMessage),
    GameUpdate(GameStateMessage),
    GameEnd(GameStateMessage, PlayerNumber),
    GameError(RoomCode, PlayerNumber, String),
}
