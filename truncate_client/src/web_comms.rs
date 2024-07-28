use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use eframe::egui::Context;
use eframe::web_sys;
use futures::channel::{mpsc, oneshot};
use futures::SinkExt;
use futures_util::{future, pin_mut, StreamExt};
use truncate_core::messages::{GameMessage, Nonce, NoncedPlayerMessage, PlayerMessage};
use web_sys::console;
use ws_stream_wasm::{WsMessage, WsMeta, WsStream};

use crate::utils::macros::current_time;

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

    let most_recent_game_token: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let most_recent_login: Arc<Mutex<Option<PlayerMessage>>> = Arc::new(Mutex::new(None));

    let requested_login = AtomicBool::new(false);

    type NonceQueue = VecDeque<(Option<Nonce>, WsMessage)>;
    let mut pending_messages: NonceQueue = VecDeque::new();
    let unconfirmed_messages: Arc<Mutex<NonceQueue>> = Arc::new(Mutex::new(VecDeque::new()));

    let mut current_nonce = 0_u64;
    let mut get_nonce = || {
        current_nonce += 1;
        Nonce {
            generated_at: current_time!().as_secs(),
            id: current_nonce,
        }
    };

    let mut outgoing_msg_stream = rx_player.map(|message| {
        // Store a token that we're interacting with, in case we need to
        // recreate the connection.
        if let PlayerMessage::RejoinGame(token) = &message {
            *most_recent_game_token.lock().unwrap() = Some(token.to_string());
        }

        if let PlayerMessage::Login { .. } = &message {
            *most_recent_login.lock().unwrap() = Some(message.clone());
        }

        match &message {
            // Avoid noncing pings since we don't care about any individual ping.
            // Avoid noncing pre-login methods, as nonces don't work if the player is not logged in.
            PlayerMessage::Ping
            | PlayerMessage::Login { .. }
            | PlayerMessage::CreateAnonymousPlayer { .. } => (
                None,
                WsMessage::Text(serde_json::to_string(&message).unwrap()),
            ),
            _ => {
                let nonce = get_nonce();

                let wrapped_msg = NoncedPlayerMessage {
                    nonce: nonce.clone(),
                    message,
                };

                (
                    Some(nonce),
                    WsMessage::Text(serde_json::to_string(&wrapped_msg).unwrap()),
                )
            }
        }
    });

    loop {
        let Ok(wsio) = websocket_connect(&connect_addr).await else {
            console::log_1(&"Failed, waiting 2000ms".into());
            gloo_timers::future::TimeoutFuture::new(2000).await;
            continue;
        };

        let (mut outgoing, incoming) = wsio.split();

        if let Some(login) = most_recent_login.lock().unwrap().clone() {
            let encoded_login_msg = WsMessage::Text(serde_json::to_string(&login).unwrap());
            if outgoing.send(encoded_login_msg).await.is_err() {
                continue;
            };
        }

        if let Some(token) = most_recent_game_token.lock().unwrap().clone() {
            let reconnection_msg = PlayerMessage::RejoinGame(token);
            let encoded_reconnection_msg =
                WsMessage::Text(serde_json::to_string(&reconnection_msg).unwrap());
            if outgoing.send(encoded_reconnection_msg).await.is_err() {
                continue;
            };
        }

        {
            let mut unconfirmed = unconfirmed_messages.lock().unwrap();
            unconfirmed.extend(pending_messages.drain(..));
            std::mem::swap(&mut *unconfirmed, &mut pending_messages);
        }

        let game_messages = {
            incoming.for_each(|msg| async {
                let parsed_msg: GameMessage = match msg {
                    WsMessage::Text(msg) => serde_json::from_str(&msg).expect("Was not valid JSON"),
                    WsMessage::Binary(msg) => {
                        serde_json::from_slice(&msg).expect("Was not valid JSON")
                    }
                };

                match &parsed_msg {
                    GameMessage::Ping => {
                        _ = tx_player.clone().send(PlayerMessage::Ping).await;
                    }
                    GameMessage::PleaseLogin => {
                        requested_login.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    GameMessage::Ack(nonce) => {
                        let mut msgs = unconfirmed_messages.lock().unwrap();
                        if let Some(pos) = msgs
                            .iter()
                            .position(|(n, _)| n.as_ref().is_some_and(|n| n == nonce))
                        {
                            if pos != 0 {
                                tracing::warn!("Received an out of order ack from server at {pos}");
                            }

                            for _ in 0..=pos {
                                msgs.pop_front();
                            }
                        }
                    }
                    GameMessage::JoinedLobby(_, _, _, _, token) => {
                        // Store a token that we're interacting with, in case we need to
                        // recreate the connection.
                        *most_recent_game_token.lock().unwrap() = Some(token.to_string());
                    }
                    _ => { /* no processing needed */ }
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

                if requested_login.load(std::sync::atomic::Ordering::Relaxed) {
                    if let Some(login) = most_recent_login.lock().unwrap().as_ref() {
                        requested_login.store(false, std::sync::atomic::Ordering::Relaxed);
                        pending_messages.push_front((
                            None,
                            WsMessage::Text(serde_json::to_string(&login).unwrap()),
                        ))
                    }
                }

                if let Some(msg) = pending_messages.get(0).cloned() {
                    match outgoing.send(msg.1).await {
                        Ok(()) => {
                            if let (Some(nonce), msg) = pending_messages
                                .pop_front()
                                .expect("nothing else should remove from pending_messages")
                            {
                                let mut unconfirmed = unconfirmed_messages.lock().unwrap();
                                unconfirmed.push_back((Some(nonce), msg))
                            }
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
