use epaint::TextureHandle;
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
};

use eframe::egui::{self, Id, Margin};

use crate::theming::{mapper::MappedBoard, Theme};

use super::EditorSquareUI;

#[derive(Clone)]
enum EditorDrag {
    Enabling,
    Disabling,
    MovingRoot(usize),
}

pub struct EditorUI<'a> {
    board: &'a mut Board,
    mapped_board: &'a MappedBoard,
}

impl<'a> EditorUI<'a> {
    pub fn new(board: &'a mut Board, mapped_board: &'a MappedBoard) -> Self {
        Self {
            board,
            mapped_board,
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

        if ui.button("Grow board").clicked() {
            self.board.grow();
            edited = true;
        }

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
                                    .enabled(matches!(square, Square::Land | Square::Town(_)))
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

                                    match (drag_action, &square) {
                                        (Some(EditorDrag::Enabling), Square::Water) => {
                                            modify_pos = Some((coord, Square::Land));
                                        }
                                        (Some(EditorDrag::Disabling), Square::Land) => {
                                            modify_pos = Some((coord, Square::Water));
                                        }
                                        (Some(EditorDrag::MovingRoot(root)), _) => {
                                            modify_pos = Some((coord, Square::Dock(root)));
                                        }
                                        _ => {}
                                    }
                                }
                                if response.drag_started() {
                                    ui.ctx().memory_mut(|mem| {
                                        mem.data.insert_temp(
                                            Id::null(),
                                            match square {
                                                Square::Water => EditorDrag::Enabling,
                                                Square::Land => EditorDrag::Disabling,
                                                Square::Town(_) => EditorDrag::Enabling, // TODO
                                                Square::Dock(root) => EditorDrag::MovingRoot(*root),
                                                Square::Occupied(_, _) => unreachable!(
                                                    "Board editor shouldn't see occupied tiles"
                                                ),
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
                                    match square {
                                        Square::Water => modify_pos = Some((coord, Square::Land)),
                                        Square::Land => modify_pos = Some((coord, Square::Water)),
                                        Square::Town(_) => {} // TODO
                                        Square::Dock(_) => {} // TODO
                                        Square::Occupied(_, _) => unreachable!(
                                            "Board editor shouldn't see occupied tiles"
                                        ),
                                    }
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
