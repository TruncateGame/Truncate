mod definitions;
mod game_state;

use std::{env, io::Error as IoError, net::SocketAddr, sync::Arc};

use dashmap::DashMap;
use definitions::WordDB;
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use jwt_simple::prelude::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tungstenite::protocol::Message;

use crate::definitions::read_defs;
use crate::game_state::{Player, PlayerClaims};
use game_state::GameState;
use truncate_core::messages::{GameMessage, LobbyPlayerMessage, PlayerMessage};

type PeerMap = Arc<DashMap<SocketAddr, UnboundedSender<GameMessage>>>;
type GameMap = Arc<DashMap<String, GameState>>;
type ActiveGameMap = Arc<DashMap<SocketAddr, String>>;
type Maps = (PeerMap, GameMap, ActiveGameMap, Arc<Mutex<WordDB>>);

async fn handle_player_msg(
    msg: Message,
    addr: SocketAddr,
    maps: Maps,
    jwt_key: HS256Key,
) -> Result<(), tungstenite::Error> {
    let (peer_map, game_map, active_map, word_map) = maps;
    let player_tx = peer_map.get(&addr).expect("TODO: Refactor");

    let mut parsed_msg: PlayerMessage =
        serde_json::from_str(msg.to_text().unwrap()).expect("Valid JSON");
    if !matches!(parsed_msg, PlayerMessage::Ping) {
        println!("Received a message from {addr}: {}", parsed_msg);
    }

    let get_current_game = |addr| {
        active_map
            .get(&addr)
            .map(|game_id| game_map.get_mut(&*game_id))
            .flatten()
    };

    use PlayerMessage::*;
    // If player is joining a room that they have a token for,
    // rejoin using that token instead.
    // TODO: Handle corner case when room code is reused and they're very unlucky
    if let JoinGame(joining_room_code, _, Some(token)) = &parsed_msg {
        let Ok(claims) = jwt_key.verify_token::<PlayerClaims>(&token, None) else {
            player_tx
                .send(GameMessage::GenericError("Invalid Token".into()))
                .unwrap();
            return Ok(());
        };
        let PlayerClaims { room_code, .. } = claims.custom;

        if joining_room_code.to_uppercase() == room_code.to_uppercase() {
            parsed_msg = PlayerMessage::RejoinGame(token.clone());
        }
    }

    match parsed_msg {
        Ping => { /* TODO: Track pings and notify the game when players disconnect */ }
        NewGame(mut player_name) => {
            let new_game_id = word_map.lock().await.get_free_code();
            let mut game = GameState::new(new_game_id.clone());

            if &player_name == "___AUTO___" {
                player_name = "Player 1".into();
            }

            game.add_player(
                Player {
                    socket: Some(addr.clone()),
                },
                player_name.clone(),
            )
            .expect("Failed to add first player to game");

            let color = game.game.players[0].color;
            let board = game.game.board.clone();
            game_map.insert(new_game_id.clone(), game);
            active_map.insert(addr, new_game_id.clone());

            let claims = Claims::with_custom_claims(
                PlayerClaims {
                    player_index: 0,
                    room_code: new_game_id.clone(),
                },
                Duration::from_days(7), // TODO: Determine game expiration time
            );
            let token = jwt_key
                .authenticate(claims)
                .expect("Claims should be serializable");

            player_tx
                .send(GameMessage::JoinedLobby(
                    0,
                    new_game_id,
                    vec![LobbyPlayerMessage {
                        name: player_name,
                        color,
                        index: 0,
                    }],
                    board,
                    token,
                ))
                .unwrap();
        }
        JoinGame(room_code, mut player_name, _) => {
            let code = room_code.to_ascii_lowercase();
            if let Some(mut existing_game) = game_map.get_mut(&code) {
                active_map.insert(addr, code.clone());

                if &player_name == "___AUTO___" {
                    player_name = format!("Player {}", existing_game.players.len() + 1);
                }

                if let Ok(player_index) = existing_game.add_player(
                    Player {
                        socket: Some(addr.clone()),
                    },
                    player_name.clone(),
                ) {
                    let claims = Claims::with_custom_claims(
                        PlayerClaims {
                            player_index,
                            room_code: room_code.clone(),
                        },
                        Duration::from_days(7), // TODO: Determine game expiration time
                    );
                    let token = jwt_key
                        .authenticate(claims)
                        .expect("Claims should be serializable");

                    player_tx
                        .send(GameMessage::JoinedLobby(
                            player_index as u64,
                            code.clone(),
                            existing_game.player_list(),
                            existing_game.game.board.clone(),
                            token,
                        ))
                        .unwrap();

                    for player in &existing_game.players {
                        let Some(socket) = player.socket else { todo!("Handle disconnected player") };
                        let Some(peer) = peer_map.get(&socket) else { todo!("Handle disconnected player") };

                        peer.send(GameMessage::LobbyUpdate(
                            player_index as u64,
                            code.clone(),
                            existing_game.player_list(),
                            existing_game.game.board.clone(),
                        ))
                        .unwrap();
                    }
                } else {
                    // TODO: Render a better error here
                    player_tx
                        .send(GameMessage::GenericError(format!(
                            "Unable to join room {}",
                            code.to_ascii_uppercase()
                        )))
                        .unwrap();
                }
            } else {
                player_tx
                    .send(GameMessage::GenericError(format!(
                        "Room {} does not exist",
                        code.to_ascii_uppercase()
                    )))
                    .unwrap();
            }
        }
        RejoinGame(token) => {
            let Ok(claims) = jwt_key.verify_token::<PlayerClaims>(&token, None) else {
                player_tx
                    .send(GameMessage::GenericError("Invalid Token".into()))
                    .unwrap();
                return Ok(());
            };
            let PlayerClaims {
                player_index,
                room_code,
            } = claims.custom;

            let code = room_code.to_ascii_lowercase();
            if let Some(mut existing_game) = game_map.get_mut(&code) {
                println!("Trying to reconnect player {player_index} to room {code}");
                match existing_game.reconnect_player(addr.clone(), player_index) {
                    Ok(_) => {
                        active_map.insert(addr, code.clone());

                        if existing_game.game.started_at.is_some() {
                            player_tx
                                .send(GameMessage::StartedGame(
                                    existing_game.game_msg(player_index, None),
                                ))
                                .unwrap();
                        } else {
                            player_tx
                                .send(GameMessage::JoinedLobby(
                                    player_index as u64,
                                    code.clone(),
                                    existing_game.player_list(),
                                    existing_game.game.board.clone(),
                                    token,
                                ))
                                .unwrap();
                        }
                    }
                    Err(_) => {
                        player_tx
                            .send(GameMessage::GenericError(
                                "Error rejoining existing game".into(),
                            ))
                            .unwrap();
                    }
                }
            } else {
                player_tx
                    .send(GameMessage::GenericError(format!(
                        "Room {} no longer exists",
                        code.to_ascii_uppercase()
                    )))
                    .unwrap();
            }
        }
        EditBoard(board) => {
            if let Some(mut game_state) = get_current_game(addr) {
                game_state.edit_board(board.clone());
                let player_list: Vec<_> = game_state
                    .game
                    .players
                    .iter()
                    .map(|p| LobbyPlayerMessage {
                        name: p.name.clone(),
                        index: p.index,
                        color: p.color,
                    })
                    .collect();

                let Some(player_index) = game_state.get_player_index(addr) else {
                    todo!("Handle player editing the board without having a turn index");
                };

                for player in &game_state.players {
                    let Some(socket) = player.socket else { todo!("Handle disconnected player") };
                    let Some(peer) = peer_map.get(&socket) else { todo!("Handle disconnected player") };

                    peer.send(GameMessage::LobbyUpdate(
                        player_index as u64,
                        game_state.game_id.clone(),
                        player_list.clone(),
                        board.clone(),
                    ))
                    .unwrap();
                }
            } else {
                todo!("Handle player not being enrolled in a game");
            }
        }
        EditName(name) => {
            if let Some(mut game_state) = get_current_game(addr) {
                if game_state.rename_player(addr, name).is_ok() {
                    let player_list: Vec<_> = game_state
                        .game
                        .players
                        .iter()
                        .map(|p| LobbyPlayerMessage {
                            name: p.name.clone(),
                            index: p.index,
                            color: p.color,
                        })
                        .collect();

                    let Some(player_index) = game_state.get_player_index(addr) else {
                        unreachable!("Player just renamed themselves");
                    };

                    for player in &game_state.players {
                        let Some(socket) = player.socket else { todo!("Handle disconnected player") };
                        let Some(peer) = peer_map.get(&socket) else { todo!("Handle disconnected player") };

                        peer.send(GameMessage::LobbyUpdate(
                            player_index as u64,
                            game_state.game_id.clone(),
                            player_list.clone(),
                            game_state.game.board.clone(),
                        ))
                        .unwrap();
                    }
                }
            } else {
                todo!("Handle player not being enrolled in a game");
            }
        }
        StartGame => {
            if let Some(mut game_state) = get_current_game(addr) {
                for (player, message) in game_state.start().await {
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
                for (player, message) in game_state.play(addr, position, tile, word_map).await {
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
                for (player, message) in game_state.swap(addr, from, to, word_map).await {
                    let Some(socket) = player.socket else { todo!("Handle disconnected player") };
                    let Some(peer) = peer_map.get(&socket) else { todo!("Handle disconnected player") };

                    peer.send(message).unwrap();
                }
                // TODO: Error handling flow
            } else {
                todo!("Handle player not being enrolled in a game");
            }
        }
        Rematch => {
            if let Some(mut game_state) = get_current_game(addr) {
                if game_state.game.winner.is_none() {
                    player_tx
                        .send(GameMessage::GenericError(
                            "Cannot rematch unfinished game".into(),
                        ))
                        .unwrap();
                } else {
                    let new_game_id = word_map.lock().await.get_free_code();
                    let mut game = GameState::new(new_game_id.clone());

                    let mut next_board = game_state.game.board.clone();
                    next_board.reset();
                    game.game.board = next_board;

                    let mut next_sockets = game_state.players.clone();
                    next_sockets.rotate_left(1);
                    game_state.players = vec![];

                    let mut next_players = game_state.game.players.clone();
                    next_players.rotate_left(1);
                    for (i, player) in next_players.into_iter().enumerate() {
                        game.add_player(
                            next_sockets
                                .get(i)
                                .expect("All players rejoining have a socket")
                                .clone(),
                            player.name,
                        )
                        .expect("Failed to add player to game");
                    }

                    game_map.insert(new_game_id.clone(), game);

                    let Some(existing_game) = game_map.get(&new_game_id) else {
                        panic!("what");
                    };

                    for (i, player) in existing_game.players.iter().enumerate() {
                        let Some(socket) = player.socket else { todo!("Handle disconnected player") };
                        let Some(peer) = peer_map.get(&socket) else { todo!("Handle disconnected player") };

                        active_map.insert(socket, new_game_id.clone());

                        let claims = Claims::with_custom_claims(
                            PlayerClaims {
                                player_index: i,
                                room_code: new_game_id.clone(),
                            },
                            Duration::from_days(7), // TODO: Determine game expiration time
                        );
                        let token = jwt_key
                            .authenticate(claims)
                            .expect("Claims should be serializable");

                        peer.send(GameMessage::JoinedLobby(
                            i as u64,
                            new_game_id.clone(),
                            existing_game.player_list(),
                            existing_game.game.board.clone(),
                            token,
                        ))
                        .unwrap();
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_connection(maps: Maps, raw_stream: TcpStream, addr: SocketAddr, jwt_key: HS256Key) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (player_tx, player_rx) = mpsc::unbounded_channel();
    maps.0.insert(addr, player_tx.clone());

    let (outgoing, incoming) = ws_stream.split();

    // TODO: try_for_each from TryStreamExt is quite nice,
    // look to bring that trait to the other stream places
    let handle_player_msg =
        incoming.try_for_each(|msg| handle_player_msg(msg, addr, maps.clone(), jwt_key.clone()));

    let messages_to_player = {
        UnboundedReceiverStream::new(player_rx)
            .map(|msg| {
                if !matches!(msg, GameMessage::Ping) {
                    println!("Sending message: {msg}");
                }
                Ok(Message::Text(serde_json::to_string(&msg).unwrap()))
            })
            .forward(outgoing)
    };

    pin_mut!(handle_player_msg, messages_to_player);
    future::select(handle_player_msg, messages_to_player).await;

    println!("{} disconnected", &addr);
    maps.0.remove(&addr);
}

async fn ping_peers(peers: PeerMap) {
    loop {
        // Ping all clients every five seconds
        tokio::time::sleep(Duration::from_secs(5).into()).await;
        let mut bad_peers = vec![];
        for peer in peers.iter() {
            match peer.send(GameMessage::Ping) {
                Ok(()) => {}
                Err(_) => {
                    println!("Failed to ping {}", peer.key());
                    bad_peers.push(peer.key().clone());
                }
            }
        }
        for bad_peer in bad_peers {
            peers.remove(&bad_peer);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    println!("Starting up...");

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:8080".to_string());

    let maps = (
        PeerMap::new(DashMap::new()),
        GameMap::new(DashMap::new()),
        ActiveGameMap::new(DashMap::new()),
        Arc::new(Mutex::new(read_defs())),
    );

    let jwt_key = HS256Key::generate();

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    tokio::spawn(ping_peers(maps.0.clone()));

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(
            maps.clone(),
            stream,
            addr,
            jwt_key.clone(),
        ));
    }

    Ok(())
}
