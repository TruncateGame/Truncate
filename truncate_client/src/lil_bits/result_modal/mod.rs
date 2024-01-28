use epaint::{emath::Align2, vec2, Color32, TextureHandle};
use instant::Duration;
use interpolation::Ease;
use truncate_core::{
    game::Game,
    messages::{DailyStats, PlayerMessage},
};

mod graph;
mod msg_mock;

use eframe::egui::{self, Id, Order, Sense};

use crate::{
    app_outer::Backchannel,
    utils::{depot::TruncateDepot, text::TextHelper, Lighten, Theme},
};

use self::{graph::DailySplashGraph, msg_mock::ShareMessageMock};

/*

TODOs for the daily splash screen:
- Vertically center contents, or size the modal to the contents better
- Add ability to close the splash screen and look at the game

 */

#[derive(Clone)]
pub struct ResultModalUI {
    pub stats: DailyStats,
    graph: DailySplashGraph,
    msg_mock: ShareMessageMock,
    streak_length: usize,
    win_rate: f32,
}

impl ResultModalUI {
    pub fn new(
        ui: &mut egui::Ui,
        game: &Game,
        depot: &mut TruncateDepot,
        current_time: Duration,
        stats: DailyStats,
    ) -> Self {
        let streak_length = stats
            .days
            .values()
            .rev()
            .enumerate()
            .find_map(|(streak_length, day)| {
                if day.attempts.last().map(|a| a.won) == Some(true) {
                    None
                } else {
                    Some(streak_length)
                }
            })
            .unwrap_or_else(|| stats.days.len());

        let win_count = stats
            .days
            .values()
            .filter(|day| day.attempts.last().map(|a| a.won) == Some(true))
            .count();

        let game_count: usize = stats.days.values().map(|day| day.attempts.len()).sum();

        ResultModalUI::seed_animations(ui);

        let graph = DailySplashGraph::new(ui, &stats, current_time);

        let msg_mock = ShareMessageMock::new(game, &depot, &stats);

        Self {
            msg_mock,
            graph,
            streak_length,
            win_rate: win_count as f32 / game_count as f32,
            stats,
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

impl ResultModalUI {
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        map_texture: &TextureHandle,
        backchannel: Option<&Backchannel>,
    ) -> Option<PlayerMessage> {
        let mut msg = None;
        let today = self.stats.days.values().last().cloned().unwrap_or_default();
        let yesterday = self
            .stats
            .days
            .values()
            .nth_back(1)
            .cloned()
            .unwrap_or_default();
        let area = egui::Area::new(egui::Id::new("daily_splash_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_TOP, vec2(0.0, 0.0));

        let ideal_modal_width = 370.0;
        let ideal_modal_height = 570.0;

        area.show(ui.ctx(), |ui| {
            let screen_dimension = ui.max_rect();

            // Capture events on our overlay to stop them falling through to the game
            ui.allocate_rect(screen_dimension, Sense::click());

            let bg_alpha = ResultModalUI::anim(ui, Anim::Background, 0.5, 1.5);
            let bg = Color32::BLACK.gamma_multiply(bg_alpha);

            ui.painter().clone().rect_filled(screen_dimension, 10.0, bg);

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
            let bg = Color32::BLACK.gamma_multiply(modal_pos); // Fade in the modal background
            let offset = (1.0 - modal_pos) * 40.0;
            modal_dimension = modal_dimension.translate(vec2(0.0, offset)); // Animate the modal in vertically

            ui.painter().rect_filled(modal_dimension, 7.0, bg);

            // Wait for the modal position to be close before showing the contents
            if modal_pos < 0.7 {
                return;
            }

            let modal_inner_dimension = modal_dimension.shrink(30.0);
            ui.allocate_ui_at_rect(modal_inner_dimension, |mut ui| {
                // TODO: Add close button (reference game sidebar on mobile)

                let modal_items = ResultModalUI::anim(ui, Anim::Items, 1.0, 0.75);
                let offset = (1.0 - modal_items) * 50.0;
                ui.add_space(offset); // Animate the main text upward

                ui.spacing_mut().item_spacing = vec2(0.0, 10.0);

                ui.add_space(10.0);

                let streak_string = format!("{} day streak", self.streak_length);
                let streak_text = TextHelper::heavy(&streak_string, 14.0, None, &mut ui);
                streak_text.paint(Color32::WHITE, ui, true);

                ui.add_space(4.0);

                let wr_string = format!("{}% win rate", (self.win_rate * 100.0) as usize);
                let wr_text = TextHelper::heavy(&wr_string, 12.0, None, &mut ui);
                wr_text.paint(Color32::WHITE, ui, true);

                // Wait for the main text to move out of the way before showing details
                if modal_items < 0.9 {
                    return;
                }

                let modal_remainder = ui.available_rect_before_wrap();

                ui.add_space(16.0);

                self.graph.render(ui);

                ui.add_space(20.0);

                if today.attempts.last().is_some_and(|a| a.won) {
                    self.msg_mock.render(ui, theme, map_texture, backchannel);
                } else {
                    ui.add_space(20.0);

                    let mut textrow = |string: String| {
                        let row =
                            TextHelper::heavy(&string, 14.0, Some(ui.available_width()), &mut ui);
                        row.paint(Color32::WHITE, ui, true);
                    };

                    if yesterday.attempts.last().is_some_and(|a| a.won) {
                        textrow("Try again".into());
                        textrow("to maintain".into());
                        textrow("your streak!".into());
                    } else {
                        textrow("No worries,".into());
                        textrow("have another go!".into());
                    }

                    ui.add_space(16.0);

                    let text = TextHelper::heavy("TRY AGAIN", 12.0, None, ui);
                    let try_again_button =
                        text.centered_button(theme.button_primary, theme.text, map_texture, ui);
                    if try_again_button.clicked() {
                        msg = Some(PlayerMessage::Rematch);
                    }
                }

                // Paint over everything below the heading stats to fade them in from black
                let fade_in_animation = ResultModalUI::anim(ui, Anim::Viz, 1.0, 0.6);
                if fade_in_animation < 1.0 {
                    ui.painter().rect_filled(
                        modal_remainder,
                        0.0,
                        Color32::BLACK.gamma_multiply(1.0 - fade_in_animation),
                    )
                }
            });
        });

        msg
    }
}
