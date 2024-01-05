use eframe::{
    egui::{self, Layout},
    emath::Align,
};
use epaint::Rect;
use truncate_core::board::Coordinate;

use crate::utils::{depot::TruncateDepot, Lighten};

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
        &mut self,
        ui: &mut egui::Ui,
        depot: &mut TruncateDepot,
        contents: impl FnOnce(&mut egui::Ui, &mut TruncateDepot),
    ) -> (egui::Response, Rect) {
        let TruncateDepot {
            interactions,
            aesthetics,
            ..
        } = depot;

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(aesthetics.theme.grid_size, aesthetics.theme.grid_size),
            egui::Sense::hover(),
        );
        let interact_rect = rect.shrink(aesthetics.theme.tile_margin);
        let response = ui.interact(
            interact_rect,
            response.id.with("interact"),
            egui::Sense::click(),
        );

        if ui.is_rect_visible(rect) {
            // mapped_board.render_coord(self.coord, rect, ui);

            if self.enabled {
                // ui.painter().rect_stroke(
                //     rect.shrink(ctx.theme.tile_margin),
                //     ctx.theme.rounding,
                //     Stroke::new(1.0, hex_color!("ffffff01")),
                // );
            }

            let is_hovered = ui.rect_contains_pointer(interact_rect);
            let is_hovered_with_drag = is_hovered && interactions.dragging_tile;
            let show_overlay = is_hovered && self.overlay.is_some();

            if (show_overlay || is_hovered_with_drag)
                && (self.empty || interactions.selected_square_on_board.is_some())
            {
                if let Some(overlay) = self.overlay {
                    TileUI::new(Some(overlay), TilePlayer::Own)
                        .ghost(true)
                        .render(
                            None,
                            &mut ui.child_ui(rect, Layout::left_to_right(Align::TOP)),
                            false,
                            None,
                            depot,
                        );
                }
            } else {
                contents(
                    &mut ui.child_ui(rect, Layout::left_to_right(Align::TOP)),
                    depot,
                );
            }

            let TruncateDepot {
                interactions,
                timing,
                aesthetics,
                ..
            } = depot;

            if self.empty {
                if let Some(squares) = interactions.highlight_squares.as_ref() {
                    if squares.contains(&self.coord) && timing.current_time.subsec_millis() > 500 {
                        let highlight_rect = rect.shrink(aesthetics.theme.tile_margin);

                        ui.painter().rect_filled(
                            highlight_rect,
                            aesthetics.theme.rounding,
                            aesthetics.theme.selection.pastel().gamma_multiply(0.5),
                        );
                    }
                }
            }
        }

        (response, rect)
    }
}
