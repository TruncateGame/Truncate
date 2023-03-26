use eframe::web_sys;
use futures::channel::mpsc::{Receiver, Sender};
use futures::SinkExt;
use futures_util::{future, pin_mut, StreamExt};
use truncate_core::messages::{GameMessage, PlayerMessage};
use ws_stream_wasm::{WsMessage, WsMeta};

pub async fn connect(
    connect_addr: String,
    tx_game: Sender<GameMessage>,
    rx_player: Receiver<PlayerMessage>,
) {
    use web_sys::console;

    console::log_1(&format!("Connecting to {connect_addr}").into());

    let (_ws, wsio) = WsMeta::connect(connect_addr, None)
        .await
        .expect("assume the connection succeeds");

    console::log_1(&"Connected".into());

    let (outgoing, incoming) = wsio.split();

    let game_messages = {
        incoming.for_each(|msg| async {
            console::log_1(&"Parsing a message".into());

            let parsed_msg: GameMessage = match msg {
                WsMessage::Text(msg) => serde_json::from_str(&msg).expect("Was not valid JSON"),
                WsMessage::Binary(msg) => serde_json::from_slice(&msg).expect("Was not valid JSON"),
            };

            console::log_1(&format!("Received {parsed_msg}").into());
            tx_game
                .clone()
                .send(parsed_msg)
                .await
                .expect("Message should have been able to go into the channel");
        })
    };

    let player_messages = {
        rx_player
            .map(|msg| {
                console::log_1(&format!("Sending {msg}").into());
                Ok(WsMessage::Text(
                    serde_json::to_string(&msg.clone()).unwrap(),
                ))
            })
            .forward(outgoing)
    };

    pin_mut!(game_messages, player_messages);
    future::select(game_messages, player_messages).await;
}
