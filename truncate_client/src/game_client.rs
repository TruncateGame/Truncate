use futures::channel::mpsc::{Receiver, Sender};
type R = Receiver<GameMessage>;
type S = Sender<PlayerMessage>;

use super::debug;
use super::theming::Theme;
use crate::game;
use eframe::egui;
use epaint::hex_color;
use truncate_core::messages::{GameMessage, PlayerMessage};

#[derive(Debug)]
pub struct GameClient {
    pub name: String,
    pub theme: Theme,
    pub game_status: game::GameStatus,
    pub rx_game: R,
    pub tx_player: S,
    pub frame_history: debug::FrameHistory,
}

impl GameClient {
    pub fn new(cc: &eframe::CreationContext<'_>, rx_game: R, tx_player: S) -> Self {
        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "Heebo-Medium".into(),
            egui::FontData::from_static(include_bytes!("../font/Heebo-Medium.ttf")),
        );
        fonts.families.insert(
            egui::FontFamily::Name("Truncate-Heavy".into()),
            vec!["Heebo-Medium".into()],
        );

        fonts.font_data.insert(
            "Heebo-Regular".into(),
            egui::FontData::from_static(include_bytes!("../font/Heebo-Regular.ttf")),
        );
        fonts.families.insert(
            egui::FontFamily::Name("Truncate-Regular".into()),
            vec!["Heebo-Regular".into()],
        );
        cc.egui_ctx.set_fonts(fonts);

        Self {
            name: "Mystery Player".into(),
            theme: Theme::default(),
            game_status: game::GameStatus::None("".into()),
            rx_game,
            tx_player,
            frame_history: Default::default(),
        }
    }
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
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = hex_color!("#141414");
        visuals.panel_fill = hex_color!("#141414");
        ctx.set_visuals(visuals);

        ctx.request_repaint();
    }
}
