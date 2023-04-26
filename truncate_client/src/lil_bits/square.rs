use eframe::{
    egui::{self, Layout},
    emath::Align,
};
use epaint::{Rect, Stroke};
use truncate_core::board::Coordinate;

use crate::theming::{mapper::MappedBoard, Theme};

use super::{character::CharacterOrient, tile::TilePlayer, CharacterUI, TileUI};

pub struct SquareUI {
    coord: Coordinate,
    decorated: bool,
    enabled: bool,
    empty: bool,
    selected: bool,
    root: bool,
    overlay: Option<char>,
}

impl SquareUI {
    pub fn new(coord: Coordinate) -> Self {
        Self {
            coord,
            decorated: true,
            enabled: true,
            empty: false,
            selected: false,
            root: false,
            overlay: None,
        }
    }

    pub fn decorated(mut self, decorated: bool) -> Self {
        self.decorated = decorated;
        self
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

    pub fn root(mut self, root: bool) -> Self {
        self.root = root;
        self
    }

    pub fn overlay(mut self, overlay: Option<char>) -> Self {
        self.overlay = overlay;
        self
    }

    pub fn render(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        mapped_board: &MappedBoard,
        contents: impl FnOnce(&mut egui::Ui, &Theme),
    ) -> (egui::Response, Rect) {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(theme.grid_size, theme.grid_size),
            egui::Sense::hover(),
        );
        let interact_rect = rect.shrink(theme.tile_margin);
        let mut response = ui.interact(
            interact_rect,
            response.id.with("interact"),
            egui::Sense::click(),
        );

        if ui.is_rect_visible(rect) {
            if self.decorated {
                mapped_board.render_coord(self.coord, rect, ui);
            }

            if self.enabled {
                // if self.empty && self.selected {
                //     ui.painter()
                //         .rect_filled(rect, theme.rounding, theme.selection);
                // }

                // if self.decorated {
                //     ui.painter()
                //         .rect_filled(rect, theme.rounding, theme.background);
                //     ui.painter()
                //         .rect_stroke(rect, 0.0, Stroke::new(1.0, theme.outlines));
                // }
            }

            let is_hovered = ui.rect_contains_pointer(interact_rect);

            let show_overlay = is_hovered && self.overlay.is_some();
            let show_contents = !self.empty || !is_hovered;

            // TODO: Show/hide this so it doesn't clash with things like dead/truncated tiles
            if self.root && !is_hovered {
                CharacterUI::new('#', CharacterOrient::North).render_with_color(
                    ui,
                    rect.shrink(theme.tile_margin),
                    theme,
                    theme.selection,
                );
            }

            if show_contents && !show_overlay {
                contents(
                    &mut ui.child_ui(rect, Layout::left_to_right(Align::TOP)),
                    theme,
                );
            }

            if is_hovered {
                if let Some(overlay) = self.overlay {
                    // TODO: Bring back overlays
                    ui.heading(format!("{overlay}"));

                    // response = TileUI::new(overlay, TilePlayer::Own).ghost(true).render(
                    //     &mut ui.child_ui(rect, Layout::left_to_right(Align::TOP)),
                    //     theme,
                    // );
                } else if self.empty && !ui.ctx().memory(|mem| mem.is_anything_being_dragged()) {
                    ui.painter().rect_filled(
                        rect.shrink(theme.tile_margin),
                        theme.rounding,
                        theme.outlines,
                    );
                }
            }
        }

        (response, rect)
    }
}
