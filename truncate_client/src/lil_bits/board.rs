use epaint::{emath::Align2, pos2, vec2, Rect, Vec2};
use instant::Duration;
use truncate_core::{
    board::{Board, Coordinate, Direction, Square},
    messages::PlayerMessage,
    player::Hand,
    reporting::BoardChange,
};

use eframe::egui::{self, Id, Order, Sense};
use hashbrown::HashMap;

use crate::utils::{
    depot::TruncateDepot,
    mapper::{MappedBoard, MappedTile, MappedTileVariant, MappedTiles},
};

pub struct BoardUI<'a> {
    board: &'a Board,
    interactive: bool,
}

impl<'a> BoardUI<'a> {
    pub fn new(board: &'a Board) -> Self {
        Self {
            board,
            interactive: true,
        }
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
}

impl<'a> BoardUI<'a> {
    // TODO: Refactor board
    pub fn render(
        self,
        hand: &Hand,
        _board_changes: &HashMap<Coordinate, BoardChange>,
        ui: &mut egui::Ui,
        mapped_board: &mut MappedBoard,
        mapped_overlay: &mut MappedTiles,
        depot: &mut TruncateDepot,
    ) -> Option<PlayerMessage> {
        let mut msg = None;
        let mut unoccupied_square_is_hovered = None;
        let mut occupied_square_is_hovered = None;
        let mut tile_is_hovered = None;
        let mut drag_underway = false;

        // TODO: Do something better for this
        let invert = depot.gameplay.player_number == 0;

        let game_area = ui.available_rect_before_wrap();
        ui.set_clip_rect(game_area);

        let ((resolved_board_width, resolved_board_height), _, theme) =
            depot.aesthetics.theme.calc_rescale(
                &game_area,
                self.board.width(),
                self.board.height(),
                0.05..2.0,
                (0, 0),
            );
        let theme = theme.rescale(depot.board_info.board_zoom);
        let outer_frame = egui::Frame::none().inner_margin(0.0);

        if !depot.board_info.board_moved {
            let panx = (game_area.width() - resolved_board_width) / 2.0 + game_area.left();
            let pany = (game_area.height() - resolved_board_height) / 2.0 + game_area.top();
            depot.board_info.board_pan = vec2(panx, pany);
        }

        // TODO: Remove this hack, which is currently non-destructive place as the board is the last thing we render.
        // We instead need a way to create a GameCtx scoped to a different theme (or go back to drilling Theme objects down through funcs).
        let prev_theme = depot.aesthetics.theme.clone();
        depot.aesthetics.theme = theme;

        let area = egui::Area::new(egui::Id::new("board_layer"))
            .movable(false)
            .order(Order::Background)
            .anchor(Align2::LEFT_TOP, depot.board_info.board_pan);
        let area_id = area.layer();

        let mut drag_pos = None;
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            // When dragging on a touchscreen, we show the tile above the pointer position.
            // We avoid doing this when not dragging, as otherwise taps show tiles in the wrong position.
            let mid_drag = ui.input(|i| i.pointer.is_decidedly_dragging());
            let drag_offset = if depot.ui_state.is_touch && mid_drag {
                -50.0
            } else {
                0.0
            };
            drag_pos = Some(pointer_pos + vec2(0.0, drag_offset));
        }

