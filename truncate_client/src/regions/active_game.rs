use epaint::{emath::Align2, vec2, Color32, Rect, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage, RoomCode},
    player::Hand,
    reporting::{BattleReport, BoardChange, Change, TimeChange},
};

use eframe::{
    egui::{self, Layout, Order, ScrollArea},
    emath::Align,
};
use hashbrown::HashMap;

use crate::{
    lil_bits::{BattleUI, BoardUI, HandUI, TimerUI},
    theming::{mapper::MappedBoard, Theme},
};

#[derive(Debug, Clone, PartialEq)]
pub struct HoveredRegion {
    pub rect: Rect,
}

#[derive(Clone)]
pub struct GameCtx {
    pub theme: Theme,
    pub current_time: Duration,
    pub room_code: RoomCode,
    pub player_number: u64,
    pub next_player_number: u64,
    pub selected_tile_in_hand: Option<usize>,
    pub released_tile: Option<usize>,
    pub selected_square_on_board: Option<Coordinate>,
    pub hovered_tile_on_board: Option<HoveredRegion>,
    pub playing_tile: Option<char>,
    pub error_msg: Option<String>,
    pub map_texture: TextureHandle,
    pub player_colors: Vec<Color32>,
    pub board_zoom: f32,
    pub board_pan: Vec2,
    pub sidebar_visible: bool,
}

#[derive(Clone)]
pub struct ActiveGame {
    pub ctx: GameCtx,
    pub players: Vec<GamePlayerMessage>,
    pub board: Board,
    pub mapped_board: MappedBoard,
    pub hand: Hand,
    pub board_changes: HashMap<Coordinate, BoardChange>,
    pub new_hand_tiles: Vec<usize>,
    pub time_changes: Vec<TimeChange>,
    pub turn_reports: Vec<Vec<Change>>,
}

