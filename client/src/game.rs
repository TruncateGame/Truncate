use eframe::{
    egui,
    epaint::{Color32, Stroke},
};

use uuid::Uuid;

use super::GameClient;
use core::{
    board::{Board, Coordinate, Square},
    messages::{GameMessage, PlayerMessage},
};

#[derive(Debug)]
pub enum GameStatus {
    None,
    PendingCreate,
    PendingStart(Uuid),
    Active(Uuid, Board),
    Concluded(Uuid),
}

pub fn render(client: &mut GameClient, ui: &mut egui::Ui) {
    let GameClient {
        name,
        game_status,
        rx_game,
        tx_player,
    } = client;

    ui.label("Truncate");

    if matches!(game_status, GameStatus::None) {
        ui.horizontal(|ui| {
            ui.label("Name: ");
            ui.text_edit_singleline(name);
        });
    } else {
        ui.label(format!("Playing as {name}"));
    }

    // TODO: Option to join an existing game

    match game_status {
        GameStatus::None => {
            if ui.button("New Game").clicked() {
                // TODO: Send player name in NewGame message
                tx_player.send(PlayerMessage::NewGame).unwrap();
                *game_status = GameStatus::PendingCreate;
            }
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

    while let Ok(msg) = rx_game.try_recv() {
        match msg {
            GameMessage::JoinedGame(id) => *game_status = GameStatus::PendingStart(id),
            GameMessage::StartedGame(id, board, hand) => {
                *game_status = GameStatus::Active(id, board);
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
