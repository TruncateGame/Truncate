use std::fmt::format;

use epaint::TextureHandle;
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
};

use eframe::egui::{self, Id, Margin};

use crate::{
    editor_state::EditingMode,
    theming::{mapper::MappedBoard, Theme},
};

use super::EditorSquareUI;

#[derive(Clone)]
enum EditorDrag {
    MakeLand,
    RemoveLand,
    MakeTown(usize),
    RemoveTown(usize),
    MakeDock(usize),
    RemoveDock(usize),
}

pub struct EditorUI<'a> {
    board: &'a mut Board,
    mapped_board: &'a MappedBoard,
    editing_mode: &'a mut EditingMode,
}

impl<'a> EditorUI<'a> {
    pub fn new(
        board: &'a mut Board,
        mapped_board: &'a MappedBoard,
        editing_mode: &'a mut EditingMode,
    ) -> Self {
        Self {
            board,
            mapped_board,
            editing_mode,
        }
    }
}

impl<'a> EditorUI<'a> {
    pub fn render(
        self,
        _invert: bool, // TODO: Transpose to any rotation
        ui: &mut egui::Ui,
        theme: &Theme,
        map_texture: &TextureHandle,
    ) -> Option<PlayerMessage> {
        let mut edited = false;

        ui.horizontal(|ui| {
            if ui.button("Grow board").clicked() {
                self.board.grow();
                edited = true;
            }
            let edit_str = match self.editing_mode {
                EditingMode::Land => "land and water".to_string(),
                EditingMode::Town(p) => format!("player {} towns", *p + 1),
                EditingMode::Dock(p) => format!("player {} docks", *p + 1),
            };
            ui.label(format!("Editing {edit_str}; Change to:"));
            if ui.button("Land").clicked() {
                *self.editing_mode = EditingMode::Land;
            }
            if ui.button("P1 Towns").clicked() {
                *self.editing_mode = EditingMode::Town(0);
            }
            if ui.button("P2 Towns").clicked() {
                *self.editing_mode = EditingMode::Town(1);
            }
            if ui.button("P1 Docks").clicked() {
                *self.editing_mode = EditingMode::Dock(0);
            }
            if ui.button("P2 Docks").clicked() {
                *self.editing_mode = EditingMode::Dock(1);
            }
        });

        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        let (_, theme) = theme.calc_rescale(
            &ui.available_rect_before_wrap(),
            self.board.width(),
            self.board.height(),
            0.3..2.0,
        );
        let outer_frame = egui::Frame::none().inner_margin(Margin::symmetric(0.0, theme.grid_size));

        let mut modify_pos = None;
        outer_frame.show(ui, |ui| {
            let frame = egui::Frame::none().inner_margin(Margin::same(theme.grid_size));
            frame
                .show(ui, |ui| {
                    for (rownum, row) in self.board.squares.iter().enumerate() {
                        ui.horizontal(|ui| {
                            for (colnum, square) in row.iter().enumerate() {
                                let coord = Coordinate::new(colnum, rownum);

                                let response = EditorSquareUI::new(coord)
                                    .enabled(matches!(square, Square::Land))
                                    .render(ui, &theme, self.mapped_board, &map_texture);

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

                                    if let Some(drag_action) = drag_action {
                                        match (drag_action, &square) {
                                            (
                                                EditorDrag::MakeLand,
                                                Square::Water | Square::Dock(_),
                                            ) => modify_pos = Some((coord, Square::Land)),
                                            (
                                                EditorDrag::RemoveLand,
                                                Square::Land | Square::Town(_),
                                            ) => modify_pos = Some((coord, Square::Water)),
                                            (EditorDrag::MakeTown(player), _) => {
                                                modify_pos = Some((coord, Square::Town(player)))
                                            }
                                            (
                                                EditorDrag::RemoveTown(player),
                                                Square::Town(sq_player),
                                            ) if player == *sq_player => {
                                                modify_pos = Some((coord, Square::Land))
                                            }
                                            (EditorDrag::MakeDock(player), _) => {
                                                modify_pos = Some((coord, Square::Dock(player)))
                                            }
                                            (
                                                EditorDrag::RemoveDock(player),
                                                Square::Dock(sq_player),
                                            ) if player == *sq_player => {
                                                modify_pos = Some((coord, Square::Water))
                                            }
                                            (_, _) => {}
                                        }
                                    }
                                }
                                if response.drag_started() {
                                    ui.ctx().memory_mut(|mem| {
                                        mem.data.insert_temp(
                                            Id::null(),
                                            match self.editing_mode {
                                                EditingMode::Land => match square {
                                                    Square::Water | Square::Dock(_) => {
                                                        EditorDrag::MakeLand
                                                    }
                                                    Square::Land | Square::Town(_) => {
                                                        EditorDrag::RemoveLand
                                                    }
                                                    Square::Occupied(_, _) => unreachable!(),
                                                },
                                                EditingMode::Town(editing_player) => match square {
                                                    Square::Town(sq_player)
                                                        if sq_player == editing_player =>
                                                    {
                                                        EditorDrag::RemoveTown(*editing_player)
                                                    }
                                                    _ => EditorDrag::MakeTown(*editing_player),
                                                },
                                                EditingMode::Dock(editing_player) => match square {
                                                    Square::Dock(sq_player)
                                                        if sq_player == editing_player =>
                                                    {
                                                        EditorDrag::RemoveDock(*editing_player)
                                                    }
                                                    _ => EditorDrag::MakeDock(*editing_player),
                                                },
                                            },
                                        )
                                    });
                                }
                                // Chain these next two together so that the drag end takes precedence,
                                // otherwise we double flip. Second branch remains to cover states without
                                // drag support, perhaps?
                                if response.drag_released() {
                                    ui.ctx().memory_mut(|mem| {
                                        mem.data.remove::<EditorDrag>(Id::null())
                                    });
                                } else if response.clicked() {
                                    unreachable!(
                                        "Maybe unreachable? Duplicate above state if not..."
                                    );
                                    // match square {
                                    //     Square::Water => modify_pos = Some((coord, Square::Land)),
                                    //     Square::Land => modify_pos = Some((coord, Square::Water)),
                                    //     Square::Town(_) => {} // TODO
                                    //     Square::Dock(_) => {} // TODO
                                    //     Square::Occupied(_, _) => unreachable!(
                                    //         "Board editor shouldn't see occupied tiles"
                                    //     ),
                                    // }
                                };
                            }
                        });
                    }
                })
                .response
                .rect
        });

        if let Some((coord, new_state)) = modify_pos {
            // Not bounds-checking values as they came from the above loop over this very state.
            self.board.squares[coord.y][coord.x] = new_state;
            edited = true;
        }

        if edited {
            Some(PlayerMessage::EditBoard(self.board.clone()))
        } else {
            None
        }
    }
}
