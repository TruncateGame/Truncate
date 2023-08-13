use time::Duration;
use truncate_core::{messages::GamePlayerMessage, reporting::TimeChange};

use eframe::egui::{self, widget_text::WidgetTextGalley, Layout, Margin, Response, Sense};
use epaint::{emath::Align, hex_color, vec2, Color32, Stroke, Vec2};

use crate::{
    regions::active_game::GameCtx,
    utils::{self, text::TextHelper, Darken, Diaphanize, Theme},
};

pub struct TimerUI<'a> {
    player: &'a GamePlayerMessage,
    current_time: instant::Duration,
    time_adjustment: isize,
    time: Duration,
    friend: bool,
    active: bool,
    winner: Option<usize>,
}

impl<'a> TimerUI<'a> {
    pub fn new(
        player: &'a GamePlayerMessage,
        current_time: instant::Duration,
        time_changes: &'a Vec<TimeChange>,
    ) -> Self {
        let time_adjustment: isize = time_changes
            .iter()
            .filter(|change| change.player == player.index)
            .map(|change| change.time_change)
            .sum();

        Self {
            player,
            current_time,
            time: Duration::default(),
            time_adjustment,
            friend: true,
            active: true,
            winner: None,
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

    pub fn winner(mut self, winner: Option<usize>) -> Self {
        self.winner = winner;
        self
    }
}

impl<'a> TimerUI<'a> {
    fn get_time_color(&self, theme: &Theme, ctx: &mut GameCtx) -> Color32 {
        if self.winner == Some(self.player.index) {
            theme.selection
        } else if !self.active {
            hex_color!("#444444")
        } else {
            ctx.player_colors[self.player.index].darken().darken()
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
        match self.winner {
            Some(player) if player == self.player.index => {
                return "Winner".into();
            }
            Some(_) => {
                return "".into();
            }
            _ => {}
        };

        match self.player.turn_starts_at {
            Some(next_turn) => {
                let now = self.current_time.as_secs();
                let elapsed = now.checked_sub(next_turn);
                if let Some(elapsed) = elapsed {
                    if let Some(time) = self.player.time_remaining {
                        self.time = time - Duration::seconds(elapsed as i64);
                        format!("{}", TimerUI::human_time(self.time.whole_seconds(), false))
                    } else {
                        format!("Playing")
                    }
                } else {
                    let starts_in = (next_turn.saturating_sub(now) as i64) * -1;
                    if let Some(time) = self.player.time_remaining {
                        self.time = time;
                        format!("Wait {}", TimerUI::human_time(starts_in, true))
                    } else {
                        format!("Wait {}", TimerUI::human_time(starts_in, true))
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

    /// Renders everything within our timer frame
    pub fn render_inner(&mut self, ui: &mut egui::Ui, theme: &Theme, ctx: &mut GameCtx) {
        let (bar_h, font_z) = (10.0, 14.0);
        let timer_color = self.get_time_color(theme, ctx);
        let timer_rounding = theme.rounding / 4.0;

        // Allocate our full space up front to fill the frame
        let inner_timer_rect = ui.available_rect_before_wrap();
        ui.allocate_rect(inner_timer_rect, Sense::hover());

        // Render the player name in the top left
        let text = TextHelper::heavy(&self.player.name, font_z, None, ui);
        let name_size = text.size();
        text.paint_at(inner_timer_rect.left_top(), timer_color, ui);

        let time_string = self.calculate_time();
        let text = TextHelper::heavy(&time_string, font_z, None, ui);
        let time_size = text.size();

        // Render the remaining time in the top left,
        // aligned to the bottom of the name
        let mut pos = inner_timer_rect.right_top();
        pos.x -= time_size.x;
        pos.y += name_size.y - time_size.y;
        text.paint_at(pos, timer_color, ui);

        // Paint bar background
        let mut bar = inner_timer_rect.clone();
        bar.set_top(bar.bottom() - bar_h);
        ui.painter()
            .rect_filled(bar, timer_rounding, timer_color.diaphanize());

        if let (Some(time_remaining), Some(allotted_time)) =
            (self.player.time_remaining, self.player.allotted_time)
        {
            // Paint time remaining sector of bar
            let remaining_time_proportion = (self.time / allotted_time) as f32;
            bar.set_right(bar.left() + remaining_time_proportion * inner_timer_rect.width());
            ui.painter().rect_filled(bar, timer_rounding, timer_color);

            // If in an active turn, paint an extension of the bar
            // to mark when the turn started
            if time_remaining != self.time {
                let time_proportion = (time_remaining / allotted_time) as f32;
                bar.set_right(bar.left() + time_proportion * inner_timer_rect.width());

                ui.painter()
                    .rect_stroke(bar, timer_rounding, Stroke::new(1.0, timer_color));
            }

            // If player has lost or gained special time this turn, render this as well
            if self.time_adjustment != 0 {
                let adj_duration = Duration::seconds(self.time_adjustment as i64).abs();
                let adj_proportion = (adj_duration / allotted_time) as f32;
                let mut penalty_bar = bar.translate(vec2(
                    (remaining_time_proportion - adj_proportion) * inner_timer_rect.width(),
                    0.0,
                ));
                penalty_bar
                    .set_right(penalty_bar.left() + adj_proportion * inner_timer_rect.width());
                penalty_bar.set_left(penalty_bar.left().max(bar.left()));

                if self.time_adjustment.is_positive() {
                    ui.painter()
                        .rect_filled(penalty_bar, timer_rounding, hex_color!("#00ff00"));
                } else {
                    penalty_bar = penalty_bar.translate(vec2(penalty_bar.width(), 0.0));
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

                ui.painter()
                    .line_segment(time_division_line, Stroke::new(1.0, theme.text));
            }
        }
    }

    /// Renders the position and border of our timer frame
    pub fn render(mut self, ui: &mut egui::Ui, theme: &Theme, ctx: &mut GameCtx) -> Response {
        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        // Calculate the placement and positioning of this timer
        // TODO: Allow alignment to handle L/R split timers
        let (timer_w, timer_h) = (430.0, 50.0);
        let timer_width = ui.available_width().min(timer_w);
        let timer_padding = (ui.available_width() - timer_width) / 2.0;

        let (timer_ui_rect, _response) =
            ui.allocate_exact_size(vec2(ui.available_width(), timer_h), Sense::hover());
        let timer_ui_rect = timer_ui_rect.shrink2(vec2(timer_padding, 0.0));

        // All layout from here should use the layout UI scoped to the timer.
        let mut ui = ui.child_ui(timer_ui_rect, Layout::top_down(Align::LEFT));

        let resp = egui::Frame::none()
            .inner_margin(Margin {
                left: 10.0,
                right: 10.0,
                top: 12.0, // Optically balance for text
                bottom: 10.0,
            })
            .show(&mut ui, |ui| {
                self.render_inner(ui, theme, ctx);
            });

        ui.painter().rect_stroke(
            resp.response.rect,
            10.0,
            Stroke::new(2.0, self.get_time_color(theme, ctx)),
        );

        resp.response
    }
}
