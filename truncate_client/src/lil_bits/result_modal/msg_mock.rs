use eframe::egui::{self, Layout, RichText, Sense};
use epaint::{
    emath::{Align, NumExt},
    hex_color, vec2, Color32, TextureHandle, Vec2,
};
use truncate_core::{game::Game, messages::DailyStats};

use crate::{
    app_outer::Backchannel,
    utils::{depot::TruncateDepot, macros::tr_log, text::TextHelper, Lighten, Theme},
};

/*

TODOs for the message mock:
- Pull all of the colours from a central theme once we refactor general theming
- Add the small speech bubble decoration bottom right to indicate it's a message bubble

 */

#[derive(Clone)]
pub struct ShareMessageMock {
    is_daily: bool,
    pub share_text: String,
    emoji_board: String,
}

impl ShareMessageMock {
    pub fn new_daily(game: &Game, depot: &TruncateDepot, stats: &DailyStats) -> Self {
        let share_text = game.board.share_message(
            depot.gameplay.player_number as usize,
            game.winner,
            Some(game),
            depot.board_info.board_seed.clone(),
            stats
                .days
                .last_key_value()
                .map(|(_, v)| v.attempts.len() - 1),
            format!("https://truncate.town/#"),
        );
        let emoji_board = game
            .board
            .emojify(depot.gameplay.player_number as usize, game.winner);

        let this_attempt = stats
            .days
            .last_key_value()
            .map(|(_, v)| v.attempts.last().map(|a| a.id.clone()))
            .flatten();

        Self {
            is_daily: true,
            share_text,
            emoji_board,
        }
    }

    pub fn new_unique(game: &Game, depot: &TruncateDepot) -> Self {
        tr_log!({
            format!(
                "We are player {:?} and the winner was player {:?}",
                depot.gameplay.player_number, game.winner
            )
        });
        let share_text = game.board.share_message(
            depot.gameplay.player_number as usize,
            game.winner,
            Some(game),
            depot.board_info.board_seed.clone(),
            None,
            format!("https://truncate.town/#"),
        );
        let emoji_board = game
            .board
            .emojify(depot.gameplay.player_number as usize, game.winner);

        Self {
            is_daily: false,
            share_text,
            emoji_board,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme, map_texture: &TextureHandle) {
        let target_height = 180.0.at_most(ui.available_height());

        let (mut message_bounds, _) = ui.allocate_exact_size(
            // This height is just a rough guess to look right.
            // The board emoji will fill the space, so it doesn't have to be perfect.
            vec2(ui.available_width(), target_height),
            Sense::hover(),
        );

        let x_difference = (message_bounds.width() - target_height) / 2.0;
        if x_difference > 0.0 {
            message_bounds = message_bounds.shrink2(vec2(x_difference, 0.0));
        }

        ui.allocate_ui_at_rect(message_bounds, |ui| {
            ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                let styles = ui.style_mut();
                styles.spacing.item_spacing = Vec2::splat(0.0);
                styles.spacing.interact_size = Vec2::splat(0.0);

                let line_count = self.emoji_board.lines().count();
                let emoji_lines = self.emoji_board.lines();
                for (line_num, line) in emoji_lines.enumerate() {
                    let mut emoji_size = ui.available_width() / line.chars().count() as f32;
                    let Vec2 { y: msg_h, .. } = ui.available_size();
                    let line_height = msg_h / (line_count - line_num) as f32;
                    // Size our emojis to fill whichever is the smallest of the available dimensions
                    if line_height < emoji_size {
                        emoji_size = line_height;
                    }
                    ui.horizontal(|ui| {
                        for emoji in line.chars() {
                            let color = match emoji {
                                '🟦' => hex_color!("#4F55E2"), // TODO: Pull from theming palette
                                '🟩' => hex_color!("#6DAF6B"),
                                '🟨' => hex_color!("#D7AE1D"),
                                '🟫' => hex_color!("#A7856F"),
                                '🟪' => hex_color!("#D27CFF"),
                                _ => Color32::BLACK,
                            };
                            let (emoji_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(emoji_size), Sense::hover());
                            let emoji_rect = emoji_rect.shrink(emoji_rect.width() * 0.1);
                            ui.painter().rect_filled(emoji_rect, 2.0, color);
                        }
                    });
                }
            });
        });
    }
}
