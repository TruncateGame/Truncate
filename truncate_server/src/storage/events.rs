use crate::{errors::TruncateServerError, ServerState};

use super::accounts::AuthedTruncateToken;

pub async fn create_event(
    server_state: &ServerState,
    event_type: &String,
    player: Option<AuthedTruncateToken>,
) -> Result<(), TruncateServerError> {
    let Some(player_token) = player else {
        return Ok(());
    };

    println!("Tracking event: {event_type}");

    let Some(pool) = &server_state.truncate_db else {
        return Err(TruncateServerError::DatabaseOffline);
    };

    let player_id = player_token.player();

    sqlx::query!(
        "INSERT INTO events (
            event_type,
            player_id
        ) VALUES ($1, $2) RETURNING player_id;",
        event_type,
        player_id
    )
    .fetch_one(pool)
    .await
    .expect("Event should be good");

    Ok(())
}
