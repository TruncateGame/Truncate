use core::{board::Board, messages::RoomCode, player::Hand};
use eframe::egui;
use hashbrown::HashMap;

use crate::{
    active_game::ActiveGame,
    lil_bits::{BoardUI, EditorUI},
};

use super::GameClient;
use core::{
    messages::{GameMessage, GameStateMessage, PlayerMessage},
    reporting::Change,
};

#[derive(Debug)]
pub enum GameStatus {
    None(RoomCode),
    PendingJoin(RoomCode),
    PendingCreate,
    PendingStart(RoomCode, Vec<String>, Board),
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
    } = client;

    if matches!(game_status, GameStatus::None(_)) {
        ui.horizontal(|ui| {
            ui.label("Name: ");
            ui.text_edit_singleline(name);
        });
    } else {
        ui.label(format!("Playing as {name}"));
    }

    ui.separator();

    let mut new_game_status = None;
    match game_status {
        GameStatus::None(room_code) => {
            if ui.button("New Game").clicked() {
                // TODO: Send player name in NewGame message
                tx_player
                    .send(PlayerMessage::NewGame(name.clone()))
                    .unwrap();
                new_game_status = Some(GameStatus::PendingCreate);
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(room_code);
                if ui.button("Join Game").clicked() {
                    tx_player
                        .send(PlayerMessage::JoinGame(room_code.clone(), name.clone()))
                        .unwrap();
                    new_game_status = Some(GameStatus::PendingJoin(room_code.clone()));
                }
            });
        }
        GameStatus::PendingJoin(room_code) => {
            ui.label(format!("Waiting to join room {room_code}"));
        }
        GameStatus::PendingCreate => {
            ui.label("Waiting for a new game to be created . . .");
        }
        GameStatus::PendingStart(game_id, players, board) => {
            ui.label(format!("Playing in game {game_id}"));
            ui.label(format!("In lobby: {}", players.join(", ")));
            ui.label("Waiting for the game to start . . .");
            if ui.button("Start game").clicked() {
                tx_player.send(PlayerMessage::StartGame).unwrap();
            }
            // TODO: Make a different board ui for the level editor
            if let Some(msg) = EditorUI::new(board).render(true, ui, theme) {
                tx_player.send(msg).unwrap();
            }
        }
        GameStatus::Active(game) => {
            if let Some(msg) = game.render(ui, theme) {
                tx_player.send(msg).unwrap();
            }
        }
        GameStatus::Concluded(game, winner) => {
            ui.label(format!("Game {} has concluded", game.room_code));
            ui.label(format!("Player {winner} won"));
            // render_board(game, ui);
            // TODO: Reset state and play again
        }
    }
    if let Some(new_game_status) = new_game_status {
        *game_status = new_game_status;
    }

    while let Ok(msg) = rx_game.try_recv() {
        match msg {
            GameMessage::JoinedLobby(id, players, board) => {
                *game_status = GameStatus::PendingStart(id.to_uppercase(), players, board)
            }
            GameMessage::LobbyUpdate(id, players, board) => {
                *game_status = GameStatus::PendingStart(id.to_uppercase(), players, board)
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
                        game.next_player_number = next_player_number;

                        game.board_changes.clear();
                        for board_change in changes.iter().filter_map(|c| match c {
                            Change::Board(change) => Some(change),
                            Change::Hand(_) => None,
                        }) {
                            game.board_changes
                                .insert(board_change.detail.coordinate, board_change.clone());
                        }

                        for hand_change in changes.iter().filter_map(|c| match c {
                            Change::Board(_) => None,
                            Change::Hand(change) => Some(change),
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

                        // TODO: Verify that our modified hand matches the actual hand in GameStateMessage

                        game.playing_tile = None;
                        game.error_msg = None;
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
                    // assert_eq!(game.room_code, id);
                    // assert_eq!(game.player_number, num);
                    game.players = players;
                    game.board = board;
                    game.board_changes.clear();
                    for board_change in changes.iter().filter_map(|c| match c {
                        Change::Board(change) => Some(change),
                        Change::Hand(_) => None,
                    }) {
                        game.board_changes
                            .insert(board_change.detail.coordinate, board_change.clone());
                    }
                    *game_status = GameStatus::Concluded(game.clone(), winner);
                }
                _ => todo!("Game error hit an unknown state"),
            },
            GameMessage::GameError(_id, _num, err) => match game_status {
                GameStatus::Active(game) => {
                    // assert_eq!(game.room_code, id);
                    // assert_eq!(game.player_number, num);
                    game.error_msg = Some(err);
                }
                _ => todo!("Game error hit an unknown state"),
            },
        }
    }
}
