use eframe::{
    egui::{self, Layout},
    emath::Align,
};
use epaint::Rect;

use crate::utils::depot::TruncateDepot;

pub struct HandSquareUI {
    empty: bool,
}

impl HandSquareUI {
    pub fn new() -> Self {
        Self { empty: false }
    }

    // TODO: Why is this not called
    #[allow(dead_code)]
    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    pub fn render(
        &self,
        ui: &mut egui::Ui,
        depot: &mut TruncateDepot,
        contents: impl FnOnce(&mut egui::Ui, &mut TruncateDepot),
    ) -> (egui::Response, Rect) {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(
                depot.aesthetics.theme.grid_size,
                depot.aesthetics.theme.grid_size,
            ),
            egui::Sense::hover(),
        );
        let interact_rect = rect.shrink(depot.aesthetics.theme.tile_margin);
        let response = ui.interact(
            interact_rect,
            response.id.with("interact"),
            egui::Sense::click(),
        );

        if ui.is_rect_visible(rect) {
            let is_hovered = ui.rect_contains_pointer(interact_rect);

            let show_contents = !self.empty || !is_hovered;

            if show_contents {
                contents(
                    &mut ui.child_ui(rect, Layout::left_to_right(Align::TOP)),
                    depot,
                );
            }
        }

        (response, rect)
    }
}
