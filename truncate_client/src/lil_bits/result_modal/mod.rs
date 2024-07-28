use epaint::{emath::Align2, hex_color, vec2, Color32, TextureHandle};
use instant::Duration;
use interpolation::Ease;
use truncate_core::{game::Game, messages::DailyStats};

mod daily_actions;
mod graph;
mod msg_mock;

use eframe::egui::{self, Align, CursorIcon, Id, Layout, Order, Sense};

use crate::{
    app_outer::Backchannel,
    utils::{
        depot::TruncateDepot,
        tex::{render_tex_quad, tiles, Tint},
        text::TextHelper,
        Lighten, Theme,
    },
};

use self::{daily_actions::DailyActions, graph::DailySplashGraph, msg_mock::ShareMessageMock};

/*

TODOs for the daily splash screen:
- Vertically center contents, or size the modal to the contents better

 */

#[derive(Clone)]
pub struct ResultModalDaily {
    pub stats: DailyStats,
    graph: DailySplashGraph,
    daily_actions: DailyActions,
    streak_length: usize,
    win_rate: f32,
}

#[derive(Clone)]
pub struct ResultModalUnique {
    won: bool,
    msg_mock: ShareMessageMock,
    share_copied_at: Option<Duration>,
}

#[derive(Clone)]
pub struct ResultModalResigning {
    msg: String,
}

#[derive(Clone)]
pub struct ResultModalLoading {}

#[derive(Clone)]
pub enum ResultModalVariant {
    Daily(ResultModalDaily),
    Unique(ResultModalUnique),
    Resigning(ResultModalResigning),
    Loading(ResultModalLoading),
}

#[derive(Clone)]
pub struct ResultModalUI {
    pub contents: ResultModalVariant,
}

impl ResultModalUI {
    pub fn new_daily(
        ui: &mut egui::Ui,
        game: &Game,
        player_move_count: u32,
        depot: &mut TruncateDepot,
        stats: DailyStats,
        best_game: Option<&Game>,
        day: u32,
    ) -> Self {
        let streak_length = stats
            .days
            .values()
            .rev()
            .enumerate()
            .find_map(|(streak_length, day)| {
                if day.attempts.iter().any(|a| a.won) {
                    None
                } else {
                    Some(streak_length)
                }
            })
            .unwrap_or_else(|| stats.days.len());

        let win_count = stats
            .days
            .values()
            .filter(|day| day.attempts.iter().any(|a| a.won))
            .count();
        let attempted_day_count = stats
            .days
            .values()
            .filter(|day| !day.attempts.is_empty())
            .count();

        ResultModalUI::seed_animations(ui);

        let graph = DailySplashGraph::new(ui, &stats, depot.timing.current_time);
        let daily_actions = DailyActions::new(
            best_game.unwrap_or(game),
            player_move_count,
            &depot,
            &stats,
            day,
        );

        Self {
            contents: ResultModalVariant::Daily(ResultModalDaily {
                stats,
                graph,
                daily_actions,
                streak_length,
                win_rate: win_count as f32 / attempted_day_count as f32,
            }),
        }
    }

    pub fn new_unique(
        ui: &mut egui::Ui,
        game: &Game,
        depot: &mut TruncateDepot,
        won: bool,
    ) -> Self {
        ResultModalUI::seed_animations(ui);

        Self {
            contents: ResultModalVariant::Unique(ResultModalUnique {
                won,
                msg_mock: ShareMessageMock::new_unique(game, &depot),
                share_copied_at: None,
            }),
        }
    }

    pub fn new_resigning(ui: &mut egui::Ui, msg: String) -> Self {
        ResultModalUI::seed_animations(ui);

        Self {
            contents: ResultModalVariant::Resigning(ResultModalResigning { msg }),
        }
    }

    pub fn new_loading(ui: &mut egui::Ui) -> Self {
        ResultModalUI::seed_animations(ui);

        Self {
            contents: ResultModalVariant::Loading(ResultModalLoading {}),
        }
    }
}

#[derive(Hash)]
enum Anim {
    Background,
    ModalPos,
    Items,
    Viz,
}

impl Into<Id> for Anim {
    fn into(self) -> Id {
        Id::new("splash").with(self)
    }
}

