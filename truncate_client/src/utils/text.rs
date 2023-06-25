use eframe::egui::{self, widget_text::WidgetTextGalley, Id, Sense};
use epaint::{vec2, Color32, Pos2, Rect, TextureHandle, Vec2};

use super::{
    glyph_meaure::GlyphMeasure,
    tex::{render_texs_clockwise, Tex, Tint},
};

pub struct TextHelper<'a> {
    original_text: &'a str,
    font: &'static str,
    size: f32,
    galley: WidgetTextGalley,
}

impl<'a> TextHelper<'a> {
    pub fn heavy(text: &'a str, size: f32, ui: &mut egui::Ui) -> TextHelper<'a> {
        let font = egui::FontSelection::FontId(egui::FontId {
            size: size,
            family: egui::FontFamily::Name("Truncate-Heavy".into()),
        });
        let galley = egui::WidgetText::RichText(egui::RichText::new(text))
            .into_galley(ui, None, 1000.0, font); // TODO: Use a non-wrapping method so this giant float isn't here
        Self {
            original_text: text,
            font: "Truncate-Heavy",
            size,
            galley,
        }
    }

    pub fn size(&self) -> Vec2 {
        self.galley.size()
    }

    pub fn paint_at(self, pos: Pos2, color: Color32, ui: &mut egui::Ui) {
        self.galley
            .paint_with_color_override(ui.painter(), pos, color);
    }

    pub fn paint(self, color: Color32, ui: &mut egui::Ui) -> egui::Response {
        let text_size = self.galley.galley.mesh_bounds.size();

        let (mut text_rect, text_resp) =
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
        self.paint_button(true, button_color, text_color, map_texture, ui)
    }

    pub fn button(
        self,
        button_color: Color32,
        text_color: Color32,
        map_texture: &TextureHandle,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        self.paint_button(false, button_color, text_color, map_texture, ui)
    }

    fn paint_button(
        self,
        full_width: bool,
        button_color: Color32,
        text_color: Color32,
        map_texture: &TextureHandle,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        let text_size = self.galley.galley.mesh_bounds.size();

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
        let (mut button_rect, button_resp) =
            ui.allocate_exact_size(vec2(button_width, button_tile_size * 2.0), Sense::click());

        if button_resp.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
            button_rect = button_rect.translate(vec2(0.0, -2.0));
        }

        render_texs_clockwise(button_texs.tint(button_color), button_rect, map_texture, ui);

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
