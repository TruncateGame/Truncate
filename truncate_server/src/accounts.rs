use sqlx::PgPool;
use uuid::Uuid;

struct AnonymousPlayer {
    player_id: Uuid,
}

pub async fn create_player(pool: PgPool) -> Uuid {
    let player = sqlx::query_as!(
        AnonymousPlayer,
        "INSERT INTO players DEFAULT VALUES RETURNING player_id;"
    )
    .fetch_one(&pool)
    .await
    .expect("Default player should be create-able");

    player.player_id
}
