use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use eframe::egui::Context;
use eframe::web_sys;
use futures::channel::{mpsc, oneshot};
use futures::SinkExt;
use futures_util::{future, pin_mut, StreamExt};
use truncate_core::messages::{GameMessage, PlayerMessage};
use web_sys::console;
use ws_stream_wasm::{WsMessage, WsMeta, WsStream};

async fn websocket_connect(connect_addr: &String) -> Result<WsStream, ()> {
    console::log_1(&format!("Connecting to {connect_addr}").into());

    let Ok((_ws, wsio)) = WsMeta::connect(connect_addr, None).await else {
        console::log_1(&"Failed to connect. . . .".into());
        return Err(());
    };

    console::log_1(&"Connected".into());

    Ok(wsio)
}

pub async fn connect(
    connect_addr: String,
    tx_game: mpsc::Sender<GameMessage>,
    tx_player: mpsc::Sender<PlayerMessage>,
    rx_player: mpsc::Receiver<PlayerMessage>,
    rx_context: oneshot::Receiver<Context>,
) {
    let mut context: Option<Context> = None;
    if let Ok(ctx) = rx_context.await {
        context = Some(ctx);
    }

    let most_recent_token: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let mut outgoing_msg_stream = rx_player.map(|msg| {
        // Store a token that we're interacting with, in case we need to
        // recreate the connection.
        if let PlayerMessage::RejoinGame(token) = &msg {
            *most_recent_token.lock().unwrap() = Some(token.to_string());
        }

        WsMessage::Text(serde_json::to_string(&msg.clone()).unwrap())
    });

    let mut pending_messages = VecDeque::new();

    loop {
        tracing::debug!("Starting to connect to websocket");

        let Ok(wsio) = websocket_connect(&connect_addr).await else {
            console::log_1(&"Failed, waiting 2000ms".into());
            gloo_timers::future::TimeoutFuture::new(2000).await;
            continue;
        };

        tracing::debug!("Connected to websocket");

        let (mut outgoing, incoming) = wsio.split();

        if let Some(token) = most_recent_token.lock().unwrap().clone() {
            let reconnection_msg = PlayerMessage::RejoinGame(token);
            let encoded_reconnection_msg =
                WsMessage::Text(serde_json::to_string(&reconnection_msg).unwrap());
            if outgoing.send(encoded_reconnection_msg).await.is_err() {
                continue;
            };
        }

        let game_messages = {
            incoming.for_each(|msg| async {
                let parsed_msg: GameMessage = match msg {
                    WsMessage::Text(msg) => serde_json::from_str(&msg).expect("Was not valid JSON"),
                    WsMessage::Binary(msg) => {
                        serde_json::from_slice(&msg).expect("Was not valid JSON")
                    }
                };

                if matches!(parsed_msg, GameMessage::Ping) {
                    _ = tx_player.clone().send(PlayerMessage::Ping).await;
                }

                // Store a token that we're interacting with, in case we need to
                // recreate the connection.
                if let GameMessage::JoinedLobby(_, _, _, _, token) = &parsed_msg {
                    *most_recent_token.lock().unwrap() = Some(token.to_string());
                }

                tx_game
                    .clone()
                    .send(parsed_msg)
                    .await
                    .expect("Message should have been able to go into the channel");
                if let Some(context) = context.as_ref() {
                    context.request_repaint();
                }
            })
        };

        let player_messages = async {
            loop {
                if pending_messages.is_empty() {
                    match outgoing_msg_stream.next().await {
                        Some(msg) => {
                            pending_messages.push_back(msg);
                        }
                        None => {
                            panic!("Internal stream closed");
                        }
                    }
                };

                if let Some(msg) = pending_messages.get(0).cloned() {
                    match outgoing.send(msg).await {
                        Ok(()) => {
                            pending_messages.pop_front();
                        }
                        Err(err) => {
                            tracing::debug!("Send err: {err:?}");
                        }
                    }
                }
            }
        };

        pin_mut!(game_messages, player_messages);
        future::select(game_messages, player_messages).await;
    }
}
