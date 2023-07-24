use std::fmt;
use time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    board::{Board, Coordinate},
    player::{Hand, Player},
    reporting::Change,
};

pub type RoomCode = String;
pub type PlayerNumber = u64;
pub type Token = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlayerMessage {
    Ping,
    NewGame(String),
    JoinGame(RoomCode, String),
    RejoinGame(Token),
    EditBoard(Board),
    EditName(String),
    StartGame,
    Place(Coordinate, char),
    Swap(Coordinate, Coordinate),
    Rematch,
}

impl fmt::Display for PlayerMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PlayerMessage::Ping => write!(f, "Player ping"),
            PlayerMessage::NewGame(name) => write!(f, "Create a new game as player {}", name),
            PlayerMessage::JoinGame(room, name) => write!(f, "Join game {room} as player {}", name),
            PlayerMessage::RejoinGame(token) => {
                write!(f, "Player wants to rejoin a game using the token {}", token)
            }
            PlayerMessage::EditBoard(board) => write!(f, "Set board to {board}"),
            PlayerMessage::EditName(name) => write!(f, "Set name to {name}"),
            PlayerMessage::StartGame => write!(f, "Start the game"),
            PlayerMessage::Place(coord, tile) => write!(f, "Place {} at {}", tile, coord),
            PlayerMessage::Swap(a, b) => write!(f, "Swap the tiles at {} and {}", a, b),
            PlayerMessage::Rematch => write!(f, "Rematch!"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyPlayerMessage {
    pub name: String,
    pub index: usize,
    pub color: (u8, u8, u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePlayerMessage {
    pub name: String,
    pub index: usize,
    pub color: (u8, u8, u8),
    pub allotted_time: Option<Duration>,
    pub time_remaining: Option<Duration>,
    pub turn_starts_at: Option<u64>,
}

impl From<&Player> for GamePlayerMessage {
    fn from(p: &Player) -> Self {
        Self {
            name: p.name.clone(),
            index: p.index,
            color: p.color,
            allotted_time: p.allotted_time,
            time_remaining: p.time_remaining,
            turn_starts_at: p.turn_starts_at,
        }
    }
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
    Ping,
    JoinedLobby(
        PlayerNumber,
        RoomCode,
        Vec<LobbyPlayerMessage>,
        Board,
        Token,
    ),
    LobbyUpdate(PlayerNumber, RoomCode, Vec<LobbyPlayerMessage>, Board),
    StartedGame(GameStateMessage),
    GameUpdate(GameStateMessage),
    GameEnd(GameStateMessage, PlayerNumber),
    GameError(RoomCode, PlayerNumber, String),
    GenericError(String),
}

impl fmt::Display for GameMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameMessage::Ping => write!(f, "Game ping"),
            GameMessage::JoinedLobby(player, room, players, board, _token) => write!(
                f,
                "Joined lobby {} as player {} with players {}. Board is:\n{}",
                player,
                room,
                players
                    .iter()
                    .map(|p| p.name.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                board
            ),
            GameMessage::LobbyUpdate(player, room, players, board) => write!(
                f,
                "Update to lobby {} as player {}. Players are {}. Board is:\n{}",
                player,
                room,
                players
                    .iter()
                    .map(|p| p.name.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                board
            ),
            GameMessage::StartedGame(game) => write!(f, "Started game:\n{}", game),
            GameMessage::GameUpdate(game) => write!(f, "Update to game:\n{}", game),
            GameMessage::GameEnd(game, winner) => {
                write!(f, "Conclusion of game, winner was {}:\n{}", winner, game)
            }
            GameMessage::GameError(_, _, msg) => write!(f, "Error in game: {}", msg),
            GameMessage::GenericError(msg) => write!(f, "Generic error: {}", msg),
        }
    }
}
