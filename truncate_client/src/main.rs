mod active_game;
mod debug;
mod game;
mod game_client;
mod lil_bits;
mod native_comms;
mod theming;

use eframe::egui;
use futures::channel::{mpsc, oneshot};
use tokio::runtime::Builder;

use game_client::GameClient;

fn main() {
    let connect_addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:8080".into());

    let (tx_game, rx_game) = mpsc::channel(2048);
    let (tx_player, rx_player) = mpsc::channel(2048);
    let (tx_context, rx_context) = oneshot::channel();

    let tokio_runtime = Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    tokio_runtime.spawn(native_comms::connect(
        connect_addr,
        tx_game,
        rx_player,
        rx_context,
    ));

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(500.0, 1000.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Truncate",
        options,
        Box::new(move |cc| {
            tx_context.send(cc.egui_ctx.clone()).unwrap();
            Box::new(GameClient::new(cc, rx_game, tx_player))
        }),
    )
    .unwrap();
}
