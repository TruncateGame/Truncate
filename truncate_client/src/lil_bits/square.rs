use eframe::{
    egui::{self, Layout},
    emath::Align,
};
use epaint::{hex_color, vec2, Rect, Stroke, TextureHandle};
use truncate_core::board::Coordinate;

use crate::{
    regions::active_game::GameCtx,
    utils::{mapper::MappedBoard, Diaphanize, Lighten, Theme},
};

use super::{tile::TilePlayer, TileUI};

pub struct SquareUI {
    coord: Coordinate,
    enabled: bool,
    empty: bool,
    selected: bool,
    overlay: Option<char>,
}

impl SquareUI {
    pub fn new(coord: Coordinate) -> Self {
        Self {
            coord,
            enabled: true,
            empty: false,
            selected: false,
            overlay: None,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn overlay(mut self, overlay: Option<char>) -> Self {
        self.overlay = overlay;
        self
    }

    pub fn render(
        &self,
        ui: &mut egui::Ui,
        ctx: &mut GameCtx,
        mapped_board: &MappedBoard,
        contents: impl FnOnce(&mut egui::Ui, &mut GameCtx),
    ) -> (egui::Response, Rect) {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ctx.theme.grid_size, ctx.theme.grid_size),
            egui::Sense::hover(),
        );
        let interact_rect = rect.shrink(ctx.theme.tile_margin);
        let mut response = ui.interact(
            interact_rect,
            response.id.with("interact"),
            egui::Sense::click(),
        );

        if ui.is_rect_visible(rect) {
            mapped_board.render_coord(self.coord, rect, ui);

            if self.enabled {
                ui.painter().rect_stroke(
                    rect.shrink(ctx.theme.tile_margin),
                    ctx.theme.rounding,
                    Stroke::new(1.0, hex_color!("ffffff01")),
                );
            }

            let is_hovered = ui.rect_contains_pointer(interact_rect);

            let show_overlay = is_hovered && self.overlay.is_some();
            let show_contents = !self.empty || !is_hovered;

            if show_contents && !show_overlay {
                contents(
                    &mut ui.child_ui(rect, Layout::left_to_right(Align::TOP)),
                    ctx,
                );
            }

            if is_hovered {
                if let Some(overlay) = self.overlay {
                    TileUI::new(overlay, TilePlayer::Own).ghost(true).render(
                        None,
                        &mut ui.child_ui(rect, Layout::left_to_right(Align::TOP)),
                        ctx,
                        false,
                        None,
                    );
                }
            }

            if self.empty {
                if let Some(squares) = ctx.highlight_squares.as_ref() {
                    if squares.contains(&self.coord) && ctx.current_time.subsec_millis() > 500 {
                        let mut highlight_rect = rect.shrink(ctx.theme.tile_margin);

                        ui.painter().rect_filled(
                            highlight_rect,
                            ctx.theme.rounding,
                            ctx.theme.selection.pastel().gamma_multiply(0.5),
                        );
                    }
                }
            }
        }

        (response, rect)
    }
}
