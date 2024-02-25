use time::Duration;
use truncate_core::{messages::GamePlayerMessage, reporting::TimeChange};

use eframe::egui::{self, Layout, Response, Sense};
use epaint::{emath::Align, hex_color, vec2, Color32, Stroke};

use crate::utils::{depot::TruncateDepot, text::TextHelper, Darken, Diaphanize};

pub struct TimerUI<'a> {
    player: &'a GamePlayerMessage,
    depot: &'a TruncateDepot,
    time_adjustment: isize,
    time: Duration,
    friend: bool,
    active: bool,
    right_align: bool,
}

impl<'a> TimerUI<'a> {
    pub fn new(
        player: &'a GamePlayerMessage,
        depot: &'a TruncateDepot,
        time_changes: &'a Vec<TimeChange>,
    ) -> Self {
        let time_adjustment: isize = time_changes
            .iter()
            .filter(|change| change.player == player.index)
            .map(|change| change.time_change)
            .sum();

        Self {
            player,
            depot,
            time: Duration::default(),
            time_adjustment,
            friend: true,
            active: true,
            right_align: false,
        }
    }

    pub fn friend(mut self, friend: bool) -> Self {
        self.friend = friend;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn right_align(mut self) -> Self {
        self.right_align = true;
        self
    }
}

impl<'a> TimerUI<'a> {
    fn get_time_color(&self) -> Color32 {
        if self.depot.gameplay.winner == Some(self.player.index) {
            self.depot.aesthetics.theme.gold_medal
        } else if !self.active {
            hex_color!("#444444")
        } else {
            self.depot.aesthetics.player_colors[self.player.index]
                .darken()
                .darken()
        }
    }

    fn human_time(seconds: i64, absolute: bool) -> String {
        let abs_secs = seconds.abs();
        let h_minutes = abs_secs / 60;
        let h_seconds = abs_secs % 60;

        let mut time_string = if h_minutes > 0 {
            format!("{h_minutes}m{h_seconds}s")
        } else {
            format!("{h_seconds}s")
        };

        if !absolute {
            if seconds.is_negative() {
                time_string.extend(" overtime".chars());
            }
        }

        time_string
    }

    fn calculate_time(&mut self) -> String {
        match self.player.turn_starts_no_later_than {
            Some(next_turn) => {
                let now = self.depot.timing.current_time.as_secs();
                let elapsed = now.checked_sub(next_turn);
                if let Some(elapsed) = elapsed {
                    if let Some(time) = self.player.time_remaining {
                        self.time = time - Duration::seconds(elapsed as i64);
                        format!("{}", TimerUI::human_time(self.time.whole_seconds(), false))
                    } else {
                        format!("")
                    }
                } else {
                    if let Some(time) = self.player.time_remaining {
                        self.time = time;
                        format!("{}", TimerUI::human_time(self.time.whole_seconds(), false))
                    } else {
                        format!("")
                    }
                }
            }
            None => {
                if let Some(time) = self.player.time_remaining {
                    self.time = time;
                    format!("{}", TimerUI::human_time(self.time.whole_seconds(), false))
                } else {
                    format!("")
                }
            }
        }
    }

    fn calculate_byline(&mut self) -> String {
        match self.depot.gameplay.winner {
            Some(player) if player == self.player.index => {
                return "Victorious".into();
            }
            Some(_) => {
                return "Defeated".into();
            }
            _ => {}
        };

        match self.player.turn_starts_no_later_than {
            Some(next_turn) => {
                let now = self.depot.timing.current_time.as_secs();
                let elapsed = now.checked_sub(next_turn);
                if elapsed.is_some() {
                    if self.friend {
                        return format!("Your turn!");
                    } else {
                        return format!("Playing");
                    }
                } else {
                    let starts_in = (next_turn.saturating_sub(now) as i64) * -1;
                    return format!("Turn starts in {}", TimerUI::human_time(starts_in, true));
                }
            }
            _ => {}
        }

        return "".into();
    }