impl ActiveGame {
    pub fn new(
        room_code: RoomCode,
        players: Vec<GamePlayerMessage>,
        player_number: u64,
        next_player_number: u64,
        board: Board,
        hand: Hand,
        map_texture: TextureHandle,
        theme: Theme,
    ) -> Self {
        let player_colors = players
            .iter()
            .map(|p| Color32::from_rgb(p.color.0, p.color.1, p.color.2))
            .collect::<Vec<_>>();
        Self {
            ctx: GameCtx {
                theme,
                current_time: Duration::from_secs(0),
                room_code,
                player_number,
                next_player_number,
                selected_tile_in_hand: None,
                released_tile: None,
                selected_square_on_board: None,
                hovered_tile_on_board: None,
                playing_tile: None,
                error_msg: None,
                map_texture: map_texture.clone(),
                player_colors: player_colors.clone(),
                board_zoom: 1.0,
                board_pan: vec2(0.0, 0.0),
                sidebar_visible: true,
            },
            mapped_board: MappedBoard::new(
                &board,
                map_texture.clone(),
                player_number == 0,
                player_colors,
            ),
            players,
            board,
            hand,
            board_changes: HashMap::new(),
            new_hand_tiles: vec![],
            time_changes: vec![],
            turn_reports: vec![],
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        winner: Option<usize>,
    ) -> Option<PlayerMessage> {
        // We have to go through the instant crate as
        // most std time functions are not implemented
        // in Rust's wasm targets.
        // instant::SystemTime::now() conditionally uses
        // a js function on wasm targets, and otherwise aliases
        // to the std SystemTime type.
        self.ctx.current_time = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("We are living in the future");

        let mut player_message = None;

        ui.allocate_ui_with_layout(
            ui.available_size(),
            Layout::right_to_left(Align::TOP),
            |ui| {
                if self.ctx.sidebar_visible {
                    if ui.available_size().x < self.ctx.theme.mobile_breakpoint {
                    } else {
                        let sidebar_area = vec2(300.0, ui.available_height());

                        ui.allocate_ui_with_layout(
                            sidebar_area,
                            Layout::top_down(Align::TOP),
                            |ui| {
                                ScrollArea::new([false, true]).show(ui, |ui| {
                                    if let Some(error) = &self.ctx.error_msg {
                                        ui.label(error);
                                        ui.separator();
                                    }

                                    for turn in self.turn_reports.iter() {
                                        for placement in
                                            turn.iter().filter_map(|change| match change {
                                                Change::Board(placement) => Some(placement),
                                                _ => None,
                                            })
                                        {
                                            let Square::Occupied(player, tile) = placement.detail.square else {
                                                continue;
                                            };
                                            let Some(player) = self.players.get(player) else {
                                                continue;
                                            };

                                            match placement.action {
                                                truncate_core::reporting::BoardChangeAction::Added => {
                                                    ui.label(format!("{} placed the tile {}", player.name, tile));
                                                },
                                                truncate_core::reporting::BoardChangeAction::Swapped => {
                                                    ui.label(format!("{} swapped the tile {}", player.name, tile));
                                                },
                                                _ => {}
                                            }
                                        }

                                        for battle in
                                            turn.iter().filter_map(|change| match change {
                                                Change::Battle(battle) => Some(battle),
                                                _ => None,
                                            })
                                        {
                                            BattleUI::new(battle).render(&mut self.ctx, ui);
                                        }
                                        ui.separator();
                                    }
                                });
                            },
                        );
                    }
                }

                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    Layout::bottom_up(Align::LEFT),
                    |ui| {

                        let area = egui::Area::new(egui::Id::new("controls_layer"))
                            .movable(false)
                            .order(Order::Foreground)
                            .anchor(Align2::LEFT_BOTTOM, vec2(0.0, 0.0));

                        area.show(ui.ctx(), |ui| {
                            if let Some(player) = self
                                .players
                                .iter()
                                .find(|p| p.index == self.ctx.player_number as usize)
                            {
                                TimerUI::new(player, self.ctx.current_time, &self.time_changes)
                                    .friend(true)
                                    .active(player.index == self.ctx.next_player_number as usize)
                                    .winner(winner.clone())
                                    .render(ui, theme, &mut self.ctx);
                            }

                            if let Some(opponent) = self
                                .players
                                .iter()
                                .find(|p| p.index != self.ctx.player_number as usize)
                            {
                                TimerUI::new(opponent, self.ctx.current_time, &self.time_changes)
                                    .friend(false)
                                    .active(opponent.index == self.ctx.next_player_number as usize)
                                    .winner(winner.clone())
                                    .render(ui, theme, &mut self.ctx);
                            }

                            HandUI::new(&mut self.hand)
                                .active(self.ctx.player_number == self.ctx.next_player_number)
                                .render(&mut self.ctx, ui);
                        });

                        ui.allocate_ui_with_layout(
                            ui.available_size(),
                            Layout::top_down(Align::LEFT),
                            |ui| {
                                player_message = BoardUI::new(&self.board).render(
                                    &self.hand,
                                    &self.board_changes,
                                    winner.clone(),
                                    &mut self.ctx,
                                    ui,
                                    &self.mapped_board,
                                );
                            },
                        );
                    },
                );
            },
        );

        player_message
    }

    pub fn apply_new_state(&mut self, state_message: GameStateMessage) {
        let GameStateMessage {
            room_code: _,
            players,
            player_number: _,
            next_player_number,
            board,
            hand: _,
            changes,
        } = state_message;

        // assert_eq!(self.room_code, room_code);
        // assert_eq!(self.player_number, player_number);
        self.players = players;
        self.board = board;
        self.ctx.next_player_number = next_player_number;

        #[cfg(target_arch = "wasm32")]
        if self.ctx.next_player_number == self.ctx.player_number {
            use eframe::wasm_bindgen::JsCast;

            let window = web_sys::window().expect("window should exist in browser");
            let document = window.document().expect("documnt should exist in window");
            if let Some(element) = document.query_selector("#tr_move").unwrap() {
                if let Ok(audio) = element.dyn_into::<web_sys::HtmlAudioElement>() {
                    audio.play().expect("Audio should be playable");
                }
            }
        }

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
            for removed in &hand_change.removed {
                self.hand.remove(
                    self.hand
                        .iter()
                        .position(|t| t == removed)
                        .expect("Player doesn't have tile being removed"),
                );
            }
            let reduced_length = self.hand.len();
            self.hand.0.extend(&hand_change.added);
            self.new_hand_tiles = (reduced_length..self.hand.len()).collect();
        }

        self.time_changes = changes
            .iter()
            .filter_map(|change| match change {
                Change::Time(time_change) => Some(time_change.clone()),
                _ => None,
            })
            .collect();

        self.turn_reports.push(changes);

        // TODO: Verify that our modified hand matches the actual hand in GameStateMessage

        self.ctx.playing_tile = None;
        self.ctx.error_msg = None;
    }
}
