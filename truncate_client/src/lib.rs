mod active_game;
mod debug;
mod game;
mod game_client;
mod lil_bits;
mod theming;
#[cfg(target_arch = "wasm32")]
mod web_comms;

use game_client::GameClient;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};
#[cfg(target_arch = "wasm32")]
use eframe::web::AppRunnerRef;
#[cfg(target_arch = "wasm32")]
use futures::channel::{mpsc, oneshot};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WebHandle {
    handle: AppRunnerRef,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen]
    pub fn stop_web(&self) -> Result<(), wasm_bindgen::JsValue> {
        let mut app = self.handle.lock();
        app.destroy()
    }

    #[wasm_bindgen]
    pub fn set_some_content_from_javasript(&mut self, _some_data: &str) {
        let _app = self.handle.lock().app_mut::<GameClient>();
        // _app.data = some_data;
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn init_wasm_hooks() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start_separate(
    canvas_id: &str,
    server_url: &str,
) -> Result<WebHandle, wasm_bindgen::JsValue> {
    let web_options = eframe::WebOptions::default();

    let (tx_game, rx_game) = mpsc::channel(2048);
    let (tx_player, rx_player) = mpsc::channel(2048);
    let (tx_context, rx_context) = oneshot::channel();

    let connect_url = if server_url.is_empty() {
        "wss://citadel.truncate.town"
    } else {
        server_url
    };

    wasm_bindgen_futures::spawn_local(web_comms::connect(
        connect_url.to_string(),
        tx_game,
        tx_player.clone(),
        rx_player,
        rx_context,
    ));

    eframe::start_web(
        canvas_id,
        web_options,
        Box::new(|cc| {
            tx_context.send(cc.egui_ctx.clone()).unwrap();
            Box::new(GameClient::new(cc, rx_game, tx_player))
        }),
    )
    .await
    .map(|handle| WebHandle { handle })
}

/// This is the entry-point for all the web-assembly.
/// This is called once from the HTML.
/// It loads the app, installs some callbacks, then returns.
/// You can add more callbacks like this if you want to call in to your code.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start(canvas_id: &str, server_url: &str) -> Result<WebHandle, wasm_bindgen::JsValue> {
    init_wasm_hooks();
    start_separate(canvas_id, server_url).await
}
