use std::fmt;
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

impl fmt::Display for PlayerMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PlayerMessage::NewGame(name) => write!(f, "Create a new game as player {}", name),
            PlayerMessage::JoinGame(room, name) => write!(f, "Join game {room} as player {}", name),
            PlayerMessage::StartGame => write!(f, "Start the game"),
            PlayerMessage::Place(coord, tile) => write!(f, "Place {} at {}", tile, coord),
            PlayerMessage::Swap(a, b) => write!(f, "Swap the tiles at {} and {}", a, b),
        }
    }
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

impl fmt::Display for GameStateMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "• Game {}, next up {}\n• Board:\n{}\n• Hand: {}\n• Just changed:\n{}",
            self.room_code,
            self.next_player_number,
            self.board,
            self.hand,
            self.changes
                .iter()
                .map(|c| format!("• • {c}\n"))
                .collect::<String>()
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameMessage {
    JoinedGame(RoomCode),
    StartedGame(GameStateMessage),
    GameUpdate(GameStateMessage),
    GameEnd(GameStateMessage, PlayerNumber),
    GameError(RoomCode, PlayerNumber, String),
}

impl fmt::Display for GameMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameMessage::JoinedGame(room) => write!(f, "Joined game {}", room),
            GameMessage::StartedGame(game) => write!(f, "Started game:\n{}", game),
            GameMessage::GameUpdate(game) => write!(f, "Update to game:\n{}", game),
            GameMessage::GameEnd(game, winner) => {
                write!(f, "Conclusion of game, winner was {}:\n{}", winner, game)
            }
            GameMessage::GameError(_, _, msg) => write!(f, "Error in game: {}", msg),
        }
    }
}
