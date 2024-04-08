use std::sync::Arc;

use eframe::egui::{self, Sense};
use epaint::{emath::Align2, pos2, vec2, Color32, Galley, Pos2, Rect, TextureHandle, Vec2};

use super::tex::{paint_dialog_background, render_texs_clockwise, Tex, Tint};

const DIALOG_TIME_PER_CHAR: f32 = 0.05;

pub enum TextStyle {
    Light,
    Heavy,
}

pub struct TextHelper<'a> {
    original_text: &'a str,
    size: f32,
    max_width: Option<f32>,
    galley: Arc<Galley>,
    text_style: TextStyle,
}

impl<'a> TextHelper<'a> {
    pub fn heavy(
        text: &'a str,
        size: f32,
        max_width: Option<f32>,
        ui: &mut egui::Ui,
    ) -> TextHelper<'a> {
        let font = egui::FontSelection::FontId(egui::FontId {
            size: size,
            family: egui::FontFamily::Name("Truncate-Heavy".into()),
        });
        let galley = egui::WidgetText::RichText(egui::RichText::new(text)).into_galley(
            ui,
            None,
            max_width.unwrap_or(10000.0),
            font,
        );
        Self {
            original_text: text,
            size,
            max_width,
            galley,
            text_style: TextStyle::Heavy,
        }
    }

    pub fn light(
        text: &'a str,
        size: f32,
        max_width: Option<f32>,
        ui: &mut egui::Ui,
    ) -> TextHelper<'a> {
        let font = egui::FontSelection::FontId(egui::FontId {
            size: size,
            family: egui::FontFamily::Proportional,
        });
        let galley = egui::WidgetText::RichText(egui::RichText::new(text)).into_galley(
            ui,
            None,
            max_width.unwrap_or(10000.0),
            font,
        );
        Self {
            original_text: text,
            size,
            max_width,
            galley,
            text_style: TextStyle::Light,
        }
    }

    pub fn size(&self) -> Vec2 {
        self.galley.size()
    }

    pub fn mesh_size(&self) -> Vec2 {
        self.galley.mesh_bounds.size()
    }

    pub fn get_partial_slice(&self, time_passed: f32, ui: &mut egui::Ui) -> Option<Self> {
        let mut breaks = self
            .original_text
            .char_indices()
            .filter_map(|(i, c)| if c == ' ' { Some(i) } else { None })
            .collect::<Vec<_>>();
        breaks.push(self.original_text.len() - 1);
        let animation_duration = breaks.len() as f32 * DIALOG_TIME_PER_CHAR;
        if time_passed > animation_duration {
            return None;
        }

        let word_count = (breaks.len() as f32 * (time_passed / animation_duration)) as usize;
        let shortened_text = &self.original_text[0..=breaks[word_count.saturating_sub(1)]];

        match self.text_style {
            TextStyle::Light => Some(TextHelper::light(
                &shortened_text,
                self.size,
                self.max_width,
                ui,
            )),
            TextStyle::Heavy => Some(TextHelper::heavy(
                &shortened_text,
                self.size,
                self.max_width,
                ui,
            )),
        }
    }

    pub fn paint_at(self, pos: Pos2, color: Color32, ui: &mut egui::Ui) {
        ui.painter()
            .galley_with_override_text_color(pos, self.galley, color);
    }

    pub fn paint_within(self, bounds: Rect, alignment: Align2, color: Color32, ui: &mut egui::Ui) {
        let dims = self.mesh_size();
        let Align2([ha, va]) = alignment;
        let x_pos = match ha {
            egui::Align::Min => bounds.left(),
            egui::Align::Center => bounds.left() + (bounds.width() - dims.x) / 2.0,
            egui::Align::Max => bounds.left() + (bounds.width() - dims.x),
        };
        let y_pos = match va {
            egui::Align::Min => bounds.top(),
            egui::Align::Center => bounds.top() + (bounds.height() - dims.y) / 2.0,
            egui::Align::Max => bounds.top() + (bounds.height() - dims.y),
        };

        ui.painter()
            .galley_with_override_text_color(pos2(x_pos, y_pos), self.galley, color);
    }

    pub fn paint(self, color: Color32, ui: &mut egui::Ui, centered: bool) -> egui::Response {
        let text_size = self.galley.mesh_bounds.size();

        let (text_rect, text_resp) = if centered {
            ui.horizontal(|ui| {
                let centered_offset = (ui.available_width() - text_size.x) * 0.5;
                ui.add_space(centered_offset);
                ui.allocate_exact_size(vec2(text_size.x, text_size.y), Sense::hover())
            })
            .inner
        } else {
            ui.allocate_exact_size(vec2(text_size.x, text_size.y), Sense::hover())
        };

        let offset = (text_rect.size() - text_size) / 2.0;
        self.paint_at(text_rect.min + offset, color, ui);

        text_resp
    }

    pub fn full_button(
        self,
        button_color: Color32,
        text_color: Color32,
        map_texture: &TextureHandle,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        self.paint_tile_background(true, false, true, button_color, text_color, map_texture, ui)
    }

    pub fn centered_button(
        self,
        button_color: Color32,
        text_color: Color32,
        map_texture: &TextureHandle,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        self.paint_tile_background(false, true, true, button_color, text_color, map_texture, ui)
    }

    pub fn button(
        self,
        button_color: Color32,
        text_color: Color32,
        map_texture: &TextureHandle,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        self.paint_tile_background(
            false,
            false,
            true,
            button_color,
            text_color,
            map_texture,
            ui,
        )
    }

    pub fn dialog(
        self,
        size_to_pos: Vec2,
        dialog_color: Color32,
        text_color: Color32,
        reserve_space_at_bottom: f32,
        map_texture: &TextureHandle,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        let (dialog_rect, dialog_resp) = paint_dialog_background(
            true,
            true,
            false,
            size_to_pos,
            dialog_color,
            map_texture,
            ui,
        );

        let dialog_size = dialog_rect.size() - vec2(0.0, reserve_space_at_bottom);
        let offset = (dialog_size - size_to_pos) / 2.0;

        self.paint_at(dialog_rect.min + offset, text_color, ui);

        dialog_resp
    }

    pub fn get_button_height(&self, _ui: &egui::Ui) -> f32 {
        let text_size = self.mesh_size();

        let button_texs = Tex::text_button(text_size.x / text_size.y * 0.7);
        let button_width = text_size.x + self.size * 2.0;

        let button_tile_size = button_width / (button_texs.len() / 2) as f32;

        button_tile_size * 2.0
    }

    fn paint_tile_background(
        self,
        full_width: bool,
        centered: bool,
        interactive: bool,
        background_color: Color32,
        text_color: Color32,
        map_texture: &TextureHandle,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        let text_size = self.mesh_size();

        let (button_texs, button_width) = if full_width {
            (
                Tex::text_button(ui.available_width() / text_size.y * 0.7),
                ui.available_width(),
            )
        } else {
            (
                Tex::text_button(text_size.x / text_size.y * 0.7),
                text_size.x + self.size * 2.0,
            )
        };

        let button_tile_size = button_width / (button_texs.len() / 2) as f32;
        let (mut button_rect, button_resp) = if centered {
            let (mut rect, row) = ui.allocate_exact_size(
                vec2(ui.available_width(), button_tile_size * 2.0),
                Sense::hover(),
            );
            let centered_offset = (rect.width() - button_width) * 0.5;
            rect = rect.shrink2(vec2(centered_offset, 0.0));

            let resp = ui.interact(rect, row.id.with("button"), Sense::click());

            (rect, resp)
        } else {
            ui.allocate_exact_size(vec2(button_width, button_tile_size * 2.0), Sense::click())
        };

        if interactive && button_resp.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
            button_rect = button_rect.translate(vec2(0.0, -2.0));
        }

        render_texs_clockwise(
            button_texs.tint(background_color),
            button_rect,
            map_texture,
            ui,
        );

        let offset = (button_rect.size() - text_size) / 2.0;
        self.paint_at(button_rect.min + offset, text_color, ui);

        button_resp
    }
}
