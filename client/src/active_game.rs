use core::{
    board::{Board, Coordinate},
    messages::{GamePlayerMessage, PlayerMessage, RoomCode},
    player::Hand,
    reporting::BoardChange,
};

use eframe::egui;
use epaint::{Color32, Stroke};
use hashbrown::HashMap;
use time::OffsetDateTime;

use crate::{
    lil_bits::{BoardUI, HandUI},
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
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme) -> Option<PlayerMessage> {
        ui.label(format!("Playing in game {}", self.room_code));

        for player in &self.players {
            ui.horizontal(|ui| {
                match player.turn_starts_at {
                    Some(next_turn) => {
                        let elapsed = OffsetDateTime::now_utc() - next_turn;
                        if elapsed.is_positive() {
                            ui.label(format!(
                                "Player: {} has {:?}s remaining.",
                                player.name,
                                (player.time_remaining - elapsed).whole_seconds()
                            ));
                            ui.label(format!(
                                "Their turn started {:?}s ago",
                                elapsed.whole_seconds()
                            ));
                        } else {
                            ui.label(format!(
                                "Player: {} has {:?}s remaining.",
                                player.name,
                                player.time_remaining.whole_seconds()
                            ));
                            ui.label(format!(
                                "Their turn starts in {:?}s",
                                elapsed.whole_seconds() * -1
                            ));
                        }
                    }
                    None => {
                        ui.label(format!(
                            "Player: {} has {:?}s remaining.",
                            player.name,
                            player.time_remaining.whole_seconds()
                        ));
                    }
                };
            });
        }

        ui.separator();

        let frame = egui::Frame::none().inner_margin(egui::Margin::same(6.0));
        let resp = frame.show(ui, |ui| {
            if self.player_number == self.next_player_number {
                ui.label("It is your turn! :)");
            } else {
                ui.label("It is not your turn :(");
            }
        });

        ui.painter().rect_stroke(
            resp.response.rect,
            2.0,
            Stroke::new(
                2.0,
                if self.player_number == self.next_player_number {
                    Color32::LIGHT_GREEN
                } else {
                    Color32::LIGHT_RED
                },
            ),
        );

        if let Some(error) = &self.error_msg {
            ui.label(error);
        } else {
            ui.label("");
        }

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

        if let Some(new_selection) =
            HandUI::new(&mut self.hand).render(self.selected_tile_in_hand, ui, theme)
        {
            self.selected_tile_in_hand = new_selection;
            self.selected_square_on_board = None;
        }

        board_result.1
    }
}
