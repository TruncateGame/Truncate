use eframe::egui::{self, Sense};
use epaint::Stroke;

use crate::theming::{Darken, Lighten, Theme};

use super::{character::CharacterOrient, CharacterUI};

pub struct EditorSquareUI {
    enabled: bool,
    root: bool,
}

impl EditorSquareUI {
    pub fn new() -> Self {
        Self {
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

    pub fn render(&self, ui: &mut egui::Ui, theme: &Theme) -> egui::Response {
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
            if self.enabled {
                ui.painter().rect_filled(rect, 0.0, theme.text.lighten());
                ui.painter()
                    .rect_stroke(rect, 0.0, Stroke::new(1.0, theme.outlines));
            } else {
                ui.painter().rect_filled(rect, 0.0, theme.text.darken());
            }

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

#[derive(Hash)]
pub enum EditorBarEdge {
    Top,
    Right,
    Bottom,
    Left,
}

pub struct EditorBarUI {
    edge: EditorBarEdge,
}

impl EditorBarUI {
    pub fn new(edge: EditorBarEdge) -> Self {
        Self { edge }
    }

    pub fn render(&self, ui: &mut egui::Ui, mut rect: egui::Rect, theme: &Theme) -> egui::Response {
        match self.edge {
            EditorBarEdge::Top => {
                rect.set_bottom(rect.top() + theme.grid_size);
                rect.set_left(rect.left() + theme.grid_size);
                rect.set_right(rect.right() - theme.grid_size);
            }
            EditorBarEdge::Right => {
                rect.set_left(rect.right() - theme.grid_size);
                rect.set_top(rect.top() + theme.grid_size);
                rect.set_bottom(rect.bottom() - theme.grid_size);
            }
            EditorBarEdge::Bottom => {
                rect.set_top(rect.bottom() - theme.grid_size);
                rect.set_left(rect.left() + theme.grid_size);
                rect.set_right(rect.right() - theme.grid_size);
            }
            EditorBarEdge::Left => {
                rect.set_right(rect.left() + theme.grid_size);
                rect.set_top(rect.top() + theme.grid_size);
                rect.set_bottom(rect.bottom() - theme.grid_size);
            }
        };

        let response = ui.interact(rect, ui.id().with(&self.edge), Sense::click());

        if response.hovered() {
            ui.painter().rect_filled(
                rect.shrink(theme.tile_margin),
                theme.rounding,
                theme.outlines,
            );
            CharacterUI::new('+', CharacterOrient::North).render_with_color(
                ui,
                rect.shrink(theme.tile_margin),
                theme,
                theme.addition,
            );
        }

        response
    }
}
