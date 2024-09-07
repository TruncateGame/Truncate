use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Align, Align2, CursorIcon, Layout, NumExt, Order, Sense};
use epaint::{vec2, Color32, Rect, TextureHandle, Vec2};
use instant::Duration;
use serde::Deserialize;
use truncate_core::{
    bag::TileBag,
    board::{Board, Coordinate},
    game::{Game, GAME_COLOR_BLUE, GAME_COLOR_RED},
    judge::Judge,
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage},
    moves::Move,
    player::{Hand, Player},
    reporting::WordMeaning,
    rules::GameRules,
};

use crate::{
    app_outer::EventDispatcher,
    utils::{
        game_evals::get_main_dict,
        includes::{Scenario, ScenarioStep, Tutorial},
        tex::{render_tex_quad, tiles},
        text::TextHelper,
        urls::back_to_menu,
        Diaphanize, Lighten, Theme,
    },
};

use super::active_game::{ActiveGame, GameLocation, HeaderType};

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

impl PartialEq<Move> for ScenarioStep {
    fn eq(&self, msg: &Move) -> bool {
        match self {
            ScenarioStep::OwnMove { you, .. } => {
                return &action_to_move(0, you) == msg;
            }
            ScenarioStep::ComputerMove { .. } => false,
            ScenarioStep::Dialog { .. } => false,
            ScenarioStep::EndAction { .. } => false,
        }
    }
}

enum ChangeStage {
    Next,
    Previous,
    None,
}

pub struct TutorialState {
    name: String,
    stage_index: usize,
    change_stage_next_frame: ChangeStage,
    stage: Option<TutorialStage>,
    stage_changed_at: Duration,
    tutorial: Tutorial,
    event_dispatcher: EventDispatcher,
}

struct TutorialStage {
    category: String,
    scenario: Scenario,
    game: Game,
    pub active_game: ActiveGame,
    step: usize,
}

impl TutorialStage {
    fn get_step(&self) -> Option<&ScenarioStep> {
        self.scenario.steps.get(self.step)
    }

    fn highlight_interactions(&mut self) {
        if let Some(ScenarioStep::OwnMove { you, .. }) = self.get_step() {
            let m = action_to_move(0, you);
            match m {
                Move::Place { tile, position, .. } => {
                    self.active_game.depot.interactions[0].highlight_tiles = Some(vec![tile]);
                    self.active_game.depot.interactions[0].highlight_squares = Some(vec![position]);
                }
                Move::Swap { positions, .. } => {
                    self.active_game.depot.interactions[0].highlight_squares =
                        Some(positions.to_vec());
                }
            }
        } else {
            self.active_game.depot.interactions[0].highlight_tiles = None;
            self.active_game.depot.interactions[0].highlight_squares = None;
        }
    }

    fn render_game(&mut self, ui: &mut egui::Ui, current_time: Duration) -> Option<PlayerMessage> {
        self.active_game
            .render(ui, current_time, None)
            .map(|m| m.message) // ignoring tutorial message player_ids
    }

    fn get_dialog_position(&self) -> Option<Rect> {
        self.active_game.depot.regions.hand_companion_rect
    }

    fn increment_step(&mut self) {
        self.step += 1;
    }

    fn handle_move(&mut self, next_move: Move) -> Result<(), ()> {
        if let Some(next_tile) = match self.get_step() {
            Some(ScenarioStep::OwnMove { gets, .. }) => Some(gets),
            Some(ScenarioStep::ComputerMove { gets, .. }) => Some(gets),
            _ => None,
        } {
            self.game.bag = TileBag::explicit(vec![*next_tile], None);
        }

        let dict_lock = get_main_dict();
        let dict = dict_lock.as_ref().unwrap();

        match self.game.play_turn(next_move, Some(dict), Some(dict), None) {
            Ok(possible_winner) => {
                let changes = self
                    .game
                    .recent_changes
                    .iter()
                    .cloned()
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
                    paused: false,
                    remaining_turns: None,
                };
                self.active_game.apply_new_state(state_message);
                self.active_game.depot.gameplay.winner = possible_winner;

                self.increment_step();
                Ok(())
            }
            Err(_msg) => {
                // TODO: Handle errored moves in tutorial gameplay
                Err(())
            }
        }
    }
}

