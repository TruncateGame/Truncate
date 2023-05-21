use time::Duration;
use truncate_core::{messages::GamePlayerMessage, reporting::TimeChange};

use eframe::egui::{self, widget_text::WidgetTextGalley, Margin, Response, Sense};
use epaint::{hex_color, vec2, Color32, Stroke, Vec2};
use time::OffsetDateTime;

use crate::{
    regions::active_game::GameCtx,
    theming::{Darken, Theme},
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
                let now = OffsetDateTime::from_unix_timestamp(self.current_time.as_secs() as i64)
                    .expect("Should be a valid timestamp");
                let elapsed = now - next_turn;
                if elapsed.is_positive() {
                    if let Some(time) = self.player.time_remaining {
                        self.time = time - elapsed;
                        format!("{:?}s remaining", self.time.whole_seconds())
                    } else {
                        format!("Playing")
                    }
                } else {
                    if let Some(time) = self.player.time_remaining {
                        self.time = time;
                        let starts_in = elapsed.whole_seconds() * -1;
                        format!(
                            "{:?}s remaining. Turn in {starts_in:?}s",
                            self.time.whole_seconds()
                        )
                    } else {
                        let starts_in = elapsed.whole_seconds() * -1;
                        format!("Turn starts in {starts_in:?}s")
                    }
                }
            }
            None => {
                if let Some(time) = self.player.time_remaining {
                    self.time = time;
                    format!("{:?}s remaining", self.time.whole_seconds())
                } else {
                    format!("")
                }
            }
        }
    }

    fn get_galley(
        &self,
        text: &String,
        font: &'static str,
        size: f32,
        ui: &mut egui::Ui,
    ) -> (Vec2, WidgetTextGalley) {
        let font = egui::FontSelection::FontId(egui::FontId {
            size: size,
            family: egui::FontFamily::Name(font.into()),
        });
        let galley = egui::WidgetText::RichText(egui::RichText::new(text))
            .into_galley(ui, None, 1000.0, font); // TODO: Use a non-wrapping method so this giant float isn't here
        (galley.size(), galley)
    }

    pub fn render(mut self, ui: &mut egui::Ui, theme: &Theme, ctx: &mut GameCtx) -> Response {
        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        let frame = egui::Frame::none().inner_margin(Margin::symmetric(10.0, 10.0));

        let resp = frame.show(ui, |ui| {
            let time_string = self.calculate_time();

            let (timer_ui_rect, _response) = ui.allocate_exact_size(
                vec2(ui.available_width(), theme.letter_size * 2.0),
                Sense::hover(),
            );
            let (name_size, galley) = self.get_galley(
                &self.player.name,
                "Truncate-Heavy",
                theme.letter_size * 0.7,
                ui,
            );
            galley.paint_with_color_override(
                ui.painter(),
                timer_ui_rect.left_top(),
                self.get_time_color(theme, ctx),
            );

            let (time_size, galley) =
                self.get_galley(&time_string, "Truncate-Heavy", theme.letter_size * 0.6, ui);
            ui.allocate_space(vec2(ui.available_width(), time_size.y));
            let mut pos = timer_ui_rect.right_top();
            pos.x -= time_size.x;
            pos.y += name_size.y - time_size.y;
            galley.paint_with_color_override(ui.painter(), pos, self.get_time_color(theme, ctx));

            let timer_rounding = theme.rounding / 4.0;

            // Paint timer background
            let mut time_bar = timer_ui_rect.clone();
            time_bar.set_top(time_bar.bottom() - theme.letter_size / 2.0);
            ui.painter()
                .rect_filled(time_bar, timer_rounding, theme.text.darken());

            if let (Some(time_remaining), Some(allotted_time)) =
                (self.player.time_remaining, self.player.allotted_time)
            {
                // Paint time remaining
                let remaining_time_proportion = (self.time / allotted_time) as f32;
                time_bar
                    .set_right(time_bar.left() + remaining_time_proportion * timer_ui_rect.width());
                ui.painter()
                    .rect_filled(time_bar, timer_rounding, self.get_time_color(theme, ctx));

                // If in an active turn, paint the point the turn started at
                if time_remaining != self.time {
                    let time_proportion = (time_remaining / allotted_time) as f32;
                    time_bar.set_right(time_bar.left() + time_proportion * timer_ui_rect.width());

                    ui.painter().rect_stroke(
                        time_bar,
                        timer_rounding,
                        Stroke::new(1.0, self.get_time_color(theme, ctx)),
                    );
                }

                if self.time_adjustment != 0 {
                    let adjustment_duration = Duration::seconds(self.time_adjustment as i64).abs();
                    let adjustment_proportion = (adjustment_duration / allotted_time) as f32;
                    let mut penalty_bar = time_bar.translate(vec2(
                        (remaining_time_proportion - adjustment_proportion) * timer_ui_rect.width(),
                        0.0,
                    ));
                    penalty_bar.set_right(
                        penalty_bar.left() + adjustment_proportion * timer_ui_rect.width(),
                    );
                    penalty_bar.set_left(penalty_bar.left().max(time_bar.left()));

                    if self.time_adjustment.is_positive() {
                        ui.painter().rect_filled(
                            penalty_bar,
                            timer_rounding,
                            hex_color!("#00ff00"),
                        );
                    } else {
                        // TODO: Pin penalty bar to the right edge of timer
                        ui.painter().rect_filled(
                            penalty_bar,
                            timer_rounding,
                            hex_color!("#ff0000"),
                        );
                    };
                }

                let time_division_count = allotted_time.whole_minutes();
                let time_division_width = timer_ui_rect.width() / time_division_count as f32;

                let mut time_division_line = [time_bar.left_top(), time_bar.left_bottom()];
                time_division_line[0].y += time_bar.height() * 0.15;
                time_division_line[1].y -= time_bar.height() * 0.15;

                for _ in 1..time_division_count {
                    time_division_line[0].x += time_division_width;
                    time_division_line[1].x += time_division_width;

                    ui.painter()
                        .line_segment(time_division_line, Stroke::new(1.0, theme.text));
                }
            }
        });

        ui.painter().rect_stroke(
            resp.response.rect,
            10.0,
            Stroke::new(2.0, self.get_time_color(theme, ctx)),
        );

        resp.response
    }
}
