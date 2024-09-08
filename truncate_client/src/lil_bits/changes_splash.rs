use eframe::egui::{self};
use epaint::{vec2, Color32, TextureHandle};
use std::f32;
use time::Duration;

use crate::utils::{text::TextHelper, Darken, Theme};

struct SplashButton {
    id: &'static str,
    text: String,
    color: Color32,
}

pub struct ChangelogSplashUI {
    message: Vec<String>,
    animated: bool,
    buttons: Vec<SplashButton>,
    animate_from: Duration,
    done_stages: usize,
}

#[derive(Default)]
pub struct SplashResponse {
    pub clicked: Option<&'static str>,
}

impl ChangelogSplashUI {
    pub fn new(message: Vec<String>, opened_at: Duration) -> Self {
        Self {
            message,
            animated: false,
            buttons: vec![],
            animate_from: opened_at,
            done_stages: 0,
        }
    }

    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    pub fn with_button(mut self, id: &'static str, text: String, color: Color32) -> Self {
        self.buttons.push(SplashButton { id, text, color });
        self
    }
}

impl ChangelogSplashUI {
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        current_time: Duration,
        map_texture: &TextureHandle,
    ) -> SplashResponse {
        let background = ui.available_rect_before_wrap();
        ui.painter()
            .rect_filled(background, 0.0, Color32::BLACK.gamma_multiply(0.2));

        let max_text_width = (ui.available_width() - 48.0).min(600.0);
        let text_pause_delay = Duration::milliseconds(500);
        let button_pause_delay = Duration::milliseconds(350);

        let text_blocks = self
            .message
            .iter()
            .map(|msg| TextHelper::heavy(&msg, 14.0, Some(max_text_width.max(0.0)), ui))
            .collect::<Vec<_>>();

        let total_text_height: f32 = text_blocks.iter().map(|b| b.mesh_size().y + 20.0).sum();
        let total_text_blocks = text_blocks.len();

        let buttons: Vec<_> = self
            .buttons
            .iter()
            .map(|button| (button, TextHelper::heavy(&button.text, 14.0, None, ui)))
            .collect();
        let total_button_height: f32 = buttons.iter().map(|(_, b)| b.size().y * 2.0 + 10.0).sum();

        let required_size = vec2(
            max_text_width,
            total_text_height + total_button_height + 20.0,
        );
        let margins = (ui.available_size_before_wrap() - required_size) / 2.0;
        let outer_frame = egui::Frame::none().inner_margin(egui::Margin::from(margins));

        let mut splash_resp = SplashResponse::default();

        outer_frame.show(ui, |ui| {
            for (i, block) in text_blocks.into_iter().enumerate() {
                if i < self.done_stages {
                    block.paint(Color32::WHITE, ui, false);
                    ui.add_space(20.0);
                } else {
                    if current_time <= self.animate_from {
                        ui.ctx().request_repaint_after(
                            (self.animate_from - current_time).try_into().unwrap(),
                        );
                        return;
                    }
                    let block_time = (current_time - self.animate_from).as_seconds_f32();
                    let animated_text = block.get_partial_slice(block_time, ui);

                    match animated_text {
                        Some(animated_block) => {
                            animated_block.paint(Color32::WHITE, ui, false);
                            ui.ctx().request_repaint();
                        }
                        None => {
                            block.paint(Color32::WHITE, ui, false);
                            self.done_stages += 1;
                            self.animate_from = current_time + text_pause_delay;
                            ui.ctx()
                                .request_repaint_after(text_pause_delay.try_into().unwrap());
                        }
                    }

                    return;
                }
            }

            if !buttons.is_empty() {
                ui.add_space(20.0);
                for (i, (button, button_text)) in buttons.into_iter().enumerate() {
                    let total_stage = total_text_blocks + i;

                    if total_stage < self.done_stages {
                        if button_text
                            .centered_button(button.color, theme.text, &map_texture, ui)
                            .clicked()
                        {
                            splash_resp.clicked = Some(button.id)
                        }
                    } else {
                        if current_time <= self.animate_from {
                            ui.ctx().request_repaint_after(
                                (self.animate_from - current_time).try_into().unwrap(),
                            );
                            return;
                        }

                        self.done_stages += 1;
                        self.animate_from = current_time + button_pause_delay;
                        ui.ctx().request_repaint();
                        return;
                    }

                    ui.add_space(10.0);
                }
            }
        });

        splash_resp
    }
}