impl TutorialState {
    pub fn new(
        name: String,
        tutorial: Tutorial,
        ctx: &egui::Context,
        map_texture: TextureHandle,
        theme: &Theme,
        mut event_dispatcher: EventDispatcher,
    ) -> Self {
        let stage_zero = TutorialState::get_stage(0, &tutorial, ctx, map_texture, &theme);

        event_dispatcher.event(format!("tutorial_{name}"));

        Self {
            name,
            stage_index: 0,
            change_stage_next_frame: ChangeStage::None,
            stage: stage_zero,
            stage_changed_at: Duration::from_secs(0),
            tutorial,
            event_dispatcher,
        }
    }

    pub fn load_definitions(&mut self, definitions: Vec<(String, Option<Vec<WordMeaning>>)>) {
        if let Some(stage) = &mut self.stage {
            if let Some(dict_ui) = &mut stage.active_game.dictionary_ui {
                dict_ui.load_definitions(definitions);
            }
        }
    }

    fn get_nth_scenario(tutorial: &Tutorial, index: usize) -> Option<(&String, &Scenario)> {
        tutorial
            .rules
            .iter()
            .map(|r| r.scenarios.iter().map(|s| (&r.category, s)))
            .flatten()
            .nth(index)
    }

    fn get_stage(
        index: usize,
        tutorial: &Tutorial,
        ctx: &egui::Context,
        map_texture: TextureHandle,
        theme: &Theme,
    ) -> Option<TutorialStage> {
        let scenario = TutorialState::get_nth_scenario(tutorial, index);

        scenario.map(|(category, scenario)| {
            let now = Some(
                instant::SystemTime::now()
                    .duration_since(instant::SystemTime::UNIX_EPOCH)
                    .expect("Please don't play Truncate earlier than 1970")
                    .as_secs(),
            );

            let mut board = Board::from_string(scenario.board.clone());

            let dict_lock = get_main_dict();
            let dict = dict_lock.as_ref().unwrap();
            board.mark_all_validity(Some(dict));

            let mut game = Game {
                rules: GameRules::latest().1,
                players: vec![
                    Player {
                        name: "You".into(),
                        index: 0,
                        hand: Hand(scenario.player_hand.chars().collect()),
                        hand_capacity: scenario.player_hand.len(),
                        allotted_time: None,
                        time_remaining: None,
                        turn_starts_no_later_than: now,
                        turn_starts_no_sooner_than: now,
                        paused_turn_delta: None,
                        swap_count: 0,
                        penalties_incurred: 0,
                        color: GAME_COLOR_BLUE,
                        seen_tiles: HashSet::new(),
                    },
                    Player {
                        name: "Computer".into(),
                        index: 1,
                        hand: Hand(scenario.computer_hand.chars().collect()),
                        hand_capacity: scenario.computer_hand.len(),
                        allotted_time: None,
                        time_remaining: None,
                        turn_starts_no_later_than: None,
                        turn_starts_no_sooner_than: None,
                        paused_turn_delta: None,
                        swap_count: 0,
                        penalties_incurred: 0,
                        color: GAME_COLOR_RED,
                        seen_tiles: HashSet::new(),
                    },
                ],
                board,
                // TODO: Use some special infinite bag?
                bag: TileBag::latest(None).1,
                judge: Judge::new(vec![]),
                battle_count: 0,
                turn_count: 0,
                player_turn_count: vec![0, 0],
                recent_changes: vec![],
                started_at: None,
                game_ends_at: None,
                next_player: Some(0),
                paused: false,
                winner: None,
            };

            let mut active_game = ActiveGame::new(
                ctx,
                "TUTORIAL_GAME".into(),
                None,
                None,
                game.players
                    .iter()
                    .map(|p| GamePlayerMessage::new(p, &game))
                    .collect(),
                vec![0],
                Some(0),
                game.board.clone(),
                vec![game.players[0].hand.clone()],
                map_texture,
                theme.clone(),
                GameLocation::Tutorial,
                None,
                None,
            );
            active_game.depot.ui_state.game_header = HeaderType::Tutorial;

            game.start();

            TutorialStage {
                category: category.clone(),
                scenario: scenario.clone(),
                game,
                active_game,
                step: 0,
            }
        })
    }

    fn can_increment_stage(&self) -> bool {
        TutorialState::get_nth_scenario(&self.tutorial, self.stage_index + 1).is_some()
    }

    fn increment_stage(&mut self, ctx: &egui::Context, map_texture: TextureHandle, theme: &Theme) {
        self.stage_index += 1;
        self.stage =
            TutorialState::get_stage(self.stage_index, &self.tutorial, ctx, map_texture, theme);
    }

    fn can_decrement_stage(&self) -> bool {
        self.stage_index != 0
    }

