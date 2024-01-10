use eframe::egui::{self, Sense};
use epaint::{
    emath::Align2, hex_color, pos2, textures::TextureOptions, vec2, Color32, ColorImage, Mesh,
    Rect, Shape, Stroke, TextureHandle,
};
use instant::Duration;

use crate::utils::{daily::DailyStats, text::TextHelper};

/*

TODOs for the splash graph:
- Pull all of the colours from a central theme once we refactor general theming
- Add interaction to tap on a day and see the notes label
- Once the graph grows larger, we will need to only show a subset of it
  - And thus we will need to implement some horizontal scrolling and zooming

 */

struct DayHighlight {
    day_from_right: usize,
    label: String,
    bar_height: f32,
}

pub struct DailySplashGraph {
    moves_graph_texture: TextureHandle,
    streak_graph_texture: TextureHandle,
    date_labels: (String, String),
    highlighted_day: Option<DayHighlight>,
    days_played: usize,
}

impl DailySplashGraph {
    pub fn new(ui: &mut egui::Ui, stats: &DailyStats, current_time: Duration) -> Self {
        let days_played = stats.days.len().max(10);
        let max_total_moves = stats
            .days
            .values()
            .map(|day| day.attempts.iter().map(|a| a.moves as usize).sum())
            .max()
            .unwrap_or(30);

        // Both graph images are small textures stretched to paint the graph,
        // so we start with images with a width of the number of days to show.

        let mut moves_image_base =
            ColorImage::new([days_played, max_total_moves], Color32::TRANSPARENT);
        let mut streak_image_base = ColorImage::new([days_played, 1], hex_color!("#333333"));

        stats
            .days
            .values()
            .rev()
            .enumerate()
            .for_each(|(col, day)| {
                // We draw from the right since we might have made the canvas
                // larger than the amount of days we have.
                let day_pixel_index = days_played - col - 1;
                let mut current_row = max_total_moves - 1; // Start drawing each day from the bottom

                day.attempts.iter().enumerate().for_each(|(i, attempt)| {
                    let attempt_color = match (attempt.won, i % 2) {
                        (true, _) => hex_color!("#6DAF6B"), // TODO: Pull these from palette
                        (false, 0) => hex_color!("#944D5E"), // Alternate failure colors to make them distinct
                        (false, _) => hex_color!("#A75E6F"),
                    };
                    for _ in 0..attempt.moves as usize {
                        moves_image_base[(day_pixel_index, current_row)] = attempt_color;
                        current_row = current_row.saturating_sub(1);
                    }
                    if attempt.won {
                        streak_image_base[(day_pixel_index, 0)] = attempt_color;
                    }
                });
            });

        let moves_graph_texture = ui.ctx().load_texture(
            "daily_moves_graph",
            moves_image_base,
            TextureOptions::NEAREST,
        );
        let streak_graph_texture = ui.ctx().load_texture(
            "daily_streak_graph",
            streak_image_base,
            TextureOptions::NEAREST,
        );

        // TODO: Once we have interactions, we'll need these date strings to be reactive
        let rightmost_date_string = "Today".to_string();
        let today_date = time::OffsetDateTime::from_unix_timestamp(current_time.as_secs() as i64)
            .expect("Current time should be valid");
        let leftmost_date = today_date - time::Duration::days(days_played as i64);
        let leftmost_date_string = leftmost_date
            .format(time::macros::format_description!(
                "[month repr:long] [day] [year]"
            ))
            .unwrap();

        // TODO: Once we have interactions, we'll need this highlight info to be reactive
        let today = stats.days.values().last().cloned().unwrap_or_default();
        let won = today.attempts.last().map(|a| a.won) == Some(true);
        let attempts = today.attempts.len();
        let moves: u32 = today.attempts.iter().map(|a| a.moves).sum();

        let today_label = format!(
            "{}: {} attempt{}, {} total move{}",
            if won { "Won" } else { "Lost" },
            attempts,
            if attempts == 1 { "" } else { "s" },
            moves,
            if moves == 1 { "" } else { "s" },
        );
        let today_height: f32 = moves as f32 / max_total_moves as f32;

        Self {
            moves_graph_texture,
            streak_graph_texture,
            date_labels: (leftmost_date_string, rightmost_date_string),
            highlighted_day: Some(DayHighlight {
                day_from_right: 0,
                label: today_label,
                bar_height: today_height,
            }),
            days_played,
        }
    }

    pub fn render(&self, ui: &mut egui::Ui) {
        let fz = 16.0; // Label font size, should come from a theme.
        let (graph_rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 128.0), Sense::hover());

        let mut moves_graph_rect = graph_rect.clone();
        *moves_graph_rect.top_mut() += 30.0;
        *moves_graph_rect.bottom_mut() -= 18.0;

        let mut streak_graph_rect = moves_graph_rect.clone();
        streak_graph_rect.set_bottom(moves_graph_rect.bottom() + 6.0);
        streak_graph_rect.set_top(moves_graph_rect.bottom() + 2.0);

        let date = TextHelper::light(&self.date_labels.1, fz, None, ui);
        date.paint_within(graph_rect, Align2::RIGHT_BOTTOM, Color32::WHITE, ui);

        let date = TextHelper::light(&self.date_labels.0, fz, None, ui);
        date.paint_within(graph_rect, Align2::LEFT_BOTTOM, Color32::WHITE, ui);

        if let Some(highlight) = &self.highlighted_day {
            let notes = TextHelper::light(&highlight.label, fz, None, ui);
            // TODO: This will need to align to the correct day when highlighting !today
            notes.paint_within(graph_rect, Align2::RIGHT_TOP, Color32::WHITE, ui);

            let day_column_width = graph_rect.width() / self.days_played as f32;
            let right_offset =
                (highlight.day_from_right as f32) * day_column_width + (day_column_width / 2.0);

            let line_start = graph_rect.right_top() + vec2(-right_offset, 15.0);
            let line_end = line_start
                + vec2(
                    0.0,
                    11.0 + (moves_graph_rect.height() * (1.0 - highlight.bar_height)),
                );
            ui.painter()
                .line_segment([line_start, line_end], Stroke::new(1.0, Color32::WHITE));
        }

        let mut mesh = Mesh::with_texture(self.moves_graph_texture.id());
        mesh.add_rect_with_uv(
            moves_graph_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        ui.painter().add(Shape::mesh(mesh));

        let mut mesh = Mesh::with_texture(self.streak_graph_texture.id());
        mesh.add_rect_with_uv(
            streak_graph_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        ui.painter().add(Shape::mesh(mesh));
    }
}
