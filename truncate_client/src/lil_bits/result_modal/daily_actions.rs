use eframe::egui::{self, Layout, RichText, Sense};
use epaint::{emath::Align, hex_color, vec2, Color32, TextureHandle, Vec2};
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
    replay_copied: bool,
    share_copied: bool,
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
            replay_copied: false,
            share_copied: false,
            won_today: win_history(0),
            won_yesterday: win_history(1),
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        map_texture: &TextureHandle,
        backchannel: Option<&Backchannel>,
    ) -> Option<ResultModalAction> {
        let mut msg = None;

        ui.allocate_ui_with_layout(ui.available_size(), Layout::bottom_up(Align::LEFT), |ui| {
            let mut textrow = |string: String, ui: &mut egui::Ui| {
                let row = TextHelper::heavy(&string, 14.0, Some(ui.available_width()), ui);
                row.paint(Color32::WHITE, ui, true);
            };

            ui.add_space(ui.available_height() * 0.05);

            if self.won_today {
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

                let text = TextHelper::heavy("SHARE BEST SCORE", 12.0, None, ui);
                let share_button =
                    text.centered_button(theme.button_primary, theme.text, map_texture, ui);

                if share_button.clicked()
                    || share_button.drag_started()
                    || share_button.is_pointer_button_down_on()
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
                }

                self.msg_mock.render(ui, theme, map_texture);
            }
        });

        // let msg = if self.share_copied {
        //     "COPIED TEXT!"
        // } else {
        //     "COPY SUMMARY"
        // };
        // let text = TextHelper::heavy(msg, 12.0, None, ui);
        // let share_button = text.centered_button(theme.button_primary, theme.text, map_texture, ui);
        // // Extra events to get this message through the backchannel early,
        // // as our frontend relies on attaching the copy to a browser event
        // // on mouseup/touchend.
        // if share_button.clicked()
        //     || share_button.drag_started()
        //     || share_button.is_pointer_button_down_on()
        // {
        //     if let Some(backchannel) = backchannel {
        //         if backchannel.is_open() {
        //             backchannel.send_msg(crate::app_outer::BackchannelMsg::Copy {
        //                 text: self.share_text.clone(),
        //             });
        //         } else {
        //             ui.ctx()
        //                 .output_mut(|o| o.copied_text = self.share_text.clone());
        //         }
        //     } else {
        //         ui.ctx()
        //             .output_mut(|o| o.copied_text = self.share_text.clone());
        //     }

        //     self.share_copied = true;
        // }

        // if let Some(replay_link) = &self.replay_link {
        //     let msg = if self.replay_copied {
        //         "COPIED TEXT!"
        //     } else {
        //         "COPY LINK TO REPLAY"
        //     };
        //     let text = TextHelper::heavy(msg, 12.0, None, ui);
        //     let replay_button =
        //         text.centered_button(theme.button_secondary, theme.text, map_texture, ui);
        //     // Extra events to get this message through the backchannel early,
        //     // as our frontend relies on attaching the copy to a browser event
        //     // on mouseup/touchend.
        //     if replay_button.clicked()
        //         || replay_button.drag_started()
        //         || replay_button.is_pointer_button_down_on()
        //     {
        //         if let Some(backchannel) = backchannel {
        //             if backchannel.is_open() {
        //                 backchannel.send_msg(crate::app_outer::BackchannelMsg::Copy {
        //                     text: replay_link.clone(),
        //                 });
        //             } else {
        //                 ui.ctx().output_mut(|o| o.copied_text = replay_link.clone());
        //             }
        //         } else {
        //             ui.ctx().output_mut(|o| o.copied_text = replay_link.clone());
        //         }

        //         self.replay_copied = true;
        //     }
        // }

        // if self.won_today {
        //     self.msg_mock.render(ui, theme, map_texture, backchannel);
        // }
        // if won_today {
        //     daily.msg_mock.render(ui, theme, map_texture, backchannel);
        // } else {
        //     ui.add_space(20.0);

        //     let mut textrow = |string: String| {
        //         let row = TextHelper::heavy(&string, 14.0, Some(ui.available_width()), &mut ui);
        //         row.paint(Color32::WHITE, ui, true);
        //     };

        //     if won_yesterday {
        //         textrow("Try again".into());
        //         textrow("to maintain".into());
        //         textrow("your streak!".into());
        //     } else {
        //         textrow("No worries,".into());
        //         textrow("have another go!".into());
        //     }

        //     ui.add_space(16.0);

        //     let text = TextHelper::heavy("TRY AGAIN", 12.0, None, ui);
        //     let try_again_button =
        //         text.centered_button(theme.button_primary, theme.text, map_texture, ui);
        //     if try_again_button.clicked() {
        //         msg = Some(ResultModalAction::TryAgain);
        //     }
        // }

        msg
    }
}