    fn decrement_stage(&mut self, ctx: &egui::Context, map_texture: TextureHandle, theme: &Theme) {
        self.stage_index = self.stage_index.saturating_sub(1);
        self.stage =
            TutorialState::get_stage(self.stage_index, &self.tutorial, ctx, map_texture, theme);
    }

    fn sub_event(&mut self, event: String) {
        self.event_dispatcher
            .event(format!("tutorial_{}_{}", self.name, event));
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        map_texture: TextureHandle,
        theme: &Theme,
        current_time: Duration,
    ) -> Vec<PlayerMessage> {
        let mut msgs_to_server = vec![];

        if self.stage_changed_at.is_zero() {
            self.stage_changed_at = current_time;
        }

        match self.change_stage_next_frame {
            ChangeStage::Next => {
                self.increment_stage(ui.ctx(), map_texture.clone(), theme);
                self.change_stage_next_frame = ChangeStage::None;
                self.stage_changed_at = current_time;
            }
            ChangeStage::Previous => {
                self.decrement_stage(ui.ctx(), map_texture.clone(), theme);
                self.change_stage_next_frame = ChangeStage::None;
                self.stage_changed_at = current_time;
            }
            ChangeStage::None => {}
        }

        while self
            .stage
            .as_ref()
            .is_some_and(|stage| stage.get_step().is_none())
        {
            let stage_name = self
                .stage
                .as_ref()
                .unwrap()
                .scenario
                .name
                .to_ascii_lowercase()
                .replace(' ', "_");
            self.event_dispatcher
                .event(format!("finish_stage_{stage_name}"));
            self.increment_stage(ui.ctx(), map_texture.clone(), theme);
        }

        let can_decrement_stage = self.can_decrement_stage();
        let can_increment_stage = self.can_increment_stage();

        let Some(current_stage) = self.stage.as_mut() else {
            ui.label("Tutorial is over!!!!!!!!!!!!!!!!!!!!!!!!");
            return msgs_to_server;
        };

        current_stage.highlight_interactions();
        let current_step = current_stage.get_step().cloned();

        let area = egui::Area::new(egui::Id::new("tutorial_stage_heading"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_TOP, vec2(0.0, 0.0));

        let heading_height = 60.0;
        let item_spacing = 10.0;
        let button_size = 48.0;

        let button_offset = (heading_height - button_size) / 2.0;
        let header_content_width = ui.available_width().at_most(450.0);
        let header_text_width = header_content_width - button_size * 2.0 - item_spacing * 4.0;

        let header_x_padding = (ui.available_width() - header_content_width) / 2.0;

        let paint_header = !current_stage.active_game.depot.ui_state.dictionary_open;

        if paint_header {
            area.show(ui.ctx(), |ui| {
                let header_rect = Rect::from_min_size(
                    ui.next_widget_position(),
                    vec2(ui.available_width(), heading_height),
                );

                ui.painter()
                    .clone()
                    .rect_filled(header_rect, 0.0, theme.water.gamma_multiply(0.9));

                ui.allocate_ui_with_layout(
                    vec2(ui.available_width(), heading_height),
                    Layout::left_to_right(Align::TOP),
                    |ui| {
                        ui.expand_to_include_rect(header_rect);
                        ui.spacing_mut().item_spacing = Vec2::splat(0.0);

                        ui.add_space(header_x_padding);

                        ui.add_space(item_spacing);

                        if can_decrement_stage {
                            let (prev_stage_rect, _) = ui.allocate_exact_size(
                                vec2(button_size, heading_height),
                                Sense::hover(),
                            );
                            let mut prev_stage_rect =
                                prev_stage_rect.shrink2(vec2(0.0, button_offset));
                            let prev_stage_resp = ui.allocate_rect(prev_stage_rect, Sense::click());
                            if prev_stage_resp.hovered() {
                                prev_stage_rect = prev_stage_rect.translate(vec2(0.0, -2.0));
                                ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                            }

                            render_tex_quad(
                                tiles::quad::SKIP_PREV_BUTTON,
                                prev_stage_rect,
                                &map_texture,
                                ui,
                            );

                            if prev_stage_resp.clicked() {
                                self.change_stage_next_frame = ChangeStage::Previous;
                            }
                        } else {
                            ui.add_space(button_size);
                        }

                        ui.add_space(item_spacing);

                        let (title_rect, _) = ui.allocate_exact_size(
                            vec2(header_text_width, heading_height),
                            Sense::hover(),
                        );

                        let mut fz = 14.0;
                        let mut title_text =
                            TextHelper::heavy(&current_stage.scenario.name, fz, None, ui);
                        while title_text.mesh_size().x > title_rect.width() {
                            fz -= 1.0;
                            title_text =
                                TextHelper::heavy(&current_stage.scenario.name, fz, None, ui);
                        }
                        title_text.paint_within(title_rect, Align2::CENTER_CENTER, theme.text, ui);

                        if can_increment_stage {
                            ui.add_space(item_spacing);

                            let (next_stage_rect, _) = ui.allocate_exact_size(
                                vec2(button_size, heading_height),
                                Sense::hover(),
                            );
                            let mut next_stage_rect =
                                next_stage_rect.shrink2(vec2(0.0, button_offset));
                            let next_stage_resp = ui.allocate_rect(next_stage_rect, Sense::click());
                            if next_stage_resp.hovered() {
                                next_stage_rect = next_stage_rect.translate(vec2(0.0, -2.0));
                                ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                            }

                            render_tex_quad(
                                tiles::quad::SKIP_NEXT_BUTTON,
                                next_stage_rect,
                                &map_texture,
                                ui,
                            );

                            if next_stage_resp.clicked() {
                                self.change_stage_next_frame = ChangeStage::Next;
                            }
                        }
                    },
                );
            });
        }

        ui.add_space(heading_height);

        let mut next_move = None;
        let mut pending_event = None;

        // Standard game helper
        if let Some(msg) = current_stage.render_game(ui, current_time) {
            if let PlayerMessage::RequestDefinitions(words) = &msg {
                msgs_to_server.push(PlayerMessage::RequestDefinitions(words.clone()));
            }

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
                return msgs_to_server;
            };

            let Some(step) = current_step.as_ref() else {
                return msgs_to_server;
            };

            if step == &game_move {
                next_move = Some(game_move);
            } else {
                // TODO: Handle player doing the wrong tutorial thing
            }
        }

        if let Some(dialog_pos) = current_stage.get_dialog_position() {
            let max_width = f32::min(700.0, dialog_pos.width());
            let dialog_padding_x = (dialog_pos.width() - max_width) / 2.0;

            let inner_dialog = dialog_pos.shrink2(vec2(dialog_padding_x, 8.0));

            let area = egui::Area::new(egui::Id::new("tutorial_layer"))
                .movable(false)
                .order(Order::Foreground)
                .fixed_pos(dialog_pos.left_top());

            area.show(ui.ctx(), |ui| {
                ui.painter()
                    .rect_filled(dialog_pos, 4.0, theme.water.gamma_multiply(0.75));
                ui.expand_to_include_rect(dialog_pos);
                ui.allocate_ui_at_rect(inner_dialog, |ui| {
                    ui.expand_to_include_rect(inner_dialog);

                    // TODO one day — put this in a theme
                    let tut_fz = if inner_dialog.width() < 600.0 {
                        24.0
                    } else {
                        32.0
                    };

                    let button_spacing = 70.0;
                    let time_in_stage = (current_time - self.stage_changed_at).as_secs_f32();

                    match current_step {
                        Some(step) => match step {
                            ScenarioStep::OwnMove { description, .. } => {
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
                                    &map_texture,
                                    ui,
                                );
                            }
                            ScenarioStep::ComputerMove {
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
                                    &map_texture,
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
                                                        &map_texture,
                                                        ui,
                                                    )
                                                    .clicked()
                                                {
                                                    next_move = Some(action_to_move(1, &action));
                                                }
                                            },
                                        );
                                    });
                                }
                            }
                            ScenarioStep::Dialog { message } => {
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
                                    &map_texture,
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
                                                        &map_texture,
                                                        ui,
                                                    )
                                                    .clicked()
                                                {
                                                    current_stage.increment_step();
                                                    self.stage_changed_at = current_time;
                                                }
                                            },
                                        );
                                    });
                                }
                            }
                            ScenarioStep::EndAction { end_message } => {
                                pending_event = Some("complete".to_string());

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
                                    &map_texture,
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
                                                        &map_texture,
                                                        ui,
                                                    )
                                                    .clicked()
                                                {
                                                    back_to_menu();
                                                }
                                            },
                                        );
                                    });
                                }
                            }
                        },
                        None => {
                            // Ideally unreachable in a well formed tutorial
                        }
                    };
                });
            });
        }

        if let Some(next_move) = next_move {
            if current_stage.handle_move(next_move).is_ok() {
                self.stage_changed_at = current_time;
            }
        }

        if let Some(pending_event) = pending_event {
            self.sub_event(pending_event);
        }

        msgs_to_server
    }
}
