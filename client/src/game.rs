use eframe::{
    egui,
    epaint::{Color32, Stroke},
};

use super::GameClient;
use core::{
    board::{Board, Coordinate, Square},
    messages::{GameMessage, PlayerMessage},
};

type RoomCode = String;

#[derive(Debug)]
pub enum GameStatus {
    None(RoomCode),
    PendingJoin(RoomCode),
    PendingCreate,
    PendingStart(RoomCode),
    Active(RoomCode, Board),
    Concluded(RoomCode),
}

pub fn render(client: &mut GameClient, ui: &mut egui::Ui) {
    let GameClient {
        name,
        game_status,
        rx_game,
        tx_player,
    } = client;

    ui.label("Truncate");

    if matches!(game_status, GameStatus::None(_)) {
        ui.horizontal(|ui| {
            ui.label("Name: ");
            ui.text_edit_singleline(name);
        });
    } else {
        ui.label(format!("Playing as {name}"));
    }

    // TODO: Option to join an existing game

    let mut new_game_status = None;
    match game_status {
        GameStatus::None(room_code) => {
            if ui.button("New Game").clicked() {
                // TODO: Send player name in NewGame message
                tx_player.send(PlayerMessage::NewGame).unwrap();
                new_game_status = Some(GameStatus::PendingCreate);
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(room_code);
                if ui.button("Join Game").clicked() {
                    tx_player
                        .send(PlayerMessage::JoinGame(room_code.clone()))
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
        GameStatus::PendingStart(game_id) => {
            // TODO: Make this state exist
            ui.label(format!("Playing in game {game_id}"));
            ui.label("Waiting for the game to start . . .");
            if ui.button("Start game").clicked() {
                tx_player.send(PlayerMessage::StartGame).unwrap();
            }
        }
        GameStatus::Active(game_id, board) => {
            // TODO: All actual board/game state
            ui.label(format!("Playing in game {game_id}"));
            if ui.button("Play a move").clicked() {
                tx_player
                    .send(PlayerMessage::Place(Coordinate::new(5, 5), 'a'))
                    .unwrap();
            }
            render_board(board, ui);
        }
        GameStatus::Concluded(game_id) => {
            ui.label(format!("Game {game_id} has concluded"));
            // TODO: Reset state and play again
        }
    }
    if let Some(new_game_status) = new_game_status {
        *game_status = new_game_status;
    }

    while let Ok(msg) = rx_game.try_recv() {
        match msg {
            GameMessage::JoinedGame(id) => {
                *game_status = GameStatus::PendingStart(id.to_uppercase())
            }
            GameMessage::StartedGame(id, board, hand) => {
                *game_status = GameStatus::Active(id.to_uppercase(), board);
                println!("Starting a game")
            }
        }
    }
}

fn render_board(board: &Board, ui: &mut egui::Ui) {
    ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);
    for row in &board.squares {
        ui.horizontal(|ui| {
            for square in row {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                if ui.is_rect_visible(rect) {
                    match square {
                        Some(Square::Empty) => {
                            ui.painter()
                                .rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GOLD));
                        }
                        Some(Square::Occupied(player, char)) => {
                            ui.painter()
                                .rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GOLD));
                        }
                        None => {}
                    };
                    if square.is_some() && response.hovered() {
                        ui.painter().rect_filled(rect, 0.0, Color32::LIGHT_YELLOW);
                    }
                }
            }
        });
    }
}
