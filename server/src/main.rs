mod game_state;
mod room_codes;

use std::{env, io::Error as IoError, net::SocketAddr, sync::Arc};

use dashmap::DashMap;
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tungstenite::protocol::Message;

use crate::game_state::Player;
use core::messages::{GameMessage, PlayerMessage};
use game_state::GameState;
use room_codes::RoomCodes;

type PeerMap = Arc<DashMap<SocketAddr, UnboundedSender<GameMessage>>>;
type GameMap = Arc<DashMap<String, GameState>>;
type ActiveGameMap = Arc<DashMap<SocketAddr, String>>;
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
        println!("Message: {}", parsed_msg);

        let get_current_game = |addr| {
            active_map
                .get(&addr)
                .map(|game_id| game_map.get_mut(&*game_id))
                .flatten()
        };

        use PlayerMessage::*;
        match parsed_msg {
            NewGame(name) => {
                let new_game_id = code_provider.get_free_code();
                let mut game = GameState::new(new_game_id.clone());
                game.add_player(Player {
                    name: name.clone(),
                    socket: Some(addr.clone()),
                })
                .expect("Failed to add first player to game");

                let board = game.game.board.clone();
                game_map.insert(new_game_id.clone(), game);
                active_map.insert(addr, new_game_id.clone());

                player_tx
                    .send(GameMessage::JoinedLobby(new_game_id, vec![name], board))
                    .unwrap();
            }
            JoinGame(room_code, player_name) => {
                let code = room_code.to_ascii_lowercase();
                if let Some(mut existing_game) = game_map.get_mut(&code) {
                    active_map.insert(addr, code.clone());

                    if existing_game
                        .add_player(Player {
                            name: player_name,
                            socket: Some(addr.clone()),
                        })
                        .is_ok()
                    {
                        let player_list: Vec<_> = existing_game.players.iter().map(|p| p.name.clone()).collect();
                        player_tx.send(GameMessage::JoinedLobby(code.clone(), player_list.clone(), existing_game.game.board.clone())).unwrap();

                        for player in &existing_game.players {
                            let Some(socket) = player.socket else { todo!("Handle disconnected player") };
                            let Some(peer) = peer_map.get(&socket) else { todo!("Handle disconnected player") };
    
                            peer.send(GameMessage::LobbyUpdate(code.clone(), player_list.clone(), existing_game.game.board.clone())).unwrap();
                        }
                    } else {
                        todo!("Handle error when adding player to a room");
                    }
                } else {
                    todo!("Handle error when a room doesn't exist");
                }
            }
            EditBoard(board) => {
                if let Some(mut game_state) = get_current_game(addr) {
                    game_state.edit_board(board.clone());
                    let player_list: Vec<_> = game_state.players.iter().map(|p| p.name.clone()).collect();

                    for player in &game_state.players {
                        let Some(socket) = player.socket else { todo!("Handle disconnected player") };
                        let Some(peer) = peer_map.get(&socket) else { todo!("Handle disconnected player") };

                        peer.send(GameMessage::LobbyUpdate(game_state.game_id.clone(), player_list.clone(), board.clone())).unwrap();
                    }
                } else {
                    todo!("Handle player not being enrolled in a game");
                }
            }
            StartGame => {
                if let Some(mut game_state) = get_current_game(addr) {
                    for (player, message) in game_state.start() {
                        let Some(socket) = player.socket else { todo!("Handle disconnected player") };
                        let Some(peer) = peer_map.get(&socket) else { todo!("Handle disconnected player") };

                        peer.send(message).unwrap();
                    }
                } else {
                    todo!("Handle player not being enrolled in a game");
                }
            }
            Place(position, tile) => {
                if let Some(mut game_state) = get_current_game(addr) {
                    for (player, message) in game_state.play(addr, position, tile) {
                        let Some(socket) = player.socket else { todo!("Handle disconnected player") };
                        let Some(peer) = peer_map.get(&socket) else { todo!("Handle disconnected player") };

                        peer.send(message).unwrap();
                    }
                    // TODO: Error handling flow
                } else {
                    todo!("Handle player not being enrolled in a game");
                }
            }
            Swap(from, to) => {
                if let Some(mut game_state) = get_current_game(addr) {
                    for (player, message) in game_state.swap(addr, from, to) {
                        let Some(socket) = player.socket else { todo!("Handle disconnected player") };
                        let Some(peer) = peer_map.get(&socket) else { todo!("Handle disconnected player") };

                        peer.send(message).unwrap();
                    }
                    // TODO: Error handling flow
                } else {
                    todo!("Handle player not being enrolled in a game");
                }
            }
        }

        future::ok(())
    });

    let messages_to_player = {
        UnboundedReceiverStream::new(player_rx)
            .map(|msg| {
                println!("Sending message: {msg}");
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
