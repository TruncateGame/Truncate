use std::ops::Range;

use eframe::egui::{self, Margin};
use epaint::{hex_color, Color32};

#[derive(Debug, Clone)]
pub struct InteractTheme {
    pub base: Color32,
    pub dark: Color32,
    pub light: Color32,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub friend: InteractTheme,
    pub enemy: InteractTheme,
    pub text: InteractTheme,
    pub selection: Color32,
    pub background: Color32,
    pub outlines: Color32,
    pub addition: Color32,
    pub modification: Color32,
    pub defeated: Color32,
    pub grid_size: f32,
    pub letter_size: f32,
    pub tile_margin: f32,
    pub rounding: f32,
    pub animation_time: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            friend: InteractTheme {
                base: hex_color!("#E1E6F4"),
                dark: hex_color!("#C3CEEA"),
                light: hex_color!("#FFFFFF"),
            },
            enemy: InteractTheme {
                base: hex_color!("#F7BDB6"),
                dark: hex_color!("#F39B91"),
                light: hex_color!("#FBDEDA"),
            },
            text: InteractTheme {
                base: hex_color!("#333333"),
                dark: hex_color!("#1E1E1E"),
                light: hex_color!("#6B6B6B"),
            },
            selection: hex_color!("#D78D1D"),
            background: hex_color!("#202020"),
            outlines: hex_color!("#9B9B9B"),
            addition: hex_color!("#9CC69B"),
            modification: hex_color!("#9055A2"),
            defeated: hex_color!("#944D5E"),
            grid_size: 50.0,
            letter_size: 30.0,
            tile_margin: 4.0,
            rounding: 10.0,
            animation_time: 0.05,
        }
    }
}

impl Theme {
    pub fn calc_rescale(
        &self,
        avail_space: &egui::Rect,
        board_width: usize,
        board_height: usize,
        scale_bounds: Range<f32>,
    ) -> (Margin, Self) {
        let mut ideal_grid = avail_space.width() / (board_width + 2) as f32;
        let y_space = avail_space.height() / (board_height + 2) as f32;
        if y_space < ideal_grid {
            ideal_grid = y_space;
        }

        let scale = ideal_grid / self.grid_size;
        let scale = scale.clamp(scale_bounds.start, scale_bounds.end);

        let width = (board_width) as f32 * (self.grid_size * scale);

        (
            Margin::symmetric((avail_space.width() - width) / 2.0, self.grid_size),
            self.rescale(scale),
        )
    }

    pub fn rescale(&self, scale: f32) -> Self {
        Self {
            grid_size: self.grid_size * scale,
            letter_size: self.letter_size * scale,
            tile_margin: self.tile_margin * scale,
            rounding: self.rounding * scale,
            ..self.clone()
        }
    }
}
