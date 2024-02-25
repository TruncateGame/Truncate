use std::collections::HashMap;

use eframe::egui::{self, Layout, Order};
use epaint::{vec2, Color32, TextureHandle};
use instant::Duration;
use serde::Deserialize;
use truncate_core::{
    bag::TileBag,
    board::{Board, Coordinate},
    game::Game,
    judge::Judge,
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage},
    moves::Move,
    player::{Hand, Player},
    rules::{GameRules, TileDistribution},
};

use crate::utils::{text::TextHelper, Diaphanize, Lighten, Theme};

use super::active_game::{ActiveGame, GameLocation, HeaderType};

const TUTORIAL_01: &[u8] = include_bytes!("../../tutorials/tutorial_01.yml");

#[derive(Deserialize, Debug)]
struct Tutorial {
    board: String,
    player_hand: String,
    computer_hand: String,
    dict: HashMap<String, String>,
    steps: Vec<TutorialStep>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum TutorialStep {
    OwnMove {
        you: String,
        gets: char,
        description: String,
    },
    ComputerMove {
        computer: String,
        gets: char,
        description: String,
    },
    Dialog {
        message: String,
    },
    EndAction {
        end_message: String,
    },
}

fn pos_to_coord(pos: &str) -> Option<Coordinate> {
    let (x, y) = pos.split_once(',')?;

    let x = x.parse::<usize>().ok()?;
    let y = y.parse::<usize>().ok()?;

    Some(Coordinate { x, y })
}

fn action_to_move(player: usize, action: &str) -> Move {
    let (from, to) = action
        .split_once(" -> ")
        .expect("Actions should be separated by ' -> '");
    let to_pos = pos_to_coord(to).expect("Coordinates should be separated by ','");
    if let Some(from_pos) = pos_to_coord(from) {
        Move::Swap {
            player,
            positions: [from_pos, to_pos],
        }
    } else if from.len() == 1 {
        Move::Place {
            player,
            tile: from.chars().next().unwrap(),
            position: to_pos,
        }
    } else {
        panic!("Couldn't parse tutorial action");
    }
}

impl PartialEq<Move> for TutorialStep {
    fn eq(&self, msg: &Move) -> bool {
        match self {
            TutorialStep::OwnMove { you, .. } => {
                return &action_to_move(0, you) == msg;
            }
            TutorialStep::ComputerMove { .. } => false,
            TutorialStep::Dialog { .. } => false,
            TutorialStep::EndAction { .. } => false,
        }
    }
}

pub struct TutorialState {
    game: Game,
    pub active_game: ActiveGame,
    stage: usize,
    stage_changed_at: Duration,
    tutorial: Tutorial,
}

impl TutorialState {
    pub fn new(ctx: &egui::Context, map_texture: TextureHandle, theme: Theme) -> Self {
        let loaded_tutorial: Tutorial =
            serde_yaml::from_slice(TUTORIAL_01).expect("Tutorial should match Tutorial format");

        let now = Some(
            instant::SystemTime::now()
                .duration_since(instant::SystemTime::UNIX_EPOCH)
                .expect("Please don't play Truncate earlier than 1970")
                .as_secs(),
        );

        let game = Game {
            rules: GameRules::default(),
            players: vec![
                Player {
                    name: "You".into(),
                    index: 0,
                    hand: Hand(loaded_tutorial.player_hand.chars().collect()),
                    hand_capacity: loaded_tutorial.player_hand.len(),
                    allotted_time: None,
                    time_remaining: None,
                    turn_starts_no_later_than: now,
                    turn_starts_no_sooner_than: now,
                    swap_count: 0,
                    penalties_incurred: 0,
                    color: (128, 128, 255),
                },
                Player {
                    name: "Computer".into(),
                    index: 1,
                    hand: Hand(loaded_tutorial.computer_hand.chars().collect()),
                    hand_capacity: loaded_tutorial.computer_hand.len(),
                    allotted_time: None,
                    time_remaining: None,
                    turn_starts_no_later_than: None,
                    turn_starts_no_sooner_than: None,
                    swap_count: 0,
                    penalties_incurred: 0,
                    color: (255, 80, 80),
                },
            ],
            board: Board::from_string(loaded_tutorial.board.clone()),
            // TODO: Use some special infinite bag?
            bag: TileBag::new(&TileDistribution::Standard, None),
            judge: Judge::new(loaded_tutorial.dict.keys().cloned().collect()),
            battle_count: 0,
            turn_count: 0,
            player_turn_count: vec![0, 0],
            recent_changes: vec![],
            started_at: None,
            game_ends_at: None,
            next_player: Some(0),
            winner: None,
        };

        let mut active_game = ActiveGame::new(
            ctx,
            "TUTORIAL_01".into(),
            None,
            game.players
                .iter()
                .map(|p| GamePlayerMessage::new(p, &game))
                .collect(),
            0,
            Some(0),
            game.board.clone(),
            game.players[0].hand.clone(),
            map_texture,
            theme,
            GameLocation::Local,
            None,
        );
        active_game.depot.ui_state.game_header = HeaderType::None;

        Self {
            game,
            active_game,
            stage: 0,
            stage_changed_at: Duration::from_secs(0),
            tutorial: loaded_tutorial,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme, current_time: Duration) {
        if self.stage_changed_at.is_zero() {
            self.stage_changed_at = current_time;
        }

        let current_step = self.tutorial.steps.get(self.stage);
        let mut next_move = None;

        // Start the game after any leading dialogs
        if self.game.started_at.is_none()
            && matches!(
                current_step,
                Some(TutorialStep::OwnMove {
                    you: _,
                    gets: _,
                    description: _
                })
            )
        {
            self.game.start();
        }

        if let Some(TutorialStep::OwnMove { you, .. }) = current_step {
            let m = action_to_move(0, you);
            match m {
                Move::Place { tile, position, .. } => {
                    self.active_game.depot.interactions.highlight_tiles = Some(vec![tile]);
                    self.active_game.depot.interactions.highlight_squares = Some(vec![position]);
                }
                Move::Swap { positions, .. } => {
                    self.active_game.depot.interactions.highlight_squares =
                        Some(positions.to_vec());
                }
            }
        } else {
            self.active_game.depot.interactions.highlight_tiles = None;
            self.active_game.depot.interactions.highlight_squares = None;
        }

        // Standard game helper
        if let Some(msg) = self.active_game.render(ui, current_time, None) {
            let Some(game_move) = (match msg {
                PlayerMessage::Place(position, tile) => Some(Move::Place {
                    player: 0,
                    tile,
                    position,
                }),
                PlayerMessage::Swap(from, to) => Some(Move::Swap {
                    player: 0,
                    positions: [from, to],
                }),
                _ => None,
            }) else {
                return;
            };

            let Some(step) = current_step else { return };

            if step == &game_move {
                next_move = Some(game_move);
            } else {
                // TODO: Handle player doing the wrong tutorial thing
                println!("Expected {msg} to be {:?}", step);
            }
        }

        if let Some(dialog_pos) = self.active_game.depot.regions.hand_companion_rect {
            let max_width = f32::min(700.0, dialog_pos.width());
            let dialog_padding_x = (dialog_pos.width() - max_width) / 2.0;

            let inner_dialog = dialog_pos.shrink2(vec2(dialog_padding_x, 8.0));

            let area = egui::Area::new(egui::Id::new("tutorial_layer"))
                .movable(false)
                .order(Order::Foreground)
                .fixed_pos(dialog_pos.left_top());

            area.show(ui.ctx(), |ui| {
                ui.painter().rect_filled(
                    dialog_pos,
                    4.0,
                    self.active_game
                        .depot
                        .aesthetics
                        .theme
                        .water
                        .gamma_multiply(0.75),
                );
                ui.expand_to_include_rect(dialog_pos);
                ui.allocate_ui_at_rect(inner_dialog, |ui| {
                    ui.expand_to_include_rect(inner_dialog);

                    // TODO one day — put this in a theme
                    let tut_fz = if inner_dialog.width() < 550.0 {
                        24.0
                    } else {
                        32.0
                    };

                    let button_spacing = 60.0;
                    let time_in_stage = (current_time - self.stage_changed_at).as_secs_f32();

                    match current_step {
                        Some(step) => match step {
                            TutorialStep::OwnMove { description, .. } => {
                                let dialog_text = TextHelper::light(
                                    &description,
                                    tut_fz,
                                    Some((ui.available_width() - 16.0).max(0.0)),
                                    ui,
                                );

                                let animated_text =
                                    dialog_text.get_partial_slice(time_in_stage, ui);
                                if animated_text.is_some() {
                                    ui.ctx().request_repaint();
                                }

                                let final_size = dialog_text.mesh_size();
                                animated_text.unwrap_or(dialog_text).dialog(
                                    final_size,
                                    Color32::WHITE.diaphanize(),
                                    Color32::BLACK,
                                    0.0,
                                    &self.active_game.depot.aesthetics.map_texture,
                                    ui,
                                );
                            }
                            TutorialStep::ComputerMove {
                                computer: action,
                                description,
                                ..
                            } => {
                                let dialog_text = TextHelper::light(
                                    &description,
                                    tut_fz,
                                    Some((ui.available_width() - 16.0).max(0.0)),
                                    ui,
                                );

                                let animated_text =
                                    dialog_text.get_partial_slice(time_in_stage, ui);
                                let has_animation = animated_text.is_some();
                                if has_animation {
                                    ui.ctx().request_repaint();
                                }

                                let final_size = dialog_text.mesh_size();
                                let dialog_resp = animated_text.unwrap_or(dialog_text).dialog(
                                    final_size,
                                    Color32::WHITE.diaphanize(),
                                    Color32::BLACK,
                                    button_spacing,
                                    &self.active_game.depot.aesthetics.map_texture,
                                    ui,
                                );

                                if !has_animation {
                                    let mut dialog_rect = dialog_resp.rect;
                                    dialog_rect.set_top(dialog_rect.bottom() - button_spacing);

                                    let text = TextHelper::heavy("NEXT", 14.0, None, ui);
                                    ui.allocate_ui_at_rect(dialog_rect, |ui| {
                                        ui.with_layout(
                                            Layout::centered_and_justified(
                                                egui::Direction::LeftToRight,
                                            ),
                                            |ui| {
                                                if text
                                                    .button(
                                                        theme.water.lighten(),
                                                        theme.text,
                                                        &self
                                                            .active_game
                                                            .depot
                                                            .aesthetics
                                                            .map_texture,
                                                        ui,
                                                    )
                                                    .clicked()
                                                {
                                                    next_move = Some(action_to_move(1, action));
                                                }
                                            },
                                        );
                                    });
                                }
                            }
                            TutorialStep::Dialog { message } => {
                                let dialog_text = TextHelper::light(
                                    &message,
                                    tut_fz,
                                    Some((ui.available_width() - 16.0).max(0.0)),
                                    ui,
                                );

                                let animated_text =
                                    dialog_text.get_partial_slice(time_in_stage, ui);
                                let has_animation = animated_text.is_some();
                                if has_animation {
                                    ui.ctx().request_repaint();
                                }

                                let final_size = dialog_text.mesh_size();
                                let dialog_resp = animated_text.unwrap_or(dialog_text).dialog(
                                    final_size,
                                    Color32::WHITE.diaphanize(),
                                    Color32::BLACK,
                                    button_spacing,
                                    &self.active_game.depot.aesthetics.map_texture,
                                    ui,
                                );

                                if !has_animation {
                                    let mut dialog_rect = dialog_resp.rect;
                                    dialog_rect.set_top(dialog_rect.bottom() - button_spacing);

                                    let text = TextHelper::heavy("NEXT", 14.0, None, ui);
                                    ui.allocate_ui_at_rect(dialog_rect, |ui| {
                                        ui.with_layout(
                                            Layout::centered_and_justified(
                                                egui::Direction::LeftToRight,
                                            ),
                                            |ui| {
                                                if text
                                                    .button(
                                                        theme.water.lighten(),
                                                        theme.text,
                                                        &self
                                                            .active_game
                                                            .depot
                                                            .aesthetics
                                                            .map_texture,
                                                        ui,
                                                    )
                                                    .clicked()
                                                {
                                                    self.stage += 1;
                                                    self.stage_changed_at = current_time;
                                                }
                                            },
                                        );
                                    });
                                }
                            }
                            TutorialStep::EndAction { end_message } => {
                                let dialog_text = TextHelper::light(
                                    &end_message,
                                    tut_fz,
                                    Some((ui.available_width() - 16.0).max(0.0)),
                                    ui,
                                );

                                let animated_text =
                                    dialog_text.get_partial_slice(time_in_stage, ui);
                                let has_animation = animated_text.is_some();
                                if has_animation {
                                    ui.ctx().request_repaint();
                                }

                                let final_size = dialog_text.mesh_size();
                                let dialog_resp = animated_text.unwrap_or(dialog_text).dialog(
                                    final_size,
                                    Color32::WHITE.diaphanize(),
                                    Color32::BLACK,
                                    button_spacing,
                                    &self.active_game.depot.aesthetics.map_texture,
                                    ui,
                                );

                                if !has_animation {
                                    let mut dialog_rect = dialog_resp.rect;
                                    dialog_rect.set_top(dialog_rect.bottom() - button_spacing);

                                    let text = TextHelper::heavy("RETURN TO MENU", 14.0, None, ui);
                                    ui.allocate_ui_at_rect(dialog_rect, |ui| {
                                        ui.with_layout(
                                            Layout::centered_and_justified(
                                                egui::Direction::LeftToRight,
                                            ),
                                            |ui| {
                                                if text
                                                    .button(
                                                        theme.water.lighten(),
                                                        theme.text,
                                                        &self
                                                            .active_game
                                                            .depot
                                                            .aesthetics
                                                            .map_texture,
                                                        ui,
                                                    )
                                                    .clicked()
                                                {
                                                    // TODO: A more elegant way to show the menu over the game would be nice,
                                                    // but we would need to add extra endpoints to the lib.rs file,
                                                    // and also give those endpoints a way to access the active game to change its state.
                                                    // As an MVP here, we simply reload the page to get back to the menu.
                                                    #[cfg(target_arch = "wasm32")]
                                                    {
                                                        _ = web_sys::window()
                                                            .unwrap()
                                                            .location()
                                                            .reload();
                                                    }
                                                }
                                            },
                                        );
                                    });
                                }
                            }
                        },
                        None => {
                            // TODO: Tutorial complete screen, back to menu
                        }
                    };
                });
            });
        }

        if let Some(game_move) = next_move {
            if let Some(next_tile) = match current_step {
                Some(TutorialStep::OwnMove { gets, .. }) => Some(gets),
                Some(TutorialStep::ComputerMove { gets, .. }) => Some(gets),
                _ => None,
            } {
                self.game.bag = TileBag::explicit(vec![*next_tile], None);
            }

            match self.game.make_move(game_move, None, None, None) {
                Ok(changes) => {
                    let changes = changes
                        .into_iter()
                        .filter(|change| match change {
                            truncate_core::reporting::Change::Board(_) => true,
                            truncate_core::reporting::Change::Hand(hand_change) => {
                                hand_change.player == 0
                            }
                            truncate_core::reporting::Change::Battle(_) => true,
                            truncate_core::reporting::Change::Time(_) => true,
                        })
                        .collect();
                    let room_code = self.active_game.depot.gameplay.room_code.clone();
                    let state_message = GameStateMessage {
                        room_code,
                        players: self
                            .game
                            .players
                            .iter()
                            .map(|p| GamePlayerMessage::new(p, &self.game))
                            .collect(),
                        player_number: 0,
                        next_player_number: self.game.next_player.map(|p| p as u64),
                        board: self.game.board.clone(),
                        hand: self.game.players[0].hand.clone(),
                        changes,
                        game_ends_at: None,
                    };
                    self.active_game.apply_new_state(state_message);
                    self.stage += 1;
                    self.stage_changed_at = current_time;
                }
                Err(msg) => {
                    println!("Failed to make a move: {msg}");
                }
            }
        }
    }
}