        let board_frame = area
            .show(ui.ctx(), |ui| {
                let styles = ui.style_mut();
                styles.spacing.item_spacing = egui::vec2(0.0, 0.0);
                styles.spacing.interact_size = egui::vec2(0.0, 0.0);

                outer_frame.show(ui, |ui| {
                    let board_texture_dest = Rect::from_min_size(
                        ui.next_widget_position(),
                        vec2(
                            self.board.width() as f32 * depot.aesthetics.theme.grid_size,
                            self.board.height() as f32 * depot.aesthetics.theme.grid_size,
                        ),
                    );

                    let mut render = |rows: Box<dyn Iterator<Item = (usize, &Vec<Square>)>>| {
                        let mut render_row =
                            |rownum, row: Box<dyn Iterator<Item = (usize, &Square)>>| {
                                ui.horizontal(|ui| {
                                    for (colnum, square) in row {
                                        let (grid_cell, square_response) = ui.allocate_exact_size(
                                            Vec2::splat(depot.aesthetics.theme.grid_size),
                                            Sense::click(),
                                        );
                                        // An extra row seems to be clipped off the bottom in a normal calculation,
                                        // so we expand each grid cell's check (painting one cell "outside" the screen)
                                        if !ui.is_rect_visible(
                                            grid_cell.expand(depot.aesthetics.theme.grid_size),
                                        ) {
                                            // Skip all work for board parts that are offscreen.
                                            continue;
                                        }
                                        if !self.interactive {
                                            continue;
                                        }

                                        let coord = Coordinate::new(colnum, rownum);

                                        let TruncateDepot {
                                            aesthetics,
                                            interactions,
                                            gameplay,
                                            ..
                                        } = depot;

                                        if matches!(square, Square::Land) {
                                            if let Some(drag_pos) = drag_pos {
                                                if grid_cell.contains(drag_pos) {
                                                    unoccupied_square_is_hovered =
                                                        Some(crate::utils::depot::HoveredRegion {
                                                            rect: grid_cell,
                                                            coord: Some(coord),
                                                            square: Some(*square),
                                                        });
                                                }
                                            }

                                            if square_response.clicked() {
                                                if let Some((tile, _)) =
                                                    interactions.selected_tile_in_hand
                                                {
                                                    msg = Some(PlayerMessage::Place(
                                                        coord,
                                                        *hand.get(tile).unwrap(),
                                                    ));

                                                    interactions.selected_tile_in_hand = None;
                                                }

                                                if interactions.selected_tile_on_board.is_some() {
                                                    interactions.selected_tile_on_board = None;
                                                }
                                            }

                                            if let Some(tile) = interactions.released_tile {
                                                if tile.1 == coord {
                                                    msg = Some(PlayerMessage::Place(
                                                        coord,
                                                        *hand.get(tile.0).unwrap(),
                                                    ));
                                                    interactions.selected_tile_in_hand = None;
                                                    interactions.selected_tile_on_board = None;
                                                    interactions.released_tile = None;
                                                }
                                            }
                                        } else if let Square::Occupied(square_player, square_char) =
                                            square
                                        {
                                            let tile_rect = grid_cell.shrink(2.0);
                                            let tile_id =
                                                Id::new("board_tile").with(coord).with("dragging");
                                            let tile_response = ui.interact(
                                                tile_rect,
                                                tile_id,
                                                Sense::click_and_drag(),
                                            );

                                            if let Some(drag_pos) = drag_pos {
                                                if grid_cell.contains(drag_pos) {
                                                    occupied_square_is_hovered =
                                                        Some(crate::utils::depot::HoveredRegion {
                                                            rect: grid_cell,
                                                            coord: Some(coord),
                                                            square: Some(*square),
                                                        });
                                                }
                                            }

                                            if tile_response.hovered() {
                                                tile_is_hovered = Some((coord, *square));
                                                ui.output_mut(|o| {
                                                    o.cursor_icon = egui::CursorIcon::PointingHand
                                                });
                                            }
                                            if tile_response.clicked() {
                                                if matches!(
                                                    interactions.selected_tile_on_board,
                                                    Some((c, _)) if c == coord
                                                ) {
                                                    interactions.selected_tile_on_board = None;
                                                } else if let Some((selected_coord, _)) =
                                                    interactions.selected_tile_on_board
                                                {
                                                    msg = Some(PlayerMessage::Swap(
                                                        coord,
                                                        selected_coord,
                                                    ));
                                                    interactions.selected_tile_on_board = None;
                                                    interactions.selected_tile_in_hand = None;
                                                } else {
                                                    interactions.selected_tile_on_board =
                                                        Some((coord, *square));
                                                }
                                            }
                                            let mut is_being_dragged =
                                                ui.memory(|mem| mem.is_being_dragged(tile_id));
                                            let is_decidedly_dragging = ui
                                                .ctx()
                                                .input(|inp| inp.pointer.is_decidedly_dragging());

                                            // Bail out of a drag if we're not "decidedly dragging",
                                            // as this could instead be just a click.
                                            if is_being_dragged && !is_decidedly_dragging {
                                                is_being_dragged = false;
                                            }

                                            if tile_response.drag_started() {
                                                if let Some(pointer_pos) =
                                                    ui.ctx().pointer_interact_pos()
                                                {
                                                    let delta =
                                                        pointer_pos - tile_response.rect.center();
                                                    ui.memory_mut(|mem| {
                                                        mem.data.insert_temp(tile_id, delta);
                                                        mem.data.insert_temp(
                                                            tile_id,
                                                            depot.timing.current_time,
                                                        );
                                                    });
                                                }

                                                ui.ctx().animate_value_with_time(
                                                    tile_id.with("initial_offset"),
                                                    0.0,
                                                    0.0,
                                                );

                                                // Map out a texture for this tile that we can use to drag it around.
                                                // (we can't use the board itself, as this tile might be
                                                //  changed out to preview a swap)
                                                mapped_overlay.remap_texture(
                                                    ui.ctx(),
                                                    vec![MappedTile {
                                                        variant: MappedTileVariant::Healthy,
                                                        character: *square_char,
                                                        color: Some(
                                                            aesthetics.theme.ring_selected_hovered,
                                                        ),
                                                        highlight: None,
                                                        orientation: if *square_player
                                                            == gameplay.player_number as usize
                                                        {
                                                            Direction::North
                                                        } else {
                                                            Direction::South
                                                        },
                                                    }],
                                                    aesthetics,
                                                    Some(interactions),
                                                );
                                            } else if tile_response.drag_released()
                                                && is_decidedly_dragging
                                            {
                                                let drop_coord = interactions
                                                    .hovered_occupied_square_on_board
                                                    .as_ref()
                                                    .map(|region| region.coord)
                                                    .flatten();

                                                if let Some(drop_coord) = drop_coord {
                                                    msg = Some(PlayerMessage::Swap(
                                                        coord, drop_coord,
                                                    ));
                                                    interactions.selected_tile_on_board = None;
                                                    interactions.selected_tile_in_hand = None;
                                                }
                                            }

                                            if is_being_dragged {
                                                drag_underway = true;
                                                interactions.dragging_tile_on_board =
                                                    Some((coord, *square));

                                                let drag_id: Duration = ui
                                                    .memory(|mem| mem.data.get_temp(tile_id))
                                                    .unwrap_or_default();

                                                let area = egui::Area::new(
                                                    tile_id.with("floating").with(drag_id),
                                                )
                                                .movable(false)
                                                .order(Order::Tooltip)
                                                .anchor(Align2::LEFT_TOP, vec2(0.0, 0.0));

                                                area.show(ui.ctx(), |ui| {
                                                    let snap_to_rect = depot
                                                        .interactions
                                                        .hovered_occupied_square_on_board
                                                        .as_ref()
                                                        .map(|region| region.rect);

                                                    let position = if let Some(snap) = snap_to_rect
                                                    {
                                                        snap.left_top()
                                                    } else if let Some(pointer_pos) =
                                                        ui.ctx().pointer_interact_pos()
                                                    {
                                                        let drag_offset = if depot.ui_state.is_touch
                                                        {
                                                            -50.0
                                                        } else {
                                                            0.0
                                                        };
                                                        let bounce_offset =
                                                            ui.ctx().animate_value_with_time(
                                                                tile_id.with("initial_offset"),
                                                                drag_offset,
                                                                depot
                                                                    .aesthetics
                                                                    .theme
                                                                    .animation_time,
                                                            );
                                                        let original_delta: Vec2 =
                                                            ui.memory(|mem| {
                                                                mem.data
                                                                    .get_temp(tile_id)
                                                                    .unwrap_or_default()
                                                            });
                                                        pointer_pos + vec2(0.0, bounce_offset)
                                                            - original_delta
                                                            - (grid_cell.size() / 2.0)
                                                    } else {
                                                        tile_rect.left_top()
                                                    };

                                                    let animated_position = pos2(
                                                        ui.ctx().animate_value_with_time(
                                                            area.layer().id.with("delta_x"),
                                                            position.x,
                                                            depot.aesthetics.theme.animation_time,
                                                        ),
                                                        ui.ctx().animate_value_with_time(
                                                            area.layer().id.with("delta_y"),
                                                            position.y,
                                                            depot.aesthetics.theme.animation_time,
                                                        ),
                                                    );

                                                    mapped_overlay.render_tile_to_rect(
                                                        0,
                                                        Rect::from_min_size(
                                                            animated_position,
                                                            grid_cell.size(),
                                                        ),
                                                        ui,
                                                    );
                                                })
                                                .response;

                                                ui.ctx().output_mut(|out| {
                                                    out.cursor_icon = egui::CursorIcon::Grabbing
                                                });
                                            }
                                        }
                                    }
                                });
                            };

                        for (rownum, row) in rows {
                            if invert {
                                render_row(rownum, Box::new(row.iter().enumerate().rev()));
                            } else {
                                render_row(rownum, Box::new(row.iter().enumerate()));
                            }
                        }
                    };
                    if invert {
                        render(Box::new(self.board.squares.iter().enumerate().rev()));
                    } else {
                        render(Box::new(self.board.squares.iter().enumerate()));
                    }

                    depot.interactions.hovered_unoccupied_square_on_board =
                        unoccupied_square_is_hovered;
                    depot.interactions.hovered_occupied_square_on_board =
                        occupied_square_is_hovered;
                    depot.interactions.hovered_tile_on_board = tile_is_hovered;

                    mapped_board.remap_texture(
                        ui.ctx(),
                        &depot.aesthetics,
                        &depot.timing,
                        Some(&depot.interactions),
                        Some(&depot.gameplay),
                        self.board,
                    );
                    mapped_board.render_to_rect(board_texture_dest, ui);
                })
            })
            .inner;

