use eframe::egui;

use super::{GameClient, GameStatus};
use core::{
    board::Coordinate,
    messages::{GameMessage, PlayerMessage},
};

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
        }
        GameStatus::Active(game_id) => {
            // TODO: All actual board/game state
            ui.label(format!("Playing in game {game_id}"));
            if ui.button("Play a move").clicked() {
                tx_player
                    .send(PlayerMessage::Place(Coordinate::new(5, 5), 'a'))
                    .unwrap();
            }
        }
        GameStatus::Concluded(game_id) => {
            ui.label(format!("Game {game_id} has concluded"));
            // TODO: Reset state and play again
        }
    }

    while let Ok(msg) = rx_game.try_recv() {
        match msg {
            GameMessage::JoinedGame(id) => *game_status = GameStatus::Active(id),
        }
    }
}
