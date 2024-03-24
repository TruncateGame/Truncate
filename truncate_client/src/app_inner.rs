use eframe::egui::{self, Margin};
use epaint::{vec2, Color32};
use instant::Duration;
use truncate_core::{
    board::Board,
    game::{self, GAME_COLORS, GAME_COLOR_BLUE, GAME_COLOR_RED},
    generation::{self, generate_board, BoardSeed},
    messages::{LobbyPlayerMessage, RoomCode, TruncateToken},
    npc::scoring::{NPCParams, NPCPersonality},
};

use crate::{
    lil_bits::SplashUI,
    regions::{
        active_game::{ActiveGame, GameLocation, HeaderType},
        generator::GeneratorState,
        lobby::Lobby,
        replayer::ReplayerState,
        single_player::SinglePlayerState,
        tutorial::TutorialState,
    },
    utils::{
        daily::{get_playable_daily_puzzle, get_puzzle_day, get_raw_daily_puzzle},
        game_evals::get_main_dict,
        macros::tr_log,
        text::TextHelper,
        urls::back_to_menu,
        Lighten,
    },
};

use super::OuterApplication;
use truncate_core::messages::{GameMessage, GameStateMessage, PlayerMessage};

pub enum GameStatus {
    None(RoomCode, Option<TruncateToken>),
    Generator(GeneratorState),
    Tutorial(TutorialState),
    PendingSinglePlayer(Lobby),
    SinglePlayer(SinglePlayerState),
    PendingDaily,
    PendingJoin(RoomCode),
    PendingCreate,
    PendingStart(Lobby),
    Active(ActiveGame),
    Concluded(ActiveGame, u64),
    PendingReplay,
    Replay(ReplayerState),
    HardError(Vec<String>),
}

