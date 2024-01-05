use std::sync::OnceLock;

use futures::channel::mpsc::{Receiver, Sender};
use serde::{Deserialize, Serialize};
type R = Receiver<GameMessage>;
type S = Sender<PlayerMessage>;

use super::utils::Theme;
use crate::{
    app_inner,
    utils::glyph_utils::{BaseTileGlyphs, Glypher},
};
use eframe::egui::{self, Frame, Id, Margin, TextureOptions};
#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::JsValue;
use epaint::TextureHandle;
use truncate_core::{
    board::Board,
    messages::{GameMessage, PlayerMessage},
    npc::scoring::BoardWeights,
    player::Player,
    rules::GameRules,
};

/// A way to communicate with an outer host, if one exists.
pub struct Backchannel {
    #[cfg(target_arch = "wasm32")]
    pub backchannel: js_sys::Function,
}

/// Messages that can be sent to an outer host
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum BackchannelMsg {
    /// Finds the best move for the next player in a given game state
    EvalGame {
        board: Board,
        rules: GameRules,
        players: Vec<Player>,
        next_player: usize,
        weights: BoardWeights,
    },
    /// Checks if any answer has been posted for a given message
    QueryFor {
        id: String,
    },
    /// Tells the outer host to add a given word to the NPC's known dictionaries
    Remember {
        word: String,
    },
    Copy {
        text: String,
    },
}

impl Backchannel {
    pub fn new(#[cfg(target_arch = "wasm32")] backchannel: js_sys::Function) -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            backchannel,
        }
    }

    pub fn is_open(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        return true;
        #[cfg(not(target_arch = "wasm32"))]
        return false;
    }

    /// Passes a message through to the outer host, optionally
    /// returning an ID that can be used to query for an async result
    /// at a later time
    #[allow(unreachable_code)]
    pub fn send_msg(&self, msg: BackchannelMsg) -> Option<String> {
        #[cfg(target_arch = "wasm32")]
        {
            let msg_id = self
                .backchannel
                .call1(
                    &JsValue::NULL,
                    &JsValue::from(serde_json::to_string(&msg).unwrap()),
                )
                .expect("Backchannel message should be sendable");
            return msg_id.as_string();
        }
        None
    }
}

pub struct OuterApplication {
    pub name: String,
    pub theme: Theme,
    pub game_status: app_inner::GameStatus,
    pub rx_game: R,
    pub tx_player: S,
    pub map_texture: TextureHandle,
    pub launched_room: Option<String>,
    pub error: Option<String>,
    pub backchannel: Backchannel,
    pub log_frames: bool,
    pub frames: debug::FrameHistory,
}

impl OuterApplication {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        rx_game: R,
        tx_player: S,
        room_code: Option<String>,
        #[cfg(target_arch = "wasm32")] backchannel: js_sys::Function,
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
                egui::FontData::from_static(include_bytes!("../font/at01.ttf")),
            );
            fonts
                .families
                .insert(egui::FontFamily::Proportional, vec!["pixel".to_owned()]);
        }

        cc.egui_ctx.set_fonts(fonts);

        let glypher = Glypher::new();
        let map_texture = load_textures(&cc.egui_ctx, &glypher);

        cc.egui_ctx.memory_mut(|mem| {
            mem.data.insert_temp(Id::NULL, glypher);
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
                (Monospace, FontId::new(10.0, FontFamily::Monospace)),
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

        #[cfg(target_arch = "wasm32")]
        let backchannel = Backchannel::new(backchannel);
        #[cfg(not(target_arch = "wasm32"))]
        let backchannel = Backchannel::new();

        #[cfg(not(target_arch = "wasm32"))]
        setup_repaint_truncate_animations(cc.egui_ctx.clone());
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(setup_repaint_truncate_animations_web(
            cc.egui_ctx.clone(),
        ));

        Self {
            name: player_name,
            theme,
            game_status,
            rx_game,
            tx_player,
            map_texture,
            launched_room: room_code,
            error: None,
            backchannel,
            log_frames: false,
            frames: debug::FrameHistory::default(),
        }
    }
}

pub struct TextureMeasurement {
    pub num_tiles_x: usize,
    pub num_tiles_y: usize,
    pub outer_tile_width: f32,
    pub outer_tile_width_px: usize,
    pub outer_tile_height: f32,
    pub outer_tile_height_px: usize,
    pub inner_tile_width: f32,
    pub inner_tile_width_px: usize,
    pub inner_tile_height: f32,
    pub inner_tile_height_px: usize,
    pub x_padding_pct: f32,
    pub y_padding_pct: f32,
}

pub static TEXTURE_MEASUREMENT: OnceLock<TextureMeasurement> = OnceLock::new();
pub static TEXTURE_IMAGE: OnceLock<egui::ColorImage> = OnceLock::new();
pub static GLYPH_IMAGE: OnceLock<BaseTileGlyphs> = OnceLock::new();

