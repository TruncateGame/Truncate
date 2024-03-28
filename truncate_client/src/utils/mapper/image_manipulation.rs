use epaint::{Color32, ColorImage, Rgba};

// Sometimes we prefer to mush images in CPU-space so that they can live on the same texture,
// without the GPU having to tint multiple UV rects when painting.
// Generally this should only be done in places with heavy caches, like the mapping utils.
pub trait ImageMusher {
    fn hard_overlay(&mut self, other: &Self, position: [usize; 2]);
    fn tint(&mut self, tint: &Color32);
    fn recolor(&mut self, color: &Color32);
    fn trim(&mut self);
    fn flip_y(&mut self);
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
                &other.pixels[other_row * other_stride..other_row * other_stride + other_stride];
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

    fn trim(&mut self) {
        let are_blank = |px: &Color32| px.a() == 0;

        let stride = self.width();
        while self.pixels[0..stride].iter().all(are_blank) {
            self.pixels.drain(0..stride);
            self.size[1] -= 1;
        }

        loop {
            let len = self.pixels.len();
            if self.pixels[len - stride..len].iter().all(are_blank) {
                self.pixels.drain(len - stride..len);
                self.size[1] -= 1;
            } else {
                break;
            }
        }

        loop {
            let width = self.width();
            if self.pixels.iter().step_by(width).all(are_blank) {
                let mut i = 0;
                self.pixels.retain(|_| {
                    i += 1;
                    (i - 1) % width > 0
                });
                self.size[0] -= 1;
            } else {
                break;
            }
        }

        loop {
            let width = self.width();
            if self.pixels.iter().rev().step_by(width).all(are_blank) {
                let mut i = self.pixels.len();
                self.pixels.retain(|_| {
                    i -= 1;
                    i % width > 0
                });
                self.size[0] -= 1;
            } else {
                break;
            }
        }
    }

    fn flip_y(&mut self) {
        self.pixels.reverse();
    }
}

pub fn alpha_blend(base: Color32, overlay: Color32, overlay_opacity: Option<f32>) -> Color32 {
    let overlay = Rgba::from(overlay);
    let base = Rgba::from(base);

    let overlay_alpha = overlay_opacity.unwrap_or_else(|| overlay.a());

    let final_a = 1.0 - (1.0 - overlay_alpha) * (1.0 - base.a());
    let blended_channel = |b: f32, o: f32| {
        o * overlay_alpha / final_a + b * base.a() * (1.0 - overlay_alpha) / final_a
    };

    Rgba::from_rgba_premultiplied(
        blended_channel(base.r(), overlay.r()),
        blended_channel(base.g(), overlay.g()),
        blended_channel(base.b(), overlay.b()),
        final_a,
    )
    .into()
}
