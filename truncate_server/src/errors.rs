use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum TruncateServerError {
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("no sqlx connection exists")]
    DatabaseOffline,
    #[error("no user exists for {0}")]
    InvalidUser(Uuid),
    #[error("invalid token")]
    InvalidToken,
    #[error("this daily puzzle has already been won")]
    PuzzleComplete,
    #[error("something about this request was malformed")]
    BadRequest,
}
