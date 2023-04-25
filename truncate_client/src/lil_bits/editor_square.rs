use eframe::egui::{self, Sense};
use epaint::{vec2, Stroke, TextureId};

use crate::theming::{Darken, Lighten, Theme};

use super::{character::CharacterOrient, tex::BGTexType, CharacterUI, Tex};

pub struct EditorSquareUI {
    map_texture: TextureId,
    enabled: bool,
    root: bool,
    neighbors: Vec<bool>,
}

impl EditorSquareUI {
    pub fn new(map_texture: TextureId, neighbors: Vec<bool>) -> Self {
        debug_assert_eq!(neighbors.len(), 8);

        Self {
            enabled: false,
            root: false,
            map_texture,
            neighbors,
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
            let neighbors: Vec<_> = self
                .neighbors
                .iter()
                .map(|n| {
                    if *n {
                        BGTexType::Land
                    } else {
                        BGTexType::Water
                    }
                })
                .collect();

            let tile_type = if self.enabled {
                BGTexType::Land
            } else {
                BGTexType::Water
            };

            let ts = rect.width() * 0.25;
            let tile_rect = rect.shrink(ts);

            for (tex, translate) in Tex::resolve_bg_tile(tile_type, neighbors)
                .into_iter()
                .zip([vec2(-ts, -ts), vec2(ts, -ts), vec2(ts, ts), vec2(-ts, ts)].into_iter())
            {
                tex.render(self.map_texture, tile_rect.translate(translate), ui);
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