        if !drag_underway {
            depot.interactions.dragging_tile_on_board = None;
        }

        if !self.interactive {
            return None;
        }

        let mut board_pos = board_frame.response.rect.clone();

        // Move the drag focus to our board layer if it looks like a drag is starting.
        // NB: This is sensitive to the board being painted _last_ on our screen,
        // such that anything else that should be getting the drag this frame will already
        // exist in the `is_anything_being_dragged` check.
        // (The `layer_id_at` check should avoid this issue in most cases, I imagine)
        if let Some(pos) = ui.input(|i| {
            if i.pointer.any_down() {
                i.pointer.interact_pos()
            } else {
                None
            }
        }) {
            if board_frame.response.contains_pointer()
                && !ui.memory(|mem| mem.is_anything_being_dragged())
            {
                if ui.ctx().layer_id_at(pos) == Some(area_id) {
                    ui.memory_mut(|mem| mem.set_dragged_id(area_id.id))
                }
            }
        }

        // Global(ish) interactions
        if let Some(hover_pos) = ui.ctx().pointer_hover_pos() {
            let zoom_delta = ui.input(|i| i.zoom_delta());
            let scroll_delta = ui.input(|i| i.raw_scroll_delta);

            let maybe_zooming = zoom_delta != 1.0;
            let maybe_panning = scroll_delta != Vec2::ZERO;

            let capture_action = (maybe_zooming || maybe_panning)
                && match ui.ctx().layer_id_at(hover_pos) {
                    // No layer, probably fine 🤷
                    None => true,
                    // Board layer, definitely ours
                    Some(layer) if layer == area_id => true,
                    // A background layer _should_ be the window itself,
                    // and thus the ocean. We'll handle this input.
                    Some(layer) if layer.order == Order::Background => true,
                    // Gesturing over something else, maybe scrolling a dialog.
                    // Cancel handling this input.
                    Some(_) => false,
                };

            if capture_action {
                // --- Zooming ---
                if zoom_delta != 1.0 {
                    depot.board_info.board_moved = true;

                    depot.board_info.board_zoom *= zoom_delta;
                    let diff = board_pos.size() - board_pos.size() * zoom_delta;
                    board_pos.set_right(board_pos.right() - diff.x);
                    board_pos.set_bottom(board_pos.bottom() - diff.y);

                    // Center the zoom around the cursor
                    let pointer_delta = hover_pos - depot.board_info.board_pan;
                    let zoom_diff = zoom_delta - 1.0;
                    let zoom_pan_delta =
                        pos2(pointer_delta.x * zoom_diff, pointer_delta.y * zoom_diff);
                    depot.board_info.board_pan -= zoom_pan_delta.to_vec2();
                    board_pos = board_pos.translate(-zoom_pan_delta.to_vec2());
                }
                // --- Panning ---
                if scroll_delta != Vec2::ZERO {
                    depot.board_info.board_moved = true;

                    depot.board_info.board_pan += scroll_delta;
                    board_pos = board_pos.translate(scroll_delta);
                }
            }
        }