impl ResultModalUI {
    fn seed_animations(ui: &mut egui::Ui) {
        ResultModalUI::anim(ui, Anim::Background, 0.0, 0.0);
        ResultModalUI::anim(ui, Anim::ModalPos, 0.0, 0.0);
        ResultModalUI::anim(ui, Anim::Items, 0.0, 0.0);
        ResultModalUI::anim(ui, Anim::Viz, 0.0, 0.0);
    }

    // Animates to a given value once (can't be retriggered), applying an easing function
    fn anim(ui: &mut egui::Ui, val: Anim, to: f32, duration: f32) -> f32 {
        let linear = ui.ctx().animate_value_with_time(
            val.into(),
            if to > 0.0 { 1.0 } else { 0.0 },
            duration,
        );
        let eased = linear.calc(interpolation::EaseFunction::QuadraticOut);
        eased * to
    }
}

pub enum ResultModalAction {
    TryAgain,
    NewPuzzle,
    Dismiss,
    Resign,
    SharedText,
    SharedReplay,
}

impl ResultModalUI {
    fn render_close(
        &mut self,
        ui: &mut egui::Ui,
        map_texture: &TextureHandle,
        animate: f32,
    ) -> Option<ResultModalAction> {
        let mut close_rect = ui.available_rect_before_wrap();
        close_rect.set_right(close_rect.right() - 5.0);
        close_rect.set_top(close_rect.top() + 5.0);
        close_rect.set_left(close_rect.right() - 32.0);
        close_rect.set_bottom(close_rect.top() + 32.0);

        let close_resp = ui.interact(close_rect, ui.id().with("close"), Sense::click());

        if close_resp.hovered() {
            close_rect = close_rect.translate(vec2(0.0, -2.0));
            ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
        }

        render_tex_quad(
            tiles::quad::CLOSE_BUTTON.tint(Color32::WHITE.gamma_multiply(animate * 0.7)),
            close_rect,
            &map_texture,
            ui,
        );

        if close_resp.clicked() {
            Some(ResultModalAction::Dismiss)
        } else {
            None
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

        let area = egui::Area::new(egui::Id::new("daily_splash_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_TOP, vec2(0.0, 0.0));

        let ideal_modal_width = 370.0;
        let ideal_modal_height = 620.0;

        area.show(ui.ctx(), |ui| {
            let screen_dimension = ui.max_rect();

            // Capture events on our overlay to stop them falling through to the game
            ui.allocate_rect(screen_dimension, Sense::click());

            let bg_alpha = ResultModalUI::anim(ui, Anim::Background, 0.5, 1.2);
            let bg = Color32::BLACK.gamma_multiply(bg_alpha);

            ui.painter().clone().rect_filled(screen_dimension, 0.0, bg);

            // Wait for the background overlay to start animating before showing the modal
            if bg_alpha < 0.3 {
                return;
            }

            let mut x_difference = (screen_dimension.width() - ideal_modal_width) / 2.0;
            let mut y_difference = (screen_dimension.height() - ideal_modal_height) / 2.0;

            if x_difference < 10.0 {
                x_difference = 10.0;
            }
            if y_difference < 10.0 {
                y_difference = 10.0;
            }

            let mut modal_dimension = screen_dimension.shrink2(vec2(x_difference, y_difference));

            let modal_pos = ResultModalUI::anim(ui, Anim::ModalPos, 1.0, 0.5);
            let bg = hex_color!("#111111").gamma_multiply(modal_pos); // Fade in the modal background
            let offset = (1.0 - modal_pos) * 40.0;
            modal_dimension = modal_dimension.translate(vec2(0.0, offset)); // Animate the modal in vertically

            ui.painter().rect_filled(modal_dimension, 10.0, bg);

            // Wait for the modal position to be close before showing the contents
            if modal_pos < 0.7 {
                return;
            }

            let modal_inner_dimension = modal_dimension.shrink(10.0);
            ui.allocate_ui_at_rect(modal_inner_dimension, |mut ui| {
                let modal_items = ResultModalUI::anim(ui, Anim::Items, 1.0, 0.75);

                if let Some(close_msg) = self.render_close(ui, map_texture, modal_items) {
                    msg = Some(close_msg);
                }

                ui.add_space(12.0);

                let offset = (1.0 - modal_items) * 50.0;
                ui.add_space(offset); // Animate the main text upward

                ui.spacing_mut().item_spacing = vec2(0.0, 10.0);

                let (heading_rect, _) = ui.allocate_exact_size(
                    vec2(ui.available_width(), ui.available_height() / 7.0),
                    Sense::hover(),
                );

                match &mut self.contents {
                    ResultModalVariant::Daily(daily) => {
                        let streak_string = format!("{} day streak", daily.streak_length);
                        let streak_text = TextHelper::heavy(&streak_string, 14.0, None, &mut ui);

                        let padding = streak_text.mesh_size().y / 2.0;

                        streak_text.paint_within(
                            heading_rect
                                .translate(vec2(0.0, -(heading_rect.height() / 2.0 + padding))),
                            Align2::CENTER_BOTTOM,
                            Color32::WHITE,
                            ui,
                        );

                        let wr_string = format!("{}% win rate", (daily.win_rate * 100.0) as usize);
                        let wr_text = TextHelper::heavy(&wr_string, 12.0, None, &mut ui);

                        wr_text.paint_within(
                            heading_rect
                                .translate(vec2(0.0, heading_rect.height() / 2.0 + padding)),
                            Align2::CENTER_TOP,
                            Color32::WHITE,
                            ui,
                        );
                    }
                    ResultModalVariant::Unique(u) => {
                        if u.won {
                            let summary_string = "Great job!".to_string();
                            let summary_text =
                                TextHelper::heavy(&summary_string, 14.0, None, &mut ui);

                            summary_text.paint_within(
                                heading_rect,
                                Align2::CENTER_CENTER,
                                Color32::WHITE,
                                ui,
                            );
                        } else {
                            let summary_string = "No worries,".to_string();
                            let summary_text =
                                TextHelper::heavy(&summary_string, 14.0, None, &mut ui);

                            let padding = summary_text.mesh_size().y / 2.0;

                            summary_text.paint_within(
                                heading_rect
                                    .translate(vec2(0.0, -(heading_rect.height() / 2.0 + padding))),
                                Align2::CENTER_BOTTOM,
                                Color32::WHITE,
                                ui,
                            );

                            let summary_string = "have another go!".to_string();
                            let summary_text =
                                TextHelper::heavy(&summary_string, 14.0, None, &mut ui);

                            summary_text.paint_within(
                                heading_rect
                                    .translate(vec2(0.0, heading_rect.height() / 2.0 + padding)),
                                Align2::CENTER_TOP,
                                Color32::WHITE,
                                ui,
                            );
                        }
                    }
                    ResultModalVariant::Resigning(r) => {
                        let summary_text = TextHelper::heavy(&r.msg, 14.0, None, &mut ui);

                        summary_text.paint_within(
                            heading_rect,
                            Align2::CENTER_CENTER,
                            Color32::WHITE,
                            ui,
                        );
                    }
                    ResultModalVariant::Loading(_l) => {
                        let summary_text = TextHelper::heavy("Loading", 12.0, None, &mut ui);

                        summary_text.paint_within(
                            heading_rect.translate(vec2(0.0, -(heading_rect.height() / 2.0 + 8.0))),
                            Align2::CENTER_BOTTOM,
                            Color32::WHITE,
                            ui,
                        );

                        let summary_text = TextHelper::heavy("Statistics", 12.0, None, &mut ui);

                        summary_text.paint_within(
                            heading_rect.translate(vec2(0.0, heading_rect.height() / 2.0 + 8.0)),
                            Align2::CENTER_TOP,
                            Color32::WHITE,
                            ui,
                        );
                    }
                }

                // Wait for the main text to move out of the way before showing details
                if modal_items < 0.9 {
                    return;
                }

                let modal_remainder = ui.available_rect_before_wrap();

                if let ResultModalVariant::Daily(daily) = &mut self.contents {
                    let (graph_rect, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), ui.available_height() / 5.0),
                        Sense::hover(),
                    );
                    daily.graph.render(ui, graph_rect.shrink2(vec2(10.0, 0.0)));
                }

                match &mut self.contents {
                    ResultModalVariant::Daily(daily) => {
                        if let Some(action) =
                            daily
                                .daily_actions
                                .render(ui, theme, map_texture, depot, backchannel)
                        {
                            msg = Some(action);
                        }
                    }
                    ResultModalVariant::Unique(unique) => {
                        if unique
                            .share_copied_at
                            .is_some_and(|s| depot.timing.current_time - s > Duration::from_secs(2))
                        {
                            unique.share_copied_at = None;
                        }

                        ui.allocate_ui_with_layout(
                            ui.available_size(),
                            Layout::bottom_up(Align::LEFT),
                            |ui| {
                                ui.add_space(ui.available_height() * 0.05);
                                let text = TextHelper::heavy("NEW PUZZLE", 12.0, None, ui);
                                let new_puzzle_button = text.centered_button(
                                    theme.button_primary,
                                    theme.text,
                                    map_texture,
                                    ui,
                                );
                                if new_puzzle_button.clicked() {
                                    msg = Some(ResultModalAction::NewPuzzle);
                                }

                                ui.add_space(ui.available_height() * 0.05);
                                let text = TextHelper::heavy("TRY AGAIN", 12.0, None, ui);
                                let try_again_button = text.centered_button(
                                    theme.button_primary,
                                    theme.text,
                                    map_texture,
                                    ui,
                                );
                                if try_again_button.clicked() {
                                    msg = Some(ResultModalAction::TryAgain);
                                }

                                ui.add_space(ui.available_height() * 0.05);
                                let button_text = if unique.share_copied_at.is_some() {
                                    "COPIED TEXT!"
                                } else {
                                    "SHARE PUZZLE"
                                };
                                let text = TextHelper::heavy(button_text, 12.0, None, ui);
                                let share_button = text.centered_button(
                                    theme.button_primary,
                                    theme.text,
                                    map_texture,
                                    ui,
                                );
                                // Extra events to get this message through the backchannel early,
                                // as our frontend relies on attaching the copy to a browser event
                                // on mouseup/touchend.
                                if unique.share_copied_at.is_none()
                                    && (share_button.clicked()
                                        || share_button.drag_started()
                                        || share_button.is_pointer_button_down_on())
                                {
                                    msg = Some(ResultModalAction::SharedText);
                                    let share_text = unique.msg_mock.share_text.clone();
                                    if let Some(backchannel) = backchannel {
                                        if backchannel.is_open() {
                                            backchannel.send_msg(
                                                crate::app_outer::BackchannelMsg::Copy {
                                                    text: share_text,
                                                    share: crate::app_outer::ShareType::Text,
                                                },
                                            );
                                        } else {
                                            ui.ctx().output_mut(|o| o.copied_text = share_text);
                                        }
                                    } else {
                                        ui.ctx().output_mut(|o| o.copied_text = share_text);
                                    }

                                    unique.share_copied_at = Some(depot.timing.current_time);
                                }

                                ui.add_space(ui.available_height() * 0.05);
                                unique.msg_mock.render(ui, theme, map_texture);
                            },
                        );
                    }
                    ResultModalVariant::Resigning(_r) => {
                        ui.add_space(20.0);
                        let text = TextHelper::heavy("RESIGN", 12.0, None, ui);
                        let try_again_button =
                            text.centered_button(theme.button_primary, theme.text, map_texture, ui);
                        if try_again_button.clicked() {
                            msg = Some(ResultModalAction::Resign);
                        }

                        ui.add_space(10.0);
                        let text = TextHelper::heavy("CONTINUE PLAYING", 12.0, None, ui);
                        let new_puzzle_button = text.centered_button(
                            theme.water.lighten().lighten(),
                            theme.text,
                            map_texture,
                            ui,
                        );
                        if new_puzzle_button.clicked() {
                            msg = Some(ResultModalAction::Dismiss);
                        }
                    }
                    ResultModalVariant::Loading(_l) => {
                        ui.add_space(50.0);

                        let summary_text = TextHelper::heavy("Waiting for", 10.0, None, &mut ui);

                        summary_text.paint(Color32::WHITE, ui, true);

                        let summary_text =
                            TextHelper::heavy("network connection", 10.0, None, &mut ui);

                        summary_text.paint(Color32::WHITE, ui, true);
                    }
                };

                // Paint over everything below the heading stats to fade them in from black
                let fade_in_animation = ResultModalUI::anim(ui, Anim::Viz, 1.0, 0.6);
                if fade_in_animation < 1.0 {
                    ui.painter().rect_filled(
                        modal_remainder,
                        0.0,
                        hex_color!("#111111").gamma_multiply(1.0 - fade_in_animation),
                    );
                }
            });
        });

        msg
    }
}
