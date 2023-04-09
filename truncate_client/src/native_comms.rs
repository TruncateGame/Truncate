use eframe::egui::Context;
use futures::channel::{mpsc, oneshot};
use futures::SinkExt;
use futures_util::{future, pin_mut, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use truncate_core::messages::{GameMessage, PlayerMessage};

pub async fn connect(
    connect_addr: String,
    tx_game: mpsc::Sender<GameMessage>,
    rx_player: mpsc::Receiver<PlayerMessage>,
    rx_context: oneshot::Receiver<Context>,
) {
    let mut context: Option<Context> = None;

    println!("Connecting to {connect_addr}");

    let (ws_stream, _) = connect_async(connect_addr)
        .await
        .expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    if let Ok(ctx) = rx_context.await {
        context = Some(ctx);
    }

    let (outgoing, incoming) = ws_stream.split();

    let game_messages = {
        incoming.for_each(|msg| async {
            let parsed_msg: GameMessage =
                serde_json::from_str(msg.unwrap().to_text().expect("Was not valid UTF-8"))
                    .expect("Was not valid JSON");
            println!("Received {parsed_msg}");
            tx_game
                .clone()
                .send(parsed_msg)
                .await
                .expect("Message should have been able to go into the unbounded channel");
            if let Some(context) = context.as_ref() {
                context.request_repaint();
            }
        })
    };

    let player_messages = {
        rx_player
            .map(|msg| {
                println!("Sending {msg}");
                Ok(Message::Text(serde_json::to_string(&msg).unwrap()))
            })
            .forward(outgoing)
    };

    pin_mut!(game_messages, player_messages);
    future::select(game_messages, player_messages).await;
}
