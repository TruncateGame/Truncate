use eframe::egui::{self, Layout, Sense};
use epaint::{
    emath::{Align, NumExt},
    hex_color, vec2, Color32, TextureHandle, Vec2,
};
use truncate_core::{
    game::Game,
    messages::{DailyAttempt, DailyStats},
};

use crate::utils::{depot::TruncateDepot, Theme};

/*

TODOs for the message mock:
- Pull all of the colours from a central theme once we refactor general theming

 */

#[derive(Clone)]
pub struct ShareMessageMock {
    pub share_text: String,
    emoji_board: String,
}

impl ShareMessageMock {
    pub fn new_daily(
        day: u32,
        game: &Game,
        depot: &TruncateDepot,
        stats: &DailyStats,
        first_win: Option<(u32, &DailyAttempt)>,
        best_win: Option<&DailyAttempt>,
        latest_attempt: (u32, &DailyAttempt),
    ) -> Self {
        let share_prefix =
            ShareMessageMock::daily_share_message(day, first_win, best_win, latest_attempt);
        let emoji_board = game
            .board
            .emojify(depot.gameplay.player_number as usize, game.winner);
        let share_text = format!("{share_prefix}\n{emoji_board}");

        let _this_attempt = stats
            .days
            .last_key_value()
            .map(|(_, v)| v.attempts.last().map(|a| a.id.clone()))
            .flatten();

        Self {
            share_text,
            emoji_board,
        }
    }

    pub fn new_unique(game: &Game, depot: &TruncateDepot) -> Self {
        let share_prefix = ShareMessageMock::unique_share_message(game, depot);
        let emoji_board = game
            .board
            .emojify(depot.gameplay.player_number as usize, game.winner);
        let share_text = format!("{share_prefix}\n{emoji_board}");

        Self {
            share_text,
            emoji_board,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, _theme: &Theme, _map_texture: &TextureHandle) {
        let target_height = 120.0.at_most(ui.available_height());

        let (mut message_bounds, _) = ui.allocate_exact_size(
            // This height is just a rough guess to look right.
            // The board emoji will fill the space, so it doesn't have to be perfect.
            vec2(ui.available_width(), target_height),
            Sense::hover(),
        );

        if message_bounds.height() < 50.0 {
            return;
        }

        let x_difference = (message_bounds.width() - target_height) / 2.0;
        if x_difference > 0.0 {
            message_bounds = message_bounds.shrink2(vec2(x_difference, 0.0));
        }

        ui.painter()
            .rect_filled(message_bounds, 15.0, hex_color!("#444444"));

        let mut tail = message_bounds.translate(vec2(message_bounds.width() - 7.0, 0.0));
        tail.set_right(tail.left() + 20.0);
        tail.set_top(tail.bottom() - 30.0);

        ui.painter().rect_filled(tail, 10.0, hex_color!("#444444"));
        tail = tail.translate(vec2(7.0, -3.0));
        tail.set_top(tail.bottom() - 40.0);
        ui.painter().rect_filled(tail, 10.0, hex_color!("#111111"));

        message_bounds = message_bounds.shrink(10.0);

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
                        let full_line = line.chars().count() as f32 * emoji_size;
                        if full_line < ui.available_width() {
                            ui.add_space((ui.available_width() - full_line) / 2.0);
                        }
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

impl ShareMessageMock {
    pub fn daily_share_message(
        day: u32,
        first_win: Option<(u32, &DailyAttempt)>,
        best_win: Option<&DailyAttempt>,
        latest_attempt: (u32, &DailyAttempt),
    ) -> String {
        let plur = |num: u32| if num == 1 { "" } else { "s" };

        let Some(first_win) = first_win else {
            if matches!(option_env!("TR_ENV"), Some("outpost")) {
                return format!(
                    "-- Truncate Outpost Day #{day} --\nLost in {} move{} on attempt #{}",
                    latest_attempt.1.moves,
                    plur(latest_attempt.1.moves),
                    latest_attempt.0 + 1,
                );
            } else {
                return format!(
                    "Truncate Town Day #{day}\nLost in {} move{} on attempt #{}",
                    latest_attempt.1.moves,
                    plur(latest_attempt.1.moves),
                    latest_attempt.0 + 1,
                );
            }
        };

        let best_win = best_win.unwrap_or(first_win.1);

        let first_win_message = if first_win.0 == 0 {
            format!(
                "Won first try in {} move{}",
                first_win.1.moves,
                plur(first_win.1.moves)
            )
        } else {
            format!(
                "Won on attempt #{} in {} move{}",
                first_win.0 + 1,
                first_win.1.moves,
                plur(first_win.1.moves)
            )
        };

        if best_win.id == first_win.1.id {
            format!("Truncate Town Day #{day}\n{first_win_message}")
        } else {
            format!(
                "Truncate Town Day #{day}\n{first_win_message}\nPersonal best: {} move{}",
                best_win.moves,
                plur(best_win.moves)
            )
        }
    }

    pub fn unique_share_message(game: &Game, depot: &TruncateDepot) -> String {
        let player = depot.gameplay.player_number as usize;
        let won = game.winner;

        let player_won = won == Some(player);

        let plur = |num: u32| if num == 1 { "" } else { "s" };

        let (Some(seed), Some(npc)) = (&depot.board_info.board_seed, &depot.gameplay.npc) else {
            if player_won {
                return format!("Won puzzle");
            }
            return format!("Lost puzzle");
        };

        let share_link = format!(
            "Play Puzzle: https://truncate.town/puzzle/?j=PUZZLE:{}:{}:{}:{}:{}",
            seed.generation,
            npc.name.to_ascii_uppercase(),
            game.rules
                .generation
                .expect("puzzles should always use a generational ruleset"),
            seed.seed,
            player
        );

        let counts = format!(
            " in {} move{}",
            game.player_turn_count[player],
            plur(game.player_turn_count[player]),
        );

        if player_won {
            format!("Truncate Town Puzzle\nWon{counts}\n{share_link}")
        } else {
            format!("Truncate Town Puzzle\nLost{counts}\n{share_link}")
        }
    }
}
