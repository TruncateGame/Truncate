mod comms;
mod game;

use eframe::egui;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use core::messages::{GameMessage, PlayerMessage};
use game::GameStatus;

#[derive(Debug)]
pub struct GameClient {
    name: String,
    game_status: GameStatus,
    rx_game: UnboundedReceiver<GameMessage>,
    tx_player: UnboundedSender<PlayerMessage>,
}

impl GameClient {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        rx_game: UnboundedReceiver<GameMessage>,
        tx_player: UnboundedSender<PlayerMessage>,
    ) -> Self {
        Self {
            name: "Mystery Player".into(),
            game_status: GameStatus::None,
            rx_game,
            tx_player,
        }
    }
}

impl eframe::App for GameClient {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| game::render(self, ui));
        ctx.request_repaint();
    }
}

fn main() {
    let connect_addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:8080".into());

    let tokio_runtime = Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let (tx_game, rx_game) = mpsc::unbounded_channel();
    let (tx_player, rx_player) = mpsc::unbounded_channel();

    tokio_runtime.spawn(comms::connect(connect_addr, tx_game, rx_player));

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(500.0, 1000.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Truncate",
        options,
        Box::new(|cc| Box::new(GameClient::new(cc, rx_game, tx_player))),
    )
    .unwrap();
}
