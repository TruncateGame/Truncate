use futures::channel::mpsc::{Receiver, Sender};
type R = Receiver<GameMessage>;
type S = Sender<PlayerMessage>;

use super::debug;
use super::utils::Theme;
use crate::{app_inner, utils::glyph_meaure::GlyphMeasure};
use eframe::egui::{self, Frame, Id, Margin, TextureOptions};
use epaint::{hex_color, vec2, TextureHandle};
use truncate_core::messages::{GameMessage, PlayerMessage};

pub struct OuterApplication {
    pub name: String,
    pub theme: Theme,
    pub game_status: app_inner::GameStatus,
    pub rx_game: R,
    pub tx_player: S,
    pub frame_history: debug::FrameHistory,
    pub map_texture: TextureHandle,
    pub launched_room: Option<String>,
}

impl OuterApplication {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        rx_game: R,
        tx_player: S,
        room_code: Option<String>,
    ) -> Self {
        let mut fonts = egui::FontDefinitions::default();

        // Main tile font
        {
            fonts.font_data.insert(
                "PS2P-Regular".into(),
                egui::FontData::from_static(include_bytes!("../font/PressStart2P-Regular.ttf")),
            );
            fonts.families.insert(
                egui::FontFamily::Name("Truncate-Heavy".into()),
                vec!["PS2P-Regular".into()],
            );
        }

        // Dialog / text font
        {
            fonts.font_data.insert(
                "pixel".into(),
                egui::FontData::from_static(include_bytes!("../font/PixelOperator.ttf")),
            );
            fonts
                .families
                .insert(egui::FontFamily::Proportional, vec!["pixel".to_owned()]);
        }

        cc.egui_ctx.set_fonts(fonts);

        cc.egui_ctx.memory_mut(|mem| {
            mem.data.insert_temp(Id::null(), GlyphMeasure::new());
        });

        let mut game_status = app_inner::GameStatus::None("".into(), None);
        let mut player_name = "___AUTO___".to_string();

        #[cfg(target_arch = "wasm32")]
        {
            let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
            if let Some(existing_game_token) =
                local_storage.get_item("truncate_active_token").unwrap()
            {
                game_status = app_inner::GameStatus::None("".into(), Some(existing_game_token));
            }

            if let Some(existing_name) = local_storage.get_item("truncate_name_history").unwrap() {
                player_name = existing_name.into();
            }
        }

        let theme = Theme::default();

        {
            use egui::FontFamily;
            use egui::FontId;
            use egui::TextStyle::*;

            let mut style = egui::Style::default();
            style.text_styles = [
                (Heading, FontId::new(32.0, FontFamily::Proportional)),
                (Body, FontId::new(16.0, FontFamily::Proportional)),
                (Monospace, FontId::new(16.0, FontFamily::Monospace)),
                (Button, FontId::new(16.0, FontFamily::Proportional)),
                (Small, FontId::new(8.0, FontFamily::Proportional)),
            ]
            .into();

            let mut visuals = egui::Visuals::light();
            visuals.window_fill = theme.water;
            visuals.panel_fill = theme.water;
            style.visuals = visuals;
            style.spacing.window_margin = Margin::same(0.0);

            cc.egui_ctx.set_style(style);
        }

        Self {
            name: player_name,
            theme,
            game_status,
            rx_game,
            tx_player,
            frame_history: Default::default(),
            map_texture: load_map_texture(&cc.egui_ctx),
            launched_room: room_code,
        }
    }
}

fn load_map_texture(ctx: &egui::Context) -> TextureHandle {
    let image_bytes = include_bytes!("../img/truncate_processed.png");
    let image = image::load_from_memory(image_bytes).unwrap();
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    let image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

    ctx.load_texture("tiles", image, TextureOptions::NEAREST)
}

impl eframe::App for OuterApplication {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_history
            .on_new_frame(ctx.input(|i| i.time), _frame.info().cpu_usage);

        egui::CentralPanel::default()
            .frame(Frame::default().fill(self.theme.water))
            .show(ctx, |ui| {
                // Show debug timings in-app
                // self.frame_history.ui(ui);
                app_inner::render(self, ui)
            });

        if let Some(time) = match &self.game_status {
            app_inner::GameStatus::Tutorial(g) => Some(g.active_game.ctx.current_time),
            app_inner::GameStatus::SinglePlayer(g) => Some(g.active_game.ctx.current_time),
            app_inner::GameStatus::Active(g) => Some(g.ctx.current_time),
            _ => None,
        } {
            let subsec = time.subsec_millis();
            // In-game animations should try align with the quarter-second tick,
            // so we try to repaint around that tick to keep them looking consistent
            let next_tick = 251 - (subsec % 250);
            ctx.request_repaint_after(std::time::Duration::from_millis(next_tick as u64));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(1000));
        }
    }
}
