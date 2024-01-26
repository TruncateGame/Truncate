use eframe::egui::{self, RichText, Sense};
use epaint::{hex_color, vec2, Color32, TextureHandle, Vec2};
use truncate_core::{game::Game, messages::DailyStats};

use crate::{
    app_outer::Backchannel,
    utils::{depot::TruncateDepot, text::TextHelper, Lighten, Theme},
};

/*

TODOs for the message mock:
- Pull all of the colours from a central theme once we refactor general theming
- Add the small speech bubble decoration bottom right to indicate it's a message bubble

 */

#[derive(Clone)]
pub struct ShareMessageMock {
    share_text: String,
    share_copied: bool,
}

impl ShareMessageMock {
    pub fn new(game: &Game, depot: &TruncateDepot, stats: &DailyStats) -> Self {
        let share_text = game.board.emojify(
            depot.gameplay.player_number as usize,
            Some(depot.gameplay.player_number as usize),
            Some(game),
            depot.board_info.board_seed.clone(),
            stats
                .days
                .last_key_value()
                .map(|(_, v)| v.attempts.len() - 1),
            format!("https://truncate.town/#"),
        );

        Self {
            share_text,
            share_copied: false,
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        map_texture: &TextureHandle,
        backchannel: Option<&Backchannel>,
    ) {
        let line_count = self.share_text.lines().count();

        let (mut message_bounds, _) = ui.allocate_exact_size(
            // This height is just a rough guess to look right.
            // The board emoji will fill the space, so it doesn't have to be perfect.
            vec2(ui.available_width(), (line_count * 16).min(230) as f32),
            Sense::hover(),
        );

        let target_msg_width = 180.0;
        let x_difference = (message_bounds.width() - target_msg_width) / 2.0;
        if x_difference > 0.0 {
            message_bounds = message_bounds.shrink2(vec2(x_difference, 0.0));
        }
        ui.painter()
            .rect_filled(message_bounds, 13.0, hex_color!("#494949"));

        let inner_message_bounds = message_bounds.shrink2(vec2(14.0, 10.0));
        ui.allocate_ui_at_rect(inner_message_bounds, |ui| {
            let styles = ui.style_mut();
            styles.spacing.item_spacing = Vec2::splat(0.0);
            styles.spacing.interact_size = Vec2::splat(0.0);

            let share_text = self.share_text.lines();
            for (line_num, line) in share_text.enumerate() {
                if line.chars().next().is_some_and(|c| c.is_ascii()) {
                    // This won't handle standard lines that start with an emoji,
                    // so we'll need to take care to avoid those for now.
                    ui.label(RichText::new(line).color(Color32::WHITE));
                } else {
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
                                _ => Color32::LIGHT_RED,
                            };
                            let (emoji_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(emoji_size), Sense::hover());
                            let emoji_rect = emoji_rect.shrink(2.0);
                            ui.painter().rect_filled(emoji_rect, 2.0, color);
                        }
                    });
                }
            }
        });

        ui.add_space(12.0);

        let msg = if self.share_copied {
            "COPIED TEXT!"
        } else {
            "SHARE"
        };
        let text = TextHelper::heavy(msg, 12.0, None, ui);
        let share_button = text.centered_button(
            theme.selection.lighten().lighten(),
            theme.text,
            map_texture,
            ui,
        );
        // Extra events to get this message through the backchannel early,
        // as our frontend relies on attaching the copy to a browser event
        // on mouseup/touchend.
        if share_button.clicked()
            || share_button.drag_started()
            || share_button.is_pointer_button_down_on()
        {
            if let Some(backchannel) = backchannel {
                if backchannel.is_open() {
                    backchannel.send_msg(crate::app_outer::BackchannelMsg::Copy {
                        text: self.share_text.clone(),
                    });
                } else {
                    ui.ctx()
                        .output_mut(|o| o.copied_text = self.share_text.clone());
                }
            } else {
                ui.ctx()
                    .output_mut(|o| o.copied_text = self.share_text.clone());
            }

            self.share_copied = true;
        }
    }
}
