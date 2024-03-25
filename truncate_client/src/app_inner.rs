use eframe::egui;
use epaint::vec2;
use instant::Duration;
use truncate_core::{
    messages::{RoomCode, TruncateToken},
    npc::scoring::NPCPersonality,
};

use crate::{
    handle_launch_code::handle_launch_code,
    handle_messages::handle_server_msg,
    lil_bits::SplashUI,
    regions::{
        active_game::{ActiveGame, HeaderType},
        generator::GeneratorState,
        lobby::Lobby,
        native_menu::render_native_menu_if_required,
        replayer::ReplayerState,
        single_player::SinglePlayerState,
        tutorial::TutorialState,
    },
    utils::{urls::back_to_menu},
};

use super::OuterApplication;
use truncate_core::messages::PlayerMessage;

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

pub fn render(outer: &mut OuterApplication, ui: &mut egui::Ui, current_time: Duration) {
    handle_server_msg(outer, ui);

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

    // Block all further actions until we have a login token from the server,
    // or until the player accepts to play offline.
    // TODO: Daily puzzle is still inaccessible offline, thus needs a similar check.
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
    if let Some(launched_code) = outer.launched_code.take() {
        new_game_status = handle_launch_code(&launched_code, outer, ui);
    }

    render_native_menu_if_required(outer, ui);

    let mut send = |msg| {
        outer.tx_player.try_send(msg).unwrap();
    };

    match &mut outer.game_status {
        GameStatus::None(_, _) => { /* handled above */ }
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
                back_to_menu();
            }
        }
    }
    if let Some(new_game_status) = new_game_status {
        outer.game_status = new_game_status;
    }
}
