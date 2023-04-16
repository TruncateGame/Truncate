use time::Duration;
use truncate_core::messages::GamePlayerMessage;

use eframe::egui::{self, widget_text::WidgetTextGalley, Margin, Sense};
use epaint::{vec2, Color32, Stroke, Vec2};
use time::OffsetDateTime;

use crate::theming::{Darken, Theme};

pub struct TimerUI<'a> {
    player: &'a GamePlayerMessage,
    current_time: instant::Duration,
    time: Duration,
    friend: bool,
    active: bool,
    winner: Option<usize>,
}

impl<'a> TimerUI<'a> {
    pub fn new(player: &'a GamePlayerMessage, current_time: instant::Duration) -> Self {
        Self {
            player,
            current_time,
            time: Duration::default(),
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
    fn get_name_color(&self, theme: &Theme) -> Color32 {
        if self.winner == Some(self.player.index) {
            theme.selection
        } else if !self.active {
            theme.outlines
        } else if self.friend {
            theme.friend.darken()
        } else {
            theme.enemy.darken()
        }
    }

    fn get_time_color(&self, theme: &Theme) -> Color32 {
        if self.winner == Some(self.player.index) {
            theme.selection
        } else if !self.active {
            theme.outlines
        } else if self.friend {
            theme.friend.darken()
        } else {
            theme.enemy.darken()
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
                    self.time = self.player.time_remaining - elapsed;
                    format!("{:?}s remaining", self.time.whole_seconds())
                } else {
                    self.time = self.player.time_remaining;
                    let starts_in = elapsed.whole_seconds() * -1;
                    format!(
                        "{:?}s remaining. Turn in {starts_in:?}s",
                        self.time.whole_seconds()
                    )
                }
            }
            None => {
                self.time = self.player.time_remaining;
                format!("{:?}s remaining", self.time.whole_seconds())
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

    pub fn render(mut self, ui: &mut egui::Ui, theme: &Theme) {
        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        let frame = egui::Frame::none().inner_margin(Margin::symmetric(theme.grid_size, 0.0));

        frame.show(ui, |ui| {
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
                self.get_name_color(theme),
            );

            let (time_size, galley) = self.get_galley(
                &time_string,
                "Truncate-Regular",
                theme.letter_size * 0.6,
                ui,
            );
            ui.allocate_space(vec2(ui.available_width(), time_size.y));
            let mut pos = timer_ui_rect.right_top();
            pos.x -= time_size.x;
            pos.y += name_size.y - time_size.y;
            galley.paint_with_color_override(ui.painter(), pos, self.get_time_color(theme));

            let timer_rounding = theme.rounding / 4.0;

            // Paint timer background
            let mut time_bar = timer_ui_rect.clone();
            time_bar.set_top(time_bar.bottom() - theme.letter_size / 2.0);
            ui.painter()
                .rect_filled(time_bar, timer_rounding, theme.text.darken());

            // Paint time remaining
            let time_proportion = (self.time / self.player.allotted_time) as f32;
            time_bar.set_right(time_bar.left() + time_proportion * timer_ui_rect.width());
            ui.painter()
                .rect_filled(time_bar, timer_rounding, self.get_time_color(theme));

            // If in an active turn, paint the point the turn started at
            if self.player.time_remaining != self.time {
                let time_proportion =
                    (self.player.time_remaining / self.player.allotted_time) as f32;
                time_bar.set_right(time_bar.left() + time_proportion * timer_ui_rect.width());

                ui.painter().rect_stroke(
                    time_bar,
                    timer_rounding,
                    Stroke::new(1.0, self.get_time_color(theme)),
                );
            }

            let time_division_count = self.player.allotted_time.whole_minutes();
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
        });
    }
}
