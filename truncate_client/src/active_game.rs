use epaint::{Pos2, Rect};
use instant::Duration;
use truncate_core::{
    board::{Board, Coordinate},
    messages::{GamePlayerMessage, PlayerMessage, RoomCode},
    player::Hand,
    reporting::{BattleReport, BoardChange},
};

use eframe::{
    egui::{self, Layout, ScrollArea},
    emath::Align,
};
use hashbrown::HashMap;

use crate::{
    lil_bits::{BoardUI, HandUI, TimerUI},
    theming::Theme,
};

#[derive(Debug, Clone, PartialEq)]
pub struct HoveredRegion {
    pub rect: Rect,
}

#[derive(Debug, Clone)]
// TODO: Split this state struct up
pub struct ActiveGame {
    pub current_time: Duration,
    pub room_code: RoomCode,
    pub players: Vec<GamePlayerMessage>,
    pub player_number: u64,
    pub next_player_number: u64,
    pub board: Board,
    pub hand: Hand,
    pub selected_tile_in_hand: Option<usize>,
    pub selected_square_on_board: Option<Coordinate>,
    pub hovered_tile_on_board: Option<HoveredRegion>,
    pub playing_tile: Option<char>,
    pub error_msg: Option<String>,
    pub board_changes: HashMap<Coordinate, BoardChange>,
    pub new_hand_tiles: Vec<usize>,
    pub battles: Vec<BattleReport>,
}

impl ActiveGame {
    pub fn new(
        room_code: RoomCode,
        players: Vec<GamePlayerMessage>,
        player_number: u64,
        next_player_number: u64,
        board: Board,
        hand: Hand,
    ) -> Self {
        let current_time = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("We are living in the future");
        Self {
            current_time,
            room_code,
            players,
            player_number,
            next_player_number,
            board,
            hand,
            selected_tile_in_hand: None,
            selected_square_on_board: None,
            hovered_tile_on_board: None,
            playing_tile: None,
            error_msg: None,
            board_changes: HashMap::new(),
            new_hand_tiles: vec![],
            battles: vec![],
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
        self.current_time = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("We are living in the future");
        let mut player_message = None;

        ui.allocate_ui_with_layout(
            ui.available_size(),
            Layout::right_to_left(Align::TOP),
            |ui| {
                let mut sidebar_area = ui.available_size();
                sidebar_area.x -= sidebar_area.x * 0.7;

                ui.allocate_ui_with_layout(sidebar_area, Layout::top_down(Align::TOP), |ui| {
                    ScrollArea::new([false, true]).show(ui, |ui| {
                        ui.label(format!("Playing in game {}", self.room_code));

                        ui.separator();

                        if let Some(error) = &self.error_msg {
                            ui.label(error);
                            ui.separator();
                        }

                        if self.battles.is_empty() {
                            ui.label("No battles yet.");
                        } else {
                            for battle in self.battles.iter().rev() {
                                ui.label(format!("{battle}"));
                                ui.separator();
                            }
                        }
                    });
                });

                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    Layout::top_down(Align::TOP),
                    |ui| {
                        if let Some(opponent) = self
                            .players
                            .iter()
                            .find(|p| p.index != self.player_number as usize)
                        {
                            TimerUI::new(opponent, self.current_time)
                                .friend(false)
                                .active(opponent.index == self.next_player_number as usize)
                                .winner(winner.clone())
                                .render(ui, theme);
                        }

                        let mut remaining_area = ui.available_size();
                        remaining_area.y -= theme.grid_size;

                        ui.allocate_ui_with_layout(
                            remaining_area,
                            Layout::bottom_up(Align::LEFT),
                            |ui| {
                                if let Some(player) = self
                                    .players
                                    .iter()
                                    .find(|p| p.index == self.player_number as usize)
                                {
                                    TimerUI::new(player, self.current_time)
                                        .friend(true)
                                        .active(player.index == self.next_player_number as usize)
                                        .winner(winner.clone())
                                        .render(ui, theme);
                                }

                                let (new_selection, released_tile) = HandUI::new(&mut self.hand)
                                    .active(self.player_number == self.next_player_number)
                                    .render(
                                        self.selected_tile_in_hand,
                                        ui,
                                        theme,
                                        &self.hovered_tile_on_board,
                                        self.current_time,
                                    );

                                if let Some(new_selection) = new_selection {
                                    self.selected_tile_in_hand = new_selection;
                                    self.selected_square_on_board = None;
                                }

                                ui.allocate_ui_with_layout(
                                    ui.available_size(),
                                    Layout::top_down(Align::LEFT),
                                    |ui| {
                                        let board_result = BoardUI::new(&self.board).render(
                                            self.selected_tile_in_hand,
                                            released_tile,
                                            self.selected_square_on_board,
                                            &self.hand,
                                            &self.board_changes,
                                            self.player_number,
                                            self.player_number == 0,
                                            ui,
                                            theme,
                                        );

                                        if let (Some(new_selection), _, _) = board_result {
                                            self.selected_square_on_board = new_selection;
                                            self.selected_tile_in_hand = None;
                                        }

                                        // Update to store the latest size of the tiles on the board.
                                        if board_result.2 != self.hovered_tile_on_board {
                                            self.hovered_tile_on_board = board_result.2;
                                        }

                                        player_message = board_result.1;
                                    },
                                )
                            },
                        );
                    },
                )
            },
        );

        player_message
    }
}
