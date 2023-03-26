use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{
    board::{Board, Coordinate, Square},
    judge::Outcome,
    rules,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BoardChangeAction {
    Added,
    Swapped,
    Defeated,
    Truncated,
    Exploded,
}

impl fmt::Display for BoardChangeAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BoardChangeAction::Added => write!(f, "Added"),
            BoardChangeAction::Swapped => write!(f, "Swapped"),
            BoardChangeAction::Defeated => write!(f, "Defeated"),
            BoardChangeAction::Truncated => write!(f, "Truncated"),
            BoardChangeAction::Exploded => write!(f, "Exploded"),
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
pub struct BattleWord {
    pub word: String,
    pub definition: Option<String>,
    pub valid: Option<bool>,
}

impl fmt::Display for BattleWord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} ({})",
            self.word,
            match self.valid {
                Some(true) => "Valid",
                Some(false) => "Invalid",
                None => "Unknown",
            }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BattleReport {
    pub attackers: Vec<BattleWord>,
    pub defenders: Vec<BattleWord>,
    pub outcome: Outcome,
}

impl fmt::Display for BattleReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Battle Report\nAttackers: {}\nDefenders: {}\nOutcome: {}",
            self.attackers
                .iter()
                .map(|w| format!("{w}"))
                .collect::<Vec<_>>()
                .join(", "),
            self.defenders
                .iter()
                .map(|w| format!("{w}"))
                .collect::<Vec<_>>()
                .join(", "),
            self.outcome
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Change {
    Board(BoardChange),
    Hand(HandChange),
    Battle(BattleReport),
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Change::Board(c) => write!(f, "{c}"),
            Change::Hand(c) => write!(f, "{c}"),
            Change::Battle(c) => write!(f, "{c}"),
        }
    }
}

pub(crate) fn filter_to_player(
    changes: &Vec<Change>,
    visible_board: &Board,
    player_index: usize,
    visibility: &rules::Visibility,
) -> Vec<Change> {
    changes
        .iter()
        .filter(|change| match change {
            Change::Hand(HandChange {
                player: changed_player,
                removed: _,
                added: _,
            }) => *changed_player == player_index,
            Change::Board(BoardChange {
                detail:
                    BoardChangeDetail {
                        coordinate,
                        square: _,
                    },
                action,
            }) => {
                if action == &BoardChangeAction::Defeated || action == &BoardChangeAction::Truncated
                {
                    return true;
                }
                match visibility {
                    rules::Visibility::Standard => true,
                    rules::Visibility::FogOfWar => match visible_board.get(*coordinate) {
                        Ok(Square::Occupied(_, _)) => true,
                        _ => false,
                    },
                }
            }
            Change::Battle(_) => true,
        })
        .cloned()
        .collect::<Vec<_>>()
}
