use core::{messages::GamePlayerMessage, player::Hand};

use eframe::{
    egui::{self, widget_text::WidgetTextGalley, Layout, Margin, Sense},
    emath::Align,
};
use epaint::{hex_color, vec2, Color32, Vec2};
use time::OffsetDateTime;

use crate::theming::Theme;

pub struct TimerUI<'a> {
    player: &'a GamePlayerMessage,
    time: i64,
    friend: bool,
    active: bool,
}

impl<'a> TimerUI<'a> {
    pub fn new(player: &'a GamePlayerMessage) -> Self {
        Self {
            player,
            time: 0,
            friend: true,
            active: true,
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
}

impl<'a> TimerUI<'a> {
    fn get_name_color(&self, theme: &Theme) -> Color32 {
        if !self.active {
            theme.outlines
        } else if self.friend {
            theme.friend.dark
        } else {
            theme.enemy.dark
        }
    }

    fn get_time_color(&self, theme: &Theme) -> Color32 {
        if !self.active {
            theme.outlines
        } else if self.friend {
            theme.friend.dark
        } else {
            theme.enemy.dark
        }
    }

    fn calculate_time(&mut self) -> (String) {
        match self.player.turn_starts_at {
            Some(next_turn) => {
                let elapsed = OffsetDateTime::now_utc() - next_turn;
                if elapsed.is_positive() {
                    self.time = (self.player.time_remaining - elapsed).whole_seconds();
                    format!("{:?}s remaining", self.time)
                } else {
                    self.time = self.player.time_remaining.whole_seconds();
                    let starts_in = elapsed.whole_seconds() * -1;
                    format!("{:?}s remaining. Turn in {starts_in:?}s", self.time)
                }
            }
            None => {
                self.time = self.player.time_remaining.whole_seconds();
                format!("{:?}s remaining", self.time)
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

        let frame = egui::Frame::none()
            .inner_margin(Margin::symmetric(theme.grid_size, theme.grid_size / 2.0));

        frame.show(ui, |ui| {
            let time_string = self.calculate_time();

            let (timer_ui_rect, response) = ui.allocate_exact_size(
                vec2(ui.available_width(), theme.letter_size * 2.5),
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

            // TODO: Add timer bar as per designs
        });
    }
}
