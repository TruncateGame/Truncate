use eframe::egui::{self, DragValue, Layout, RichText, Sense};
use epaint::{emath::Align, vec2, Color32, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    game::Game,
    generation::{self, generate_board, BoardGenerationResult, BoardParams, BoardSeed, BoardType},
};

use crate::utils::{Lighten, Theme};

use super::active_game::{ActiveGame, HeaderType};

pub struct GeneratorState {
    active_game: ActiveGame,
    seed: u32,
    infinite: bool,
    width: usize,
    height: usize,
    land_slop: usize,
    water_level: f64,
    dispersion: f64,
    town_density: f64,
    jitter: f64,
    town_jitter: f64,
    minimum_choke: usize,
    board_type: BoardType,
    generation_result: Option<Result<BoardGenerationResult, BoardGenerationResult>>,
}

impl GeneratorState {
    pub fn new(ctx: &egui::Context, map_texture: TextureHandle, theme: Theme) -> Self {
        let mut game = Game::new(10, 10, None);
        game.add_player("p1".into());
        let mut active_game = ActiveGame::new(
            ctx,
            "TARGET".into(),
            None,
            game.players.iter().map(Into::into).collect(),
            0,
            0,
            game.board.clone(),
            game.players[0].hand.clone(),
            map_texture.clone(),
            theme.clone(),
        );
        active_game.depot.ui_state.game_header = HeaderType::None;
        active_game.depot.ui_state.hand_hidden = true;
        active_game.depot.ui_state.sidebar_hidden = true;
        active_game.depot.interactions.view_only = true;

        Self {
            active_game,
            seed: 1234,
            infinite: false,
            width: 10,
            height: 10,
            land_slop: 2,
            water_level: 0.5,
            dispersion: 3.0,
            town_density: 0.5,
            jitter: 0.25,
            town_jitter: 0.5,
            minimum_choke: 4,
            board_type: BoardType::Island,
            generation_result: None,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme, current_time: Duration) {
        let max_attempts = 500;
        let mut changed = self.generation_result.is_none();

        let r = egui::Grid::new("weightings")
            .spacing(Vec2::splat(8.0))
            .min_col_width(150.0)
            .show(ui, |ui| {
                ui.label(RichText::new("Seed").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.seed)
                        .clamp_range(1..=10000)
                        .speed(1),
                );
                if r.changed() {
                    changed = true;
                }
                if ui.button("+1").clicked() {
                    changed = true;
                    self.seed += 1;
                }
                if ui.button("++++++").clicked() {
                    changed = true;
                    self.infinite = !self.infinite;
                }
                ui.end_row();

                ui.label(RichText::new("Width").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.width)
                        .clamp_range(4..=100)
                        .speed(1),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Height").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.height)
                        .clamp_range(4..=100)
                        .speed(1),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Slop").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.land_slop)
                        .clamp_range(0..=100)
                        .speed(1),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Water").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.water_level)
                        .clamp_range(0.0..=1.0)
                        .speed(0.001),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Dispersion").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.dispersion)
                        .clamp_range(0.0..=100.0)
                        .speed(0.001),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Towns").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.town_density)
                        .clamp_range(0.0..=1.0)
                        .speed(0.001),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Land Jitter").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.jitter)
                        .clamp_range(0.0..=1.0)
                        .speed(0.001),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Town Jitter").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.town_jitter)
                        .clamp_range(0.0..=1.0)
                        .speed(0.001),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Min Choke").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.minimum_choke)
                        .clamp_range(1..=100)
                        .speed(1),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Board Type").color(Color32::WHITE));
                if ui.button(format!("{:?}", self.board_type)).clicked() {
                    self.board_type = match self.board_type {
                        BoardType::Island => BoardType::Continental,
                        BoardType::Continental => BoardType::Island,
                    };
                    changed = true;
                }
                ui.end_row();
            });

        if self.infinite {
            self.seed += 1;
            ui.ctx().request_repaint();
            changed = true;
        }
        if changed {
            self.generation_result = Some(generate_board(BoardSeed {
                generation: 999999,
                seed: self.seed,
                day: None,
                current_iteration: 0,
                resize_state: None,
                max_attempts,
                params: BoardParams {
                    ideal_land_dimensions: [self.width, self.height],
                    land_slop: self.land_slop,
                    water_level: self.water_level,
                    dispersion: self.dispersion,
                    town_density: self.town_density,
                    jitter: self.jitter,
                    town_jitter: self.town_jitter,
                    minimum_choke: self.minimum_choke,
                    board_type: self.board_type,
                },
            }));
        }

        let Some(generation_result) = &self.generation_result else {
            ui.heading(
                RichText::new(format!("Nothing generated."))
                    .color(Color32::RED.lighten().lighten()),
            );
            return;
        };

        let generation_failed = generation_result.is_err();
        let BoardGenerationResult { board, iterations } = match generation_result {
            Ok(b) => b,
            Err(b) => b,
        };
        self.active_game.board = board.clone();
        self.active_game.board.cache_special_squares();
        self.active_game.mapped_board.remap_texture(
            &ui.ctx(),
            &self.active_game.depot.aesthetics,
            None,
            None,
            &self.active_game.board,
        );

        if generation_failed {
            self.infinite = false;
            ui.heading(
                RichText::new(format!(
                    "Failed to generate board within {max_attempts} iteration(s)"
                ))
                .color(Color32::RED.lighten().lighten()),
            );
        } else {
            ui.heading(
                RichText::new(format!("Generated a board in {iterations} iteration(s)"))
                    .color(Color32::GREEN.lighten().lighten()),
            );
        }
        ui.add_space(8.0);

        let (game_rect, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), ui.available_height()),
            Sense::hover(),
        );
        let mut game_ui = ui.child_ui(game_rect, Layout::left_to_right(Align::TOP));

        self.active_game.render(&mut game_ui, current_time, None);
    }
}
