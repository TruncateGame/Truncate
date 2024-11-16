use epaint::{emath::Align, vec2, Color32, Rect, TextureHandle, Vec2};

use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
};

use eframe::egui::{self, Id, Layout, Margin, RichText, Sense};

use crate::{
    regions::lobby::BoardEditingMode,
    utils::{
        depot::{AestheticDepot, TimingDepot},
        mapper::MappedBoard,
        tex::{render_tex_quads, Tex, TexQuad},
        text::TextHelper,
        Lighten, Theme,
    },
};

use super::EditorSquareUI;

#[derive(Clone)]
enum EditorDrag {
    MakeLand,
    RemoveLand,
    MakeTown(usize),
    RemoveTown(usize),
    MakeArtifact(usize),
    RemoveArtifact(usize),
}

pub struct EditorUI<'a> {
    board: &'a mut Board,
    mapped_board: &'a mut MappedBoard,
    editing_mode: &'a mut BoardEditingMode,
    player_colors: &'a Vec<Color32>,
}

impl<'a> EditorUI<'a> {
    pub fn new(
        board: &'a mut Board,
        mapped_board: &'a mut MappedBoard,
        editing_mode: &'a mut BoardEditingMode,
        player_colors: &'a Vec<Color32>,
    ) -> Self {
        Self {
            board,
            mapped_board,
            editing_mode,
            player_colors,
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
        let mut msg = None;

        let mut highlights = [None; 5];
        match self.editing_mode {
            BoardEditingMode::Land => highlights[0] = Some(theme.ring_selected),
            BoardEditingMode::Town(0) => highlights[1] = Some(theme.ring_selected),
            BoardEditingMode::Town(1) => highlights[2] = Some(theme.ring_selected),
            BoardEditingMode::Artifact(0) => highlights[3] = Some(theme.ring_selected),
            BoardEditingMode::Artifact(1) => highlights[4] = Some(theme.ring_selected),
            _ => unreachable!("Unknown board editing mode — player count has likely increased"),
        }

        let button_frame = egui::Frame::none().inner_margin(Margin::same(20.0));
        button_frame.show(ui, |ui| {
            ui.style_mut().spacing.item_spacing = Vec2::splat(6.0);

            let tiled_button = |quads: Vec<TexQuad>, ui: &mut egui::Ui| {
                let (mut rect, resp) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::click());
                if resp.hovered() {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                    rect = rect.translate(vec2(0.0, -2.0));
                }
                render_tex_quads(&quads, rect, map_texture, ui);
                resp
            };
            let pcol = |pnum: usize| self.player_colors.get(pnum).copied();

            let text = TextHelper::heavy("STOP EDITING BOARD", 10.0, None, ui);
            if text
                .button(theme.button_scary, theme.text, map_texture, ui)
                .clicked()
            {
                *self.editing_mode = BoardEditingMode::None;
            }

            let text = TextHelper::heavy("GROW BOARD", 10.0, None, ui);
            if text
                .button(Color32::WHITE, theme.text, map_texture, ui)
                .clicked()
            {
                self.board.grow();
                let aesthetics = AestheticDepot {
                    theme: theme.clone(),
                    qs_tick: 0,
                    map_texture: map_texture.clone(),
                    player_colors: self.player_colors.clone(),
                    destruction_tick: 0.0,
                    destruction_duration: 0.0,
                };
                self.mapped_board.remap_texture(
                    ui.ctx(),
                    &aesthetics,
                    &TimingDepot::default(),
                    None,
                    None,
                    &self.board,
                );
                msg = Some(PlayerMessage::EditBoard(self.board.clone()));
            }

            ui.label(RichText::new("Actions").color(Color32::WHITE));

            ui.add_space(28.0);

            ui.horizontal(|ui| {
                if tiled_button(Tex::artifact_button(pcol(0), highlights[3]), ui).clicked() {
                    *self.editing_mode = BoardEditingMode::Artifact(0);
                }

                if tiled_button(Tex::artifact_button(pcol(1), highlights[4]), ui).clicked() {
                    *self.editing_mode = BoardEditingMode::Artifact(1);
                }
            });
            ui.label(RichText::new("Artifact").color(Color32::WHITE));

            ui.add_space(28.0);

            ui.horizontal(|ui| {
                if tiled_button(Tex::town_button(pcol(0), highlights[1]), ui).clicked() {
                    *self.editing_mode = BoardEditingMode::Town(0);
                }

                if tiled_button(Tex::town_button(pcol(1), highlights[2]), ui).clicked() {
                    *self.editing_mode = BoardEditingMode::Town(1);
                }
            });
            ui.label(RichText::new("Towns").color(Color32::WHITE));

            if tiled_button(Tex::land_button(highlights[0]), ui).clicked() {
                *self.editing_mode = BoardEditingMode::Land;
            }
            ui.label(RichText::new("Land & Water").color(Color32::WHITE));
        });

        let styles = ui.style_mut();
        styles.spacing.item_spacing = egui::vec2(0.0, 0.0);
        styles.spacing.interact_size = egui::vec2(0.0, 0.0);

        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            let (_, margin, theme) = theme.calc_rescale(
                &ui.available_rect_before_wrap(),
                self.board.width(),
                self.board.height(),
                0.3..2.0,
                (2.0, 2.0),
            );
            let outer_frame = egui::Frame::none().inner_margin(margin);

            let mut modify_pos = None;
            outer_frame.show(ui, |ui| {
                let mut dest = Rect::from_min_size(
                    ui.next_widget_position(),
                    vec2(
                        self.board.width() as f32 * theme.grid_size,
                        self.board.height() as f32 * theme.grid_size,
                    ),
                );
                dest = dest.expand(theme.grid_size * self.mapped_board.buffer() as f32);
                self.mapped_board.render_to_rect(dest, None, ui);

                for (rownum, row) in self.board.squares.iter().enumerate() {
                    ui.horizontal(|ui| {
                        for (colnum, square) in row.iter().enumerate() {
                            let coord = Coordinate::new(colnum, rownum);
                            let editing_mode = self.editing_mode.clone();

                            let response = EditorSquareUI::new()
                                .square(square.clone())
                                .action(editing_mode.clone())
                                .render(ui, &theme, &map_texture);

                            if matches!(editing_mode, BoardEditingMode::None) {
                                continue;
                            }

                            if ui.rect_contains_pointer(response.rect) {
                                let drag_action = ui.memory(|mem| {
                                    if mem.is_anything_being_dragged() {
                                        mem.data.get_temp::<EditorDrag>(Id::NULL)
                                    } else {
                                        None
                                    }
                                });

                                if let Some(drag_action) = drag_action {
                                    match (drag_action, &square) {
                                        (
                                            EditorDrag::MakeLand,
                                            Square::Water { .. } | Square::Artifact { .. },
                                        ) => modify_pos = Some((coord, Square::land())),
                                        (
                                            EditorDrag::RemoveLand,
                                            Square::Land { .. } | Square::Town { .. },
                                        ) => modify_pos = Some((coord, Square::water())),
                                        (EditorDrag::MakeTown(player), _) => {
                                            modify_pos = Some((
                                                coord,
                                                Square::Town {
                                                    player,
                                                    defeated: false,
                                                    foggy: false,
                                                },
                                            ))
                                        }
                                        (
                                            EditorDrag::RemoveTown(player),
                                            Square::Town {
                                                player: sq_player, ..
                                            },
                                        ) if player == *sq_player => {
                                            modify_pos = Some((coord, Square::land()))
                                        }
                                        (EditorDrag::MakeArtifact(player), _) => {
                                            modify_pos = Some((coord, Square::artifact(player)))
                                        }
                                        (
                                            EditorDrag::RemoveArtifact(player),
                                            Square::Artifact {
                                                player: sq_player, ..
                                            },
                                        ) if player == *sq_player => {
                                            modify_pos = Some((coord, Square::water()))
                                        }
                                        (_, _) => {}
                                    }
                                }
                            }
                            if response.drag_started() {
                                ui.ctx().memory_mut(|mem| {
                                    mem.data.insert_temp(
                                        Id::NULL,
                                        match &self.editing_mode {
                                            BoardEditingMode::None => unreachable!(
                                            "With no board editing set we should not be editing"
                                        ),
                                            BoardEditingMode::Land => match square {
                                                Square::Water { .. } | Square::Artifact { .. } => {
                                                    EditorDrag::MakeLand
                                                }
                                                Square::Land { .. }
                                                | Square::Town { .. }
                                                | Square::Obelisk { .. } => EditorDrag::RemoveLand,
                                                Square::Occupied { .. } => unreachable!(),
                                                Square::Fog { .. } => unreachable!(),
                                            },
                                            BoardEditingMode::Town(editing_player) => {
                                                match square {
                                                    Square::Town {
                                                        player: sq_player, ..
                                                    } if sq_player == editing_player => {
                                                        EditorDrag::RemoveTown(*editing_player)
                                                    }
                                                    _ => EditorDrag::MakeTown(*editing_player),
                                                }
                                            }
                                            BoardEditingMode::Artifact(editing_player) => {
                                                match square {
                                                    Square::Artifact {
                                                        player: sq_player, ..
                                                    } if sq_player == editing_player => {
                                                        EditorDrag::RemoveArtifact(*editing_player)
                                                    }
                                                    _ => EditorDrag::MakeArtifact(*editing_player),
                                                }
                                            }
                                        },
                                    )
                                });
                            }
                            // Chain these next two together so that the drag end takes precedence,
                            // otherwise we double flip. Second branch remains to cover states without
                            // drag support, perhaps?
                            if response.drag_released() {
                                ui.ctx()
                                    .memory_mut(|mem| mem.data.remove::<EditorDrag>(Id::NULL));
                            } else if response.clicked() {
                                ui.ctx()
                                    .memory_mut(|mem| mem.data.remove::<EditorDrag>(Id::NULL));
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
                    let recip = self.board.reciprocal_coordinate(coord);

                    // TODO: Player mirroring won't work for >2 players
                    let mirrored_state = match new_state {
                        Square::Water { .. } | Square::Land { .. } | Square::Obelisk { .. } => {
                            new_state
                        }
                        Square::Town { player: p, .. } => {
                            if p == 0 {
                                Square::Town {
                                    player: 1,
                                    defeated: false,
                                    foggy: false,
                                }
                            } else {
                                Square::Town {
                                    player: 0,
                                    defeated: false,
                                    foggy: false,
                                }
                            }
                        }
                        Square::Artifact { player: p, .. } => {
                            if p == 0 {
                                Square::artifact(1)
                            } else {
                                Square::artifact(0)
                            }
                        }
                        Square::Occupied { .. } => {
                            unreachable!("Board editor should not contain occupied tiles")
                        }
                        Square::Fog { .. } => {
                            unreachable!("Board editor should not contain fog")
                        }
                    };

                    self.board.squares[recip.y][recip.x] = mirrored_state;
                }

                edited = true;
            }
        });

        if edited {
            Some(PlayerMessage::EditBoard(self.board.clone()))
        } else {
            msg
        }
    }
}
