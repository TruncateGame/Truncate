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
    #[error("Player must own the squares they swap")]
    UnownedSwap,
    #[error("Player cannot swap tiles from disconnected groups")]
    DisjointSwap,
    #[error("Swapping is disabled")]
    NoSwapping,

    #[error("Cannot place a tile in an occupied square")]
    OccupiedPlace,
    #[error("Must place tile on square that neighbours one of your already placed tiles, or on your root")]
    NonAdjacentPlace,

    #[error("Player {player:?} doesn't have a '{tile:?}' tile")]
    PlayerDoesNotHaveTile { player: usize, tile: char },
}
