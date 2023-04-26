use epaint::TextureId;
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
};

use eframe::egui::{self, Id, Margin};

use crate::theming::Theme;

use super::EditorSquareUI;

#[derive(Clone)]
enum EditorDrag {
    Enabling,
    Disabling,
    MovingRoot(usize),
}

pub struct EditorUI<'a> {
    board: &'a mut Board,
    map_texture: TextureId,
}

impl<'a> EditorUI<'a> {
    pub fn new(board: &'a mut Board, map_texture: TextureId) -> Self {
        Self { board, map_texture }
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
        let editor_rect = outer_frame
            .show(ui, |ui| {
                let frame = egui::Frame::none().inner_margin(Margin::same(theme.grid_size));
                frame
                    .show(ui, |ui| {
                        for (rownum, row) in self.board.squares.iter().enumerate() {
                            ui.horizontal(|ui| {
                                for (colnum, square) in row.iter().enumerate() {
                                    let coord = Coordinate::new(colnum, rownum);
                                    let is_root = self.board.roots.iter().position(|r| r == &coord);

                                    let neighbors = coord
                                        .neighbors_8()
                                        .into_iter()
                                        .map(|pos| self.board.get(pos).is_ok())
                                        .collect();

                                    let response =
                                        EditorSquareUI::new(coord, self.map_texture, neighbors)
                                            .enabled(square.is_some())
                                            .root(is_root.is_some())
                                            .render(ui, &theme);

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
                                                modify_pos = Some((coord, Some(Square::Empty)));
                                            }
                                            (Some(EditorDrag::Disabling), Some(_)) => {
                                                modify_pos = Some((coord, None));
                                            }
                                            (Some(EditorDrag::MovingRoot(root)), _) => {
                                                // TODO: Re-enable moving the roots

                                                // if is_root.is_none() {
                                                //     self.board.roots[root] = coord;
                                                //     edited = true;
                                                // }
                                            }
                                            _ => {}
                                        }
                                    }
                                    if response.drag_started() {
                                        ui.ctx().memory_mut(|mem| {
                                            mem.data.insert_temp(
                                                Id::null(),
                                                if let Some(root) = is_root {
                                                    EditorDrag::MovingRoot(root)
                                                } else if square.is_some() {
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
                                        ui.ctx().memory_mut(|mem| {
                                            mem.data.remove::<EditorDrag>(Id::null())
                                        });
                                    } else if response.clicked() {
                                        if square.is_some() {
                                            modify_pos = Some((coord, None));
                                        } else {
                                            modify_pos = Some((coord, Some(Square::Empty)));
                                        }
                                    };
                                }
                            });
                        }
                    })
                    .response
                    .rect
            })
            .inner;

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
