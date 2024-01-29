mod definitions;
mod errors;
mod game_state;
mod storage;

use parking_lot::Mutex;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashMap;
use std::{env, io::Error as IoError, net::SocketAddr, sync::Arc};

use definitions::WordDB;
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use jwt_simple::{prelude::*, token};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tungstenite::protocol::Message;

use crate::definitions::read_defs;
use crate::game_state::{Player, PlayerClaims};
use crate::storage::daily;
use game_state::GameManager;
use storage::accounts;
use truncate_core::messages::{
    DailyStateMessage, GameMessage, GameStateMessage, LobbyPlayerMessage, PlayerMessage,
};

#[derive(Clone)]
pub struct ServerState {
    games: Arc<Mutex<HashMap<String, Arc<Mutex<GameManager>>>>>,
    assignments: Arc<Mutex<HashMap<SocketAddr, String>>>,
    peers: Arc<Mutex<HashMap<SocketAddr, UnboundedSender<GameMessage>>>>,
    word_db: Arc<Mutex<WordDB>>,
    truncate_db: Option<PgPool>,
    jwt_key: HS256Key,
}

impl ServerState {
    fn words(&self) -> Arc<Mutex<WordDB>> {
        Arc::clone(&self.word_db)
    }

    fn game_code(&self) -> String {
        self.word_db.lock().get_free_code()
    }

    fn add_new_game(&self, game_id: &String, game_state: GameManager) -> Arc<Mutex<GameManager>> {
        let game = Arc::new(Mutex::new(game_state));
        let game_id = game_id.to_lowercase();

        self.games.lock().insert(game_id, Arc::clone(&game));

        game
    }

    fn attach_player_to_game(&self, addr: &SocketAddr, game_id: &String) {
        let mut assignments = self.assignments.lock();
        let game_id = game_id.to_lowercase();
        println!("Assigning {addr} to {game_id}");
        assignments.insert(*addr, game_id);
    }

    fn get_game_by_code(&self, game_id: &String) -> Option<Arc<Mutex<GameManager>>> {
        let game_id = game_id.to_lowercase();
        self.games.lock().get(&game_id).map(Arc::clone)
    }

    fn get_game_by_player(&self, addr: &SocketAddr) -> Option<Arc<Mutex<GameManager>>> {
        let assignments = self.assignments.lock();
        println!("Getting game for {addr}");
        let game_id = assignments.get(addr)?;
        println!("{addr} is assigned to game {game_id}");
        self.games.lock().get(game_id).map(Arc::clone)
    }

    fn track_peer(&self, addr: &SocketAddr, tx: UnboundedSender<GameMessage>) {
        let mut peers = self.peers.lock();
        peers.insert(*addr, tx);
    }

    fn get_player_tx(&self, addr: &SocketAddr) -> Option<UnboundedSender<GameMessage>> {
        self.peers.lock().get(addr).cloned()
    }

    fn send_to_player(&self, addr: &SocketAddr, msg: GameMessage) -> Result<(), ()> {
        // TODO: Use a better error crate and stop using `Result<(), ()>`

        let Some(peer_tx) = self.get_player_tx(addr) else {
            return Err(());
        };

        let Ok(_) = peer_tx.send(msg) else {
            return Err(());
        };

        Ok(())
    }
}

