mod app_inner;
mod app_outer;
mod handle_launch_code;
mod handle_messages;
mod lil_bits;
mod regions;
mod utils;

// Enable the debug module to expose frame/performance timings
// mod debug;

#[cfg(target_arch = "wasm32")]
mod web_comms;

use app_outer::OuterApplication;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};
#[cfg(target_arch = "wasm32")]
use futures::channel::{mpsc, oneshot};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebHandle {
    /// Installs a panic hook, then returns.
    #[allow(clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        use web_sys::console;

        // Make sure panics are logged using `console.error`.
        console_error_panic_hook::set_once();

        // Redirect tracing to console.log and friends:
        tracing_wasm::set_as_global_default();

        if let (Some(commit_msg), Some(commit_hash)) =
            (option_env!("TR_MSG"), option_env!("TR_COMMIT"))
        {
            console::log_1(&format!("Running \"{commit_msg}\"").into());
            console::log_1(
                &format!("https://github.com/TruncateGame/Truncate/commit/{commit_hash}").into(),
            );
        } else {
            console::log_1(&format!("No tagged git commit for current release.").into());
        }

        Self {
            runner: eframe::WebRunner::new(),
        }
    }

    /// Call this once from JavaScript to start your app.
    #[wasm_bindgen]
    pub async fn start(
        &self,
        canvas_id: &str,
        server_url: &str,
        room_code: &str,
        backchannel: js_sys::Function,
    ) -> Result<(), wasm_bindgen::JsValue> {
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

        let room_code = room_code.to_string();
        self.runner
            .start(
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
    }

    /// Shut down eframe and clean up resources.
    #[wasm_bindgen]
    pub fn destroy(&self) {
        self.runner.destroy();
    }

    #[wasm_bindgen]
    pub fn has_panicked(&self) -> bool {
        self.runner.has_panicked()
    }

    #[wasm_bindgen]
    pub fn panic_message(&self) -> Option<String> {
        self.runner.panic_summary().map(|s| s.message())
    }

    #[wasm_bindgen]
    pub fn panic_callstack(&self) -> Option<String> {
        self.runner.panic_summary().map(|s| s.callstack())
    }
}

// Functions used in the web worker
// Used to evaluate games as the NPC using a separate thread,
// preventing the UI from hanging during computation.

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
            npc_params,
        } => {
            let mut game = truncate_core::game::Game::new(3, 3, None, 0);
            game.board = board;
            game.rules = rules;
            game.player_turn_count = vec![0; players.len()];
            game.players = players;
            game.next_player = Some(next_player);

            game.players[next_player].turn_starts_no_later_than = Some(
                instant::SystemTime::now()
                    .duration_since(instant::SystemTime::UNIX_EPOCH)
                    .expect("Please don't play Truncate before 1970")
                    .as_secs(),
            );
            let best = utils::game_evals::client_best_move(&game, &npc_params);

            return serde_json::to_string(&best).expect("Resultant move should be serializable");
        }
        BackchannelMsg::Remember { word } => {
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
