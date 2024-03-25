use std::sync::{Arc, Mutex};

use crate::utils::mapper::ImageMusher;
use ab_glyph::{Font, FontRef, ScaleFont};
use epaint::{
    ahash::{HashMap, HashMapExt},
    Color32, ColorImage,
};

struct InnerGlypher {
    font: FontRef<'static>,
    cache: HashMap<(char, usize), ColorImage>,
}

impl InnerGlypher {
    fn new() -> Self {
        Self {
            font: ab_glyph::FontRef::try_from_slice(include_bytes!(
                "../../font/PressStart2P-Regular.ttf"
            ))
            .unwrap(),
            cache: HashMap::with_capacity(256),
        }
    }

    fn cached_paint(&mut self, glyph_id: char, scale: usize) -> ColorImage {
        self.cache
            .entry((glyph_id, scale))
            .or_insert_with(|| paint(&self.font, glyph_id, scale))
            .clone()
    }
}

fn paint(font: &FontRef<'static>, glyph_id: char, scale: usize) -> ColorImage {
    let font = font.as_scaled(ab_glyph::PxScale::from(scale as f32));

    let image = font
        .outline_glyph(font.scaled_glyph(glyph_id))
        .map(|og| {
            let mut image = ColorImage::new(
                [
                    og.px_bounds().width().ceil() as usize,
                    og.px_bounds().height().ceil() as usize,
                ],
                Color32::TRANSPARENT,
            );

            og.draw(|x, y, v| {
                if v > 0.0 {
                    image[(x as usize, y as usize)] =
                        Color32::from_rgba_premultiplied(255, 255, 255, (v * 255.0) as u8);
                }
            });

            image.trim();

            image
        })
        .unwrap_or_default();

    image
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

    pub fn paint(&self, glyph_id: char, scale: usize) -> ColorImage {
        self.inner.lock().unwrap().cached_paint(glyph_id, scale)
    }
}

pub struct BaseTileGlyphs {
    pub glyphs: Vec<(char, ColorImage)>,
}
