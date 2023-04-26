use eframe::egui::{self, Sense};
use epaint::{vec2, Stroke, TextureId};
use truncate_core::board::Coordinate;

use crate::theming::mapper::MappedBoard;
use crate::theming::tex::{BGTexType, Tex, TexQuad};
use crate::theming::{Darken, Lighten, Theme};

use super::{character::CharacterOrient, CharacterUI};

pub struct EditorSquareUI {
    coord: Coordinate,
    enabled: bool,
    root: bool,
}

impl EditorSquareUI {
    pub fn new(coord: Coordinate) -> Self {
        Self {
            coord,
            enabled: false,
            root: false,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn root(mut self, root: bool) -> Self {
        self.root = root;
        self
    }

    pub fn render(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        mapped_board: &MappedBoard,
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

            if self.root {
                CharacterUI::new('#', CharacterOrient::North).render_with_color(
                    ui,
                    rect.shrink(theme.tile_margin),
                    theme,
                    theme.selection,
                );
                if response.hovered() {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Move);
                };
            } else if response.hovered() {
                let action = if self.enabled { '-' } else { '+' };
                let color = if self.enabled {
                    theme.background
                } else {
                    theme.addition
                };
                CharacterUI::new(action, CharacterOrient::North).render_with_color(
                    ui,
                    rect.shrink(theme.tile_margin),
                    theme,
                    color,
                );
            }
        }
        response
    }
}