        // Handle the drag focus in all cases
        // (in case the pointer is now over something else but we are still dragging)
        // (egui handles releasing this drag state when a pointer is up)
        if ui.memory(|mem| mem.is_being_dragged(area_id.id)) {
            let pointer_delta = ui.ctx().input(|i| i.pointer.delta());
            depot.board_info.board_pan += pointer_delta;
            depot.board_info.board_moved = true;
            board_pos = board_pos.translate(pointer_delta);
        }

        if let Some(touch) = ui.input(|i| i.multi_touch()) {
            let capture_action = match ui.ctx().layer_id_at(touch.start_pos) {
                // No layer, probably fine 🤷
                None => true,
                // Board layer, definitely ours
                Some(layer) if layer == area_id => true,
                // A background layer _should_ be the window itself,
                // and thus the ocean. We'll handle this input.
                Some(layer) if layer.order == Order::Background => true,
                // Gesturing over something else, maybe scrolling a dialog.
                // Cancel handling this input.
                Some(_) => false,
            };

            if capture_action {
                depot.board_info.board_zoom *= (touch.zoom_delta - 1.0) * 0.25 + 1.0;
                depot.board_info.board_pan += touch.translation_delta;
                depot.board_info.board_moved = true;
                board_pos = board_pos.translate(touch.translation_delta);
            }
        }

