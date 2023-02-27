use eframe::{
    egui::{self, Layout},
    emath::Align,
};
use epaint::Stroke;

use crate::theming::Theme;

use super::{character::CharacterOrient, CharacterUI};

pub struct SquareUI {
    enabled: bool,
    empty: bool,
    selected: bool,
    overlay: Option<char>,
}

impl SquareUI {
    pub fn new() -> Self {
        Self {
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
        theme: &Theme,
        contents: impl FnOnce(&mut egui::Ui, &Theme),
    ) -> egui::Response {
        let (rect, mut response) = ui.allocate_exact_size(
            egui::vec2(theme.grid_size, theme.grid_size),
            egui::Sense::click(),
        );

        if ui.is_rect_visible(rect) {
            if self.enabled {
                if self.empty && self.selected {
                    ui.painter().rect_filled(rect, 4.0, theme.selection);
                } else {
                    ui.painter().rect_filled(rect, 4.0, theme.background);
                }

                ui.painter()
                    .rect_stroke(rect, 0.0, Stroke::new(1.0, theme.outlines));
            }

            // TODO: The inner components here capture the hover event,
            // which clashes. Need some way to render the innards without
            // interactivity.
            if !self.empty || !response.hovered() {
                contents(
                    &mut ui.child_ui(rect, Layout::left_to_right(Align::TOP)),
                    theme,
                );
            }

            // This is maybe a kludge to resolve the above comment.
            // If we're in the "empty" state, we detect our interactivity
            // after drawing the inners, so that this top layer
            // catches the senses.
            if self.empty {
                response = ui.allocate_rect(rect, egui::Sense::click());

                if response.hovered() {
                    if let Some(overlay) = self.overlay {
                        ui.painter()
                            .rect_filled(rect.shrink(4.0), 4.0, theme.text.dark);
                        CharacterUI::new(overlay, CharacterOrient::North)
                            .ghost(true)
                            .render(ui, rect.shrink(4.0), theme);
                    } else {
                        ui.painter()
                            .rect_filled(rect.shrink(4.0), 4.0, theme.outlines);
                    }
                }
            }
        }

        response
    }
}
