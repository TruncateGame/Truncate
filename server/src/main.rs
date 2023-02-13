mod game_state;
mod room_codes;

use std::{env, io::Error as IoError, net::SocketAddr, sync::Arc};

use dashmap::DashMap;
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tungstenite::protocol::Message;

use crate::game_state::Player;
use core::messages::{GameMessage, PlayerMessage};
use room_codes::RoomCodes;

type PeerMap = Arc<DashMap<SocketAddr, UnboundedSender<GameMessage>>>;
type GameMap = Arc<DashMap<&'static str, UnboundedSender<PlayerMessage>>>;
type ActiveGameMap = Arc<DashMap<SocketAddr, &'static str>>;
type Maps = (PeerMap, GameMap, ActiveGameMap);

async fn handle_connection(
    maps: Maps,
    raw_stream: TcpStream,
    addr: SocketAddr,
    code_provider: RoomCodes,
) {
    let (peer_map, game_map, active_map) = maps;
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (player_tx, player_rx) = mpsc::unbounded_channel();
    peer_map.insert(addr, player_tx.clone());

    let (outgoing, incoming) = ws_stream.split();

    // TODO: try_for_each from TryStreamExt is quite nice,
    // look to bring that trait to the other stream places
    let handle_player_msg = incoming.try_for_each(|msg| {
        println!("Received a message from {addr}");

        let parsed_msg: PlayerMessage =
            serde_json::from_str(msg.to_text().unwrap()).expect("Valid JSON");
        println!("Parsed message as: {:?}", parsed_msg);
        use PlayerMessage::*;
        match parsed_msg {
            NewGame => {
                let new_game_id = code_provider.get_free_code();
                let (game_tx, game_rx) = mpsc::unbounded_channel();

                let peers = peer_map.clone();
                std::thread::spawn(move || {
                    game_state::run_game(
                        new_game_id.to_string(),
                        game_rx,
                        peers,
                        Player {
                            name: "TODO".into(),
                            socket: Some(addr.clone()),
                        },
                    )
                });
                game_map.insert(new_game_id, game_tx);
                active_map.insert(addr, new_game_id);
                player_tx
                    .send(GameMessage::JoinedGame(new_game_id.into()))
                    .unwrap();
            }
            Place(_, _) | StartGame => {
                let existing_game = active_map
                    .get(&addr)
                    .map(|game_id| game_map.get(&*game_id))
                    .flatten();
                if let Some(tx_game) = existing_game {
                    tx_game.send(parsed_msg).unwrap();
                } else {
                    // TODO: Send error message to user as they are not enrolled in any game
                }
            }
        }

        future::ok(())
    });

    let messages_to_player = {
        UnboundedReceiverStream::new(player_rx)
            .map(|msg| {
                println!("Sending {msg:#?}");
                Ok(Message::Text(serde_json::to_string(&msg).unwrap()))
            })
            .forward(outgoing)
    };

    pin_mut!(handle_player_msg, messages_to_player);
    future::select(handle_player_msg, messages_to_player).await;

    println!("{} disconnected", &addr);
    peer_map.remove(&addr);
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:8080".to_string());

    let maps = (
        PeerMap::new(DashMap::new()),
        GameMap::new(DashMap::new()),
        ActiveGameMap::new(DashMap::new()),
    );

    let code_provider = RoomCodes::new(maps.1.clone());

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(
            maps.clone(),
            stream,
            addr,
            // TODO: This is a very expensive clone,
            // refactor RoomCodes to be an Arc/Mutex shindig
            code_provider.clone(),
        ));
    }

    Ok(())
}