async fn handle_player_msg(
    msg: Message,
    player_addr: SocketAddr,
    server_state: ServerState,
) -> Result<(), tungstenite::Error> {
    let mut parsed_msg: PlayerMessage =
        serde_json::from_str(msg.to_text().unwrap()).expect("Valid JSON");
    if !matches!(parsed_msg, PlayerMessage::Ping) {
        println!("Received a message from {player_addr}: {}", parsed_msg);
    }

    use PlayerMessage::*;
    // If player is joining a room that they have a token for,
    // rejoin using that token instead.
    // TODO: Handle corner case when room code is reused and they're very unlucky
    if let JoinGame(joining_room_code, _, Some(token)) = &parsed_msg {
        if let Ok(JWTClaims {
            custom: PlayerClaims { room_code, .. },
            ..
        }) = server_state
            .jwt_key
            .verify_token::<PlayerClaims>(&token, None)
        {
            if joining_room_code.to_uppercase() == room_code.to_uppercase() {
                parsed_msg = PlayerMessage::RejoinGame(token.clone());
            }
        }
    }

    let player_err = |msg: String| {
        server_state
            .send_to_player(&player_addr, GameMessage::GenericError(msg))
            .unwrap();
        Ok(())
    };

    match parsed_msg {
        Ping => { /* TODO: Track pings and notify the game when players disconnect */ }
        NewGame(mut player_name) => {
            let new_game_id = server_state.game_code();
            let mut game = GameManager::new(new_game_id.clone());

            if &player_name == "___AUTO___" {
                player_name = "Player 1".into();
            }

            game.add_player(
                Player {
                    socket: Some(player_addr.clone()),
                },
                player_name.clone(),
            )
            .expect("Failed to add first player to game");

            let color = game.core_game.players[0].color;
            let board = game.core_game.board.clone();

            server_state.add_new_game(&new_game_id, game);
            server_state.attach_player_to_game(&player_addr, &new_game_id);

            let claims = Claims::with_custom_claims(
                PlayerClaims {
                    player_index: 0,
                    room_code: new_game_id.clone(),
                },
                Duration::from_days(7), // TODO: Determine game expiration time
            );
            let token = server_state
                .jwt_key
                .authenticate(claims)
                .expect("Claims should be serializable");

            server_state
                .send_to_player(
                    &player_addr,
                    GameMessage::JoinedLobby(
                        0,
                        new_game_id,
                        vec![LobbyPlayerMessage {
                            name: player_name,
                            color,
                            index: 0,
                        }],
                        board,
                        token,
                    ),
                )
                .unwrap();
        }
        JoinGame(room_code, mut player_name, _) => {
            let code = room_code.to_ascii_lowercase();
            if let Some(existing_game) = server_state.get_game_by_code(&code) {
                let mut game_manager = existing_game.lock();

                // TODO: This is the easiest place to check for lobby capacity right now,
                // but we'll need to reevaluate if we ever support >2 players, or spectators.
                if game_manager.players.len() >= 2 {
                    return player_err(format!(
                        "Room {} already has two players, cannot join",
                        code.to_ascii_uppercase()
                    ));
                }

                server_state.attach_player_to_game(&player_addr, &room_code);

                if &player_name == "___AUTO___" {
                    player_name = format!("Player {}", game_manager.players.len() + 1);
                }

                if let Ok(player_index) = game_manager.add_player(
                    Player {
                        socket: Some(player_addr.clone()),
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
                    let token = server_state
                        .jwt_key
                        .authenticate(claims)
                        .expect("Claims should be serializable");

                    server_state
                        .send_to_player(
                            &player_addr,
                            GameMessage::JoinedLobby(
                                player_index as u64,
                                code.clone(),
                                game_manager.player_list(),
                                game_manager.core_game.board.clone(),
                                token,
                            ),
                        )
                        .unwrap();

                    for player in &game_manager.players {
                        let Some(socket) = player.socket else {
                            continue;
                        };

                        server_state
                            .send_to_player(
                                &socket,
                                GameMessage::LobbyUpdate(
                                    player_index as u64,
                                    code.clone(),
                                    game_manager.player_list(),
                                    game_manager.core_game.board.clone(),
                                ),
                            )
                            .unwrap();
                    }
                } else {
                    // TODO: Render a better error here
                    return player_err(format!(
                        "Unable to join room {}",
                        code.to_ascii_uppercase()
                    ));
                }
            } else {
                return player_err(format!("Room {} does not exist", code.to_ascii_uppercase()));
            }
        }
        RejoinGame(token) => {
            let Ok(claims) = server_state
                .jwt_key
                .verify_token::<PlayerClaims>(&token, None)
            else {
                return player_err("Invalid Token".into());
            };
            let PlayerClaims {
                player_index,
                room_code,
            } = claims.custom;

            let code = room_code.to_ascii_lowercase();
            if let Some(existing_game) = server_state.get_game_by_code(&code) {
                let mut game_manager = existing_game.lock();
                println!("Trying to reconnect player {player_index} to room {code}");
                match game_manager.reconnect_player(player_addr.clone(), player_index) {
                    Ok(_) => {
                        server_state.attach_player_to_game(&player_addr, &code);

                        if game_manager.core_game.started_at.is_some() {
                            server_state
                                .send_to_player(
                                    &player_addr,
                                    GameMessage::StartedGame(
                                        game_manager.game_msg(player_index, None),
                                    ),
                                )
                                .unwrap();
                        } else {
                            server_state
                                .send_to_player(
                                    &player_addr,
                                    GameMessage::JoinedLobby(
                                        player_index as u64,
                                        code.clone(),
                                        game_manager.player_list(),
                                        game_manager.core_game.board.clone(),
                                        token,
                                    ),
                                )
                                .unwrap();
                        }
                    }
                    Err(_) => {
                        return player_err("Error rejoining existing game".into());
                    }
                }
            } else {
                return player_err(format!(
                    "Room {} no longer exists",
                    code.to_ascii_uppercase()
                ));
            }
        }
        EditBoard(board) => {
            if let Some(existing_game) = server_state.get_game_by_player(&player_addr) {
                let mut game_manager = existing_game.lock();
                game_manager.edit_board(board.clone());
                let player_list: Vec<_> = game_manager
                    .core_game
                    .players
                    .iter()
                    .map(|p| LobbyPlayerMessage {
                        name: p.name.clone(),
                        index: p.index,
                        color: p.color,
                    })
                    .collect();

                let Some(player_index) = game_manager.get_player_index(player_addr) else {
                    todo!("Handle player editing the board without having a turn index");
                };

                for player in &game_manager.players {
                    let Some(socket) = player.socket else {
                        continue;
                    };
                    server_state
                        .send_to_player(
                            &socket,
                            GameMessage::LobbyUpdate(
                                player_index as u64,
                                game_manager.game_id.clone(),
                                player_list.clone(),
                                board.clone(),
                            ),
                        )
                        .unwrap();
                }
            } else {
                todo!("Handle player not being enrolled in a game");
            }
        }
        EditName(name) => {
            if let Some(existing_game) = server_state.get_game_by_player(&player_addr) {
                let mut game_manager = existing_game.lock();
                if game_manager.rename_player(player_addr, name).is_ok() {
                    let player_list: Vec<_> = game_manager
                        .core_game
                        .players
                        .iter()
                        .map(|p| LobbyPlayerMessage {
                            name: p.name.clone(),
                            index: p.index,
                            color: p.color,
                        })
                        .collect();

                    let Some(player_index) = game_manager.get_player_index(player_addr) else {
                        unreachable!("Player just renamed themselves");
                    };

                    for player in &game_manager.players {
                        let Some(socket) = player.socket else {
                            continue;
                        };
                        server_state
                            .send_to_player(
                                &socket,
                                GameMessage::LobbyUpdate(
                                    player_index as u64,
                                    game_manager.game_id.clone(),
                                    player_list.clone(),
                                    game_manager.core_game.board.clone(),
                                ),
                            )
                            .unwrap();
                    }
                }
            } else {
                todo!("Handle player not being enrolled in a game");
            }
        }
        StartGame => {
            if let Some(existing_game) = server_state.get_game_by_player(&player_addr) {
                let mut game_manager = existing_game.lock();
                for (player, message) in game_manager.start() {
                    let Some(socket) = player.socket else {
                        continue;
                    };
                    server_state.send_to_player(&socket, message).unwrap();
                }
            } else {
                todo!("Handle player not being enrolled in a game");
            }
        }
        Resign => {
            if let Some(existing_game) = server_state.get_game_by_player(&player_addr) {
                let mut game_manager = existing_game.lock();
                for (player, message) in game_manager.resign(player_addr) {
                    let Some(socket) = player.socket else {
                        continue;
                    };
                    server_state.send_to_player(&socket, message).unwrap();
                }
            } else {
                todo!("Handle player not being enrolled in a game");
            }
        }
        Place(position, tile) => {
            if let Some(existing_game) = server_state.get_game_by_player(&player_addr) {
                let mut game_manager = existing_game.lock();
                for (player, message) in
                    game_manager.play(player_addr, position, tile, server_state.words())
                {
                    let Some(socket) = player.socket else {
                        continue;
                    };
                    server_state.send_to_player(&socket, message).unwrap();
                }
                // TODO: Error handling flow
            } else {
                todo!("Handle player not being enrolled in a game");
            }
        }
        Swap(from, to) => {
            if let Some(existing_game) = server_state.get_game_by_player(&player_addr) {
                let mut game_manager = existing_game.lock();
                for (player, message) in
                    game_manager.swap(player_addr, from, to, server_state.words())
                {
                    let Some(socket) = player.socket else {
                        continue;
                    };
                    server_state.send_to_player(&socket, message).unwrap();
                }
                // TODO: Error handling flow
            } else {
                todo!("Handle player not being enrolled in a game");
            }
        }
        Rematch => {
            if let Some(existing_game) = server_state.get_game_by_player(&player_addr) {
                let mut existing_game_manager = existing_game.lock();
                if existing_game_manager.core_game.winner.is_none() {
                    return player_err("Cannot rematch unfinished game".into());
                } else {
                    let new_game_id = server_state.game_code();
                    let mut new_game = GameManager::new(new_game_id.clone());

                    let mut next_board = existing_game_manager.core_game.board.clone();
                    next_board.reset();
                    new_game.core_game.board = next_board;

                    let mut next_sockets = existing_game_manager.players.clone();
                    next_sockets.rotate_left(1);
                    existing_game_manager.players = vec![];

                    let mut next_players = existing_game_manager.core_game.players.clone();
                    next_players.rotate_left(1);
                    for (i, player) in next_players.into_iter().enumerate() {
                        new_game
                            .add_player(
                                next_sockets
                                    .get(i)
                                    .expect("All players rejoining have a socket")
                                    .clone(),
                                player.name,
                            )
                            .expect("Failed to add player to game");
                    }

                    drop(existing_game_manager); // Done with the old game, don't accidentally use it.

                    let new_game = server_state.add_new_game(&new_game_id, new_game);
                    let new_game_manager = new_game.lock();

                    for (i, player) in new_game_manager.players.iter().enumerate() {
                        let Some(socket) = player.socket else {
                            continue;
                        };

                        server_state.attach_player_to_game(&socket, &new_game_id);

                        let claims = Claims::with_custom_claims(
                            PlayerClaims {
                                player_index: i,
                                room_code: new_game_id.clone(),
                            },
                            Duration::from_days(7), // TODO: Determine game expiration time
                        );
                        let token = server_state
                            .jwt_key
                            .authenticate(claims)
                            .expect("Claims should be serializable");

                        server_state
                            .send_to_player(
                                &socket,
                                GameMessage::JoinedLobby(
                                    i as u64,
                                    new_game_id.clone(),
                                    new_game_manager.player_list(),
                                    new_game_manager.core_game.board.clone(),
                                    token,
                                ),
                            )
                            .unwrap();
                    }
                }
            }
        }
        RequestDefinitions(words) => {
            let word_db = server_state.word_db.lock();
            let definitions: Vec<_> = words
                .iter()
                .map(|word| (word.clone(), word_db.get_word(&word.to_lowercase()).clone()))
                .collect();
            // Don't hold the lock while sending messages
            drop(word_db);

            server_state
                .send_to_player(&player_addr, GameMessage::SupplyDefinitions(definitions))
                .unwrap();
        }
        CreateAnonymousPlayer => match accounts::create_player(&server_state).await {
            Ok(new_player) => {
                let token = accounts::get_player_token(&server_state, new_player);

                server_state
                    .send_to_player(&player_addr, GameMessage::LoggedInAs(token))
                    .unwrap();
            }
            Err(_) => {
                todo!("Error handling for database actions");
            }
        },
        Login(token) => match accounts::login(&server_state, token.clone()).await {
            Ok(_player_id) => {
                server_state
                    .send_to_player(&player_addr, GameMessage::LoggedInAs(token))
                    .unwrap();
            }
            Err(e) => {
                eprintln!("Player did not exist, creating a new player instead: {e}\n{e:?}");
                match accounts::create_player(&server_state).await {
                    Ok(new_player) => {
                        let token = accounts::get_player_token(&server_state, new_player);

                        server_state
                            .send_to_player(&player_addr, GameMessage::LoggedInAs(token))
                            .unwrap();
                    }
                    Err(_) => {
                        todo!("Error handling for database actions");
                    }
                }
            }
        },
        LoadDailyPuzzle(token, day) => {
            let Ok(authed) = accounts::auth_player_token(&server_state, token) else {
                return player_err("Invalid Token".into());
            };

            if let Ok(Some(puzzle)) = daily::load_attempt(&server_state, authed, day as i32).await {
                server_state
                    .send_to_player(&player_addr, GameMessage::ResumeDailyPuzzle(puzzle))
                    .unwrap();
            } else {
                server_state
                    .send_to_player(
                        &player_addr,
                        GameMessage::ResumeDailyPuzzle(DailyStateMessage {
                            puzzle_day: day,
                            attempt: 0,
                            current_moves: vec![],
                        }),
                    )
                    .unwrap();
            }
        }
        PersistPuzzleMoves {
            player_token,
            day,
            human_player,
            moves,
            won,
        } => {
            let Ok(authed) = accounts::auth_player_token(&server_state, player_token) else {
                return player_err("Invalid Token".into());
            };

            if let Err(e) = daily::persist_moves(
                &server_state,
                authed,
                day as i32,
                human_player as i32,
                moves,
                won,
            )
            .await
            {
                eprintln!("Errored persisting daily game moves: {e}\n{e:?}");
            }
        }
        RequestStats(token) => {
            let Ok(authed) = accounts::auth_player_token(&server_state, token) else {
                return player_err("Invalid Token".into());
            };

            match daily::load_stats(&server_state, authed).await {
                Ok(stats) => {
                    server_state
                        .send_to_player(&player_addr, GameMessage::DailyStats(stats))
                        .unwrap();
                }
                Err(e) => {
                    eprintln!("Errored loading stats for player: {e}\n{e:?}");
                }
            }
        }
    }

    Ok(())
}

async fn handle_connection(server_state: ServerState, raw_stream: TcpStream, addr: SocketAddr) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    let (player_tx, player_rx) = mpsc::unbounded_channel();
    server_state.track_peer(&addr, player_tx);

    let (outgoing, incoming) = ws_stream.split();

    // TODO: try_for_each from TryStreamExt is quite nice,
    // look to bring that trait to the other stream places
    let handle_player_msg =
        incoming.try_for_each(|msg| handle_player_msg(msg, addr, server_state.clone()));

    let messages_to_player = {
        UnboundedReceiverStream::new(player_rx)
            .map(|msg| {
                if !matches!(msg, GameMessage::Ping) {
                    println!("Sending message: {msg}");
                }

                println!("Checking the game message");
                match &msg {
                    GameMessage::GameUpdate(GameStateMessage {
                        room_code,
                        players,
                        next_player_number,
                        ..
                    })
                    | GameMessage::StartedGame(GameStateMessage {
                        room_code,
                        players,
                        next_player_number,
                        ..
                    }) => {
                        println!("The message is a game update");
                        let next_player = &players[*next_player_number as usize];
                        if let Some(time_remaining) = next_player.time_remaining {
                            println!("Some player has {time_remaining} time left");
                            tokio::spawn(check_game_over(
                                room_code.clone(),
                                time_remaining.whole_milliseconds(),
                                server_state.clone(),
                            ));
                        }
                    }
                    _ => {}
                }

                Ok(Message::Text(serde_json::to_string(&msg).unwrap()))
            })
            .forward(outgoing)
    };

    pin_mut!(handle_player_msg, messages_to_player);
    future::select(handle_player_msg, messages_to_player).await;

    println!("{} disconnected", &addr);
    let mut peer_map = server_state.peers.lock();
    peer_map.remove(&addr);
}

