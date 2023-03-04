use core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
};

use eframe::egui;

use crate::theming::Theme;

use super::SquareUI;

pub struct EditorUI<'a> {
    board: &'a mut Board,
}

impl<'a> EditorUI<'a> {
    pub fn new(board: &'a mut Board) -> Self {
        Self { board }
    }
}

impl<'a> EditorUI<'a> {
    pub fn render(
        self,
        _invert: bool, // TODO: Transpose to any rotation
        ui: &mut egui::Ui,
        theme: &Theme,
    ) -> Option<PlayerMessage> {
        let mut edited = false;

        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        for (rownum, row) in self.board.squares.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                for (colnum, square) in row.iter_mut().enumerate() {
                    let coord = Coordinate::new(colnum, rownum);

                    if SquareUI::new()
                        .enabled(square.is_some())
                        .empty(false)
                        .render(ui, theme, |ui, theme| {})
                        .clicked()
                    {
                        if square.is_some() {
                            *square = None;
                        } else {
                            *square = Some(Square::Empty);
                        }
                        edited = true;
                    };
                }
            });
        }

        if edited {
            Some(PlayerMessage::EditBoard(self.board.clone()))
        } else {
            None
        }
    }
}
