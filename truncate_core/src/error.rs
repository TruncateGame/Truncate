use super::board::Coordinate;
use thiserror::Error;

#[derive(Clone, Error, Debug, PartialEq)]
pub enum GamePlayError {
    #[error("Invalid position ({:?}, {:?})", position.x, position.y)]
    InvalidPosition { position: Coordinate },
    #[error("Coordinate is not within board dimensions ({:?}, {:?})", position.x, position.y)]
    // TODO: should this be combined with InvalidPosition? How would we distinguish between dead squares and out of bounds? Should we?
    OutSideBoardDimensions { position: Coordinate },
    #[error("Empty square found in a word, where the word should be an unbroken line of non empty tiles")]
    EmptySquareInWord,

    #[error("Player {index:?} does not exist")]
    NonExistentPlayer { index: usize },

    #[error("Can't swap a square with itself")]
    SelfSwap,
    #[error("Must swap between occupied squares")]
    UnoccupiedSwap,
    #[error("You can't swap with an opponent's tile")]
    UnownedSwap,
    #[error("You can't swap tiles between disconnected groups")]
    DisjointSwap,
    #[error("Swapping is disabled")]
    NoSwapping,
    #[error("You can't swap {count} in a row")]
    TooManySwaps { count: String },

    #[error("You can't place a tile on top of another")]
    OccupiedPlace,
    #[error("You can only place tiles touching your dock or your existing tiles")]
    NonAdjacentPlace,

    #[error("Player {player:?} doesn't have a '{tile:?}' tile")]
    PlayerDoesNotHaveTile { player: usize, tile: char },
}
