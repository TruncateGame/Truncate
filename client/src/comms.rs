use futures_util::{future, pin_mut, StreamExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use core::messages::{GameMessage, PlayerMessage};

pub async fn connect(
    connect_addr: String,
    tx_game: UnboundedSender<GameMessage>,
    rx_player: UnboundedReceiver<PlayerMessage>,
) {
    let (ws_stream, _) = connect_async(connect_addr)
        .await
        .expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (outgoing, incoming) = ws_stream.split();

    let game_messages = {
        incoming.for_each(|msg| async {
            let parsed_msg: GameMessage =
                serde_json::from_str(msg.unwrap().to_text().expect("Was not valid UTF-8"))
                    .expect("Was not valid JSON");
            println!("Received {parsed_msg:#?}");
            tx_game.send(parsed_msg).unwrap();
        })
    };

    let player_messages = {
        UnboundedReceiverStream::new(rx_player)
            .map(|msg| {
                println!("Sending {msg:#?}");
                Ok(Message::Text(serde_json::to_string(&msg).unwrap()))
            })
            .forward(outgoing)
    };

    pin_mut!(game_messages, player_messages);
    future::select(game_messages, player_messages).await;
}
