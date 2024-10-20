use eframe::egui::{self};
use epaint::{hex_color, Stroke, TextureHandle};
use truncate_core::board::Square;

use crate::regions::lobby::BoardEditingMode;
use crate::utils::tex::{render_tex_quad, Tex};
use crate::utils::Theme;

pub struct EditorSquareUI {
    square: Square,
    action: BoardEditingMode,
}

impl EditorSquareUI {
    pub fn new() -> Self {
        Self {
            square: Square::water(),
            action: BoardEditingMode::Land,
        }
    }

    pub fn square(mut self, square: Square) -> Self {
        self.square = square;
        self
    }

    pub fn action(mut self, action: BoardEditingMode) -> Self {
        self.action = action;
        self
    }

    pub fn render(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        map_texture: &TextureHandle,
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(theme.grid_size, theme.grid_size),
            egui::Sense::hover(),
        );
        let response = ui.interact(
            rect.shrink(theme.tile_margin),
            response.id.with("editor_tile"),
            egui::Sense::drag(),
        );

        let inner_bounds = rect.shrink(theme.tile_margin);

        if ui.is_rect_visible(rect) {
            if !matches!(self.action, BoardEditingMode::None) && response.hovered() {
                if !response.is_pointer_button_down_on() {
                    if let Some(overlay) = Tex::landscaping(&self.square, &self.action) {
                        render_tex_quad(overlay, rect, map_texture, ui);
                    } else {
                        ui.painter().rect_filled(
                            inner_bounds,
                            theme.rounding,
                            hex_color!("ffffff03"),
                        );
                    }
                }
            }
        }
        if matches!(self.square, Square::Land { .. }) {
            ui.painter().rect_stroke(
                inner_bounds,
                theme.rounding,
                Stroke::new(1.0, hex_color!("ffffff01")),
            );
        }

        response
    }
}
