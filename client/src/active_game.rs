use core::{
    board::{Board, Coordinate},
    messages::{GamePlayerMessage, PlayerMessage, RoomCode},
    player::Hand,
    reporting::{BattleReport, BoardChange},
};

use eframe::{
    egui::{self, Layout},
    emath::Align,
};
use epaint::{vec2, Color32, Stroke};
use hashbrown::HashMap;
use time::OffsetDateTime;

use crate::{
    lil_bits::{BoardUI, HandUI, TimerUI},
    theming::Theme,
};

#[derive(Debug, Clone)]
pub struct ActiveGame {
    pub room_code: RoomCode,
    pub players: Vec<GamePlayerMessage>,
    pub player_number: u64,
    pub next_player_number: u64,
    pub board: Board,
    pub hand: Hand,
    pub selected_tile_in_hand: Option<usize>,
    pub selected_square_on_board: Option<Coordinate>,
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
        Self {
            room_code,
            players,
            player_number,
            next_player_number,
            board,
            hand,
            selected_tile_in_hand: None,
            selected_square_on_board: None,
            playing_tile: None,
            error_msg: None,
            board_changes: HashMap::new(),
            new_hand_tiles: vec![],
            battles: vec![],
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme) -> Option<PlayerMessage> {
        ui.label(format!("Playing in game {}", self.room_code));

        ui.separator();

        if let Some(error) = &self.error_msg {
            ui.label(error);
        } else {
            ui.label("");
        }

        if let Some(opponent) = self
            .players
            .iter()
            .find(|p| p.index != self.player_number as usize)
        {
            TimerUI::new(opponent)
                .friend(false)
                .active(opponent.index == self.next_player_number as usize)
                .render(ui, theme);
        }

        let mut remaining_area = ui.available_size();
        remaining_area.y -= theme.grid_size;

        ui.allocate_ui_with_layout(remaining_area, Layout::bottom_up(Align::LEFT), |ui| {
            if let Some(player) = self
                .players
                .iter()
                .find(|p| p.index == self.player_number as usize)
            {
                TimerUI::new(player)
                    .friend(true)
                    .active(player.index == self.next_player_number as usize)
                    .render(ui, theme);
            }
            if let Some(new_selection) =
                HandUI::new(&mut self.hand).render(self.selected_tile_in_hand, ui, theme)
            {
                self.selected_tile_in_hand = new_selection;
                self.selected_square_on_board = None;
            }

            ui.allocate_ui_with_layout(ui.available_size(), Layout::top_down(Align::LEFT), |ui| {
                let board_result = BoardUI::new(&self.board).render(
                    self.selected_tile_in_hand,
                    self.selected_square_on_board,
                    &self.hand,
                    &self.board_changes,
                    self.player_number,
                    self.player_number == 0,
                    ui,
                    theme,
                );

                if let (Some(new_selection), _) = board_result {
                    self.selected_square_on_board = new_selection;
                    self.selected_tile_in_hand = None;
                }

                board_result.1
            })
            .inner
        })
        .inner

        // if self.battles.is_empty() {
        //     ui.label("No battles yet.");
        // } else {
        //     for battle in &self.battles {
        //         ui.label(format!("{battle}"));
        //         ui.separator();
        //     }
        // }

        // board_result.1
    }
}
