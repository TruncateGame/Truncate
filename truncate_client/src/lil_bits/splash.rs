use eframe::egui::{self};
use epaint::{vec2, Color32, TextureHandle};
use instant::Duration;
use std::f32;

use crate::utils::{text::TextHelper, Theme};

struct SplashButton {
    id: &'static str,
    text: String,
    color: Color32,
}

pub struct SplashUI {
    message: Vec<String>,
    byline: Vec<String>,
    animated: bool,
    buttons: Vec<SplashButton>,
}

#[derive(Default)]
pub struct SplashResponse {
    pub clicked: Option<&'static str>,
}

impl SplashUI {
    pub fn new(message: Vec<String>) -> Self {
        Self {
            message,
            byline: vec![],
            animated: false,
            buttons: vec![],
        }
    }

    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    pub fn byline(mut self, text: Vec<String>) -> Self {
        self.byline = text;
        self
    }

    pub fn with_button(mut self, id: &'static str, text: String, color: Color32) -> Self {
        self.buttons.push(SplashButton { id, text, color });
        self
    }
}

impl SplashUI {
    pub fn render(
        mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        current_time: Duration,
        map_texture: &TextureHandle,
    ) -> SplashResponse {
        let dots = || {
            if self.animated {
                let dot_count = (current_time.as_millis() / 500) % 4;
                let mut dots = vec!["."; dot_count as usize];
                dots.extend(vec![" "; 4 - dot_count as usize]);
                dots.join("")
            } else {
                String::new()
            }
        };

        if let Some(line) = self.message.last_mut() {
            *line = format!("{line}{}", dots());
        }

        let msg_text: Vec<_> = self
            .message
            .iter()
            .map(|m| TextHelper::heavy(&m, 14.0, None, ui))
            .collect();
        let msg_height: f32 = msg_text.iter().map(|t| t.size().y).sum();

        let byline_text: Vec<_> = self
            .byline
            .iter()
            .map(|m| TextHelper::light(&m, 20.0, None, ui))
            .collect();
        let byline_height: f32 = byline_text.iter().map(|t| t.size().y).sum();

        let buttons: Vec<_> = self
            .buttons
            .iter()
            .map(|button| (button, TextHelper::heavy(&button.text, 14.0, None, ui)))
            .collect();
        let button_height = buttons
            .first()
            .map(|(_, b)| b.size().y * 2.0)
            .unwrap_or_default();

        let required_size = vec2(
            ui.available_width(),
            msg_height + byline_height + button_height,
        );
        let margins = (ui.available_size_before_wrap() - required_size) / 2.0;
        let outer_frame = egui::Frame::none().inner_margin(egui::Margin::from(margins));

        let mut splash_resp = SplashResponse::default();

        outer_frame.show(ui, |ui| {
            for line in msg_text {
                line.paint(Color32::WHITE, ui, true);
            }

            if !byline_text.is_empty() {
                ui.add_space(8.0);
                for line in byline_text {
                    line.paint(Color32::WHITE, ui, true);
                }
            }

            if !buttons.is_empty() {
                ui.add_space(10.0);
                for (button, button_text) in buttons {
                    if button_text
                        .centered_button(button.color, theme.text, &map_texture, ui)
                        .clicked()
                    {
                        splash_resp.clicked = Some(button.id)
                    }
                }
            }
        });

        splash_resp
    }
}
