mod app_inner;
mod app_outer;
mod debug;
mod lil_bits;
mod regions;
mod utils;
#[cfg(target_arch = "wasm32")]
mod web_comms;

use app_outer::OuterApplication;

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
        let _app = self.handle.lock().app_mut::<OuterApplication>();
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
    room_code: String,
    backchannel: js_sys::Function,
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
            Box::new(OuterApplication::new(
                cc,
                rx_game,
                tx_player,
                Some(room_code),
                backchannel,
            ))
        }),
    )
    .await
    .map(|handle| WebHandle { handle })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start(
    canvas_id: &str,
    server_url: &str,
    room_code: &str,
    backchannel: js_sys::Function,
) -> Result<WebHandle, wasm_bindgen::JsValue> {
    use web_sys::console;

    init_wasm_hooks();

    if let (Some(commit_msg), Some(commit_hash)) = (option_env!("TR_MSG"), option_env!("TR_COMMIT"))
    {
        console::log_1(&format!("Running \"{commit_msg}\"").into());
        console::log_1(
            &format!("https://github.com/TruncateGame/Truncate/commit/{commit_hash}").into(),
        );
    } else {
        console::log_1(&format!("No tagged git commit for current release.").into());
    }

    start_separate(canvas_id, server_url, room_code.to_string(), backchannel).await
}

// Functions used in the web worker

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn backchannel(msg: String) -> String {
    use app_outer::BackchannelMsg;

    let msg: BackchannelMsg =
        serde_json::from_str(&msg).expect("Incoming message should be a BackchannelMsg");

    match msg {
        BackchannelMsg::EvalGame {
            board,
            rules,
            players,
            next_player,
            weights,
        } => {
            web_sys::console::log_1(&"Evaluating best move".into());

            let mut game = truncate_core::game::Game::new(3, 3);
            game.board = board;
            game.rules = rules;
            game.players = players;
            game.next_player = next_player;

            game.players[next_player].turn_starts_at = Some(
                instant::SystemTime::now()
                    .duration_since(instant::SystemTime::UNIX_EPOCH)
                    .expect("Please don't play Truncate before 1970")
                    .as_secs(),
            );
            let best = utils::game_evals::best_move(&game, &weights);

            return serde_json::to_string(&best).expect("Resultant move should be serializable");
        }
        BackchannelMsg::Remember { word } => {
            web_sys::console::log_1(&format!("Worker: Remembering {word}").into());
            utils::game_evals::remember(&word);
            return String::new();
        }
        BackchannelMsg::QueryFor { .. } => {
            unreachable!("Backchannel should not be passing through QueryFor")
        }
        BackchannelMsg::Copy { .. } => {
            unreachable!("Backchannel should not be passing through Copy")
        }
    }
}
