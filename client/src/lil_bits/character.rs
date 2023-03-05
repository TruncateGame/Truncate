use eframe::egui;
use epaint::{Color32, TextShape};
use std::f32;

use crate::theming::Theme;

pub enum CharacterOrient {
    North,
    East,
    South,
    West,
}

pub struct CharacterUI {
    letter: char,
    orientation: CharacterOrient,
    hovered: bool,
    selected: bool,
    ghost: bool,
    truncated: bool,
    defeated: bool,
    size: f32,
}

impl CharacterUI {
    pub fn new(letter: char, orientation: CharacterOrient) -> Self {
        Self {
            letter,
            orientation,
            hovered: false,
            selected: false,
            ghost: false,
            truncated: false,
            defeated: false,
            size: 30.0,
        }
    }

    pub fn hovered(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn ghost(mut self, ghost: bool) -> Self {
        self.ghost = ghost;
        self
    }

    pub fn truncated(mut self, truncated: bool) -> Self {
        self.truncated = truncated;
        self
    }

    pub fn defeated(mut self, defeated: bool) -> Self {
        self.defeated = defeated;
        self
    }
}

impl CharacterUI {
    fn char_color(&self, theme: &Theme) -> Color32 {
        if self.ghost {
            theme.friend.dark
        } else if self.hovered {
            theme.text.dark
        } else if self.selected {
            theme.text.dark
        } else if self.defeated {
            theme.defeated
        } else if self.truncated {
            theme.text.light
        } else {
            theme.text.base
        }
    }

    pub fn render(self, ui: &mut egui::Ui, rect: egui::Rect, theme: &Theme) {
        let color = self.char_color(theme);
        self.render_with_color(ui, rect, color);
    }

    pub fn render_with_color(self, ui: &mut egui::Ui, rect: egui::Rect, color: Color32) {
        let galley = ui.painter().layout_no_wrap(
            self.letter.to_string(),
            egui::FontId::new(self.size, egui::FontFamily::Name("Tile".into())),
            color,
        );

        let (angle, pos, shift) = match self.orientation {
            CharacterOrient::North => (
                0.0,
                rect.left_top(),
                egui::Vec2::new((rect.width() - galley.size().x) * 0.5, self.size * -0.2),
            ),
            CharacterOrient::East => todo!("Render Sideways characters"),
            CharacterOrient::South => (
                f32::consts::PI,
                rect.right_bottom(),
                egui::Vec2::new((rect.width() - galley.size().x) * -0.5, self.size * 0.2),
            ),
            CharacterOrient::West => todo!("Render Sideways characters"),
        };

        ui.painter().add(TextShape {
            angle,
            override_text_color: Some(color),
            ..TextShape::new(pos + shift, galley)
        });
    }
}
