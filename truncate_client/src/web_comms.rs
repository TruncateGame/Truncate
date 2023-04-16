use eframe::egui::Context;
use eframe::web_sys;
use futures::channel::{mpsc, oneshot};
use futures::SinkExt;
use futures_util::{future, pin_mut, StreamExt};
use truncate_core::messages::{GameMessage, PlayerMessage};
use ws_stream_wasm::{WsMessage, WsMeta};

pub async fn connect(
    connect_addr: String,
    tx_game: mpsc::Sender<GameMessage>,
    tx_player: mpsc::Sender<PlayerMessage>,
    rx_player: mpsc::Receiver<PlayerMessage>,
    rx_context: oneshot::Receiver<Context>,
) {
    use web_sys::console;

    let mut context: Option<Context> = None;

    console::log_1(&format!("Connecting to {connect_addr}").into());

    let (_ws, wsio) = WsMeta::connect(connect_addr, None)
        .await
        .expect("assume the connection succeeds");

    console::log_1(&"Connected".into());

    if let Ok(ctx) = rx_context.await {
        context = Some(ctx);
    }

    let (outgoing, incoming) = wsio.split();

    let game_messages = {
        incoming.for_each(|msg| async {
            let parsed_msg: GameMessage = match msg {
                WsMessage::Text(msg) => serde_json::from_str(&msg).expect("Was not valid JSON"),
                WsMessage::Binary(msg) => serde_json::from_slice(&msg).expect("Was not valid JSON"),
            };

            if matches!(parsed_msg, GameMessage::Ping) {
                _ = tx_player.clone().send(PlayerMessage::Ping).await;
            } else {
                console::log_1(&format!("Received {parsed_msg}").into());
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

    let player_messages = {
        rx_player
            .map(|msg| {
                if !matches!(msg, PlayerMessage::Ping) {
                    console::log_1(&format!("Sending {msg}").into());
                }
                Ok(WsMessage::Text(
                    serde_json::to_string(&msg.clone()).unwrap(),
                ))
            })
            .forward(outgoing)
    };

    pin_mut!(game_messages, player_messages);
    future::select(game_messages, player_messages).await;
}
