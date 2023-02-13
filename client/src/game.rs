use eframe::{
    egui,
    emath::{Align2, Rot2},
    epaint::{Color32, Rect, Stroke, TextShape, Vec2},
};

use std::f32;

use super::GameClient;
use core::{
    board::{Board, Coordinate, Square},
    hand::Hand,
    messages::{GameMessage, PlayerMessage},
};

type RoomCode = String;

#[derive(Debug)]
pub enum GameStatus {
    None(RoomCode),
    PendingJoin(RoomCode),
    PendingCreate,
    PendingStart(RoomCode),
    Active(RoomCode, Board, Hand),
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
        GameStatus::Active(game_id, board, hand) => {
            // TODO: All actual board/game state
            ui.label(format!("Playing in game {game_id}"));
            if ui.button("Play a move").clicked() {
                tx_player
                    .send(PlayerMessage::Place(Coordinate::new(5, 5), 'a'))
                    .unwrap();
            }
            render_board(board, ui);
            render_hand(hand, ui);
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
                *game_status = GameStatus::Active(id.to_uppercase(), board, hand);
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

fn render_hand(hand: &Hand, ui: &mut egui::Ui) {
    ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);
    ui.separator();
    ui.horizontal(|ui| {
        for char in hand {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
            if ui.is_rect_visible(rect) {
                ui.painter()
                    .rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GOLD));
                if response.hovered() {
                    ui.painter().rect_filled(rect, 0.0, Color32::LIGHT_YELLOW);
                }
                render_char(char, false, rect, ui);
            }
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        for char in hand {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
            if ui.is_rect_visible(rect) {
                ui.painter()
                    .rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GOLD));
                if response.hovered() {
                    ui.painter().rect_filled(rect, 0.0, Color32::LIGHT_YELLOW);
                }
                render_char(char, true, rect, ui);
            }
        }
    });
}

fn render_char(char: &char, inverted: bool, rect: Rect, ui: &mut egui::Ui) {
    let angle = if inverted { f32::consts::PI } else { 0.0 };
    let pos = if inverted {
        rect.right_bottom()
    } else {
        rect.left_top()
    };

    let galley = ui.painter().layout_no_wrap(
        char.to_uppercase().to_string(),
        egui::FontId::new(20.0, egui::FontFamily::Name("Tile".into())),
        Color32::LIGHT_GREEN,
    );

    let shift = Vec2::new(
        (rect.width() - galley.size().x) / if inverted { -2.0 } else { 2.0 },
        if inverted { 4.0 } else { -4.0 }, // TODO: Fix magic number for font alignment
    );

    ui.painter().add(TextShape {
        angle,
        override_text_color: Some(Color32::LIGHT_GREEN),
        ..TextShape::new(pos + shift, galley)
    });
}