fn load_textures(ctx: &egui::Context, glypher: &Glypher) -> TextureHandle {
    let image_bytes = include_bytes!("../img/truncate_packed.png");
    let image = image::load_from_memory(image_bytes).unwrap();
    let image_width = image.width();
    let image_height = image.height();
    let size = [image_width as _, image_height as _];

    let num_tiles_x = (image.width() / 18) as usize;
    let num_tiles_y = (image.height() / 18) as usize;
    let outer_tile_width = 1.0 / num_tiles_x as f32;
    let outer_tile_height = 1.0 / num_tiles_y as f32;
    let x_padding_pct = outer_tile_width / 18.0;
    let y_padding_pct = outer_tile_height / 18.0;
    let inner_tile_width = outer_tile_width - (x_padding_pct * 2.0);
    let inner_tile_height = outer_tile_height - (y_padding_pct * 2.0);

    let measurements = TextureMeasurement {
        num_tiles_x,
        num_tiles_y,
        outer_tile_width,
        outer_tile_width_px: 18,
        outer_tile_height,
        outer_tile_height_px: 18,
        inner_tile_width,
        inner_tile_width_px: 16,
        inner_tile_height,
        inner_tile_height_px: 16,
        x_padding_pct,
        y_padding_pct,
    };
    _ = TEXTURE_MEASUREMENT.set(measurements);

    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    let image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
    _ = TEXTURE_IMAGE.set(image.clone());

    let ascii_tiles = (32..127).map(Into::into).collect::<Vec<char>>();
    let tiles = glypher.render_to_image(ascii_tiles, 16);
    _ = GLYPH_IMAGE.set(tiles);

    ctx.load_texture("tiles", image, TextureOptions::NEAREST)
}

impl eframe::App for OuterApplication {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // We have to go through the instant crate as
        // most std time functions are not implemented
        // in Rust's wasm targets.
        // instant::SystemTime::now() conditionally uses
        // a js function on wasm targets, and otherwise aliases
        // to the std SystemTime type.
        let current_time = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("Please don't play Truncate earlier than 1970");

        egui::CentralPanel::default()
            .frame(Frame::default().fill(self.theme.water))
            .show(ctx, |ui| app_inner::render(self, ui, current_time));

        if self.log_frames {
            self.frames
                .on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn setup_repaint_truncate_animations(egui_ctx: egui::Context) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || loop {
        let current_time = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("Please don't play Truncate earlier than 1970");

        let subsec = current_time.subsec_millis();
        // In-game animations should try align with the quarter-second tick,
        // so we try to repaint around that tick to keep them looking consistent.
        // (Adding an extra millisecond so we don't have to worry about `> 250` vs `>= 250`)
        let next_tick = 251 - (subsec % 250);
        std::thread::sleep(instant::Duration::from_millis(next_tick as u64));
        egui_ctx.request_repaint();
    })
}

#[cfg(target_arch = "wasm32")]
async fn setup_repaint_truncate_animations_web(egui_ctx: egui::Context) {
    loop {
        let current_time = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("Please don't play Truncate earlier than 1970");

        let subsec = current_time.subsec_millis();
        // In-game animations should try align with the quarter-second tick,
        // so we try to repaint around that tick to keep them looking consistent.
        let next_tick = 250 - (subsec % 250);
        gloo_timers::future::TimeoutFuture::new(next_tick).await;
        egui_ctx.request_repaint();
    }
}

mod debug {
    use super::*;
    use egui::util::History;

    pub struct FrameHistory {
        frame_times: History<f32>,
    }

    impl Default for FrameHistory {
        fn default() -> Self {
            let max_age: f32 = 5.0;
            let max_len = (max_age * 100.0).round() as usize;
            Self {
                frame_times: History::new(10..max_len, max_age),
            }
        }
    }

    impl FrameHistory {
        // Called first
        pub fn on_new_frame(&mut self, now: f64, previous_frame_time: Option<f32>) {
            let previous_frame_time = previous_frame_time.unwrap_or_default();
            if let Some(latest) = self.frame_times.latest_mut() {
                *latest = previous_frame_time; // rewrite history now that we know
            }
            self.frame_times.add(now, previous_frame_time); // projected
        }

        pub fn ui(&mut self, ui: &mut egui::Ui) {
            // Includes egui layout and tessellation time.
            // Does not include GPU usage, nor overhead for sending data to GPU.
            ui.label(format!(
                "Mean CPU usage: {:.2} ms / frame",
                1e3 * self.frame_times.average().unwrap_or_default()
            ));
            ui.label(format!(
                "Mean framerate: {:.2} fps",
                self.frame_times.rate().unwrap_or_default()
            ));

            let mut lf = 0.0;
            self.frame_times.iter().for_each(|(_, v)| {
                if v > lf {
                    lf = v
                }
            });

            ui.label(format!("Longest frame: {:.2} ms", 1e3 * lf));

            egui::warn_if_debug_build(ui);
        }
    }
}