    /// Renders everything within our timer frame
    pub fn render_inner(&mut self, ui: &mut egui::Ui) {
        let (bar_h, font_z, font_z_small) = (10.0, 14.0, 10.0);
        let timer_color = self.get_time_color();
        let timer_rounding = self.depot.aesthetics.theme.rounding / 4.0;

        // Allocate our full space up front to fill the frame
        let inner_timer_rect = ui.available_rect_before_wrap();
        ui.allocate_rect(inner_timer_rect, Sense::hover());

        // Paint bar background
        let mut bar = inner_timer_rect.clone();
        bar.set_bottom(bar.top() + bar_h);
        bar = bar.translate(vec2(0.0, 5.0));
        ui.painter()
            .rect_filled(bar, timer_rounding, timer_color.diaphanize());

        if let (Some(time_remaining), Some(allotted_time)) =
            (self.player.time_remaining, self.player.allotted_time)
        {
            // Paint time remaining sector of bar
            let remaining_time_proportion = (self.time / allotted_time) as f32;
            if self.right_align {
                bar.set_left(bar.right() - remaining_time_proportion * inner_timer_rect.width());
                ui.painter().rect_filled(bar, timer_rounding, timer_color);
            } else {
                bar.set_right(bar.left() + remaining_time_proportion * inner_timer_rect.width());
                ui.painter().rect_filled(bar, timer_rounding, timer_color);
            }

            // If in an active turn, paint an extension of the bar
            // to mark when the turn started
            if time_remaining != self.time {
                let time_proportion = (time_remaining / allotted_time) as f32;
                if self.right_align {
                    bar.set_left(bar.right() - time_proportion * inner_timer_rect.width());
                } else {
                    bar.set_right(bar.left() + time_proportion * inner_timer_rect.width());
                }

                ui.painter()
                    .rect_stroke(bar, timer_rounding, Stroke::new(1.0, timer_color));
            }

            // If player has lost or gained special time this turn, render this as well
            if self.time_adjustment != 0 {
                let adj_duration = Duration::seconds(self.time_adjustment as i64).abs();
                let adj_proportion = (adj_duration / allotted_time) as f32;
                let penalty =
                    (remaining_time_proportion - adj_proportion) * inner_timer_rect.width();
                let mut penalty_bar =
                    bar.translate(vec2(if self.right_align { -penalty } else { penalty }, 0.0));

                if self.right_align {
                    penalty_bar
                        .set_left(penalty_bar.right() - adj_proportion * inner_timer_rect.width());
                    penalty_bar.set_right(penalty_bar.right().min(bar.right()));
                } else {
                    penalty_bar
                        .set_right(penalty_bar.left() + adj_proportion * inner_timer_rect.width());
                    penalty_bar.set_left(penalty_bar.left().max(bar.left()));
                }

                if self.time_adjustment.is_positive() {
                    ui.painter()
                        .rect_filled(penalty_bar, timer_rounding, hex_color!("#00ff00"));
                } else {
                    if self.right_align {
                        penalty_bar = penalty_bar.translate(vec2(-penalty_bar.width(), 0.0));
                    } else {
                        penalty_bar = penalty_bar.translate(vec2(penalty_bar.width(), 0.0));
                    }
                    ui.painter()
                        .rect_filled(penalty_bar, timer_rounding, hex_color!("#ff0000"));
                };
            }

            let time_division_count = allotted_time.whole_minutes();
            let time_division_width = inner_timer_rect.width() / time_division_count as f32;

            let mut time_division_line = [bar.left_top(), bar.left_bottom()];
            time_division_line[0].y += bar.height() * 0.15;
            time_division_line[1].y -= bar.height() * 0.15;

            for _ in 1..time_division_count {
                time_division_line[0].x += time_division_width;
                time_division_line[1].x += time_division_width;

                ui.painter().line_segment(
                    time_division_line,
                    Stroke::new(1.0, self.depot.aesthetics.theme.text),
                );
            }
        }

        let text = if let Some(ends_at) = self.depot.timing.game_ends_at {
            let now = self.depot.timing.current_time.as_secs();
            let remaining = ends_at.saturating_sub(now);
            let remaining_label = TimerUI::human_time(remaining as i64, false);
            format!("{} : {}", remaining_label, &self.player.name)
        } else {
            self.player.name.clone()
        };

        // Render the player name
        let text = TextHelper::heavy(&text, font_z, None, ui);
        let name_size = text.size();
        if self.right_align {
            let mut pos = bar.right_bottom() + vec2(0.0, 10.0);
            pos.x -= name_size.x;
            text.paint_at(pos, timer_color, ui);
        } else {
            text.paint_at(bar.left_bottom() + vec2(0.0, 10.0), timer_color, ui);
        }

        let time_string = self.calculate_time();
        let text = TextHelper::heavy(&time_string, font_z, None, ui);
        let time_size = text.size();

        // Render the remaining time
        if self.right_align {
            text.paint_at(bar.left_bottom() + vec2(0.0, 10.0), timer_color, ui);
        } else {
            let mut pos = bar.right_bottom() + vec2(0.0, 10.0);
            pos.x -= time_size.x;
            text.paint_at(pos, timer_color, ui);
        }

        let byline_string = self.calculate_byline();
        let text = TextHelper::heavy(&byline_string, font_z_small, None, ui);
        let byline_size = text.size();
        let byline_y_offset = vec2(0.0, 10.0 + name_size.y + 5.0);

        // Render the byline
        if self.right_align {
            let mut pos = bar.right_bottom() + byline_y_offset;
            pos.x -= byline_size.x;
            text.paint_at(pos, timer_color, ui);
        } else {
            let pos = bar.left_bottom() + byline_y_offset;
            text.paint_at(pos, timer_color, ui);
        }
    }

    /// Renders the position and border of our timer frame
    pub fn render(
        mut self,
        explicit_width: Option<f32>,
        center: bool,
        ui: &mut egui::Ui,
    ) -> Response {
        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        let (timer_w, timer_h) = (430.0, 50.0);
        let timer_width = explicit_width.unwrap_or_else(|| ui.available_width().min(timer_w));
        let timer_padding = if center {
            (ui.available_width() - timer_width) / 2.0
        } else {
            0.0
        };

        let (timer_ui_rect, response) =
            ui.allocate_exact_size(vec2(timer_width, timer_h), Sense::hover());
        let timer_ui_rect = timer_ui_rect.shrink2(vec2(timer_padding, 0.0));

        // All layout from here should use the layout UI scoped to the timer.
        let mut ui = ui.child_ui(timer_ui_rect, Layout::top_down(Align::LEFT));

        self.render_inner(&mut ui);

        response
    }
}
