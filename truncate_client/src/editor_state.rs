use epaint::{Color32, TextureHandle};
use truncate_core::{
    board::Board,
    messages::{LobbyPlayerMessage, PlayerMessage, RoomCode},
};

use eframe::egui;

use crate::{
    lil_bits::EditorUI,
    theming::{mapper::MappedBoard, Theme},
};

#[derive(Clone)]
// TODO: Split this state struct up
pub struct EditorState {
    pub board: Board,
    pub room_code: RoomCode,
    pub players: Vec<LobbyPlayerMessage>,
    pub mapped_board: MappedBoard,
    pub map_texture: TextureHandle,
}

impl EditorState {
    pub fn new(
        room_code: RoomCode,
        players: Vec<LobbyPlayerMessage>,
        board: Board,
        map_texture: TextureHandle,
    ) -> Self {
        Self {
            room_code,
            mapped_board: MappedBoard::new(
                &board,
                map_texture.clone(),
                false,
                players
                    .iter()
                    .map(|p| Color32::from_rgb(p.color.0, p.color.1, p.color.2))
                    .collect(),
            ),
            players,
            map_texture,
            board,
        }
    }

    pub fn update_board(&mut self, board: Board) {
        self.mapped_board.remap(
            &board,
            self.players
                .iter()
                .map(|p| Color32::from_rgb(p.color.0, p.color.1, p.color.2))
                .collect(),
        );
        self.board = board;
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme) -> Option<PlayerMessage> {
        let mut msg = None;

        ui.label(format!("Playing in game {}", self.room_code));
        ui.label(format!(
            "In lobby: {}",
            self.players
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        ui.label("Waiting for the game to start . . .");

        if ui.button("Start game").clicked() {
            msg = Some(PlayerMessage::StartGame);
        }

        if let Some(board_update) = EditorUI::new(&mut self.board, &self.mapped_board).render(
            true,
            ui,
            theme,
            &self.map_texture,
        ) {
            msg = Some(board_update);
        }

        msg
    }
}
