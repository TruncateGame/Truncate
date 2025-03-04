use epaint::{Color32, TextureHandle};
use instant::Duration;
use truncate_core::{
    board::{Board, Coordinate},
    generation::BoardSeed,
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage, RoomCode},
    moves::{packing::unpack_moves, Move},
    npc::scoring::NPCPersonality,
    player::Hand,
    reporting::{BoardChange, BoardChangeAction, BoardChangeDetail, Change, TimeChange},
    rules::GameRules,
};

use eframe::{
    egui::{self, Layout},
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
    pub rules: GameRules,
    pub board: Board,
    pub mapped_board: MappedBoard,
    pub mapped_hand: MappedTiles,
    pub mapped_overlay: MappedTiles,
    pub hand: Hand,
    pub move_sequence: Option<Vec<Move>>,
    pub board_changes: HashMap<Coordinate, BoardChange>,
    pub new_hand_tiles: Vec<usize>,
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
        player_number: u64,
        next_player_number: Option<u64>,
        rules: GameRules,
        board: Board,
        hand: Hand,
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
            interactions: InteractionDepot::default(),
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
                player_number,
                next_player_number,
                error_msg: None,
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
                2,
                player_number as usize,
                theme.daytime,
            ),
            mapped_hand: MappedTiles::new(ctx, 7),
            mapped_overlay: MappedTiles::new(ctx, 1),
            depot,
            players,
            rules,
            board,
            hand,
            move_sequence: None,
            board_changes: HashMap::new(),
            new_hand_tiles: vec![],
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
        game_ref: Option<&truncate_core::game::Game>,
    ) -> Option<PlayerMessage> {
        self.depot.timing.current_time = current_time;
        let cur_tick = get_qs_tick(current_time);
        if cur_tick > self.depot.aesthetics.qs_tick {
            self.depot.aesthetics.qs_tick = cur_tick;
        }

        let kb_msg = control_devices::keyboard::handle_input(
            ui.ctx(),
            &self.board,
            &self.hand,
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
        let (control_strip_rect, control_player_message) =
            self.render_control_strip(&mut control_strip_ui);

        let mut timer_strip_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));
        let (timer_strip_rect, timer_player_message) =
            self.render_header_strip(&mut timer_strip_ui, game_ref);

        let mut sidebar_space_ui = ui.child_ui(sidebar_space, Layout::top_down(Align::LEFT));
        let sidebar_player_message = self.render_sidebar(&mut sidebar_space_ui);

        if let Some(timer_strip_rect) = timer_strip_rect {
            game_space.set_top(timer_strip_rect.bottom());
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

        let player_message = BoardUI::new(&self.board)
            .interactive(!self.depot.interactions.view_only)
            .render(
                &self.hand,
                &self.board_changes,
                &mut game_space_ui,
                &mut self.mapped_board,
                &mut self.mapped_overlay,
                &mut self.depot,
            )
            .or(actions_player_message)
            .or(control_player_message)
            .or(timer_player_message)
            .or(dict_player_message)
            .or(sidebar_player_message);

        kb_msg.or(player_message)
    }

    pub fn apply_new_timing(&mut self, state_message: GameStateMessage) {
        let GameStateMessage {
            room_code: _,
            players,
            player_number: _,
            next_player_number: _,
            packed_move_sequence: _,
            rules: _,
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
            player_number: _,
            next_player_number,
            packed_move_sequence,
            rules,
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
        self.rules = rules;

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

        if let Ok(moves) = packed_move_sequence
            .map(|m| unpack_moves(&m, self.players.len()))
            .transpose()
        {
            self.move_sequence = moves;
        }

        self.board_changes.clear();
        for board_change in changes.iter().filter_map(|c| match c {
            Change::Board(change) => Some(change),
            _ => None,
        }) {
            self.board_changes
                .insert(board_change.detail.coordinate, board_change.clone());
        }

        self.hand = hand;

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

        self.depot.interactions.playing_tile = None;
        self.depot.gameplay.error_msg = None;
    }
}
