use core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
};

use eframe::egui::{self, Margin, Sense};

use crate::theming::Theme;

use super::{character::CharacterOrient, CharacterUI, EditorBarEdge, EditorBarUI, EditorSquareUI};

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

        let frame = egui::Frame::none().inner_margin(Margin::same(theme.grid_size));
        let editor_rect = frame
            .show(ui, |ui| {
                for (rownum, row) in self.board.squares.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        for (colnum, square) in row.iter_mut().enumerate() {
                            let coord = Coordinate::new(colnum, rownum);

                            if EditorSquareUI::new()
                                .enabled(square.is_some())
                                .render(ui, theme)
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
            })
            .response
            .rect;

        if EditorBarUI::new(EditorBarEdge::Top)
            .render(ui, editor_rect.clone(), theme)
            .clicked()
        {
            self.board.squares.insert(0, vec![None; self.board.width()]);
            edited = true;
        }

        if EditorBarUI::new(EditorBarEdge::Bottom)
            .render(ui, editor_rect.clone(), theme)
            .clicked()
        {
            self.board.squares.push(vec![None; self.board.width()]);
            edited = true;
        }

        if EditorBarUI::new(EditorBarEdge::Right)
            .render(ui, editor_rect.clone(), theme)
            .clicked()
        {
            for row in &mut self.board.squares {
                row.push(None);
            }
            edited = true;
        }

        if EditorBarUI::new(EditorBarEdge::Left)
            .render(ui, editor_rect.clone(), theme)
            .clicked()
        {
            for row in &mut self.board.squares {
                row.insert(0, None);
            }
            edited = true;
        }

        if edited {
            Some(PlayerMessage::EditBoard(self.board.clone()))
        } else {
            None
        }
    }
}
