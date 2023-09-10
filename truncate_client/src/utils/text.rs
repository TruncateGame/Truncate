use eframe::egui::{self, widget_text::WidgetTextGalley, Id, Sense};
use epaint::{vec2, Color32, Pos2, Rect, TextureHandle, Vec2};

use super::{
    glyph_meaure::GlyphMeasure,
    tex::{paint_dialog_background, render_texs_clockwise, Tex, Tint},
};

const DIALOG_TIME_PER_CHAR: f32 = 0.05;

pub struct TextHelper<'a> {
    original_text: &'a str,
    font: &'static str,
    size: f32,
    max_width: Option<f32>,
    galley: WidgetTextGalley,
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
            font: "Truncate-Heavy",
            size,
            max_width,
            galley,
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
            font: "Truncate-Light",
            size,
            max_width,
            galley,
        }
    }

    pub fn size(&self) -> Vec2 {
        self.galley.size()
    }

    pub fn mesh_size(&self) -> Vec2 {
        self.galley.galley.mesh_bounds.size()
    }

    pub fn paint_at(self, pos: Pos2, color: Color32, ui: &mut egui::Ui) {
        self.galley
            .paint_with_color_override(ui.painter(), pos, color);
    }

    pub fn get_partial_slice(&self, time_passed: f32, ui: &mut egui::Ui) -> Option<Self> {
        let breaks = self
            .original_text
            .char_indices()
            .filter_map(|(i, c)| if c == ' ' { Some(i) } else { None })
            .collect::<Vec<_>>();
        let animation_duration = breaks.len() as f32 * DIALOG_TIME_PER_CHAR;
        if time_passed > animation_duration {
            return None;
        }

        let word_count = (breaks.len() as f32 * (time_passed / animation_duration)) as usize;
        let shortened_text = &self.original_text[0..=breaks[word_count.saturating_sub(1)]];

        Some(TextHelper::light(
            &shortened_text,
            self.size,
            self.max_width,
            ui,
        ))
    }

    pub fn paint(self, color: Color32, ui: &mut egui::Ui) -> egui::Response {
        let text_size = self.galley.galley.mesh_bounds.size();

        let (text_rect, text_resp) =
            ui.allocate_exact_size(vec2(text_size.x, text_size.y), Sense::hover());

        let mut offset = (text_rect.size() - text_size) / 2.0;

        // Recenter the font to strict glyph bounds
        if self.font == "Truncate-Heavy" {
            // TODO: Don't calculate this every frame
            let glyph_measure: GlyphMeasure =
                ui.memory_mut(|mem| mem.data.get_temp(Id::null()).unwrap());
            let char_height = glyph_measure.measure(self.original_text.chars().next().unwrap());
            offset.y += char_height.y * 2.0 * self.size;
        }

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
            ui.horizontal(|ui| {
                let centered_offset = (ui.available_width() - button_width) * 0.5;
                ui.add_space(centered_offset);
                ui.allocate_exact_size(vec2(button_width, button_tile_size * 2.0), Sense::click())
            })
            .inner
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

        let mut offset = (button_rect.size() - text_size) / 2.0;

        // Recenter the font to strict glyph bounds
        if self.font == "Truncate-Heavy" {
            // TODO: Don't calculate this every frame
            let glyph_measure: GlyphMeasure =
                ui.memory_mut(|mem| mem.data.get_temp(Id::null()).unwrap());
            let char_height = glyph_measure.measure(self.original_text.chars().next().unwrap());
            offset.y += char_height.y * 2.0 * self.size;
        }

        self.paint_at(button_rect.min + offset, text_color, ui);

        button_resp
    }
}
