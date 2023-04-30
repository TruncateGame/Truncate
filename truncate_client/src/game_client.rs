use futures::channel::mpsc::{Receiver, Sender};
type R = Receiver<GameMessage>;
type S = Sender<PlayerMessage>;

use super::debug;
use super::theming::Theme;
use crate::{game, glyph_meaure::GlyphMeasure};
use eframe::egui::{self, Id, TextureOptions};
use epaint::{hex_color, TextureHandle};
use truncate_core::messages::{GameMessage, PlayerMessage};

pub struct GameClient {
    pub name: String,
    pub theme: Theme,
    pub game_status: game::GameStatus,
    pub rx_game: R,
    pub tx_player: S,
    pub frame_history: debug::FrameHistory,
    pub map_texture: TextureHandle,
    pub launched_room: Option<String>,
}

impl GameClient {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        rx_game: R,
        tx_player: S,
        room_code: Option<String>,
    ) -> Self {
        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "PS2P-Medium".into(),
            egui::FontData::from_static(include_bytes!("../font/PressStart2P-Regular.ttf")),
        );
        fonts.families.insert(
            egui::FontFamily::Name("Truncate-Heavy".into()),
            vec!["PS2P-Medium".into()],
        );

        fonts.font_data.insert(
            "PS2P-Regular".into(),
            egui::FontData::from_static(include_bytes!("../font/PressStart2P-Regular.ttf")),
        );
        fonts.families.insert(
            egui::FontFamily::Name("Truncate-Regular".into()),
            vec!["PS2P-Regular".into()],
        );
        cc.egui_ctx.set_fonts(fonts);

        cc.egui_ctx.memory_mut(|mem| {
            mem.data.insert_temp(Id::null(), GlyphMeasure::new());
        });

        let mut game_status = game::GameStatus::None("".into(), None);

        #[cfg(target_arch = "wasm32")]
        {
            let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
            if let Some(existing_game_token) =
                local_storage.get_item("truncate_active_token").unwrap()
            {
                game_status = game::GameStatus::None("".into(), Some(existing_game_token));
            }
        }

        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = hex_color!("#141414");
        visuals.panel_fill = hex_color!("#141414");
        cc.egui_ctx.set_visuals(visuals);

        Self {
            name: "Mystery Player".into(),
            theme: Theme::default(),
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

impl eframe::App for GameClient {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_history
            .on_new_frame(ctx.input(|i| i.time), _frame.info().cpu_usage);

        egui::CentralPanel::default().show(ctx, |ui| {
            // Show debug timings in-app
            // self.frame_history.ui(ui);
            game::render(self, ui)
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}