async fn check_game_over(game_id: String, check_in_ms: i128, server_state: ServerState) {
    println!("[GAME CHECKER] Queueing an end game check in {check_in_ms}ms");
    if check_in_ms.is_negative() {
        return;
    }
    println!("[GAME CHECKER] Sleeping. . .");
    tokio::time::sleep(Duration::from_millis(check_in_ms as u64 + 10).into()).await;
    println!("[GAME CHECKER] Awaking after {check_in_ms}");

    let mut game_map = server_state.games.lock();
    let Some(existing_game) = game_map.get_mut(&game_id) else {
        println!("[GAME CHECKER] Game not found for {game_id}, exiting");
        return;
    };
    println!("[GAME CHECKER] Game found for {game_id}");
    let mut game_manager = existing_game.lock();
    game_manager.core_game.calculate_game_over(None);
    println!(
        "[GAME CHECKER] Checked game, winner is: {:?}",
        game_manager.core_game.winner
    );

    if let Some(winner) = game_manager.core_game.winner {
        for (player_index, player) in game_manager.players.iter().enumerate() {
            let Some(socket) = player.socket else {
                continue;
            };
            let mut end_game_msg = game_manager.game_msg(player_index, None);
            // Don't send any of the latest battles or hand changes
            end_game_msg.changes = vec![];
            server_state
                .send_to_player(&socket, GameMessage::GameEnd(end_game_msg, winner as u64))
                .unwrap();
        }
    }
}

