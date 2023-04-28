use std::sync::{Arc, Mutex};

use ab_glyph::{Font, FontRef, ScaleFont};
use epaint::{vec2, Vec2};

struct GlyphMeasurer {
    font: FontRef<'static>,
}

impl GlyphMeasurer {
    fn new() -> Self {
        Self {
            font: ab_glyph::FontRef::try_from_slice(include_bytes!(
                "../font/PressStart2P-Regular.ttf"
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
}

#[derive(Clone)]
pub struct GlyphMeasure {
    inner: Arc<Mutex<GlyphMeasurer>>,
}

impl GlyphMeasure {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(GlyphMeasurer::new())),
        }
    }

    pub fn measure(&self, glyph_id: char) -> Vec2 {
        self.inner.lock().unwrap().measure(glyph_id)
    }
}
