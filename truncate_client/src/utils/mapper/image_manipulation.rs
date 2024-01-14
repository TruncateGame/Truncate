use epaint::{Color32, ColorImage, Hsva, Rect};

// Sometimes we prefer to mush images in CPU-space so that they can live on the same texture,
// without the GPU having to tint multiple UV rects when painting.
// Generally this should only be done in places with heavy caches, like the mapping utils.
pub trait ImageMusher {
    fn hard_overlay(&mut self, other: &Self, position: [usize; 2]);
    fn tint(&mut self, tint: &Color32);
    fn recolor(&mut self, color: &Color32);
}

impl ImageMusher for ColorImage {
    fn hard_overlay(&mut self, other: &Self, position: [usize; 2]) {
        let min_x = position[0];
        let max_x = min_x + other.width();
        let min_y = position[1];
        let max_y = min_y + other.height();

        assert!(min_x <= max_x && max_x <= self.width());
        assert!(min_y <= max_y && max_y <= self.height());

        let self_stride = self.size[0];
        let other_stride = other.size[0];

        for (other_row, self_row) in (min_y..max_y).enumerate() {
            let other_row =
                &other.pixels[other_row * other_stride..other_row * other_stride + other.size[1]];
            self.pixels[self_row * self_stride + min_x..self_row * self_stride + max_x]
                .iter_mut()
                .zip(other_row)
                .for_each(|(base_px, other_px)| {
                    if other_px.a() > 0 {
                        *base_px = *other_px;
                    }
                });
        }
    }

    /// Tints white/grey images to tints of the given color
    fn tint(&mut self, tint: &Color32) {
        self.pixels.iter_mut().for_each(|px| {
            if px.a() == 0 {
                return;
            }
            let shade_factor = px.r() as f32 / 255.0;

            *px = Color32::from_rgb(
                (tint.r() as f32 * shade_factor) as u8,
                (tint.g() as f32 * shade_factor) as u8,
                (tint.b() as f32 * shade_factor) as u8,
            );
        });
    }

    fn recolor(&mut self, color: &Color32) {
        self.pixels.iter_mut().for_each(|px| {
            if px.a() > 0 {
                *px = *color;
            }
        });
    }
}
