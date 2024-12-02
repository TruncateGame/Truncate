use std::{env, time::Duration};

use clickhouse::Row;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::sync::mpsc::UnboundedReceiver;
use uuid::Uuid;

use crate::storage::accounts::AuthedTruncateToken;

pub struct Event {
    pub event_type: String,
    pub player: AuthedTruncateToken,
}

#[derive(Row, Serialize, Deserialize)]
struct BackendEvent {
    event_type: String,
    #[serde(with = "::clickhouse::serde::uuid")]
    player_id: Uuid,
    #[serde(with = "::clickhouse::serde::uuid")]
    event_id: Uuid,
    #[serde(with = "clickhouse::serde::time::datetime")]
    event_timestamp: OffsetDateTime,
}

pub async fn ch_events(mut rx: UnboundedReceiver<Event>) {
    loop {
        println!("Connecting to Clickhouse");
        let client = clickhouse::Client::default()
            .with_url(env::var("CLICKHOUSE_URL").unwrap())
            .with_user(env::var("CLICKHOUSE_USER").unwrap())
            .with_password(env::var("CLICKHOUSE_PASSWORD").unwrap())
            .with_database(env::var("CLICKHOUSE_DATABASE").unwrap());

        let mut i = client
            .inserter("events")
            .unwrap()
            .with_timeouts(Some(Duration::from_secs(5)), Some(Duration::from_secs(20)))
            .with_max_bytes(50_000_000)
            .with_max_rows(750_000)
            .with_period(Some(Duration::from_secs(15)));

        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            tokio::select! {
                Some(event) = rx.recv() => {
                    println!("Inserting event");
                    if let Err(e) = i.write(&BackendEvent {
                        event_type: event.event_type,
                        player_id: event.player.player(),
                        event_id: Uuid::new_v4(),
                        event_timestamp: OffsetDateTime::now_utc(),
                    }) {
                        eprintln!("Failed to write event: {e}");
                        break;
                    }
                }
                _ = interval.tick() => {
                    // Commit every 10s
                }
            }
            if i.pending().rows > 0 {
                println!("Committing events");
                if let Err(e) = i.commit().await {
                    eprintln!("Failed to commit events: {e}");
                    break;
                }
            }
        }
    }
}
