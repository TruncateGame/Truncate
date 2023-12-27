use std::sync::{Arc, Mutex};

use ab_glyph::{Font, FontRef, OutlinedGlyph, ScaleFont};
use epaint::{vec2, Color32, ColorImage, Vec2};

struct InnerGlypher {
    font: FontRef<'static>,
}

impl InnerGlypher {
    fn new() -> Self {
        Self {
            font: ab_glyph::FontRef::try_from_slice(include_bytes!(
                "../../font/PressStart2P-Regular.ttf"
            ))
            .unwrap(),
        }
    }

    fn measure(&self, glyph_id: char) -> Vec2 {
        let font = self.font.as_scaled(ab_glyph::PxScale::from(1000.0));

        let ascent = font.ascent();
        let descent = font.descent();
        let line_gap = font.line_gap();

        let outer_height = ascent - descent + line_gap;
        let outer_width = font.h_advance(font.glyph_id(glyph_id));
        let sideload = font.h_side_bearing(font.glyph_id(glyph_id));

        if let Some(glyph) = font.outline_glyph(font.scaled_glyph(glyph_id)) {
            let inner_rect = glyph.px_bounds();

            vec2(
                (((outer_width - inner_rect.width()) / 2.0) - sideload) / 1000.0,
                ((outer_height - inner_rect.height()) / 2.0) / 1000.0,
            )
        } else {
            vec2(0.0, 0.0)
        }
    }

    fn outline(&self, glyph_id: char, scale: usize) -> Option<OutlinedGlyph> {
        let font = self.font.as_scaled(ab_glyph::PxScale::from(scale as f32));

        font.outline_glyph(font.scaled_glyph(glyph_id))
    }
}

#[derive(Clone)]
pub struct Glypher {
    inner: Arc<Mutex<InnerGlypher>>,
}

impl Glypher {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerGlypher::new())),
        }
    }

    pub fn measure(&self, glyph_id: char) -> Vec2 {
        self.inner.lock().unwrap().measure(glyph_id)
    }

    pub fn render_to_image(&self, glyph_ids: Vec<char>, scale: usize) -> BaseTileGlyphs {
        let inner = self.inner.lock().unwrap();

        let glyphs = glyph_ids
            .into_iter()
            .map(|g| {
                let mut image = ColorImage::new([scale, scale], Color32::TRANSPARENT);
                inner.outline(g, scale).map(|og| {
                    og.draw(|x, y, v| {
                        if v > 0.0 {
                            image[(x as usize, y as usize)] =
                                Color32::from_rgba_premultiplied(255, 255, 255, (v * 255.0) as u8);
                        }
                    })
                });
                (g, image)
            })
            .collect();

        BaseTileGlyphs { glyphs }
    }
}

pub struct BaseTileGlyphs {
    pub glyphs: Vec<(char, ColorImage)>,
}
