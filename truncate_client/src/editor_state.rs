use epaint::{Pos2, Rect, TextureHandle};
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
    lil_bits::{BattleUI, BoardUI, EditorUI, HandUI, TimerUI},
    theming::{mapper::MappedBoard, tex::Tex, Theme},
};

#[derive(Clone)]
// TODO: Split this state struct up
pub struct EditorState {
    pub board: Board,
    pub room_code: RoomCode,
    pub players: Vec<String>,
    pub mapped_board: MappedBoard,
}

impl EditorState {
    pub fn new(
        room_code: RoomCode,
        players: Vec<String>,
        board: Board,
        map_texture: TextureHandle,
    ) -> Self {
        Self {
            room_code,
            players,
            mapped_board: MappedBoard::map(&board, map_texture, false),
            board,
        }
    }

    pub fn update_board(&mut self, board: Board) {
        self.mapped_board.remap(&board);
        self.board = board;
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme) -> Option<PlayerMessage> {
        let mut msg = None;

        ui.label(format!("Playing in game {}", self.room_code));
        ui.label(format!("In lobby: {}", self.players.join(", ")));
        ui.label("Waiting for the game to start . . .");

        if ui.button("Start game").clicked() {
            msg = Some(PlayerMessage::StartGame);
        }

        if let Some(board_update) =
            EditorUI::new(&mut self.board, &self.mapped_board).render(true, ui, theme)
        {
            msg = Some(board_update);
        }

        msg
    }
}
