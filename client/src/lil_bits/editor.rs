use core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
};

use eframe::egui::{self, Id, Margin, Sense};

use crate::theming::Theme;

use super::{character::CharacterOrient, CharacterUI, EditorBarEdge, EditorBarUI, EditorSquareUI};

#[derive(Clone)]
enum EditorDrag {
    Enabling,
    Disabling,
}

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
                            let response = EditorSquareUI::new()
                                .enabled(square.is_some())
                                .render(ui, theme);

                            if ui.rect_contains_pointer(response.rect) {
                                // TODO: This shouldn't be mut
                                // https://github.com/emilk/egui/issues/2741
                                let drag_action = ui.memory_mut(|mem| {
                                    if mem.is_anything_being_dragged() {
                                        mem.data.get_temp::<EditorDrag>(Id::null())
                                    } else {
                                        None
                                    }
                                });

                                match (drag_action, &square) {
                                    (Some(EditorDrag::Enabling), None) => {
                                        *square = Some(Square::Empty)
                                    }
                                    (Some(EditorDrag::Disabling), Some(_)) => *square = None,
                                    _ => {}
                                }
                            }
                            if response.drag_started() {
                                ui.ctx().memory_mut(|mem| {
                                    mem.data.insert_temp(
                                        Id::null(),
                                        if square.is_some() {
                                            EditorDrag::Disabling
                                        } else {
                                            EditorDrag::Enabling
                                        },
                                    )
                                });
                            }
                            // Chain these next two together so that the drag end takes precedence,
                            // otherwise we double flip. Second branch remains to cover states without
                            // drag support, perhaps?
                            if response.drag_released() {
                                ui.ctx()
                                    .memory_mut(|mem| mem.data.remove::<EditorDrag>(Id::null()));
                            } else if response.clicked() {
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