async fn ping_peers(server_state: ServerState) {
    loop {
        // Ping all clients every five seconds
        tokio::time::sleep(Duration::from_secs(5).into()).await;
        let mut bad_peers = vec![];
        let mut peer_map = server_state.peers.lock();
        let all_peers = peer_map.iter();
        for (peer_key, peer_tx) in all_peers {
            match peer_tx.send(GameMessage::Ping) {
                Ok(()) => {}
                Err(_) => {
                    println!("Failed to ping {}", peer_key);
                    bad_peers.push(peer_key.clone());
                }
            }
        }
        for bad_peer in bad_peers {
            peer_map.remove(&bad_peer);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    println!("Starting up...");

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:8080".to_string());

    // Load from env file if one exists (local dev).
    _ = dotenvy::dotenv();

    let jwt_key = if let Some(s) = env::var("SIGNING_SECRET").ok() {
        println!("Loading the signing secret for JWTs");
        HS256Key::from_bytes(&hex::decode(s).expect("Signing secret should be valid hex"))
    } else {
        let k = HS256Key::generate();
        println!("Running without a dedicated secret — generating a new one:");
        println!("{}", hex::encode(k.to_bytes()));
        k
    };

    let mut server_state = ServerState {
        games: Arc::new(Mutex::new(HashMap::new())),
        assignments: Arc::new(Mutex::new(HashMap::new())),
        peers: Arc::new(Mutex::new(HashMap::new())),
        word_db: Arc::new(Mutex::new(read_defs())),
        truncate_db: None,
        jwt_key,
    };

    if let Ok(db_url) = env::var("DATABASE_URL") {
        println!("Initializing database shtuff");

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .expect("Database should be alive");

        println!("Running database migrations");
        sqlx::migrate!("./migrations")
            .set_ignore_missing(true)
            .run(&pool)
            .await
            .expect("Database migrations should succeed");

        server_state.truncate_db = Some(pool);

        println!("Database is ready.");
    } else {
        println!("Running the Truncate server without a database connection.");
    }

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    tokio::spawn(ping_peers(server_state.clone()));

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
        let deadlocks = parking_lot::deadlock::check_deadlock();
        if deadlocks.is_empty() {
            continue;
        }

        println!("{} deadlocks detected", deadlocks.len());
        for (i, threads) in deadlocks.iter().enumerate() {
            println!("Deadlock #{}", i);
            for t in threads {
                println!("Thread Id {:#?}", t.thread_id());
                println!("{:#?}", t.backtrace());
            }
        }
    });

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(server_state.clone(), stream, addr));
    }

    Ok(())
}
