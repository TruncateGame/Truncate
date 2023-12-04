use std::ops::Range;

use eframe::egui::{self, Margin};
use epaint::{hex_color, Color32, Hsva};

#[derive(Debug, Clone)]
pub struct Theme {
    pub water: Color32,
    pub grass: Color32,
    pub enemy: Color32,
    pub text: Color32,
    pub selection: Color32,
    pub outlines: Color32,
    pub addition: Color32,
    pub modification: Color32,
    pub defeated: Color32,
    pub grid_size: f32,
    pub letter_size: f32,
    pub tile_margin: f32,
    pub rounding: f32,
    pub animation_time: f32,
    pub mobile_breakpoint: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            water: hex_color!("#000000"),
            grass: hex_color!("#7BCB69"),
            enemy: hex_color!("#F7BDB6"),
            text: hex_color!("#333333"),
            selection: hex_color!("#D78D1D"),
            outlines: hex_color!("#9B9B9B"),
            addition: hex_color!("#cdff9d"),
            modification: hex_color!("#9452ad"),
            defeated: hex_color!("#944D5E"),
            grid_size: 50.0,
            letter_size: 25.0,
            tile_margin: 4.0,
            rounding: 10.0,
            animation_time: 0.05,
            mobile_breakpoint: 800.0,
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
        pad_by: (usize, usize),
    ) -> ((f32, f32), Margin, Self) {
        let mut ideal_grid = avail_space.width() / (board_width + pad_by.0) as f32;
        let y_space = avail_space.height() / (board_height + pad_by.1) as f32;
        if y_space < ideal_grid {
            ideal_grid = y_space;
        }

        let scale = ideal_grid / self.grid_size;
        let scale = scale.clamp(scale_bounds.start, scale_bounds.end);

        let width = (board_width) as f32 * (self.grid_size * scale);
        let height = (board_height) as f32 * (self.grid_size * scale);

        (
            (width, height),
            Margin::symmetric(
                (avail_space.width() - width) / 2.0,
                (avail_space.height() - height) / 2.0,
            ),
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

pub trait Diaphanize {
    fn diaphanize(&self) -> Self;
}

impl Diaphanize for Color32 {
    fn diaphanize(&self) -> Self {
        let mut color = Hsva::from(*self);
        color.a *= 0.5;
        color.into()
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
    fn pastel(&self) -> Self;
}

impl Lighten for Color32 {
    fn lighten(&self) -> Self {
        let mut color = Hsva::from(*self);
        color.v *= 2.0;
        color.s *= 0.9;
        if color.v > 1.0 {
            color.v = 1.0;
        }
        color.into()
    }

    fn pastel(&self) -> Self {
        let mut color = Hsva::from(*self);
        color.v *= 3.0;
        color.s *= 0.7;
        if color.v > 1.0 {
            color.v = 1.0;
        }
        color.into()
    }
}
