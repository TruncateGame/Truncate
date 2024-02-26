use eframe::egui::{self, Layout, RichText, Sense};
use epaint::{emath::Align, hex_color, vec2, Color32, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    game::Game,
    messages::{DailyStats, PlayerMessage},
};

use crate::{
    app_outer::{Backchannel, ShareType},
    utils::{depot::TruncateDepot, macros::tr_log, text::TextHelper, Lighten, Theme},
};

use super::{msg_mock::ShareMessageMock, ResultModalAction};

#[derive(Clone)]
pub struct DailyActions {
    msg_mock: ShareMessageMock,
    replay_link: Option<String>,
    replay_copied_at: Option<Duration>,
    share_copied_at: Option<Duration>,
    won_today: bool,
    won_yesterday: bool,
}

impl DailyActions {
    pub fn new(game: &Game, depot: &TruncateDepot, stats: &DailyStats) -> Self {
        let this_attempt = stats
            .days
            .last_key_value()
            .map(|(_, v)| v.attempts.last().map(|a| a.id.clone()))
            .flatten();

        let msg_mock = ShareMessageMock::new_daily(game, &depot, &stats);

        let win_history = |rev_day: usize| {
            stats
                .days
                .values()
                .nth_back(rev_day)
                .cloned()
                .unwrap_or_default()
                .attempts
                .iter()
                .any(|a| a.won)
        };

        Self {
            msg_mock,
            replay_link: this_attempt.map(|a| format!("https://truncate.town/#REPLAY:{a}")),
            replay_copied_at: None,
            share_copied_at: None,
            won_today: win_history(0),
            won_yesterday: win_history(1),
        }
    }

    fn reset_buttons(&mut self, depot: &TruncateDepot) {
        if self
            .share_copied_at
            .is_some_and(|s| depot.timing.current_time - s > Duration::from_secs(2))
        {
            self.share_copied_at = None;
        }

        if self
            .replay_copied_at
            .is_some_and(|s| depot.timing.current_time - s > Duration::from_secs(2))
        {
            self.replay_copied_at = None;
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        map_texture: &TextureHandle,
        depot: &TruncateDepot,
        backchannel: Option<&Backchannel>,
    ) -> Option<ResultModalAction> {
        let mut msg = None;

        self.reset_buttons(depot);

        ui.allocate_ui_with_layout(ui.available_size(), Layout::bottom_up(Align::LEFT), |ui| {
            let mut textrow = |string: String, ui: &mut egui::Ui| {
                let row = TextHelper::heavy(&string, 14.0, Some(ui.available_width()), ui);
                row.paint(Color32::WHITE, ui, true);
            };

            ui.add_space(ui.available_height() * 0.05);

            let text = TextHelper::heavy("PLAY AGAIN", 12.0, None, ui);
            let try_again_button =
                text.centered_button(theme.button_secondary, theme.text, map_texture, ui);
            if try_again_button.clicked() {
                msg = Some(ResultModalAction::TryAgain);
            }

            let row = TextHelper::heavy(
                "Try for a better score?".into(),
                10.0,
                Some(ui.available_width()),
                ui,
            );
            row.paint(theme.button_secondary, ui, true);

            ui.add_space(ui.available_height() * 0.05);

            if let Some(replay_link) = &self.replay_link {
                let button_text = if self.replay_copied_at.is_some() {
                    "COPIED LINK!"
                } else {
                    "SHARE REPLAY"
                };
                let text = TextHelper::heavy(button_text, 12.0, None, ui);
                let replay_button =
                    text.centered_button(theme.button_primary, theme.text, map_texture, ui);

                if self.replay_copied_at.is_none()
                    && (replay_button.clicked()
                        || replay_button.drag_started()
                        || replay_button.is_pointer_button_down_on())
                {
                    if let Some(backchannel) = backchannel {
                        if backchannel.is_open() {
                            backchannel.send_msg(crate::app_outer::BackchannelMsg::Copy {
                                text: replay_link.clone(),
                                share: ShareType::Url,
                            });
                        } else {
                            ui.ctx().output_mut(|o| o.copied_text = replay_link.clone());
                        }
                    } else {
                        ui.ctx().output_mut(|o| o.copied_text = replay_link.clone());
                    }

                    self.replay_copied_at = Some(depot.timing.current_time);
                }

                ui.add_space(ui.available_height() * 0.01);
            }

            let button_text = if self.share_copied_at.is_some() {
                "COPIED TEXT!"
            } else {
                "SHARE BEST SCORE"
            };
            let text = TextHelper::heavy(button_text, 12.0, None, ui);
            let share_button =
                text.centered_button(theme.button_primary, theme.text, map_texture, ui);

            if self.share_copied_at.is_none()
                && (share_button.clicked()
                    || share_button.drag_started()
                    || share_button.is_pointer_button_down_on())
            {
                if let Some(backchannel) = backchannel {
                    if backchannel.is_open() {
                        backchannel.send_msg(crate::app_outer::BackchannelMsg::Copy {
                            text: self.msg_mock.share_text.clone(),
                            share: ShareType::Text,
                        });
                    } else {
                        ui.ctx()
                            .output_mut(|o| o.copied_text = self.msg_mock.share_text.clone());
                    }
                } else {
                    ui.ctx()
                        .output_mut(|o| o.copied_text = self.msg_mock.share_text.clone());
                }

                self.share_copied_at = Some(depot.timing.current_time);
            }

            self.msg_mock.render(ui, theme, map_texture);
        });

        msg
    }
}
