use eframe::egui::{self, DragValue, Layout, RichText, Sense};
use epaint::{emath::Align, vec2, Color32, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    game::Game,
    generation::{self, generate_board, BoardGenerationResult, BoardParams, BoardSeed},
};

use crate::utils::{Lighten, Theme};

use super::active_game::{ActiveGame, GameLocation, HeaderType};

pub struct GeneratorState {
    active_game: ActiveGame,
    seed: u32,
    infinite: bool,
    width: usize,
    height: usize,
    dispersion: f64,
    isolation: f64,
    maximum_town_density: f64,
    maximum_town_distance: f64,
    island_influence: f64,
    minimum_choke: usize,
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
            None,
            game.players.iter().map(Into::into).collect(),
            0,
            0,
            game.board.clone(),
            game.players[0].hand.clone(),
            map_texture.clone(),
            theme.clone(),
            GameLocation::Local,
        );
        active_game.depot.ui_state.game_header = HeaderType::None;
        active_game.depot.ui_state.hand_hidden = true;
        active_game.depot.ui_state.sidebar_hidden = true;
        active_game.depot.interactions.view_only = true;

        let (_, default) = BoardParams::latest();

        Self {
            active_game,
            seed: 1843,
            infinite: false,
            width: default.land_dimensions[0],
            height: default.land_dimensions[1],
            dispersion: default.dispersion[0],
            isolation: default.isolation,
            maximum_town_density: default.maximum_town_density,
            maximum_town_distance: default.maximum_town_distance,
            island_influence: default.island_influence,
            minimum_choke: default.minimum_choke,
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
                        .speed(0.05),
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
                        .speed(0.05),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Height").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.height)
                        .clamp_range(4..=100)
                        .speed(0.05),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Dispersion").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.dispersion)
                        .clamp_range(0.0..=100.0)
                        .speed(0.01),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Isolation").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.isolation)
                        .clamp_range(1.0..=10.0)
                        .speed(0.01),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Town Density").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.maximum_town_density)
                        .clamp_range(0.0..=1.0)
                        .speed(0.005),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Town Distance").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.maximum_town_distance)
                        .clamp_range(0.0..=1.0)
                        .speed(0.005),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Island Influence").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.island_influence)
                        .clamp_range(0.0..=1.0)
                        .speed(0.005),
                );
                if r.changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label(RichText::new("Min Choke").color(Color32::WHITE));
                let r = ui.add(
                    DragValue::new(&mut self.minimum_choke)
                        .clamp_range(1..=100)
                        .speed(0.05),
                );
                if r.changed() {
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
            println!("Generating {}", self.seed);
            self.generation_result = Some(generate_board(BoardSeed {
                generation: 999999,
                seed: self.seed,
                day: None,
                current_iteration: 0,
                width_resize_state: None,
                height_resize_state: None,
                water_level: 0.5,
                max_attempts,
                params: BoardParams {
                    land_dimensions: [self.width, self.height],
                    dispersion: [self.dispersion, self.dispersion],
                    isolation: self.isolation,
                    maximum_town_density: self.maximum_town_density,
                    maximum_town_distance: self.maximum_town_distance,
                    island_influence: self.island_influence,
                    minimum_choke: self.minimum_choke,
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
            &self.active_game.depot.timing,
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
