use std::sync::OnceLock;

use futures::channel::mpsc::{Receiver, Sender};
use instant::Duration;
use serde::{Deserialize, Serialize};
type R = Receiver<GameMessage>;
type S = Sender<PlayerMessage>;

use super::utils::Theme;
use crate::app_inner::AppInnerStorage;
use crate::utils::daily::get_puzzle_day;
use crate::utils::includes::changelogs;
use crate::utils::macros::current_time;
use crate::{app_inner, utils::glyph_utils::Glypher};
use eframe::egui::{self, Frame, Margin, TextureOptions};
#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::JsValue;
use epaint::{Color32, Stroke, TextureHandle};
use truncate_core::{
    board::Board,
    messages::{GameMessage, PlayerMessage},
    npc::scoring::NPCParams,
    player::Player,
    rules::GameRules,
};

/// A way to communicate with an outer host, if one exists. (Typically Browser JS)
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
        npc_params: NPCParams,
    },
    /// Tells the outer host to add a given word to the NPC's known dictionaries
    Remember { word: String },
    /// Tells the outer host to forget all words learned via BackchannelMsg::Remember
    Forget,
    /// Tells the outer host to copy the given text, and optionally
    /// open a system share dialog.
    /// More reliable than copying within egui, as the browser JS
    /// makes sure to attach it to an input event.
    Copy { text: String, share: ShareType },
    /// Checks if any answer has been posted for a given message
    QueryFor { id: String },
}

#[derive(Serialize, Deserialize)]
pub enum ShareType {
    None,
    Text,
    Url,
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

pub static TEXTURE_MEASUREMENT: OnceLock<TextureMeasurement> = OnceLock::new();
pub static TEXTURE_IMAGE: OnceLock<egui::ColorImage> = OnceLock::new();
pub static GLYPHER: OnceLock<Glypher> = OnceLock::new();

pub struct OuterApplication {
    pub name: String,
    pub theme: Theme,
    pub launched_at_day: u32,
    pub started_login_at: Option<Duration>,
    pub logged_in_as: Option<String>,
    pub unread_changelogs: Vec<String>,
    pub inner_storage: AppInnerStorage,
    pub game_status: app_inner::GameStatus,
    pub rx_game: R,
    pub tx_player: S,
    pub map_texture: TextureHandle,
    pub launched_code: Option<String>,
    pub error: Option<String>,
    pub backchannel: Backchannel,
    pub log_frames: bool,
    pub frames: debug::FrameHistory,
    pub event_dispatcher: EventDispatcher,
}

impl OuterApplication {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        rx_game: R,
        mut tx_player: S,
        room_code: Option<String>,
        #[cfg(target_arch = "wasm32")] backchannel: js_sys::Function,
    ) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        let launched_at_day = get_puzzle_day(current_time!());

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
        _ = GLYPHER.set(glypher);

        let mut game_status = app_inner::GameStatus::None("".into(), None);
        let mut player_name = "___AUTO___".to_string();
        let mut player_token: Option<String> = None;

        let mut screen_width = 0;
        let mut screen_height = 0;
        let mut user_agent = String::new();
        let mut referrer = String::new();

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

            if let Some(existing_player_token) =
                local_storage.get_item("truncate_player_token").unwrap()
            {
                player_token = Some(existing_player_token);
            }

            if let Some(width) = web_sys::window().unwrap().inner_width().unwrap().as_f64() {
                screen_width = width as u32;
            }
            if let Some(height) = web_sys::window().unwrap().inner_height().unwrap().as_f64() {
                screen_height = height as u32;
            }
            if let Ok(agent) = web_sys::window().unwrap().navigator().user_agent() {
                user_agent = agent;
            }
            if let Some(document) = web_sys::window().unwrap().document() {
                referrer = document.referrer();
            }
        }

        match &player_token {
            Some(existing_token) => {
                tx_player
                    .try_send(PlayerMessage::Login {
                        player_token: existing_token.clone(),
                        screen_width,
                        screen_height,
                        user_agent,
                        referrer,
                    })
                    .unwrap();
            }
            None => {
                let unread_changelogs: Vec<_> = changelogs()
                    .iter()
                    .filter_map(|(name, changelog)| {
                        if changelog.effective_day > launched_at_day {
                            Some(name.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                tx_player
                    .try_send(PlayerMessage::CreateAnonymousPlayer {
                        screen_width,
                        screen_height,
                        user_agent,
                        referrer,
                        unread_changelogs,
                    })
                    .unwrap();
            }
        }

        let theme = Theme::day();

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
            visuals.text_cursor = Stroke::new(2.0, Color32::WHITE);
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
            launched_at_day,
            started_login_at: Some(current_time!()),
            logged_in_as: None,
            unread_changelogs: vec![],
            game_status,
            inner_storage: AppInnerStorage::default(),
            rx_game,
            tx_player: tx_player.clone(),
            map_texture,
            launched_code: room_code,
            error: None,
            backchannel,
            log_frames: false,
            frames: debug::FrameHistory::default(),
            event_dispatcher: EventDispatcher {
                tx_player,
                sent: vec![],
            },
        }
    }
}

/// Each EventDispatcher instance ensures a given event is sent only once.
/// EventDispatcher should be cloned for each unique game area (e.g. tutorial, puzzle).
pub struct EventDispatcher {
    tx_player: S,
    sent: Vec<String>,
}

impl EventDispatcher {
    /// Tracks an event, deduplicating all sends for the lifetime of this EventDispatcher
    pub fn event(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref();

        if self.sent.iter().any(|s| s == name) {
            return;
        }
        self.sent.push(name.to_string());

        self.tx_player
            .clone()
            .try_send(PlayerMessage::GenericEvent {
                name: name.to_string(),
            })
            .unwrap();
    }
}

// Cloning an EventDispatcher resets its tracking, and all events can be sent again
impl Clone for EventDispatcher {
    fn clone(&self) -> Self {
        Self {
            tx_player: self.tx_player.clone(),
            sent: vec![],
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

    ctx.load_texture("tiles", image, TextureOptions::NEAREST)
}

impl eframe::App for OuterApplication {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(Frame::default().fill(self.theme.water))
            .show(ctx, |ui| app_inner::render(self, ui, current_time!()));

        if self.log_frames {
            self.frames
                .on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn setup_repaint_truncate_animations(egui_ctx: egui::Context) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || loop {
        let current_time = current_time!();
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
        let current_time = current_time!();
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
