mod active_game;
mod debug;
mod game;
mod game_client;
mod lil_bits;
mod native_comms;
mod theming;

use eframe::egui;
use tokio::runtime::Builder;
use tokio::sync::mpsc;

use game_client::GameClient;

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

    tokio_runtime.spawn(native_comms::connect(connect_addr, tx_game, rx_player));

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
