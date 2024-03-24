use jwt_simple::prelude::*;
use serde::{Deserialize, Serialize};
use truncate_core::messages::TruncateToken;
use uuid::Uuid;
use woothee::parser::Parser as UAParser;

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

    pub fn token(&self) -> TruncateToken {
        self.token.clone()
    }
}

#[derive(Serialize, Deserialize)]
struct PlayerClaims {
    player_id: Uuid,
}

pub fn get_player_token(server_state: &ServerState, player_id: Uuid) -> AuthedTruncateToken {
    let claims =
        Claims::with_custom_claims(PlayerClaims { player_id }, Duration::from_days(100000));

    let token = server_state
        .jwt_key
        .authenticate(claims)
        .expect("Claims should be serializable");
    let authed_token = server_state
        .jwt_key
        .verify_token::<PlayerClaims>(&token, None)
        .map(|t| AuthedTruncateToken {
            token,
            player_id: t.custom.player_id,
        })
        .expect("We just generated this");

    authed_token
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

pub async fn create_player(
    server_state: &ServerState,
    screen_width: u32,
    screen_height: u32,
    user_agent: String,
    referrer: String,
) -> Result<Uuid, TruncateServerError> {
    let Some(pool) = &server_state.truncate_db else {
        return Err(TruncateServerError::DatabaseOffline);
    };

    let parsed_ua = UAParser::new().parse(&user_agent);

    let (browser_name, browser_version) = if let Some(ua) = parsed_ua {
        (ua.name, ua.version)
    } else {
        ("Unknown", "Unknown")
    };

    let player = sqlx::query_as!(
        AnonymousPlayer,
        "INSERT INTO players (
            last_screen_width,
            last_screen_height,
            last_browser_name,
            last_browser_version,
            first_referrer
        ) VALUES ($1, $2, $3, $4, $5) RETURNING player_id;",
        screen_width as i32,
        screen_height as i32,
        browser_name,
        browser_version,
        referrer
    )
    .fetch_one(pool)
    .await
    .expect("Default player should be create-able");

    Ok(player.player_id)
}

pub async fn login(
    server_state: &ServerState,
    token: TruncateToken,
    screen_width: u32,
    screen_height: u32,
    user_agent: String,
) -> Result<(Uuid, AuthedTruncateToken), TruncateServerError> {
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
        let parsed_ua = UAParser::new().parse(&user_agent);

        let (browser_name, browser_version) = if let Some(ua) = parsed_ua {
            (ua.name, ua.version)
        } else {
            ("Unknown", "Unknown")
        };

        sqlx::query!(
            "UPDATE players
            SET 
                last_login = CURRENT_TIMESTAMP, 
                last_screen_width = $2,
                last_screen_height = $3,
                last_browser_name = $4,
                last_browser_version = $5
            WHERE player_id = $1",
            player_id,
            screen_width as i32,
            screen_height as i32,
            browser_name,
            browser_version,
        )
        .execute(pool)
        .await?;

        Ok((player_id, authed))
    } else {
        Err(TruncateServerError::InvalidUser(player_id))
    }
}
