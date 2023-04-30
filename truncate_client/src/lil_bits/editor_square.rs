use eframe::egui::{self};
use epaint::{hex_color, Stroke, TextureHandle};
use truncate_core::board::Coordinate;

use crate::theming::mapper::MappedBoard;
use crate::theming::tex::{render_tex_quad, Tex};
use crate::theming::Theme;

pub struct EditorSquareUI {
    coord: Coordinate,
    enabled: bool,
}

impl EditorSquareUI {
    pub fn new(coord: Coordinate) -> Self {
        Self {
            coord,
            enabled: false,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn render(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        mapped_board: &MappedBoard,
        map_texture: &TextureHandle,
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(theme.grid_size, theme.grid_size),
            egui::Sense::click(),
        );
        let response = ui.interact(
            rect.shrink(theme.tile_margin),
            response.id.with("editor_tile"),
            egui::Sense::click_and_drag(),
        );

        if ui.is_rect_visible(rect) {
            mapped_board.render_coord(self.coord, rect, ui);

            if response.hovered() {
                if !response.is_pointer_button_down_on() {
                    let mapped_addition = Tex::resolve_landscaping_tex(!self.enabled);
                    render_tex_quad(mapped_addition, rect, map_texture, ui);
                }
            }
        }
        if self.enabled {
            ui.painter().rect_stroke(
                rect.shrink(theme.tile_margin),
                theme.rounding,
                Stroke::new(1.0, hex_color!("ffffff01")),
            );
        }

        response
    }
}
