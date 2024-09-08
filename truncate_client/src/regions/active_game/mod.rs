use std::sync::{Arc, Mutex};

use epaint::{vec2, Color32, TextureHandle};
use instant::Duration;
use truncate_core::{
    board::{Board, Coordinate},
    generation::BoardSeed,
    messages::{
        AssignedPlayerMessage, GamePlayerMessage, GameStateMessage, PlayerMessage, RoomCode,
    },
    npc::scoring::NPCPersonality,
    player::Hand,
    reporting::{BoardChange, BoardChangeAction, BoardChangeDetail, Change, TimeChange},
};

use eframe::{
    egui::{self, Align2, Layout},
    emath::Align,
};
use hashbrown::HashMap;

use crate::{
    lil_bits::{BoardUI, DictionaryUI},
    utils::{
        control_devices,
        depot::{
            AestheticDepot, AudioDepot, BoardDepot, GameplayDepot, InteractionDepot, RegionDepot,
            TimingDepot, TruncateDepot, UIStateDepot,
        },
        mapper::{MappedBoard, MappedTiles},
        timing::get_qs_tick,
        Theme,
    },
};

#[cfg(not(target_arch = "wasm32"))]
use crate::utils::control_devices::gamepad::GamepadInput;

mod actions_menu;
mod control_strip;
mod dictionary;
mod header_strip;
mod sidebar;

#[derive(Clone, Default, Debug)]
pub enum HeaderType {
    #[default]
    Timers,
    Summary {
        title: String,
        attempt: Option<usize>,
    },
    Tutorial,
    None,
}

#[derive(Clone)]
pub enum GameLocation {
    Tutorial,
    Local,
    Online,
}

#[derive(Clone)]
pub struct ActiveGame {
    pub depot: TruncateDepot,
    pub players: Vec<GamePlayerMessage>,
    pub board: Board,
    pub mapped_board: MappedBoard,
    pub mapped_overlay: MappedTiles,
    pub mapped_hands: Vec<MappedTiles>,
    pub hands: Vec<Hand>,
    pub board_changes: HashMap<Coordinate, BoardChange>,
    pub time_changes: Vec<TimeChange>,
    pub turn_reports: Vec<Vec<Change>>,
    pub location: GameLocation,
    pub dictionary_ui: Option<DictionaryUI>,
    #[cfg(not(target_arch = "wasm32"))]
    pub gamepad_input: Arc<Mutex<GamepadInput>>,
}

impl ActiveGame {
    pub fn new(
        ctx: &egui::Context,
        room_code: RoomCode,
        game_seed: Option<BoardSeed>,
        npc: Option<NPCPersonality>,
        players: Vec<GamePlayerMessage>,
        player_numbers: Vec<u64>,
        next_player_number: Option<u64>,
        board: Board,
        hands: Vec<Hand>,
        map_texture: TextureHandle,
        theme: Theme,
        location: GameLocation,
        game_ends_at: Option<u64>,
        remaining_turns: Option<u64>,
    ) -> Self {
        let player_colors = players
            .iter()
            .map(|p| Color32::from_rgb(p.color.0, p.color.1, p.color.2))
            .collect::<Vec<_>>();

        let mut depot = TruncateDepot {
            interactions: player_numbers
                .iter()
                .map(|_| InteractionDepot::default())
                .collect(),
            regions: RegionDepot::default(),
            ui_state: UIStateDepot::default(),
            board_info: BoardDepot {
                board_seed: game_seed,
                ..BoardDepot::default()
            },
            timing: TimingDepot {
                game_ends_at,
                ..TimingDepot::default()
            },
            gameplay: GameplayDepot {
                room_code,
                player_numbers: player_numbers.clone(),
                next_player_number,
                winner: None,
                changes: Vec::new(),
                last_battle_origin: None,
                npc,
                remaining_turns,
            },
            aesthetics: AestheticDepot {
                theme: theme.clone(),
                qs_tick: 0,
                map_texture,
                player_colors,
                destruction_tick: 0.05,
                destruction_duration: 0.6,
            },
            audio: AudioDepot::default(),
        };

        #[cfg(target_arch = "wasm32")]
        {
            let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
            depot.audio.muted = local_storage
                .get_item("truncate_muted")
                .unwrap()
                .unwrap_or_default()
                .parse()
                .unwrap_or_default();
        }

        Self {
            mapped_board: MappedBoard::new(
                ctx,
                &depot.aesthetics,
                &board,
                player_numbers,
                theme.daytime,
            ),
            mapped_overlay: MappedTiles::new(ctx, 1),
            mapped_hands: hands
                .iter()
                .map(|h| MappedTiles::new(ctx, h.len()))
                .collect(),
            depot,
            players,
            board,
            hands,
            board_changes: HashMap::new(),
            time_changes: vec![],
            turn_reports: vec![],
            location,
            dictionary_ui: None,
            #[cfg(not(target_arch = "wasm32"))]
            gamepad_input: Arc::new(Mutex::new(GamepadInput::new())),
        }
    }
}

