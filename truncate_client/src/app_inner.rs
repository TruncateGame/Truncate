use eframe::egui;
use epaint::vec2;
use instant::Duration;
use truncate_core::{
    messages::{RoomCode, TruncateToken},
    npc::scoring::NPCPersonality,
    rules::GameRules,
};

use crate::{
    handle_launch_code::handle_launch_code,
    handle_messages::handle_server_msg,
    lil_bits::{ChangelogSplashUI, SplashUI},
    regions::{
        active_game::{ActiveGame, HeaderType},
        generator::GeneratorState,
        lobby::Lobby,
        native_menu::render_native_menu_if_required,
        replayer::ReplayerState,
        single_player::SinglePlayerState,
        tutorial::TutorialState,
    },
    utils::{
        includes::{changelogs, ChangePriority, Tutorial},
        urls::back_to_menu,
    },
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

#[derive(Default)]
pub struct AppInnerStorage {
    pub changelog_ui: Option<ChangelogSplashUI>,
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
                14.0,
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
    let loading_changelog = outer
        .launched_code
        .as_ref()
        .is_some_and(|c| c == "CHANGE_LOG");

    if !outer.unread_changelogs.is_empty() && !loading_changelog {
        let all_changelogs = changelogs();

        // TODO: handle showing multiple changelogs

        for unread in outer.unread_changelogs.clone() {
            let Some(tutorial) = all_changelogs.get(unread.as_str()) else {
                continue;
            };
            let Some(splash_message) = &tutorial.splash_message else {
                continue;
            };

            if tutorial.effective_day > outer.launched_at_day {
                continue;
            }

            if tutorial.priority == Some(ChangePriority::High) {
                outer.event_dispatcher.event(format!("interrupt_{unread}"));
                let changelog_ui = outer.inner_storage.changelog_ui.get_or_insert_with(|| {
                    ChangelogSplashUI::new(splash_message.clone(), current_time)
                        .with_button(
                            "view",
                            "VIEW SCENARIO".to_string(),
                            outer.theme.button_primary,
                        )
                        .with_button(
                            "skip",
                            "REMIND ME LATER".to_string(),
                            outer.theme.button_primary,
                        )
                        .with_button(
                            "ignore",
                            "IGNORE FOREVER".to_string(),
                            outer.theme.button_scary,
                        )
                });

                let resp = changelog_ui.render(ui, &outer.theme, current_time, &outer.map_texture);

                if resp.clicked == Some("view") {
                    outer
                        .event_dispatcher
                        .event(format!("interrupt_view_{unread}"));
                    outer
                        .tx_player
                        .try_send(PlayerMessage::MarkChangelogRead(unread.clone()))
                        .unwrap();
                    new_game_status = Some(GameStatus::Tutorial(TutorialState::new(
                        unread.clone(),
                        tutorial.clone(),
                        ui.ctx(),
                        outer.map_texture.clone(),
                        &outer.theme,
                        outer.event_dispatcher.clone(),
                    )));

                    outer.launched_code = None;
                    outer.unread_changelogs = vec![];

                    break;
                }

                if resp.clicked == Some("skip") {
                    outer
                        .event_dispatcher
                        .event(format!("interrupt_skip_{unread}"));
                    outer.unread_changelogs = vec![];
                    break;
                }

                if resp.clicked == Some("ignore") {
                    outer
                        .event_dispatcher
                        .event(format!("interrupt_ignore_{unread}"));
                    outer.unread_changelogs = vec![];
                    outer
                        .tx_player
                        .try_send(PlayerMessage::MarkChangelogRead(unread.clone()))
                        .unwrap();
                    break;
                }

                return;
            }
        }
    }

    if loading_changelog {
        outer.event_dispatcher.event(format!("updates_listing"));
        let mut changelog_ui = SplashUI::new(vec!["Latest updates".to_string()]);

        let mut ordered_changelogs = changelogs()
            .into_iter()
            .filter(|(_, log)| log.effective_day <= outer.launched_at_day)
            .collect::<Vec<_>>();
        ordered_changelogs.sort_by_key(|(_, log)| log.effective_day);

        for (changelog_id, changelog_tut) in ordered_changelogs {
            changelog_ui = changelog_ui.with_button(
                changelog_id,
                changelog_tut
                    .changelog_name
                    .unwrap_or_else(|| "Update".to_string()),
                outer.theme.button_primary,
                11.0,
            )
        }

        let resp = changelog_ui.render(ui, &outer.theme, current_time, &outer.map_texture);

        if let Some(requested_changelog) = resp.clicked {
            let changelog_tut = changelogs().get(requested_changelog).unwrap().clone();

            outer
                .event_dispatcher
                .event(format!("updates_view_{requested_changelog}"));
            outer
                .tx_player
                .try_send(PlayerMessage::MarkChangelogRead(
                    requested_changelog.to_string(),
                ))
                .unwrap();
            new_game_status = Some(GameStatus::Tutorial(TutorialState::new(
                requested_changelog.to_string(),
                changelog_tut,
                ui.ctx(),
                outer.map_texture.clone(),
                &outer.theme,
                outer.event_dispatcher.clone(),
            )));

            outer.launched_code = None;
            outer.unread_changelogs = vec![];
        } else {
            return;
        }
    }

    if let Some(launched_code) = outer.launched_code.take() {
        new_game_status = handle_launch_code(&launched_code, outer, ui);
    }

    if new_game_status.is_none() {
        new_game_status = render_native_menu_if_required(outer, ui);
    }

    let mut send = |msg| {
        outer.tx_player.try_send(msg).unwrap();
    };

    match &mut outer.game_status {
        GameStatus::None(_, _) => { /* handled above */ }
        GameStatus::Generator(generator) => {
            generator.render(ui, &outer.theme, current_time);
        }
        GameStatus::Tutorial(tutorial) => {
            for msg in tutorial.render(ui, outer.map_texture.clone(), &outer.theme, current_time) {
                send(msg);
            }
        }
        GameStatus::PendingSinglePlayer(editor_state) => {
            if let Some(msg) = editor_state.render(ui, &outer.theme) {
                match msg {
                    PlayerMessage::StartGame => {
                        let rules_generation = GameRules::latest(Some(outer.launched_at_day)).0;
                        let single_player_game = SinglePlayerState::new(
                            "classic".to_string(),
                            ui.ctx(),
                            outer.map_texture.clone(),
                            outer.theme.clone(),
                            editor_state.board.clone(),
                            editor_state.board_seed.clone(),
                            rules_generation,
                            true,
                            HeaderType::Timers,
                            NPCPersonality::jet(),
                            outer.event_dispatcher.clone(),
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
            .with_button(
                "cancel",
                "CANCEL".to_string(),
                outer.theme.button_primary,
                14.0,
            );

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
            .with_button(
                "cancel",
                "CANCEL".to_string(),
                outer.theme.button_primary,
                14.0,
            );

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
            .with_button(
                "cancel",
                "CANCEL".to_string(),
                outer.theme.button_primary,
                14.0,
            );

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
            .with_button(
                "cancel",
                "CANCEL".to_string(),
                outer.theme.button_primary,
                14.0,
            );

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
                14.0,
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