pub fn handle_server_msg(outer: &mut OuterApplication, ui: &mut egui::Ui, current_time: Duration) {
    let mut recv = || match outer.rx_game.try_next() {
        Ok(Some(msg)) => Ok(msg),
        _ => Err(()),
    };

    while let Ok(msg) = recv() {
        match msg {
            GameMessage::Ping => {}
            GameMessage::JoinedLobby(player_index, id, players, board, token) => {
                // If we're already in a lobby, treat this as a lobby update
                // (the websocket probably dropped and reconnected)
                if let GameStatus::PendingStart(lobby) = &mut outer.game_status {
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
                    _ = web_sys::window().unwrap().location().set_pathname("/join/");
                    _ = web_sys::window()
                        .unwrap()
                        .location()
                        .set_search(&format!("j={}", id.to_uppercase()));
                }

                outer.game_status = GameStatus::PendingStart(Lobby::new(
                    ui.ctx(),
                    id.to_uppercase(),
                    players,
                    player_index,
                    board,
                    outer.map_texture.clone(),
                ))
            }
            GameMessage::LobbyUpdate(_player_index, _id, players, board) => {
                match &mut outer.game_status {
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
                if let GameStatus::Active(game) = &mut outer.game_status {
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

                outer.game_status = GameStatus::Active(ActiveGame::new(
                    ui.ctx(),
                    room_code.to_uppercase(),
                    None,
                    None,
                    players,
                    player_number,
                    next_player_number,
                    board,
                    hand,
                    outer.map_texture.clone(),
                    outer.theme.clone(),
                    GameLocation::Online,
                ));
            }
            GameMessage::GameUpdate(state_message) => match &mut outer.game_status {
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

                match &mut outer.game_status {
                    GameStatus::Active(game) => {
                        game.apply_new_state(state_message);
                        outer.game_status = GameStatus::Concluded(game.clone(), winner);
                    }
                    _ => {}
                }
            }
            GameMessage::GameError(_id, _num, err) => match &mut outer.game_status {
                GameStatus::Active(game) => {
                    // assert_eq!(game.room_code, id);
                    // assert_eq!(game.player_number, num);
                    game.depot.gameplay.error_msg = Some(err);
                }
                _ => {}
            },
            GameMessage::GenericError(err) => {
                outer.error = Some(err);
            }
            GameMessage::SupplyDefinitions(definitions) => {
                match &mut outer.game_status {
                    GameStatus::SinglePlayer(game) => {
                        game.hydrate_meanings(definitions.clone());
                        if let Some(dict_ui) = &mut game.active_game.dictionary_ui {
                            dict_ui.load_definitions(definitions);
                        }
                    }
                    GameStatus::Active(active_game) => {
                        if let Some(dict_ui) = &mut active_game.dictionary_ui {
                            dict_ui.load_definitions(definitions);
                        }
                    }
                    _ => { /* Soft unreachable */ }
                }
            }
            GameMessage::LoggedInAs(player_token) => {
                #[cfg(target_arch = "wasm32")]
                {
                    let local_storage =
                        web_sys::window().unwrap().local_storage().unwrap().unwrap();
                    local_storage
                        .set_item("truncate_player_token", &player_token)
                        .unwrap();
                }

                outer.logged_in_as = Some(player_token);
            }
            GameMessage::ResumeDailyPuzzle(latest_puzzle_state, best_puzzle) => {
                let mut puzzle_game = get_playable_daily_puzzle(
                    ui.ctx(),
                    latest_puzzle_state.puzzle_day,
                    &outer.map_texture,
                    &outer.theme,
                    &outer.backchannel,
                );

                if let Some(best_puzzle) = best_puzzle {
                    let mut best_game = puzzle_game.game.clone();
                    best_game.rules.battle_delay = 0;
                    let dict_lock = get_main_dict();
                    let dict = dict_lock.as_ref().unwrap();

                    for next_move in best_puzzle.current_moves.into_iter() {
                        if best_game
                            .play_turn(next_move, Some(dict), Some(dict), None)
                            .is_err()
                        {
                            break;
                        }
                    }

                    puzzle_game.best_game = Some(best_game);
                }

                let unplayed_puzzle = puzzle_game.clone();

                match &mut puzzle_game.header {
                    HeaderType::Summary { attempt, .. } => {
                        *attempt = Some(latest_puzzle_state.attempt as usize)
                    }
                    _ => {}
                }
                puzzle_game.move_sequence = latest_puzzle_state.current_moves.clone();

                let delay = puzzle_game.game.rules.battle_delay;
                puzzle_game.game.rules.battle_delay = 0;
                for next_move in latest_puzzle_state.current_moves.into_iter() {
                    if puzzle_game
                        .handle_move(next_move, &outer.backchannel)
                        .is_err()
                    {
                        puzzle_game = unplayed_puzzle;
                        break;
                    }
                }
                puzzle_game.game.rules.battle_delay = delay;

                puzzle_game.active_game.depot.ui_state.game_header = puzzle_game.header.clone();
                outer.game_status = GameStatus::SinglePlayer(puzzle_game);
            }
            GameMessage::DailyStats(stats) => match &mut outer.game_status {
                GameStatus::SinglePlayer(game) => {
                    game.daily_stats = Some(stats);
                }
                _ => {}
            },
            GameMessage::LoadDailyReplay(puzzle_state) => {
                let (seed, info) = get_raw_daily_puzzle(puzzle_state.puzzle_day);
                let human_starts = info.as_ref().map(|(h, _)| *h).unwrap_or(true);

                let mut game = game::Game::new(9, 9, Some(seed.seed as u64));
                if human_starts {
                    game.add_player("You".into());
                    game.add_player("Computer".into());

                    game.players[0].color = GAME_COLOR_BLUE;
                    game.players[1].color = GAME_COLOR_RED;
                } else {
                    game.add_player("Computer".into());
                    game.add_player("You".into());

                    game.players[0].color = GAME_COLOR_RED;
                    game.players[1].color = GAME_COLOR_BLUE;
                }

                let mut board = generation::generate_board(seed.clone())
                    .expect("Common seeds should always generate a board")
                    .board;
                board.cache_special_squares();
                game.board = board.clone();

                let replayer = ReplayerState::new(
                    ui.ctx(),
                    outer.map_texture.clone(),
                    outer.theme.clone(),
                    game,
                    puzzle_state.current_moves,
                    if human_starts { 0 } else { 1 },
                );
                outer.game_status = GameStatus::Replay(replayer);
            }
        }
    }
}

pub fn render(outer: &mut OuterApplication, ui: &mut egui::Ui, current_time: Duration) {
    handle_server_msg(outer, ui, current_time);

    if outer.log_frames {
        let ctx = ui.ctx().clone();

        egui::Window::new("🔍 Inspection")
            .vscroll(true)
            .default_pos(ui.next_widget_position() + vec2(ui.available_width(), 0.0))
            .show(&ctx, |ui| {
                outer.frames.ui(ui);
                ctx.inspection_ui(ui);
            });
    }

    let mut send = |msg| {
        outer.tx_player.try_send(msg).unwrap();
    };

    // Block all further actions until we have a login token from the server,
    // or until the player accepts to play offline.
    if let (Some(waiting_for_login), None) = (&outer.started_login_at, &outer.logged_in_as) {
        if (current_time - *waiting_for_login) < Duration::from_secs(5) {
            SplashUI::new(vec!["INITIALIZING".to_string()])
                .animated(true)
                .render(ui, &outer.theme, current_time, &outer.map_texture);
            return;
        } else {
            let resp = SplashUI::new(vec![
                "COULD NOT CONNECT".to_string(),
                "TO TRUNCATE".to_string(),
            ])
            .byline(vec![
                "Offline play may not be saved.".to_string(),
                "Reload to try again,".to_string(),
                "or continue to play offline.".to_string(),
            ])
            .with_button(
                "continue",
                "CONTINUE".to_string(),
                outer.theme.button_primary,
            )
            .render(ui, &outer.theme, current_time, &outer.map_texture);

            if resp.clicked == Some("continue") {
                outer.started_login_at = None;
            } else {
                return;
            }
        }
    }

    let mut new_game_status = None;

    if let Some(launched_room) = outer.launched_room.take() {
        if launched_room == "__REJOIN__" {
            match &mut outer.game_status {
                GameStatus::None(_, Some(token)) => {
                    send(PlayerMessage::RejoinGame(token.to_string()));
                    new_game_status = Some(GameStatus::PendingJoin("...".into()));
                }
                _ => {
                    new_game_status =
                        Some(GameStatus::HardError(vec!["Could not rejoin".to_string()]));
                }
            }
        } else if launched_room == "TUTORIAL_RULES" {
            new_game_status = Some(GameStatus::Tutorial(TutorialState::new_rules(
                ui.ctx(),
                outer.map_texture.clone(),
                &outer.theme,
            )));
            send(PlayerMessage::StartedTutorial {
                name: "rules".to_string(),
            });
        } else if launched_room == "TUTORIAL_EXAMPLE" {
            new_game_status = Some(GameStatus::Tutorial(TutorialState::new_example(
                ui.ctx(),
                outer.map_texture.clone(),
                &outer.theme,
            )));
            send(PlayerMessage::StartedTutorial {
                name: "example".to_string(),
            });
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
                outer.map_texture.clone(),
            )));
            send(PlayerMessage::StartedSinglePlayer);
        } else if launched_room == "DAILY_PUZZLE" {
            let day = get_puzzle_day(current_time);
            if let Some(token) = &outer.logged_in_as {
                send(PlayerMessage::LoadDailyPuzzle(token.clone(), day));
            }

            new_game_status = Some(GameStatus::PendingDaily);
        } else if launched_room == "RANDOM_PUZZLE" {
            let seed = (current_time.as_micros() % 243985691) as u32;
            let board_seed = BoardSeed::new(seed);
            let board = generate_board(board_seed.clone())
                .expect("Common seeds can be reasonably expected to produce a board")
                .board;
            let header = HeaderType::Summary {
                title: format!("Regular Puzzle"),
                attempt: None,
            };
            let puzzle_game = SinglePlayerState::new(
                ui.ctx(),
                outer.map_texture.clone(),
                outer.theme.clone(),
                board,
                Some(board_seed),
                true,
                header,
                NPCPersonality::jet(),
            );
            new_game_status = Some(GameStatus::SinglePlayer(puzzle_game));
            send(PlayerMessage::StartedRandomPuzzle {
                personality: "jet".into(),
            });
        } else if launched_room == "RANDOM_EASY_PUZZLE" {
            let seed = (current_time.as_micros() % 243985691) as u32;
            let board_seed = BoardSeed::new(seed);
            let board = generate_board(board_seed.clone())
                .expect("Common seeds can be reasonably expected to produce a board")
                .board;
            let header = HeaderType::Summary {
                title: format!("Easy Puzzle"),
                attempt: None,
            };
            let puzzle_game = SinglePlayerState::new(
                ui.ctx(),
                outer.map_texture.clone(),
                outer.theme.clone(),
                board,
                Some(board_seed),
                true,
                header,
                NPCPersonality::mellite(),
            );
            new_game_status = Some(GameStatus::SinglePlayer(puzzle_game));
            send(PlayerMessage::StartedRandomPuzzle {
                personality: "mellite".into(),
            });
        } else if launched_room.starts_with("PUZZLE:") {
            let url_segments = launched_room.chars().filter(|c| *c == ':').count();
            let has_board_generation = url_segments >= 3;
            let has_npc_id = url_segments >= 4;

            let mut parts = launched_room.split(':').skip(1);
            let board_generation = if has_board_generation {
                parts.next().map(str::parse::<u32>)
            } else {
                Some(Ok(0))
            };
            let npc = if has_npc_id {
                parts
                    .next()
                    .map(|p| NPCPersonality::from_id(p.to_ascii_lowercase()))
            } else {
                Some(Some(NPCPersonality::jet()))
            };
            let seed = parts.next().map(str::parse::<u32>);
            let player = parts
                .next()
                .map(|p| p.parse::<usize>().unwrap_or(0))
                .unwrap_or(0);

            if let (Some(Ok(board_generation)), Some(Some(npc)), Some(Ok(seed))) =
                (board_generation, npc, seed)
            {
                let board_seed = BoardSeed::new_with_generation(board_generation, seed);
                let board = generate_board(board_seed.clone())
                    .expect("Common seeds can be reasonably expected to produce a board")
                    .board;
                let header = HeaderType::Summary {
                    title: format!("Truncate Puzzle"),
                    attempt: None,
                };
                send(PlayerMessage::StartedRandomPuzzle {
                    personality: npc.name.clone(),
                });
                let puzzle_game = SinglePlayerState::new(
                    ui.ctx(),
                    outer.map_texture.clone(),
                    outer.theme.clone(),
                    board,
                    Some(board_seed),
                    player == 0,
                    header,
                    npc,
                );
                new_game_status = Some(GameStatus::SinglePlayer(puzzle_game));
            } else {
                new_game_status = Some(GameStatus::HardError(vec![
                    "Sorry, that puzzle URL".to_string(),
                    "doesn't look right!".to_string(),
                ]));
            }
        } else if launched_room.starts_with("REPLAY:") {
            if let Some(id) = launched_room.split(':').skip(1).next() {
                send(PlayerMessage::LoadReplay(id.to_string()));
                new_game_status = Some(GameStatus::PendingReplay);
            } else {
                new_game_status = Some(GameStatus::HardError(vec![
                    "Sorry, that replay URL".to_string(),
                    "doesn't look right!".to_string(),
                ]));
            }
        } else if launched_room == "DEBUG_BEHEMOTH" {
            let behemoth_board = Board::from_string(include_str!("../tutorials/test_board.txt"));
            let seed_for_hand_tiles = BoardSeed::new_with_generation(0, 1);
            let behemoth_game = SinglePlayerState::new(
                ui.ctx(),
                outer.map_texture.clone(),
                outer.theme.clone(),
                behemoth_board,
                Some(seed_for_hand_tiles),
                true,
                HeaderType::Timers,
                NPCPersonality::jet(),
            );
            new_game_status = Some(GameStatus::SinglePlayer(behemoth_game));
            outer.log_frames = true;
        } else if launched_room.is_empty() {
            // No room code means we start a new game.
            send(PlayerMessage::NewGame(outer.name.clone()));
            new_game_status = Some(GameStatus::PendingCreate);
        } else {
            let token = if let GameStatus::None(_, token) = &outer.game_status {
                token.clone()
            } else {
                None
            };

            send(PlayerMessage::JoinGame(
                launched_room.clone(),
                outer.name.clone(),
                token,
            ));

            new_game_status = Some(GameStatus::PendingJoin(launched_room));
        }
    }

    match &mut outer.game_status {
        GameStatus::None(room_code, token) => {
            if ui.button("Generator").clicked() {
                new_game_status = Some(GameStatus::Generator(GeneratorState::new(
                    ui.ctx(),
                    outer.map_texture.clone(),
                    outer.theme.clone(),
                )));
            }
            if ui.button("Tutorial: Rules").clicked() {
                new_game_status = Some(GameStatus::Tutorial(TutorialState::new_rules(
                    ui.ctx(),
                    outer.map_texture.clone(),
                    &outer.theme,
                )));
            }
            if ui.button("Tutorial: Example").clicked() {
                new_game_status = Some(GameStatus::Tutorial(TutorialState::new_example(
                    ui.ctx(),
                    outer.map_texture.clone(),
                    &outer.theme,
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
                    outer.map_texture.clone(),
                )));
            }
            if ui.button("Behemoth").clicked() {
                let behemoth_board =
                    Board::from_string(include_str!("../tutorials/test_board.txt"));
                let seed_for_hand_tiles = BoardSeed::new_with_generation(0, 1);
                let behemoth_game = SinglePlayerState::new(
                    ui.ctx(),
                    outer.map_texture.clone(),
                    outer.theme.clone(),
                    behemoth_board,
                    Some(seed_for_hand_tiles),
                    true,
                    HeaderType::Timers,
                    NPCPersonality::jet(),
                );
                new_game_status = Some(GameStatus::SinglePlayer(behemoth_game));
                outer.log_frames = true;
            }
            if ui.button("New Game").clicked() {
                // TODO: Send player name in NewGame message
                send(PlayerMessage::NewGame(outer.name.clone()));
                new_game_status = Some(GameStatus::PendingCreate);
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(room_code);
                if ui.button("Join Game").clicked() {
                    send(PlayerMessage::JoinGame(
                        room_code.clone(),
                        outer.name.clone(),
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
            generator.render(ui, &outer.theme, current_time);
        }
        GameStatus::Tutorial(tutorial) => {
            tutorial.render(ui, outer.map_texture.clone(), &outer.theme, current_time);
        }
        GameStatus::PendingSinglePlayer(editor_state) => {
            if let Some(msg) = editor_state.render(ui, &outer.theme) {
                match msg {
                    PlayerMessage::StartGame => {
                        let single_player_game = SinglePlayerState::new(
                            ui.ctx(),
                            outer.map_texture.clone(),
                            outer.theme.clone(),
                            editor_state.board.clone(),
                            editor_state.board_seed.clone(),
                            true,
                            HeaderType::Timers,
                            NPCPersonality::jet(),
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
            if outer.log_frames {
                sp.active_game.depot.ui_state.sidebar_hidden = true;
            }

            // Single player can talk to the server, e.g. to ask for word definitions and to persist data
            for msg in sp.render(
                ui,
                &outer.theme,
                current_time,
                &outer.backchannel,
                &outer.logged_in_as,
            ) {
                send(msg);
            }
        }
        GameStatus::PendingDaily => {
            let splash = SplashUI::new(if let Some(error) = &outer.error {
                vec![error.clone()]
            } else {
                vec![format!("LOADING DAILY PUZZLE")]
            })
            .animated(outer.error.is_none())
            .with_button("cancel", "CANCEL".to_string(), outer.theme.button_primary);

            let resp = splash.render(ui, &outer.theme, current_time, &outer.map_texture);

            if resp.clicked == Some("cancel") {
                back_to_menu();
            }
        }
        GameStatus::PendingJoin(room_code) => {
            let splash = SplashUI::new(if let Some(error) = &outer.error {
                vec![error.clone()]
            } else {
                vec![format!("JOINING {room_code}")]
            })
            .animated(outer.error.is_none())
            .with_button("cancel", "CANCEL".to_string(), outer.theme.button_primary);

            let resp = splash.render(ui, &outer.theme, current_time, &outer.map_texture);

            if resp.clicked == Some("cancel") {
                back_to_menu();
            }
        }
        GameStatus::PendingCreate => {
            let splash = SplashUI::new(if let Some(error) = &outer.error {
                vec![error.clone()]
            } else {
                vec!["CREATING ROOM".to_string()]
            })
            .animated(outer.error.is_none())
            .with_button("cancel", "CANCEL".to_string(), outer.theme.button_primary);

            let resp = splash.render(ui, &outer.theme, current_time, &outer.map_texture);

            if resp.clicked == Some("cancel") {
                back_to_menu();
            }
        }
        GameStatus::PendingStart(editor_state) => {
            if let Some(msg) = editor_state.render(ui, &outer.theme) {
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
        GameStatus::PendingReplay => {
            let splash = SplashUI::new(if let Some(error) = &outer.error {
                vec![error.clone()]
            } else {
                vec!["LOADING REPLAY".to_string()]
            })
            .animated(outer.error.is_none())
            .with_button("cancel", "CANCEL".to_string(), outer.theme.button_primary);

            let resp = splash.render(ui, &outer.theme, current_time, &outer.map_texture);

            if resp.clicked == Some("cancel") {
                back_to_menu();
            }
        }
        GameStatus::Replay(replay) => {
            replay.render(ui, &outer.theme, current_time, &outer.backchannel);
        }
        GameStatus::HardError(msg) => {
            let splash = SplashUI::new(msg.clone()).with_button(
                "reload",
                "RELOAD".to_string(),
                outer.theme.button_primary,
            );

            let resp = splash.render(ui, &outer.theme, current_time, &outer.map_texture);
            if matches!(resp.clicked, Some("reload")) {
                // TODO: Neatly kick back to the wrapper page without a reload
                #[cfg(target_arch = "wasm32")]
                {
                    _ = web_sys::window().unwrap().location().set_hash("");
                    _ = web_sys::window().unwrap().location().reload();
                }
            }
        }
    }
    if let Some(new_game_status) = new_game_status {
        outer.game_status = new_game_status;
    }
}
