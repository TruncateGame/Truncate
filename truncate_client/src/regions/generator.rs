use eframe::egui::{self, DragValue, Layout, RichText, Sense};
use epaint::{emath::Align, vec2, Color32, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    game::Game,
    generation::{
        self, generate_board, BoardElements, BoardGenerationResult, BoardNoiseParams, BoardParams,
        BoardSeed, DockType, Symmetry, WaterLayer,
    },
    messages::GamePlayerMessage,
    rules::{BoardGenesis, GameRules},
};

use crate::utils::{Lighten, Theme};

use super::active_game::{ActiveGame, GameLocation, HeaderType};

pub struct GeneratorState {
    active_game: ActiveGame,
    seed: u32,
    infinite: bool,
    params: BoardParams,
    generation_result: Option<Result<BoardGenerationResult, BoardGenerationResult>>,
}

impl GeneratorState {
    pub fn new(ctx: &egui::Context, map_texture: TextureHandle, theme: Theme) -> Self {
        let mut game = Game::new(10, 10, None, GameRules::latest().1);
        game.add_player("p1".into());
        let mut active_game = ActiveGame::new(
            ctx,
            "TARGET".into(),
            None,
            None,
            game.players
                .iter()
                .map(|p| GamePlayerMessage::new(p, &game))
                .collect(),
            0,
            Some(0),
            game.board.clone(),
            game.players[0].hand.clone(),
            map_texture.clone(),
            theme.clone(),
            GameLocation::Local,
            None,
            None,
        );
        active_game.depot.ui_state.game_header = HeaderType::None;
        active_game.depot.ui_state.hand_hidden = true;
        active_game.depot.ui_state.sidebar_hidden = true;
        active_game.depot.interactions.view_only = true;

        // let (_, default) = BoardParams::latest();
        let BoardGenesis::Random(default) = GameRules::tuesday().board_genesis else {
            panic!("ack");
        };

        Self {
            active_game,
            seed: 1844,
            infinite: false,
            params: default,
            generation_result: None,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, _theme: &Theme, current_time: Duration) {
        let max_attempts = 500;
        let mut changed = self.generation_result.is_none();

        egui::Window::new("controls").show(ui.ctx(), |ui| {
            let _r = egui::Grid::new("weightings")
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
                        DragValue::new(&mut self.params.land_dimensions[0])
                            .clamp_range(4..=1000)
                            .speed(0.05),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("Height").color(Color32::WHITE));
                    let r = ui.add(
                        DragValue::new(&mut self.params.land_dimensions[1])
                            .clamp_range(4..=1000)
                            .speed(0.05),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("Canvas Width").color(Color32::WHITE));
                    let r = ui.add(
                        DragValue::new(&mut self.params.canvas_dimensions[0])
                            .clamp_range(4..=1000)
                            .speed(0.05),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("Canvas Height").color(Color32::WHITE));
                    let r = ui.add(
                        DragValue::new(&mut self.params.canvas_dimensions[1])
                            .clamp_range(4..=1000)
                            .speed(0.05),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("Dispersion X").color(Color32::WHITE));
                    let r = ui.add(
                        DragValue::new(&mut self.params.land_layer.dispersion[0])
                            .clamp_range(0.0..=100.0)
                            .speed(0.01),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("Dispersion y").color(Color32::WHITE));
                    let r = ui.add(
                        DragValue::new(&mut self.params.land_layer.dispersion[1])
                            .clamp_range(0.0..=100.0)
                            .speed(0.01),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("Island Influence").color(Color32::WHITE));
                    let r = ui.add(
                        DragValue::new(&mut self.params.land_layer.island_influence)
                            .clamp_range(0.0..=1.0)
                            .speed(0.005),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("Symmetric").color(Color32::WHITE));
                    if ui
                        .button(format!("{:?}", self.params.land_layer.symmetric))
                        .clicked()
                    {
                        self.params.land_layer.symmetric = match self.params.land_layer.symmetric {
                            Symmetry::TwoFoldRotational => Symmetry::SmoothTwoFoldRotational,
                            Symmetry::SmoothTwoFoldRotational => Symmetry::Asymmetric,
                            Symmetry::Asymmetric => Symmetry::TwoFoldRotational,
                        };
                        changed = true;
                    }
                    ui.end_row();

                    if self.params.water_layer.is_some() {
                        if ui.button("Remove Water Layer").clicked() {
                            self.params.water_layer = None;
                            changed = true;
                        };

                        ui.end_row();
                    }
                    if let Some(water_layer) = self.params.water_layer.as_mut() {
                        ui.label(RichText::new("[WATER] Dispersion X").color(Color32::WHITE));
                        let r = ui.add(
                            DragValue::new(&mut water_layer.params.dispersion[0])
                                .clamp_range(0.0..=100.0)
                                .speed(0.01),
                        );
                        if r.changed() {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label(RichText::new("[WATER] Dispersion y").color(Color32::WHITE));
                        let r = ui.add(
                            DragValue::new(&mut water_layer.params.dispersion[1])
                                .clamp_range(0.0..=100.0)
                                .speed(0.01),
                        );
                        if r.changed() {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label(RichText::new("[WATER] Island Influence").color(Color32::WHITE));
                        let r = ui.add(
                            DragValue::new(&mut water_layer.params.island_influence)
                                .clamp_range(0.0..=1.0)
                                .speed(0.005),
                        );
                        if r.changed() {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label(RichText::new("[WATER] Symmetric").color(Color32::WHITE));
                        if ui
                            .button(format!("{:?}", water_layer.params.symmetric))
                            .clicked()
                        {
                            water_layer.params.symmetric = match water_layer.params.symmetric {
                                Symmetry::TwoFoldRotational => Symmetry::SmoothTwoFoldRotational,
                                Symmetry::SmoothTwoFoldRotational => Symmetry::Asymmetric,
                                Symmetry::Asymmetric => Symmetry::TwoFoldRotational,
                            };
                            changed = true;
                        }
                        ui.end_row();

                        ui.label(RichText::new("[WATER] Density").color(Color32::WHITE));
                        let r = ui.add(
                            DragValue::new(&mut water_layer.density)
                                .clamp_range(0.0..=1.0)
                                .speed(0.005),
                        );
                        if r.changed() {
                            changed = true;
                        }
                        ui.end_row();
                    } else {
                        if ui.button("Add Water Layer").clicked() {
                            self.params.water_layer = Some(WaterLayer {
                                params: BoardNoiseParams {
                                    dispersion: [20.0, 20.0],
                                    island_influence: 0.0,
                                    symmetric: Symmetry::TwoFoldRotational,
                                },
                                density: 0.5,
                            });
                            changed = true;
                        }
                        ui.end_row();
                    }

                    ui.label(RichText::new("Town Density").color(Color32::WHITE));
                    let r = ui.add(
                        DragValue::new(&mut self.params.maximum_town_density)
                            .clamp_range(0.0..=1.0)
                            .speed(0.005),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("Town Distance").color(Color32::WHITE));
                    let r = ui.add(
                        DragValue::new(&mut self.params.maximum_town_distance)
                            .clamp_range(0.0..=1.0)
                            .speed(0.005),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("Min Choke").color(Color32::WHITE));
                    let r = ui.add(
                        DragValue::new(&mut self.params.minimum_choke)
                            .clamp_range(1..=100)
                            .speed(0.05),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("Dock Type").color(Color32::WHITE));
                    if ui.button(format!("{:?}", self.params.dock_type)).clicked() {
                        self.params.dock_type = match self.params.dock_type {
                            DockType::IslandV1 => DockType::Coastal,
                            DockType::Coastal => DockType::Continental,
                            DockType::Continental => DockType::IslandV1,
                        };
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("ideal_dock_extremity").color(Color32::WHITE));
                    let r = ui.add(
                        DragValue::new(&mut self.params.ideal_dock_extremity)
                            .clamp_range(0.0..=1.0)
                            .speed(0.01),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    ui.end_row();
                });
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
                width_resize_state: None,
                height_resize_state: None,
                water_level: 0.5,
                max_attempts,
                params: self.params.clone(),
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
