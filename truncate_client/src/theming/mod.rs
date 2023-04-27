use std::ops::Range;

pub mod mapper;
pub mod tex;

use eframe::egui::{self, Margin};
use epaint::{hex_color, Color32, Hsva};

#[derive(Debug, Clone)]
pub struct Theme {
    pub friend: Color32,
    pub enemy: Color32,
    pub text: Color32,
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
            friend: hex_color!("#C3CEEA"),
            enemy: hex_color!("#F7BDB6"),
            text: hex_color!("#333333"),
            selection: hex_color!("#D78D1D"),
            background: hex_color!("#202020"),
            outlines: hex_color!("#9B9B9B"),
            addition: hex_color!("#55b14c"),
            modification: hex_color!("#9452ad"),
            defeated: hex_color!("#944D5E"),
            grid_size: 50.0,
            letter_size: 25.0,
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

pub trait Darken {
    fn darken(&self) -> Self;
}

impl Darken for Color32 {
    fn darken(&self) -> Self {
        let mut color = Hsva::from(*self);
        color.v *= 0.5;
        color.into()
    }
}

pub trait Lighten {
    fn lighten(&self) -> Self;
}

impl Lighten for Color32 {
    fn lighten(&self) -> Self {
        let mut color = Hsva::from(*self);
        color.v *= 2.0;
        if color.v > 1.0 {
            color.v = 1.0;
        }
        color.into()
    }
}
