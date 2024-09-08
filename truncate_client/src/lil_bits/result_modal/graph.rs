use eframe::egui::{self};
use epaint::{
    emath::Align2, hex_color, pos2, textures::TextureOptions, vec2, Color32, ColorImage, Mesh,
    Rect, Shape, Stroke, TextureHandle,
};
use time::Duration;
use truncate_core::messages::DailyStats;

use crate::utils::text::TextHelper;

/*

TODOs for the splash graph:
- Pull all of the colours from a central theme once we refactor general theming
- Add interaction to tap on a day and see the notes label
- Once the graph grows larger, we will need to only show a subset of it
  - And thus we will need to implement some horizontal scrolling and zooming

 */

#[derive(Clone)]
struct DayHighlight {
    day_from_right: usize,
    label: String,
    bar_height: f32,
}

#[derive(Clone)]
pub struct DailySplashGraph {
    moves_graph_texture: TextureHandle,
    streak_graph_texture: TextureHandle,
    date_labels: (String, String),
    highlighted_day: Option<DayHighlight>,
    days_played: usize,
}

impl DailySplashGraph {
    pub fn new(ui: &mut egui::Ui, stats: &DailyStats, current_time: Duration) -> Self {
        let days_played = stats.days.len().clamp(10, 1000);
        let max_total_moves = stats
            .days
            .values()
            .map(|day| {
                if let Some(best_win) = day
                    .attempts
                    .iter()
                    .filter(|a| a.won)
                    .min_by_key(|a| a.moves)
                {
                    return best_win.moves as usize;
                };

                day.attempts.iter().map(|a| a.moves as usize).sum()
            })
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
                if days_played == col {
                    return;
                }
                // We draw from the right since we might have made the canvas
                // larger than the amount of days we have.
                let day_pixel_index = days_played - col - 1;
                let mut current_row = max_total_moves - 1; // Start drawing each day from the bottom

                let best_win = day
                    .attempts
                    .iter()
                    .filter(|a| a.won)
                    .min_by_key(|a| a.moves);

                if let Some(best_win) = best_win {
                    for _ in 0..best_win.moves as usize {
                        moves_image_base[(day_pixel_index, current_row)] = hex_color!("#6DAF6B");
                        current_row = current_row.saturating_sub(1);
                    }
                    streak_image_base[(day_pixel_index, 0)] = hex_color!("#6DAF6B");
                } else {
                    day.attempts.iter().enumerate().for_each(|(i, attempt)| {
                        let attempt_color = match i % 2 {
                            0 => hex_color!("#944D5E"), // Alternate failure colors to make them distinct
                            _ => hex_color!("#A75E6F"),
                        };
                        for _ in 0..attempt.moves as usize {
                            moves_image_base[(day_pixel_index, current_row)] = attempt_color;
                            current_row = current_row.saturating_sub(1);
                        }
                    });
                }
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
        let today_date = time::OffsetDateTime::from_unix_timestamp(current_time.whole_seconds())
            .expect("Current time should be valid");
        let leftmost_date = today_date - time::Duration::days(days_played as i64);
        let leftmost_date_string = leftmost_date
            .format(time::macros::format_description!(
                "[month repr:long] [day] [year]"
            ))
            .unwrap();

        // TODO: Once we have interactions, we'll need this highlight info to be reactive
        let today = stats.days.values().last().cloned().unwrap_or_default();
        let best_win = today
            .attempts
            .iter()
            .filter(|a| a.won)
            .min_by_key(|a| a.moves);

        let (moves, today_label) = if let Some(best_win) = best_win {
            (
                best_win.moves,
                format!(
                    "Won! Personal best: {} move{}",
                    best_win.moves,
                    if best_win.moves == 1 { "" } else { "s" }
                ),
            )
        } else {
            let attempts = today.attempts.len();
            (
                today.attempts.iter().map(|a| a.moves).sum(),
                format!(
                    "No win yet! {} attempt{}",
                    attempts,
                    if attempts == 1 { "" } else { "s" }
                ),
            )
        };

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

    pub fn render(&self, ui: &mut egui::Ui, graph_rect: Rect) {
        let fz = 16.0; // Label font size, should come from a theme.

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
