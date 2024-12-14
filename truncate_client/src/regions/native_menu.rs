use eframe::egui;

use truncate_core::{
    board::Board, generation::BoardSeed, messages::LobbyPlayerMessage,
    npc::scoring::NPCPersonality, rules::GameRules,
};

use crate::{
    app_inner::GameStatus,
    app_outer::OuterApplication,
    regions::{
        active_game::HeaderType, generator::GeneratorState, lobby::Lobby,
        single_player::SinglePlayerState, tutorial::TutorialState,
    },
    utils,
};

use truncate_core::messages::PlayerMessage;

pub fn render_native_menu_if_required(
    outer: &mut OuterApplication,
    ui: &mut egui::Ui,
) -> Option<GameStatus> {
    match &mut outer.game_status {
        GameStatus::None(room_code, token) => {
            let mut send_to_server = |msg| {
                outer.tx_player.try_send(msg).unwrap();
            };

            if ui.button("Generator").clicked() {
                return Some(GameStatus::Generator(GeneratorState::new(
                    ui.ctx(),
                    outer.map_texture.clone(),
                    outer.theme.clone(),
                    outer.launched_at_day,
                )));
            }
            if ui.button("Tutorial: Rules").clicked() {
                return Some(GameStatus::Tutorial(TutorialState::new(
                    "rules".to_string(),
                    utils::includes::tutorial(outer.launched_at_day),
                    ui.ctx(),
                    outer.map_texture.clone(),
                    &outer.theme,
                    outer.event_dispatcher.clone(),
                )));
            }
            if ui.button("Single Player").clicked() {
                let mut board = Board::new(9, 9);
                board.grow();
                return Some(GameStatus::PendingSinglePlayer(Lobby::new(
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
                    Board::from_string(include_str!("../../tutorials/test_board.txt"));
                let seed_for_hand_tiles = BoardSeed::new_with_generation(0, 1);
                let rules_generation = GameRules::latest(Some(outer.launched_at_day)).0;
                let behemoth_game = SinglePlayerState::new(
                    "behemoth".to_string(),
                    ui.ctx(),
                    outer.map_texture.clone(),
                    outer.theme.clone(),
                    behemoth_board,
                    Some(seed_for_hand_tiles),
                    rules_generation,
                    true,
                    HeaderType::Timers,
                    NPCPersonality::jet(),
                    outer.event_dispatcher.clone(),
                );
                outer.log_frames = true;
                return Some(GameStatus::SinglePlayer(behemoth_game));
            }
            if ui.button("New Game").clicked() {
                send_to_server(PlayerMessage::NewGame {
                    player_name: outer.name.clone(),
                    effective_day: outer.launched_at_day,
                });
                return Some(GameStatus::PendingCreate);
            }
            ui.text_edit_singleline(room_code);
            if ui.button("Join Game").clicked() {
                send_to_server(PlayerMessage::JoinGame(
                    room_code.clone(),
                    outer.name.clone(),
                    token.clone(),
                ));
                return Some(GameStatus::PendingJoin(room_code.clone()));
            }
            if let Some(existing_token) = token {
                ui.label("Existing game found, would you like to rejoin?");
                if ui.button("Rejoin").clicked() {
                    send_to_server(PlayerMessage::RejoinGame(existing_token.to_string()));
                    return Some(GameStatus::PendingJoin("...".into()));
                }
            }
            None
        }
        _ => None,
    }
}
