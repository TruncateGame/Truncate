use eframe::egui::{self, Layout, RichText, Sense};
use epaint::{emath::Align, hex_color, vec2, Color32, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    game::Game,
    messages::{DailyAttempt, DailyStats, PlayerMessage},
};

use crate::{
    app_outer::{Backchannel, ShareType},
    utils::{depot::TruncateDepot, macros::tr_log, text::TextHelper, Lighten, Theme},
};

use super::{msg_mock::ShareMessageMock, ResultModalAction};

#[derive(Clone)]
pub struct DailyActions {
    msg_mock: ShareMessageMock,
    replay_link: String,
    replay_copied_at: Option<Duration>,
    share_copied_at: Option<Duration>,
    won_today: bool,
    won_yesterday: bool,
    is_sharing: bool,
}

impl DailyActions {
    pub fn new(
        game: &Game,
        player_move_count: u32,
        depot: &TruncateDepot,
        stats: &DailyStats,
        day: u32,
    ) -> Self {
        let mut first_win = None;
        let mut best_win = None;
        let mut latest_attempt = None;

        if let Some(today_result) = stats.days.get(&day) {
            best_win = today_result
                .attempts
                .iter()
                .filter(|a| a.won)
                .min_by_key(|a| a.moves);

            first_win = today_result
                .attempts
                .iter()
                .enumerate()
                .find(|(_, a)| a.won)
                .map(|(i, a)| (i as u32, a));

            latest_attempt = today_result
                .attempts
                .last()
                .cloned()
                .map(|a| (today_result.attempts.len() as u32 - 1, a));
        };

        let latest_attempt = latest_attempt.unwrap_or_else(|| {
            (
                0,
                DailyAttempt {
                    id: "UNAVAILABLE".to_string(),
                    moves: player_move_count,
                    won: game.winner == Some(depot.gameplay.player_number as usize),
                },
            )
        });

        let shared_attempt = best_win.unwrap_or(&latest_attempt.1);

        let msg_mock = ShareMessageMock::new_daily(
            day,
            game,
            depot,
            stats,
            first_win,
            best_win,
            (latest_attempt.0, &latest_attempt.1),
        );

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
            replay_link: format!(
                "https://truncate.town/#REPLAY:{}",
                shared_attempt.id.clone()
            ),
            replay_copied_at: None,
            share_copied_at: None,
            won_today: win_history(0),
            won_yesterday: win_history(1),
            is_sharing: false,
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

    pub fn render_shares(
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
            ui.add_space(ui.available_height() * 0.05);

            let text = TextHelper::heavy("PLAY AGAIN", 12.0, None, ui);
            let try_again_button =
                text.centered_button(theme.button_secondary, theme.text, map_texture, ui);
            if try_again_button.clicked() {
                msg = Some(ResultModalAction::TryAgain);
            }

            ui.allocate_ui_with_layout(ui.available_size(), Layout::top_down(Align::LEFT), |ui| {
                let share_text = if self.share_copied_at.is_some() {
                    "COPIED TEXT!"
                } else {
                    "SHARE BEST SCORE"
                };
                let share_button_text = TextHelper::heavy(share_text, 12.0, None, ui);

                let replay_text = if self.replay_copied_at.is_some() {
                    "COPIED LINK!"
                } else {
                    "SHARE REPLAY LINK"
                };
                let replay_button_text = TextHelper::heavy(replay_text, 12.0, None, ui);

                let button_height = share_button_text.get_button_height(ui);
                let block_height = button_height * 2.5;

                ui.add_space((ui.available_height() - block_height) / 2.0);

                let share_button = share_button_text.centered_button(
                    theme.button_primary,
                    theme.text,
                    map_texture,
                    ui,
                );

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

                ui.add_space(button_height * 0.5);

                let replay_button = replay_button_text.centered_button(
                    theme.button_primary,
                    theme.text,
                    map_texture,
                    ui,
                );

                if self.replay_copied_at.is_none()
                    && (replay_button.clicked()
                        || replay_button.drag_started()
                        || replay_button.is_pointer_button_down_on())
                {
                    if let Some(backchannel) = backchannel {
                        if backchannel.is_open() {
                            backchannel.send_msg(crate::app_outer::BackchannelMsg::Copy {
                                text: self.replay_link.clone(),
                                share: ShareType::Url,
                            });
                        } else {
                            ui.ctx()
                                .output_mut(|o| o.copied_text = self.replay_link.clone());
                        }
                    } else {
                        ui.ctx()
                            .output_mut(|o| o.copied_text = self.replay_link.clone());
                    }

                    self.replay_copied_at = Some(depot.timing.current_time);
                }
            });
        });

        msg
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        map_texture: &TextureHandle,
        depot: &TruncateDepot,
        backchannel: Option<&Backchannel>,
    ) -> Option<ResultModalAction> {
        if self.is_sharing {
            return self.render_shares(ui, theme, map_texture, depot, backchannel);
        }

        let mut msg = None;

        ui.allocate_ui_with_layout(ui.available_size(), Layout::bottom_up(Align::LEFT), |ui| {
            ui.add_space(ui.available_height() * 0.05);

            let text = TextHelper::heavy("PLAY AGAIN", 12.0, None, ui);
            let try_again_button =
                text.centered_button(theme.button_secondary, theme.text, map_texture, ui);
            if try_again_button.clicked() {
                msg = Some(ResultModalAction::TryAgain);
            }

            let play_again_msg = if self.won_yesterday && !self.won_today {
                "Keep your streak alive!"
            } else {
                "Try for a better score?"
            };

            let row =
                TextHelper::heavy(play_again_msg.into(), 10.0, Some(ui.available_width()), ui);
            row.paint(theme.button_secondary, ui, true);

            ui.add_space(ui.available_height() * 0.05);

            let text = TextHelper::heavy("SHARE", 12.0, None, ui);
            let share_buton =
                text.centered_button(theme.button_primary, theme.text, map_texture, ui);

            if share_buton.clicked() {
                self.is_sharing = true;
            }

            ui.add_space(ui.available_height() * 0.05);

            self.msg_mock.render(ui, theme, map_texture);
        });

        msg
    }
}
