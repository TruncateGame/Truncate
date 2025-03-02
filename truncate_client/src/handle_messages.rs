use eframe::egui;
use truncate_core::{
    game::{self, GAME_COLOR_BLUE, GAME_COLOR_RED},
    generation,
    rules::GameRules,
};

use crate::{
    app_inner::GameStatus,
    regions::{
        active_game::{ActiveGame, GameLocation, HeaderType},
        lobby::Lobby,
        replayer::ReplayerState,
    },
    utils::{
        daily::{get_playable_daily_puzzle, get_raw_daily_puzzle},
        game_evals::get_main_dict,
    },
};

use super::OuterApplication;
use truncate_core::messages::{GameMessage, GameStateMessage};

/// Main delegator for all messages from the server to the client,
/// both in-game and other.
pub fn handle_server_msg(outer: &mut OuterApplication, ui: &mut egui::Ui) {
    let mut recv = || match outer.rx_game.try_next() {
        Ok(Some(msg)) => Ok(msg),
        _ => Err(()),
    };

    while let Ok(msg) = recv() {
        match msg {
            GameMessage::Ping | GameMessage::Ack(_) | GameMessage::PleaseLogin => { /* handled at comms layer */
            }
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
                    _ = web_sys::window()
                        .unwrap()
                        .location()
                        .set_hash(id.to_uppercase().as_str());
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
                packed_move_sequence,
                board,
                hand,
                changes: _,
                game_ends_at,
                paused,
                remaining_turns,
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
                            packed_move_sequence,
                            board,
                            hand,
                            changes: vec![], // TODO: Try get latest changes on reconnect without dupes
                            game_ends_at,
                            paused,
                            remaining_turns,
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
                    game_ends_at,
                    remaining_turns,
                ));
            }
            GameMessage::GameUpdate(state_message) => match &mut outer.game_status {
                GameStatus::Active(game) => {
                    game.apply_new_state(state_message);
                }
                _ => {
                    outer.game_status = GameStatus::HardError(vec![
                        "Game hit unknown case".into(),
                        "Received game message".into(),
                        "while not in a game".into(),
                    ])
                }
            },
            GameMessage::GameTimingUpdate(state_message) => match &mut outer.game_status {
                GameStatus::Active(game) => {
                    game.apply_new_timing(state_message);
                }
                _ => {
                    outer.game_status = GameStatus::HardError(vec![
                        "Game hit unknown case".into(),
                        "Received game message".into(),
                        "while not in a game".into(),
                    ])
                }
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
                        game.depot.gameplay.winner = Some(winner as usize);
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
                    GameStatus::Tutorial(tut) => {
                        tut.load_definitions(definitions);
                    }
                    _ => { /* Soft unreachable */ }
                }
            }
            GameMessage::LoggedInAs {
                token: player_token,
                unread_changelogs,
            } => {
                #[cfg(target_arch = "wasm32")]
                {
                    let local_storage =
                        web_sys::window().unwrap().local_storage().unwrap().unwrap();
                    local_storage
                        .set_item("truncate_player_token", &player_token)
                        .unwrap();
                }

                outer.logged_in_as = Some(player_token);
                outer.unread_changelogs = unread_changelogs;
            }
            GameMessage::ResumeDailyPuzzle(latest_puzzle_state, best_puzzle) => {
                let mut puzzle_game = get_playable_daily_puzzle(
                    ui.ctx(),
                    latest_puzzle_state.puzzle_day,
                    &outer.map_texture,
                    &outer.theme,
                    &outer.backchannel,
                    outer.event_dispatcher.clone(),
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
                        .handle_move(next_move, &outer.backchannel, false)
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
                let rules_generation = info
                    .as_ref()
                    .map(|(_, i)| i.rules_generation)
                    .unwrap_or_else(|| GameRules::latest(Some(outer.launched_at_day)).0);

                let mut game = game::Game::new(
                    9,
                    9,
                    Some(seed.seed as u64),
                    GameRules::generation(rules_generation),
                );
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
