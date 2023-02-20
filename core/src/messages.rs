use time::{Duration, OffsetDateTime};

use serde::{Deserialize, Serialize};

use crate::{
    board::{Board, Coordinate},
    player::Hand,
    reporting::Change,
};

pub type RoomCode = String;
pub type PlayerNumber = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerMessage {
    NewGame(String),
    JoinGame(RoomCode, String),
    StartGame,
    Place(Coordinate, char),
    Swap(Coordinate, Coordinate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePlayerMessage {
    pub name: String,
    pub index: usize,
    pub time_remaining: Duration,
    pub turn_starts_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateMessage {
    pub room_code: RoomCode,
    pub players: Vec<GamePlayerMessage>,
    pub player_number: PlayerNumber,
    pub next_player_number: PlayerNumber,
    pub board: Board,
    pub hand: Hand,
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameMessage {
    JoinedGame(RoomCode),
    StartedGame(GameStateMessage),
    GameUpdate(GameStateMessage),
    GameEnd(GameStateMessage, PlayerNumber),
    GameError(RoomCode, PlayerNumber, String),
}
