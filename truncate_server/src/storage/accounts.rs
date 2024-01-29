use jwt_simple::prelude::*;
use serde::{Deserialize, Serialize};
use truncate_core::messages::TruncateToken;
use uuid::Uuid;

use crate::{errors::TruncateServerError, ServerState};

#[derive(Clone)]
pub struct AuthedTruncateToken {
    token: TruncateToken,
    player_id: Uuid,
}

impl AuthedTruncateToken {
    pub fn player(&self) -> Uuid {
        self.player_id
    }
}

#[derive(Serialize, Deserialize)]
struct PlayerClaims {
    player_id: Uuid,
}

pub fn get_player_token(server_state: &ServerState, player_id: Uuid) -> TruncateToken {
    let claims = Claims::with_custom_claims(PlayerClaims { player_id }, Duration::from_days(1000));

    server_state
        .jwt_key
        .authenticate(claims)
        .expect("Claims should be serializable")
}

pub fn auth_player_token(
    server_state: &ServerState,
    token: TruncateToken,
) -> Result<AuthedTruncateToken, jwt_simple::Error> {
    server_state
        .jwt_key
        .verify_token::<PlayerClaims>(&token, None)
        .map(|t| AuthedTruncateToken {
            token,
            player_id: t.custom.player_id,
        })
}

struct AnonymousPlayer {
    player_id: Uuid,
}

pub async fn create_player(server_state: &ServerState) -> Result<Uuid, TruncateServerError> {
    let Some(pool) = &server_state.truncate_db else {
        return Err(TruncateServerError::DatabaseOffline);
    };

    let player = sqlx::query_as!(
        AnonymousPlayer,
        "INSERT INTO players DEFAULT VALUES RETURNING player_id;"
    )
    .fetch_one(pool)
    .await
    .expect("Default player should be create-able");

    Ok(player.player_id)
}

pub async fn login(
    server_state: &ServerState,
    token: TruncateToken,
) -> Result<Uuid, TruncateServerError> {
    let Some(pool) = &server_state.truncate_db else {
        return Err(TruncateServerError::DatabaseOffline);
    };

    let Ok(authed) = auth_player_token(server_state, token) else {
        return Err(TruncateServerError::InvalidToken);
    };
    let player_id = authed.player();

    let user_exists = sqlx::query!(
        "SELECT EXISTS(SELECT 1 FROM players WHERE player_id = $1)",
        player_id
    )
    .fetch_one(pool)
    .await?
    .exists
    .unwrap_or(false);

    if user_exists {
        sqlx::query!(
            "UPDATE players SET last_login = CURRENT_TIMESTAMP WHERE player_id = $1",
            player_id
        )
        .execute(pool)
        .await?;

        Ok(player_id)
    } else {
        Err(TruncateServerError::InvalidUser(player_id))
    }
}
