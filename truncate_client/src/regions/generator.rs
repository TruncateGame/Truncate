use eframe::egui::{self, DragValue, Layout, RichText, Sense};
use epaint::{emath::Align, hex_color, vec2, Color32, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    game::Game,
    generation::{generate_board, BoardParams},
};

use crate::utils::Theme;

use super::active_game::ActiveGame;

pub struct GeneratorState {
    active_game: ActiveGame,
    seed: u32,
    width: usize,
    height: usize,
    water_level: f64,
    town_density: f64,
    jitter: f64,
    town_jitter: f64,
}

impl GeneratorState {
    pub fn new(map_texture: TextureHandle, theme: Theme) -> Self {
        let mut game = Game::new(10, 10, None);
        game.add_player("p1".into());
        let mut active_game = ActiveGame::new(
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
        active_game.ctx.timers_visible = false;
        active_game.ctx.hand_visible = false;
        active_game.ctx.sidebar_visible = false;
        active_game.ctx.interactive = false;

        Self {
            active_game,
            seed: 1234,
            width: 14,
            height: 16,
            water_level: 0.5,
            town_density: 0.5,
            jitter: 0.5,
            town_jitter: 0.5,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme, current_time: Duration) {
        let r = egui::Grid::new("weightings")
            .spacing(Vec2::splat(8.0))
            .min_col_width(150.0)
            .show(ui, |ui| {
                ui.label(RichText::new("Seed").color(Color32::WHITE));
                ui.add(
                    DragValue::new(&mut self.seed)
                        .clamp_range(1..=10000)
                        .speed(1),
                );
                if ui.button("+").clicked() {
                    self.seed += 1;
                }
                ui.end_row();

                ui.label(RichText::new("Width").color(Color32::WHITE));
                ui.add(
                    DragValue::new(&mut self.width)
                        .clamp_range(4..=100)
                        .speed(1),
                );
                ui.end_row();

                ui.label(RichText::new("Height").color(Color32::WHITE));
                ui.add(
                    DragValue::new(&mut self.height)
                        .clamp_range(4..=100)
                        .speed(1),
                );
                ui.end_row();

                ui.label(RichText::new("Water").color(Color32::WHITE));
                ui.add(
                    DragValue::new(&mut self.water_level)
                        .clamp_range(0.0..=1.0)
                        .speed(0.01),
                );
                ui.end_row();

                ui.label(RichText::new("Towns").color(Color32::WHITE));
                ui.add(
                    DragValue::new(&mut self.town_density)
                        .clamp_range(0.0..=1.0)
                        .speed(0.01),
                );
                ui.end_row();

                ui.label(RichText::new("Land Jitter").color(Color32::WHITE));
                ui.add(
                    DragValue::new(&mut self.jitter)
                        .clamp_range(0.0..=1.0)
                        .speed(0.01),
                );
                ui.end_row();

                ui.label(RichText::new("Town Jitter").color(Color32::WHITE));
                ui.add(
                    DragValue::new(&mut self.town_jitter)
                        .clamp_range(0.0..=1.0)
                        .speed(0.01),
                );
                ui.end_row();
            });

        ui.add_space(8.0);

        let board = generate_board(BoardParams {
            seed: self.seed,
            bounding_width: self.width,
            bounding_height: self.height,
            maximum_land_width: None,
            maximum_land_height: None,
            water_level: self.water_level,
            town_density: self.town_density,
            jitter: self.jitter,
            town_jitter: self.town_jitter,
            current_iteration: 0,
        });
        self.active_game.board = board;
        self.active_game.board.cache_special_squares();
        self.active_game.mapped_board.remap(
            &self.active_game.board,
            &self.active_game.ctx.player_colors,
            0,
        );

        let (game_rect, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), ui.available_height()),
            Sense::hover(),
        );
        let mut game_ui = ui.child_ui(game_rect, Layout::left_to_right(Align::TOP));

        self.active_game
            .render(&mut game_ui, theme, None, current_time, None);
    }
}
