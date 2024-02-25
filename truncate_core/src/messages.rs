use std::{
    collections::BTreeMap,
    fmt::{self, Debug},
};
use time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    board::{Board, Coordinate},
    game::Game,
    moves::Move,
    player::{Hand, Player},
    reporting::{Change, WordMeaning},
};

pub type RoomCode = String;
pub type PlayerNumber = u64;
pub type TruncateToken = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlayerMessage {
    Ping,
    NewGame(String),
    JoinGame(RoomCode, String, Option<TruncateToken>),
    RejoinGame(TruncateToken),
    EditBoard(Board),
    EditName(String),
    StartGame,
    Resign,
    Place(Coordinate, char),
    Swap(Coordinate, Coordinate),
    Rematch,
    RequestDefinitions(Vec<String>),
    CreateAnonymousPlayer,
    Login(TruncateToken),
    LoadDailyPuzzle(TruncateToken, u32),
    PersistPuzzleMoves {
        player_token: TruncateToken,
        day: u32,
        human_player: u32,
        moves: Vec<Move>,
        won: bool,
    },
    RequestStats(TruncateToken),
    LoadReplay(String),
}

impl fmt::Display for PlayerMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PlayerMessage::Ping => write!(f, "Player ping"),
            PlayerMessage::NewGame(name) => write!(f, "Create a new game as player {}", name),
            PlayerMessage::JoinGame(room, name, token) => {
                write!(
                    f,
                    "Join game {room} as player {name}, but also maybe with token {token:#?}"
                )
            }
            PlayerMessage::RejoinGame(token) => {
                write!(f, "Player wants to rejoin a game using the token {}", token)
            }
            PlayerMessage::EditBoard(board) => write!(f, "Set board to {board}"),
            PlayerMessage::EditName(name) => write!(f, "Set name to {name}"),
            PlayerMessage::StartGame => write!(f, "Start the game"),
            PlayerMessage::Resign => write!(f, "Resign"),
            PlayerMessage::Place(coord, tile) => write!(f, "Place {} at {}", tile, coord),
            PlayerMessage::Swap(a, b) => write!(f, "Swap the tiles at {} and {}", a, b),
            PlayerMessage::Rematch => write!(f, "Rematch!"),
            PlayerMessage::RequestDefinitions(words) => write!(f, "Get definition of {words:?}"),
            PlayerMessage::CreateAnonymousPlayer => {
                write!(f, "Create a new anonymous player in the database")
            }
            PlayerMessage::Login(_token) => {
                write!(f, "Login as an existing player")
            }
            PlayerMessage::LoadDailyPuzzle(_token, day) => {
                write!(f, "Load any partial puzzle for day {day:?}")
            }
            PlayerMessage::PersistPuzzleMoves {
                player_token: _,
                human_player: _,
                day,
                moves,
                won: _,
            } => {
                write!(f, "Persist {} move(s) for day {day:?}", moves.len())
            }
            PlayerMessage::RequestStats(_token) => write!(f, "Requesting daily puzzle stats!"),
            PlayerMessage::LoadReplay(id) => write!(f, "Requesting the replay for {id}!"),
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
    pub turn_starts_no_later_than: Option<u64>,
}

impl GamePlayerMessage {
    pub fn new(p: &Player, _game: &Game) -> Self {
        Self {
            name: p.name.clone(),
            index: p.index,
            color: p.color,
            allotted_time: p.allotted_time,
            time_remaining: p.time_remaining,
            turn_starts_no_later_than: p.turn_starts_no_later_than,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateMessage {
    pub room_code: RoomCode,
    pub players: Vec<GamePlayerMessage>,
    pub player_number: PlayerNumber,
    pub next_player_number: Option<PlayerNumber>,
    pub board: Board,
    pub hand: Hand,
    pub changes: Vec<Change>,
    pub game_ends_at: Option<u64>,
}

impl fmt::Display for GameStateMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "• Game {}, next up {:?}\n• Board:\n{}\n• Hand: {}\n• Just changed:\n{}",
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
pub struct DailyStateMessage {
    pub puzzle_day: u32,
    pub attempt: u32,
    pub current_moves: Vec<Move>,
}

impl fmt::Display for DailyStateMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Attempt #{}, Moves: {}",
            self.attempt,
            self.current_moves
                .iter()
                .map(|m| format!("{m:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DailyAttempt {
    pub id: String,
    pub moves: u32,
    pub won: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DailyResult {
    pub attempts: Vec<DailyAttempt>,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DailyStats {
    pub days: BTreeMap<u32, DailyResult>,
}

impl DailyStats {
    pub fn hydrate_missing_days(&mut self) {
        let Some((start_day, _)) = self.days.first_key_value() else {
            return;
        };
        let Some((end_day, _)) = self.days.last_key_value() else {
            return;
        };
        for day in *start_day..*end_day {
            if !self.days.contains_key(&day) {
                self.days.insert(day, DailyResult::default());
            }
        }
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
        TruncateToken,
    ),
    LobbyUpdate(PlayerNumber, RoomCode, Vec<LobbyPlayerMessage>, Board),
    StartedGame(GameStateMessage),
    GameUpdate(GameStateMessage),
    GameEnd(GameStateMessage, PlayerNumber),
    GameError(RoomCode, PlayerNumber, String),
    GenericError(String),
    SupplyDefinitions(Vec<(String, Option<Vec<WordMeaning>>)>),
    LoggedInAs(TruncateToken),
    ResumeDailyPuzzle(DailyStateMessage),
    DailyStats(DailyStats),
    LoadDailyReplay(DailyStateMessage),
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
            GameMessage::SupplyDefinitions(_) => {
                write!(f, "Supplying definitions for words")
            }
            GameMessage::LoggedInAs(_token) => {
                write!(f, "Logged in as a player")
            }
            GameMessage::ResumeDailyPuzzle(puzzle) => write!(f, "Starting puzzle:\n{}", puzzle),
            GameMessage::DailyStats(stats) => write!(f, "Stats for {} days", stats.days.len()),
            GameMessage::LoadDailyReplay(puzzle) => write!(f, "Loading puzzle replay:\n{}", puzzle),
        }
    }
}
