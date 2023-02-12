use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::board::Coordinate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerMessage {
    NewGame, // TODO: Add player Name
    Place(Coordinate, char),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameMessage {
    JoinedGame(Uuid),
    // TODO: All other events. GameStart(Board, Hand) next
}
