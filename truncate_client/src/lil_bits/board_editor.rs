use epaint::TextureHandle;
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
};

use eframe::egui::{self, Id, Margin};

use crate::{
    regions::lobby::BoardEditingMode,
    utils::{mapper::MappedBoard, Theme},
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
    editing_mode: &'a mut BoardEditingMode,
}

impl<'a> EditorUI<'a> {
    pub fn new(
        board: &'a mut Board,
        mapped_board: &'a MappedBoard,
        editing_mode: &'a mut BoardEditingMode,
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

        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        let (margin, theme) = theme.calc_rescale(
            &ui.available_rect_before_wrap(),
            self.board.width(),
            self.board.height(),
            0.3..2.0,
            (2, 2),
        );
        let outer_frame = egui::Frame::none().inner_margin(margin);

        let mut modify_pos = None;
        outer_frame.show(ui, |ui| {
            for (rownum, row) in self.board.squares.iter().enumerate() {
                ui.horizontal(|ui| {
                    for (colnum, square) in row.iter().enumerate() {
                        let coord = Coordinate::new(colnum, rownum);

                        let response = EditorSquareUI::new(coord)
                            .square(square.clone())
                            .action(self.editing_mode.clone())
                            .render(ui, &theme, self.mapped_board, &map_texture);

                        if matches!(self.editing_mode, BoardEditingMode::None) {
                            continue;
                        }

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
                                    (EditorDrag::MakeLand, Square::Water | Square::Dock(_)) => {
                                        modify_pos = Some((coord, Square::Land))
                                    }
                                    (EditorDrag::RemoveLand, Square::Land | Square::Town(_)) => {
                                        modify_pos = Some((coord, Square::Water))
                                    }
                                    (EditorDrag::MakeTown(player), _) => {
                                        modify_pos = Some((coord, Square::Town(player)))
                                    }
                                    (EditorDrag::RemoveTown(player), Square::Town(sq_player))
                                        if player == *sq_player =>
                                    {
                                        modify_pos = Some((coord, Square::Land))
                                    }
                                    (EditorDrag::MakeDock(player), _) => {
                                        modify_pos = Some((coord, Square::Dock(player)))
                                    }
                                    (EditorDrag::RemoveDock(player), Square::Dock(sq_player))
                                        if player == *sq_player =>
                                    {
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
                                    match &self.editing_mode {
                                        BoardEditingMode::None => unreachable!(
                                            "With no board editing set we should not be editing"
                                        ),
                                        BoardEditingMode::Land => match square {
                                            Square::Water | Square::Dock(_) => EditorDrag::MakeLand,
                                            Square::Land | Square::Town(_) => {
                                                EditorDrag::RemoveLand
                                            }
                                            Square::Occupied(_, _) => unreachable!(),
                                        },
                                        BoardEditingMode::Town(editing_player) => match square {
                                            Square::Town(sq_player)
                                                if sq_player == editing_player =>
                                            {
                                                EditorDrag::RemoveTown(*editing_player)
                                            }
                                            _ => EditorDrag::MakeTown(*editing_player),
                                        },
                                        BoardEditingMode::Dock(editing_player) => match square {
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
                            ui.ctx()
                                .memory_mut(|mem| mem.data.remove::<EditorDrag>(Id::null()));
                        } else if response.clicked() {
                            unreachable!("Maybe unreachable? Duplicate above state if not...");
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
        });

        if let Some((coord, new_state)) = modify_pos {
            // Not bounds-checking values as they came from the above loop over this very state.
            self.board.squares[coord.y][coord.x] = new_state;

            // TODO: Put board mirroring behind a flag
            {
                let board_mid = (
                    self.board.width() as isize / 2,
                    self.board.height() as isize / 2,
                );
                let recip_x = board_mid.0 - (coord.x as isize - board_mid.0);
                let recip_y = board_mid.1 - (coord.y as isize - board_mid.1);

                // TODO: Player mirroring won't work for >2 players
                let mirrored_state = match new_state {
                    Square::Water | Square::Land => new_state,
                    Square::Town(p) => {
                        if p == 0 {
                            Square::Town(1)
                        } else {
                            Square::Town(0)
                        }
                    }
                    Square::Dock(p) => {
                        if p == 0 {
                            Square::Dock(1)
                        } else {
                            Square::Dock(0)
                        }
                    }
                    Square::Occupied(_, _) => {
                        unreachable!("Board editor should not contain occupied tiles")
                    }
                };

                self.board.squares[recip_y as usize][recip_x as usize] = mirrored_state;
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