impl ActiveGame {
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        current_time: Duration,
        game_ref: Option<&truncate_core::game::Game>,
    ) -> Option<AssignedPlayerMessage> {
        self.depot.timing.current_time = current_time;
        let cur_tick = get_qs_tick(current_time);
        if cur_tick > self.depot.aesthetics.qs_tick {
            self.depot.aesthetics.qs_tick = cur_tick;
        }

        let kb_msg = control_devices::keyboard::handle_input(
            ui.ctx(),
            &self.board,
            &self.hands,
            &mut self.depot,
        );

        #[cfg(not(target_arch = "wasm32"))]
        let pad_msg = self.gamepad_input.lock().unwrap().handle_input(
            ui.ctx(),
            &self.board,
            &self.hands,
            &mut self.depot,
        );

        if !self.depot.ui_state.is_touch {
            // If we ever receive any touch event,
            // irrevocably put Truncate into touch mode.
            if ui.input(|i| {
                i.events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Touch { .. }))
            }) {
                self.depot.ui_state.is_touch = true;
            }
        }

        let mut game_space = ui.available_rect_before_wrap();
        let mut sidebar_space = game_space.clone();
        let actions_menu_open_last_frame = self.depot.ui_state.actions_menu_open;

        if !self.depot.ui_state.sidebar_hidden
            && ui.available_size().x >= self.depot.aesthetics.theme.mobile_breakpoint
        {
            self.depot.ui_state.is_mobile = false;
        } else {
            if self.depot.ui_state.is_mobile == false {
                // Close the sidebar overlay when transitioning to the mobile breakpoint
                self.depot.ui_state.sidebar_toggled = false;
            }
            self.depot.ui_state.is_mobile = true;
        }

        if !self.depot.ui_state.is_mobile && self.depot.ui_state.sidebar_toggled {
            game_space.set_right(game_space.right() - 300.0);
            sidebar_space.set_left(sidebar_space.right() - 300.0);
        }

        let mut control_strip_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));
        let (control_strip_rect, control_player_message) = self.render_control_strip(
            &mut control_strip_ui,
            self.depot.gameplay.player_numbers[0] as _,
            Align2::LEFT_BOTTOM,
            vec2(0.0, 0.0),
        );

        let mut top_control_strip_rect = None;
        if let Some(player_two) = self.depot.gameplay.player_numbers.get(1) {
            let mut top_control_strip_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));
            let (control_strip_rect, control_player_message) = self.render_control_strip(
                &mut top_control_strip_ui,
                self.depot.gameplay.player_numbers[1] as _,
                Align2::LEFT_TOP,
                vec2(0.0, 0.0),
            );
            top_control_strip_rect = control_strip_rect;
            // TODO: handle control_player_message
        }
        debug_assert!(self.depot.gameplay.player_numbers.len() <= 2);

        let mut timer_strip_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));
        let (timer_strip_rect, timer_player_message) =
            self.render_header_strip(&mut timer_strip_ui, game_ref, 0);

        let mut sidebar_space_ui = ui.child_ui(sidebar_space, Layout::top_down(Align::LEFT));
        let sidebar_player_message = self.render_sidebar(&mut sidebar_space_ui);

        if let Some(timer_strip_rect) = timer_strip_rect {
            game_space.set_top(timer_strip_rect.bottom());
        }
        if let Some(top_control_strip_rect) = top_control_strip_rect {
            game_space.set_top(top_control_strip_rect.bottom());
        }
        if let Some(control_strip_rect) = control_strip_rect {
            game_space.set_bottom(control_strip_rect.top());
        }

        let mut game_space_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));

        let actions_player_message = if self.depot.ui_state.actions_menu_open {
            self.render_actions_menu(&mut game_space_ui, actions_menu_open_last_frame)
        } else {
            None
        };

        let dict_player_message = self.render_dictionary(ui);

        let wrap = |message: PlayerMessage| AssignedPlayerMessage {
            message,
            player_id: None,
        };
        let player_message = BoardUI::new(&self.board)
            .interactive(!self.depot.interactions[0].view_only)
            .render(
                &self.hands,
                &self.board_changes,
                &mut game_space_ui,
                &mut self.mapped_board,
                &mut self.mapped_overlay,
                &mut self.depot,
            )
            .or(actions_player_message.map(|message| wrap(message)))
            .or(control_player_message.map(|message| wrap(message)))
            .or(timer_player_message.map(|message| wrap(message)))
            .or(dict_player_message.map(|message| wrap(message)))
            .or(sidebar_player_message.map(|message| wrap(message)));

        #[cfg(not(target_arch = "wasm32"))]
        return kb_msg.or(pad_msg).or(player_message);
        #[cfg(target_arch = "wasm32")]
        kb_msg.or(player_message)
    }

    pub fn apply_new_timing(&mut self, state_message: GameStateMessage) {
        let GameStateMessage {
            room_code: _,
            players,
            player_number: _,
            next_player_number: _,
            board: _,
            hand: _,
            changes: _,
            game_ends_at,
            paused,
            remaining_turns: _,
        } = state_message;

        self.players = players;
        self.depot.timing.game_ends_at = game_ends_at;

        self.depot.timing.paused = paused;
    }

    pub fn apply_new_state(&mut self, state_message: GameStateMessage) {
        let GameStateMessage {
            room_code: _,
            players,
            player_number: this_player_number,
            next_player_number,
            board,
            hand: _,
            changes,
            game_ends_at,
            paused,
            remaining_turns,
        } = state_message;

        // assert_eq!(self.room_code, room_code);
        // assert_eq!(self.player_number, player_number);
        self.players = players;
        self.board = board;

        #[cfg(target_arch = "wasm32")]
        if !self.depot.audio.muted {
            // Play the turn sound if the player has changed
            if self.depot.gameplay.next_player_number != next_player_number {
                use eframe::wasm_bindgen::JsCast;

                let window = web_sys::window().expect("window should exist in browser");
                let document = window.document().expect("documnt should exist in window");
                if let Some(element) = document.query_selector("#tr_move").unwrap() {
                    if let Ok(audio) = element.dyn_into::<web_sys::HtmlAudioElement>() {
                        // TODO: Rework audio, as this sound often gets filtered out from headphones
                        _ = audio.play().expect("Audio should be playable");
                    }
                }
            }
        }

        self.depot.gameplay.next_player_number = next_player_number;
        self.depot.timing.last_turn_change = self.depot.timing.current_time;
        self.depot.timing.game_ends_at = game_ends_at;
        self.depot.timing.paused = paused;
        self.depot.gameplay.remaining_turns = remaining_turns;

        self.depot.gameplay.changes = changes.clone();

        self.board_changes.clear();
        for board_change in changes.iter().filter_map(|c| match c {
            Change::Board(change) => Some(change),
            _ => None,
        }) {
            self.board_changes
                .insert(board_change.detail.coordinate, board_change.clone());
        }

        for hand_change in changes.iter().filter_map(|c| match c {
            Change::Hand(change) => Some(change),
            _ => None,
        }) {
            let player_index = self
                .depot
                .gameplay
                .player_numbers
                .iter()
                .position(|p| *p == hand_change.player as u64);

            if let Some(player_index) = player_index {
                for removed in &hand_change.removed {
                    if let Some(pos) = self.hands[player_index].iter().position(|t| t == removed) {
                        self.hands[player_index].remove(pos);
                    }
                }
                let reduced_length = self.hands[player_index].len();
                self.hands[player_index].0.extend(&hand_change.added);
            }
        }

        self.time_changes = changes
            .iter()
            .filter_map(|change| match change {
                Change::Time(time_change) => Some(time_change.clone()),
                _ => None,
            })
            .collect();

        let battle_occurred = changes
            .iter()
            .any(|change| matches!(change, Change::Battle(_)));

        if battle_occurred {
            self.depot.ui_state.unread_sidebar = true;

            self.depot.gameplay.last_battle_origin =
                changes.iter().find_map(|change| match change {
                    Change::Board(BoardChange {
                        detail: BoardChangeDetail { coordinate, .. },
                        action: BoardChangeAction::Added,
                    }) => Some(*coordinate),
                    _ => None,
                });
        } else {
            self.depot.gameplay.last_battle_origin = None;
        }

        self.turn_reports.push(changes);

        // TODO: Verify that our modified hand matches the actual hand in GameStateMessage

        if let Some(ints) = self.depot.interactions.get_mut(this_player_number as usize) {
            ints.error_msg = None;
        }
    }
}
