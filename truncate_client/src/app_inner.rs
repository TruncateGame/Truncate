use eframe::egui::{self, Margin};
use epaint::{vec2, Color32};
use instant::Duration;
use truncate_core::{
    board::Board,
    generation::{generate_board, BoardSeed},
    messages::RoomCode,
    messages::{LobbyPlayerMessage, Token},
};

use crate::{
    regions::active_game::ActiveGame,
    regions::{
        active_game::HeaderType, generator::GeneratorState, lobby::Lobby,
        single_player::SinglePlayerState, tutorial::TutorialState,
    },
    utils::{daily::get_daily_puzzle, text::TextHelper, Lighten},
};

use super::OuterApplication;
use truncate_core::messages::{GameMessage, GameStateMessage, PlayerMessage};

pub enum GameStatus {
    None(RoomCode, Option<Token>),
    Generator(GeneratorState),
    Tutorial(TutorialState),
    PendingSinglePlayer(Lobby),
    SinglePlayer(SinglePlayerState),
    PendingJoin(RoomCode),
    PendingCreate,
    PendingStart(Lobby),
    Active(ActiveGame),
    Concluded(ActiveGame, u64),
}

pub fn render(client: &mut OuterApplication, ui: &mut egui::Ui, current_time: Duration) {
    let OuterApplication {
        name,
        theme,
        game_status,
        rx_game,
        tx_player,
        map_texture,
        launched_room,
        error,
        backchannel,
        log_frames,
        frames,
    } = client;

    if *log_frames {
        let ctx = ui.ctx().clone();

        egui::Window::new("🔍 Inspection")
            .vscroll(true)
            .default_pos(ui.next_widget_position() + vec2(ui.available_width(), 0.0))
            .show(&ctx, |ui| {
                frames.ui(ui);
                ctx.inspection_ui(ui);
            });
    }

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
        } else if launched_room == "TUTORIAL_01" {
            new_game_status = Some(GameStatus::Tutorial(TutorialState::new(
                ui.ctx(),
                map_texture.clone(),
                theme.clone(),
            )));
        } else if launched_room == "SINGLE_PLAYER" {
            let mut board = Board::new(9, 9);
            board.grow();
            new_game_status = Some(GameStatus::PendingSinglePlayer(Lobby::new(
                ui.ctx(),
                "Single Player".into(),
                vec![
                    LobbyPlayerMessage {
                        name: "You".into(),
                        index: 0,
                        color: (128, 128, 255),
                    },
                    LobbyPlayerMessage {
                        name: "Computer".into(),
                        index: 1,
                        color: (255, 80, 80),
                    },
                ],
                0,
                board,
                map_texture.clone(),
            )));
        } else if launched_room == "DAILY_PUZZLE" {
            let puzzle_game =
                get_daily_puzzle(ui.ctx(), current_time, map_texture, theme, backchannel);
            new_game_status = Some(GameStatus::SinglePlayer(puzzle_game));
        } else if launched_room == "RANDOM_PUZZLE" {
            let seed = (current_time.as_micros() % 243985691) as u32;
            let board_seed = BoardSeed::new(seed);
            let board = generate_board(board_seed.clone())
                .expect("Common seeds can be reasonably expected to produce a board")
                .board;
            let header = HeaderType::Summary {
                title: format!("Random Puzzle"),
                sentinel: '•',
                attempt: None,
            };
            let puzzle_game = SinglePlayerState::new(
                ui.ctx(),
                map_texture.clone(),
                theme.clone(),
                board,
                Some(board_seed),
                true,
                header,
            );
            new_game_status = Some(GameStatus::SinglePlayer(puzzle_game));
        } else if launched_room.starts_with("PUZZLE:") {
            let mut parts = launched_room.split(':').skip(1);
            let generation = parts.next().map(str::parse::<u32>);
            let seed = parts.next().map(str::parse::<u32>);
            let player = parts
                .next()
                .map(|p| p.parse::<usize>().unwrap_or(0))
                .unwrap_or(0);

            let (Some(Ok(generation)), Some(Ok(seed))) = (generation, seed) else {
                panic!("Bad URL provided for puzzle");
            };

            let board_seed = BoardSeed::new_with_generation(generation, seed);
            let board = generate_board(board_seed.clone())
                .expect("Common seeds can be reasonably expected to produce a board")
                .board;
            let header = HeaderType::Summary {
                title: format!("Truncate Puzzle {generation}:{seed}"),
                sentinel: '•',
                attempt: None,
            };
            let puzzle_game = SinglePlayerState::new(
                ui.ctx(),
                map_texture.clone(),
                theme.clone(),
                board,
                Some(board_seed),
                player == 0,
                header,
            );
            new_game_status = Some(GameStatus::SinglePlayer(puzzle_game));
        } else if launched_room == "DEBUG_BEHEMOTH" {
            let behemoth_board = Board::from_string(include_str!("../tutorials/test_board.txt"));
            let seed_for_hand_tiles = BoardSeed::new_with_generation(0, 1);
            let behemoth_game = SinglePlayerState::new(
                ui.ctx(),
                map_texture.clone(),
                theme.clone(),
                behemoth_board,
                Some(seed_for_hand_tiles),
                true,
                HeaderType::Timers,
            );
            new_game_status = Some(GameStatus::SinglePlayer(behemoth_game));
            *log_frames = true;
        } else if launched_room.is_empty() {
            // No room code means we start a new game.
            send(PlayerMessage::NewGame(name.clone()));
            new_game_status = Some(GameStatus::PendingCreate);
        } else {
            let token = if let GameStatus::None(_, token) = game_status {
                token.clone()
            } else {
                None
            };

            send(PlayerMessage::JoinGame(
                launched_room.clone(),
                name.clone(),
                token,
            ));

            new_game_status = Some(GameStatus::PendingJoin(launched_room));
        }
    }

    // ui.horizontal(|ui| {
    //     if option_env!("TR_PROD").is_none() {
    //         if let (Some(commit_msg), Some(commit_hash)) =
    //             (option_env!("TR_MSG"), option_env!("TR_COMMIT"))
    //         {
    //             ui.hyperlink_to(
    //                 format!("Running \"{commit_msg}\""),
    //                 format!("https://github.com/TruncateGame/Truncate/commit/{commit_hash}"),
    //             );
    //         } else {
    //             ui.label(format!("No tagged commit."));
    //         }
    //     }

    //     if matches!(game_status, GameStatus::None(_, _)) {
    //         ui.horizontal(|ui| {
    //             ui.label("Name: ");
    //             ui.text_edit_singleline(name);
    //         });
    //     } else {
    //         ui.label(format!("Playing as {name}"));
    //     }
    // });

    // ui.separator();

    match game_status {
        GameStatus::None(room_code, token) => {
            if ui.button("Generator").clicked() {
                new_game_status = Some(GameStatus::Generator(GeneratorState::new(
                    ui.ctx(),
                    map_texture.clone(),
                    theme.clone(),
                )));
            }
            if ui.button("Tutorial").clicked() {
                new_game_status = Some(GameStatus::Tutorial(TutorialState::new(
                    ui.ctx(),
                    map_texture.clone(),
                    theme.clone(),
                )));
            }
            if ui.button("Single Player").clicked() {
                let mut board = Board::new(9, 9);
                board.grow();
                new_game_status = Some(GameStatus::PendingSinglePlayer(Lobby::new(
                    ui.ctx(),
                    "Single Player".into(),
                    vec![
                        LobbyPlayerMessage {
                            name: "You".into(),
                            index: 0,
                            color: (128, 128, 255),
                        },
                        LobbyPlayerMessage {
                            name: "Computer".into(),
                            index: 1,
                            color: (255, 80, 80),
                        },
                    ],
                    0,
                    board,
                    map_texture.clone(),
                )));
            }
            if ui.button("Behemoth").clicked() {
                let behemoth_board =
                    Board::from_string(include_str!("../tutorials/test_board.txt"));
                let seed_for_hand_tiles = BoardSeed::new_with_generation(0, 1);
                let behemoth_game = SinglePlayerState::new(
                    ui.ctx(),
                    map_texture.clone(),
                    theme.clone(),
                    behemoth_board,
                    Some(seed_for_hand_tiles),
                    true,
                    HeaderType::Timers,
                );
                new_game_status = Some(GameStatus::SinglePlayer(behemoth_game));
                *log_frames = true;
            }
            if ui.button("New Game").clicked() {
                // TODO: Send player name in NewGame message
                send(PlayerMessage::NewGame(name.clone()));
                new_game_status = Some(GameStatus::PendingCreate);
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(room_code);
                if ui.button("Join Game").clicked() {
                    send(PlayerMessage::JoinGame(
                        room_code.clone(),
                        name.clone(),
                        token.clone(),
                    ));
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
        GameStatus::Generator(generator) => {
            generator.render(ui, current_time);
        }
        GameStatus::Tutorial(tutorial) => {
            tutorial.render(ui, theme, current_time);
        }
        GameStatus::PendingSinglePlayer(editor_state) => {
            if let Some(msg) = editor_state.render(ui, theme) {
                match msg {
                    PlayerMessage::StartGame => {
                        let single_player_game = SinglePlayerState::new(
                            ui.ctx(),
                            map_texture.clone(),
                            theme.clone(),
                            editor_state.board.clone(),
                            editor_state.board_seed.clone(),
                            true,
                            HeaderType::Timers,
                        );
                        new_game_status = Some(GameStatus::SinglePlayer(single_player_game));
                    }
                    _ => {
                        // Ignore anything else the lobby might return.
                    }
                }
            }
        }
        GameStatus::SinglePlayer(sp) => {
            // Special performance debug mode — hide the sidebar to give us more space
            if *log_frames {
                sp.active_game.depot.ui_state.sidebar_hidden = true;
            }

            // Single player _can_ talk to the server, e.g. to ask for word definitions
            if let Some(msg) = sp.render(ui, theme, current_time, &backchannel) {
                send(msg);
            };
        }
        GameStatus::PendingJoin(room_code) => {
            let dot_count = (current_time.as_millis() / 500) % 4;
            let mut dots = vec!["."; dot_count as usize];
            dots.extend(vec![" "; 4 - dot_count as usize]);
            let msg = if let Some(error) = error {
                error.clone()
            } else {
                format!("JOINING {room_code}{}", dots.join(""))
            };

            let msg_text = TextHelper::heavy(&msg, 14.0, None, ui);
            let button_text = TextHelper::heavy("CANCEL", 14.0, None, ui);
            let required_size = vec2(
                msg_text.size().x,
                msg_text.size().y + button_text.size().y * 2.0,
            );

            let margins = (ui.available_size_before_wrap() - required_size) / 2.0;
            let outer_frame = egui::Frame::none().inner_margin(Margin::from(margins));
            outer_frame.show(ui, |ui| {
                msg_text.paint(Color32::WHITE, ui, false);
                ui.add_space(8.0);
                if button_text
                    .centered_button(
                        theme.selection.lighten().lighten(),
                        theme.text,
                        &map_texture,
                        ui,
                    )
                    .clicked()
                {
                    #[cfg(target_arch = "wasm32")]
                    {
                        _ = web_sys::window().unwrap().location().set_hash("");
                        _ = web_sys::window().unwrap().location().reload();
                    }
                }
            });
        }
        GameStatus::PendingCreate => {
            let dot_count = (current_time.as_millis() / 500) % 4;
            let mut dots = vec!["."; dot_count as usize];
            dots.extend(vec![" "; 4 - dot_count as usize]);
            let msg = if let Some(error) = error {
                error.clone()
            } else {
                format!("CREATING ROOM{}", dots.join(""))
            };

            let msg_text = TextHelper::heavy(&msg, 14.0, None, ui);
            let button_text = TextHelper::heavy("CANCEL", 14.0, None, ui);
            let required_size = vec2(
                msg_text.size().x,
                msg_text.size().y + button_text.size().y * 2.0,
            );

            let margins = (ui.available_size_before_wrap() - required_size) / 2.0;
            let outer_frame = egui::Frame::none().inner_margin(Margin::from(margins));
            outer_frame.show(ui, |ui| {
                msg_text.paint(Color32::WHITE, ui, false);
                ui.add_space(8.0);
                if button_text
                    .centered_button(
                        theme.selection.lighten().lighten(),
                        theme.text,
                        &map_texture,
                        ui,
                    )
                    .clicked()
                {
                    #[cfg(target_arch = "wasm32")]
                    {
                        _ = web_sys::window().unwrap().location().set_hash("");
                        _ = web_sys::window().unwrap().location().reload();
                    }
                }
            });
        }
        GameStatus::PendingStart(editor_state) => {
            if let Some(msg) = editor_state.render(ui, theme) {
                send(msg);
            }
        }
        GameStatus::Active(game) => {
            if let Some(msg) = game.render(ui, current_time, None) {
                send(msg);
            }
        }
        GameStatus::Concluded(game, _winner) => {
            if let Some(PlayerMessage::Rematch) = game.render(ui, current_time, None) {
                send(PlayerMessage::Rematch);
            }
        }
    }
    if let Some(new_game_status) = new_game_status {
        *game_status = new_game_status;
    }

    while let Ok(msg) = recv() {
        match msg {
            GameMessage::Ping => {}
            GameMessage::JoinedLobby(player_index, id, players, board, token) => {
                // If we're already in a lobby, treat this as a lobby update
                // (the websocket probably dropped and reconnected)
                if let GameStatus::PendingStart(lobby) = game_status {
                    if lobby.room_code.to_uppercase() == id.to_uppercase() {
                        lobby.players = players;
                        lobby.update_board(board, ui);
                        continue;
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    let local_storage =
                        web_sys::window().unwrap().local_storage().unwrap().unwrap();
                    local_storage
                        .set_item("truncate_active_token", &token)
                        .unwrap();

                    // If we're joining a lobby, update the URL to match
                    _ = web_sys::window()
                        .unwrap()
                        .location()
                        .set_hash(id.to_uppercase().as_str());
                }

                *game_status = GameStatus::PendingStart(Lobby::new(
                    ui.ctx(),
                    id.to_uppercase(),
                    players,
                    player_index,
                    board,
                    map_texture.clone(),
                ))
            }
            GameMessage::LobbyUpdate(_player_index, _id, players, board) => {
                match game_status {
                    GameStatus::PendingStart(editor_state) => {
                        // TODO: Assert that this message is for the correct lobby
                        editor_state.players = players;
                        editor_state.update_board(board, ui);
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
                // If we're already in a game, treat this as a game update
                // (the websocket probably dropped and reconnected)
                if let GameStatus::Active(game) = game_status {
                    if game.depot.gameplay.room_code.to_uppercase() == room_code.to_uppercase() {
                        let update = GameStateMessage {
                            room_code,
                            players,
                            player_number,
                            next_player_number,
                            board,
                            hand,
                            changes: vec![], // TODO: Try get latest changes on reconnect without dupes
                        };
                        game.apply_new_state(update);
                        continue;
                    }
                }

                *game_status = GameStatus::Active(ActiveGame::new(
                    ui.ctx(),
                    room_code.to_uppercase(),
                    None,
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
            GameMessage::GameUpdate(state_message) => match game_status {
                GameStatus::Active(game) => {
                    game.apply_new_state(state_message);
                }
                _ => todo!("Game update hit an unknown state"),
            },
            GameMessage::GameEnd(state_message, winner) => {
                #[cfg(target_arch = "wasm32")]
                {
                    let local_storage =
                        web_sys::window().unwrap().local_storage().unwrap().unwrap();
                    local_storage.remove_item("truncate_active_token").unwrap();
                }

                match game_status {
                    GameStatus::Active(game) => {
                        game.apply_new_state(state_message);
                        *game_status = GameStatus::Concluded(game.clone(), winner);
                    }
                    _ => {}
                }
            }
            GameMessage::GameError(_id, _num, err) => match game_status {
                GameStatus::Active(game) => {
                    // assert_eq!(game.room_code, id);
                    // assert_eq!(game.player_number, num);
                    game.depot.gameplay.error_msg = Some(err);
                }
                _ => {}
            },
            GameMessage::GenericError(err) => {
                *error = Some(err);
            }
            GameMessage::SupplyDefinitions(definitions) => {
                match game_status {
                    GameStatus::SinglePlayer(game) => {
                        game.hydrate_meanings(definitions);
                    }
                    _ => { /* Soft unreachable */ }
                }
            }
        }
    }
}
