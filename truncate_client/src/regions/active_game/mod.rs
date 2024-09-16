use std::sync::{Arc, Mutex};

use epaint::{vec2, Color32, TextureHandle};
use time::Duration;
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
        control_devices::{self, Switchboard},
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
use crate::utils::control_devices::gamepad::GamepadManager;

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
    Arcade,
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
        game_ends_at: Option<Duration>,
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
        }
    }
}

impl ActiveGame {
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        current_time: Duration,
        switchboard: &mut Switchboard,
        game_ref: Option<&truncate_core::game::Game>,
    ) -> Vec<AssignedPlayerMessage> {
        self.depot.timing.current_time = current_time;
        let mut msgs = vec![];
        let cur_tick = get_qs_tick(current_time);
        if cur_tick > self.depot.aesthetics.qs_tick {
            self.depot.aesthetics.qs_tick = cur_tick;
        }

        for (for_player_index, player) in self.players.iter().enumerate() {
            let active_hand = self.depot.gameplay.next_player_number.is_none()
                || self
                    .depot
                    .gameplay
                    .next_player_number
                    .is_some_and(|n| n == player.index as u64);
            let waiting = player
                .turn_starts_no_later_than
                .is_some_and(|t| t > self.depot.timing.current_time);

            if let Some(turn) = self.depot.interactions.get_mut(for_player_index) {
                turn.current_turn = active_hand && !waiting;
            }
        }

        msgs.extend(switchboard.operate(ui.ctx(), &self.board, &self.hands, &mut self.depot));

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

        if let Some(player_message) = player_message {
            msgs.push(player_message);
        }

        msgs
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
            hand,
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

        self.hands[this_player_number as usize] = hand;

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
