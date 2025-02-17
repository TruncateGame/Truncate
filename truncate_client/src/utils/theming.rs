use std::ops::Range;

use eframe::egui::{self, Margin};
use epaint::{hex_color, Color32, Hsva};

#[derive(Debug, Clone)]
pub struct Theme {
    pub use_old_art: bool, // TODO: Remove after art change has flushed through
    pub daytime: bool,
    pub water: Color32,
    pub grass: Color32,
    pub text: Color32,
    pub faded: Color32,
    pub button_primary: Color32,
    pub button_emphasis: Color32,
    pub button_secondary: Color32,
    pub button_scary: Color32,
    pub ring_selected: Color32,
    pub ring_selected_hovered: Color32,
    pub ring_hovered: Color32,
    pub ring_added: Color32,
    pub ring_modified: Color32,
    pub word_valid: Color32,
    pub word_invalid: Color32,
    pub gold_medal: Color32,
    pub grid_size: f32,
    pub letter_size: f32,
    pub tile_margin: f32,
    pub rounding: f32,
    pub animation_time: f32,
    pub mobile_breakpoint: f32,
}

impl Theme {
    pub fn day() -> Self {
        Self {
            use_old_art: false,
            daytime: true,
            water: hex_color!("#0BADFF"),
            grass: hex_color!("#7BCB69"),
            text: hex_color!("#333333"),
            faded: hex_color!("#777777"),
            button_primary: hex_color!("#FFDE85"),
            button_emphasis: hex_color!("#EDBBFF"),
            button_secondary: hex_color!("#B1DAF9"),
            button_scary: hex_color!("#FF8E8E"),
            ring_selected: hex_color!("#FFBE0B"),
            ring_selected_hovered: hex_color!("#FFDE85"),
            ring_hovered: hex_color!("#CDF7F6"),
            ring_added: hex_color!("#0AFFC6"),
            ring_modified: hex_color!("#FC3692"),
            word_valid: hex_color!("#00A37D"),
            word_invalid: hex_color!("#89043D"),
            gold_medal: hex_color!("#E0A500"),
            grid_size: 50.0,
            letter_size: 25.0,
            tile_margin: 4.0,
            rounding: 10.0,
            animation_time: 0.05,
            mobile_breakpoint: 800.0,
        }
    }

    pub fn old_day() -> Self {
        Self {
            use_old_art: true,
            daytime: true,
            water: hex_color!("#50a7e8"),
            grass: hex_color!("#7BCB69"),
            text: hex_color!("#333333"),
            faded: hex_color!("#777777"),
            button_primary: hex_color!("#FFDE85"),
            button_emphasis: hex_color!("#EDBBFF"),
            button_secondary: hex_color!("#B1DAF9"),
            button_scary: hex_color!("#FF8E8E"),
            ring_selected: hex_color!("#FFBE0B"),
            ring_selected_hovered: hex_color!("#FFDE85"),
            ring_hovered: hex_color!("#CDF7F6"),
            ring_added: hex_color!("#0AFFC6"),
            ring_modified: hex_color!("#FC3692"),
            word_valid: hex_color!("#00A37D"),
            word_invalid: hex_color!("#89043D"),
            gold_medal: hex_color!("#E0A500"),
            grid_size: 50.0,
            letter_size: 25.0,
            tile_margin: 4.0,
            rounding: 10.0,
            animation_time: 0.05,
            mobile_breakpoint: 800.0,
        }
    }

    pub fn fog() -> Self {
        Self {
            use_old_art: false,
            daytime: true,
            water: hex_color!("#000000"),
            grass: hex_color!("#7BCB69"),
            text: hex_color!("#333333"),
            faded: hex_color!("#777777"),
            button_primary: hex_color!("#FFDE85"),
            button_emphasis: hex_color!("#EDBBFF"),
            button_secondary: hex_color!("#B1DAF9"),
            button_scary: hex_color!("#FF8E8E"),
            ring_selected: hex_color!("#FFBE0B"),
            ring_selected_hovered: hex_color!("#FFDE85"),
            ring_hovered: hex_color!("#CDF7F6"),
            ring_added: hex_color!("#0AFFC6"),
            ring_modified: hex_color!("#FC3692"),
            word_valid: hex_color!("#00A37D"),
            word_invalid: hex_color!("#89043D"),
            gold_medal: hex_color!("#E0A500"),
            grid_size: 50.0,
            letter_size: 25.0,
            tile_margin: 4.0,
            rounding: 10.0,
            animation_time: 0.05,
            mobile_breakpoint: 800.0,
        }
    }

    pub fn night() -> Self {
        Self {
            use_old_art: false,
            daytime: false,
            water: hex_color!("#000000"),
            grass: hex_color!("#112b15"),
            text: hex_color!("#FFFFFF"),
            faded: hex_color!("#CCCCCC"),
            button_primary: hex_color!("#8F6900"),
            button_emphasis: hex_color!("#EDBBFF"),
            button_secondary: hex_color!("#094472"),
            button_scary: hex_color!("#FF8E8E"),
            ring_selected: hex_color!("#FFBE0B"),
            ring_selected_hovered: hex_color!("#FFDE85"),
            ring_hovered: hex_color!("#CDF7F6"),
            ring_added: hex_color!("#0AFFC6"),
            ring_modified: hex_color!("#FC3692"),
            word_valid: hex_color!("#00A37D"),
            word_invalid: hex_color!("#89043D"),
            gold_medal: hex_color!("#E0A500"),
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
        pad_by: (f32, f32),
    ) -> ((f32, f32), Margin, Self) {
        let mut ideal_grid = avail_space.width() / (board_width as f32 + pad_by.0);
        let y_space = avail_space.height() / (board_height as f32 + pad_by.1);
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
    fn slighten(&self) -> Self;
    fn lighten(&self) -> Self;
    fn pastel(&self) -> Self;
}

impl Lighten for Color32 {
    fn slighten(&self) -> Self {
        let mut color = Hsva::from(*self);
        color.v *= 1.2;
        color.s *= 0.9;
        if color.v > 1.0 {
            color.v = 1.0;
        }
        color.into()
    }

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
