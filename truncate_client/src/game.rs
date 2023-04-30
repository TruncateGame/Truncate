use eframe::egui;
use truncate_core::{messages::RoomCode, messages::Token};

use crate::{active_game::ActiveGame, editor_state::EditorState};

use super::GameClient;
use truncate_core::{
    messages::{GameMessage, GameStateMessage, PlayerMessage},
    reporting::Change,
};

pub enum GameStatus {
    None(RoomCode, Option<Token>),
    PendingJoin(RoomCode),
    PendingCreate,
    PendingStart(EditorState),
    Active(ActiveGame),
    Concluded(ActiveGame, u64),
}

pub fn render(client: &mut GameClient, ui: &mut egui::Ui) {
    let GameClient {
        name,
        theme,
        game_status,
        rx_game,
        tx_player,
        frame_history: _,
        map_texture,
        launched_room,
    } = client;

    let mut send = |msg| {
        tx_player.try_send(msg).unwrap();
    };

    let mut recv = || match rx_game.try_next() {
        Ok(Some(msg)) => Ok(msg),
        _ => Err(()),
    };

    let mut new_game_status = None;

    if let Some(launched_room) = launched_room.take() {
        if launched_room == "__REJOIN__" {
            match game_status {
                GameStatus::None(_, Some(token)) => {
                    send(PlayerMessage::RejoinGame(token.to_string()));
                    new_game_status = Some(GameStatus::PendingJoin("...".into()));
                }
                _ => {
                    panic!("Tried to rejoin a game but no token found in localStorage");
                }
            }
        } else if launched_room.is_empty() {
            // No room code means we start a new game.
            send(PlayerMessage::NewGame(name.clone()));
            new_game_status = Some(GameStatus::PendingCreate);
        } else {
            send(PlayerMessage::JoinGame(
                launched_room.clone(),
                "Player X".into(),
            ));
            new_game_status = Some(GameStatus::PendingJoin(launched_room));
        }
    }

    ui.horizontal(|ui| {
        if option_env!("TR_PROD").is_none() {
            if let (Some(commit_msg), Some(commit_hash)) =
                (option_env!("TR_MSG"), option_env!("TR_COMMIT"))
            {
                ui.hyperlink_to(
                    format!("Running \"{commit_msg}\""),
                    format!("https://github.com/TruncateGame/Truncate/commit/{commit_hash}"),
                );
            } else {
                ui.label(format!("No tagged commit."));
            }
        }

        if matches!(game_status, GameStatus::None(_, _)) {
            ui.horizontal(|ui| {
                ui.label("Name: ");
                ui.text_edit_singleline(name);
            });
        } else {
            ui.label(format!("Playing as {name}"));
        }
    });

    ui.separator();

    match game_status {
        GameStatus::None(room_code, token) => {
            if ui.button("New Game").clicked() {
                // TODO: Send player name in NewGame message
                send(PlayerMessage::NewGame(name.clone()));
                new_game_status = Some(GameStatus::PendingCreate);
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(room_code);
                if ui.button("Join Game").clicked() {
                    send(PlayerMessage::JoinGame(room_code.clone(), name.clone()));
                    new_game_status = Some(GameStatus::PendingJoin(room_code.clone()));
                }
            });
            if let Some(existing_token) = token {
                ui.label("Existing game found, would you like to rejoin?");
                if ui.button("Rejoin").clicked() {
                    send(PlayerMessage::RejoinGame(existing_token.to_string()));
                    new_game_status = Some(GameStatus::PendingJoin("...".into()));
                }
            }
        }
        GameStatus::PendingJoin(room_code) => {
            ui.label(format!("Waiting to join room {room_code}"));
        }
        GameStatus::PendingCreate => {
            ui.label("Waiting for a new game to be created . . .");
        }
        GameStatus::PendingStart(editor_state) => {
            if let Some(msg) = editor_state.render(ui, theme) {
                send(msg);
            }
        }
        GameStatus::Active(game) => {
            if let Some(msg) = game.render(ui, theme, None) {
                send(msg);
            }
        }
        GameStatus::Concluded(game, winner) => {
            game.render(ui, theme, Some(*winner as usize));
            // render_board(game, ui);
            // TODO: Reset state and play again
        }
    }
    if let Some(new_game_status) = new_game_status {
        *game_status = new_game_status;
    }

    while let Ok(msg) = recv() {
        match msg {
            GameMessage::Ping => {}
            GameMessage::JoinedLobby(id, players, board, token) => {
                #[cfg(target_arch = "wasm32")]
                {
                    let local_storage =
                        web_sys::window().unwrap().local_storage().unwrap().unwrap();
                    local_storage
                        .set_item("truncate_active_token", &token)
                        .unwrap();
                }

                *game_status = GameStatus::PendingStart(EditorState::new(
                    id.to_uppercase(),
                    players,
                    board,
                    map_texture.clone(),
                ))
            }
            GameMessage::LobbyUpdate(id, players, board) => {
                match game_status {
                    GameStatus::PendingStart(editor_state) => {
                        // assert_eq!(game.room_code, room_code);
                        // assert_eq!(game.player_number, player_number);
                        editor_state.players = players;
                        editor_state.update_board(board);
                    }
                    _ => panic!("Game update hit an unknown state"),
                }
            }
            GameMessage::StartedGame(GameStateMessage {
                room_code,
                players,
                player_number,
                next_player_number,
                board,
                hand,
                changes: _,
            }) => {
                *game_status = GameStatus::Active(ActiveGame::new(
                    room_code.to_uppercase(),
                    players,
                    player_number,
                    next_player_number,
                    board,
                    hand,
                    map_texture.clone(),
                    theme.clone(),
                ));
                println!("Starting a game")
            }
            GameMessage::GameUpdate(GameStateMessage {
                room_code: _,
                players,
                player_number: _,
                next_player_number,
                board,
                hand: _,
                changes,
            }) => {
                match game_status {
                    GameStatus::Active(game) => {
                        // assert_eq!(game.room_code, room_code);
                        // assert_eq!(game.player_number, player_number);
                        game.players = players;
                        game.board = board;
                        game.ctx.next_player_number = next_player_number;

                        #[cfg(target_arch = "wasm32")]
                        if game.ctx.next_player_number == game.ctx.player_number {
                            use eframe::wasm_bindgen::JsCast;

                            let window = web_sys::window().expect("window should exist in browser");
                            let document =
                                window.document().expect("documnt should exist in window");
                            if let Some(element) = document.query_selector("#tr_move").unwrap() {
                                if let Ok(audio) = element.dyn_into::<web_sys::HtmlAudioElement>() {
                                    audio.play().expect("Audio should be playable");
                                }
                            }
                        }

                        game.board_changes.clear();
                        for board_change in changes.iter().filter_map(|c| match c {
                            Change::Board(change) => Some(change),
                            _ => None,
                        }) {
                            game.board_changes
                                .insert(board_change.detail.coordinate, board_change.clone());
                        }

                        for hand_change in changes.iter().filter_map(|c| match c {
                            Change::Hand(change) => Some(change),
                            _ => None,
                        }) {
                            for removed in &hand_change.removed {
                                game.hand.remove(
                                    game.hand
                                        .iter()
                                        .position(|t| t == removed)
                                        .expect("Player doesn't have tile being removed"),
                                );
                            }
                            let reduced_length = game.hand.len();
                            game.hand.0.extend(&hand_change.added);
                            game.new_hand_tiles = (reduced_length..game.hand.len()).collect();
                        }

                        for battle in changes.into_iter().filter_map(|c| match c {
                            Change::Battle(battle) => Some(battle),
                            _ => None,
                        }) {
                            game.battles.push(battle);
                        }

                        // TODO: Verify that our modified hand matches the actual hand in GameStateMessage

                        game.ctx.playing_tile = None;
                        game.ctx.error_msg = None;
                    }
                    _ => todo!("Game update hit an unknown state"),
                }
            }
            GameMessage::GameEnd(
                GameStateMessage {
                    room_code: _,
                    players,
                    player_number: _,
                    next_player_number: _,
                    board,
                    hand: _,
                    changes,
                },
                winner,
            ) => match game_status {
                GameStatus::Active(game) => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        let local_storage =
                            web_sys::window().unwrap().local_storage().unwrap().unwrap();
                        local_storage.remove_item("truncate_active_token").unwrap();
                    }

                    // assert_eq!(game.room_code, id);
                    // assert_eq!(game.player_number, num);
                    game.players = players;
                    game.board = board;
                    game.board_changes.clear();
                    for board_change in changes.iter().filter_map(|c| match c {
                        Change::Board(change) => Some(change),
                        _ => None,
                    }) {
                        game.board_changes
                            .insert(board_change.detail.coordinate, board_change.clone());
                    }
                    for battle in changes.into_iter().filter_map(|c| match c {
                        Change::Battle(battle) => Some(battle),
                        _ => None,
                    }) {
                        game.battles.push(battle);
                    }
                    *game_status = GameStatus::Concluded(game.clone(), winner);
                }
                _ => todo!("Game error hit an unknown state"),
            },
            GameMessage::GameError(_id, _num, err) => match game_status {
                GameStatus::Active(game) => {
                    // assert_eq!(game.room_code, id);
                    // assert_eq!(game.player_number, num);
                    game.ctx.error_msg = Some(err);
                }
                _ => todo!("Game error hit an unknown state"),
            },
            GameMessage::GenericError(_err) => {
                todo!("Handle generic errors")
            }
        }
    }
}