        let buffer = game_area.size() * 0.25;
        let bounce = 0.3;

        let left_overage = game_area.left() - board_pos.right() + buffer.x;
        let right_overage = board_pos.left() - game_area.right() + buffer.x;
        let top_overage = game_area.top() - board_pos.bottom() + buffer.y;
        let bottom_overage = board_pos.top() - game_area.bottom() + buffer.y;

        let mut bounced = false;
        if left_overage > 0.0 {
            depot.board_info.board_pan.x += (left_overage * bounce).max(bounce).min(left_overage);
            bounced = true;
        }
        if right_overage > 0.0 {
            depot.board_info.board_pan.x -= (right_overage * bounce).max(bounce).min(right_overage);
            bounced = true;
        }
        if top_overage > 0.0 {
            depot.board_info.board_pan.y += (top_overage * bounce).max(bounce).min(top_overage);
            bounced = true;
        }
        if bottom_overage > 0.0 {
            depot.board_info.board_pan.y -=
                (bottom_overage * bounce).max(bounce).min(bottom_overage);
            bounced = true;
        }
        if bounced {
            // Paint at a high FPS while animating the board back to stasis.
            ui.ctx().request_repaint();
        }

        // let resolved_x = (self.board.width() * aesthetics.theme.grid_size * ctx.board_zoom) ctx.board_pan

        depot.aesthetics.theme = prev_theme;

        msg
    }
}
