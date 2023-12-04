use epaint::{hex_color, emath::Align2, vec2, Vec2, pos2, Rect};
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
    player::Hand,
    reporting::BoardChange,
};

use eframe::egui::{self, LayerId, Order};
use hashbrown::HashMap;

use crate::{utils::mapper::MappedBoard, regions::active_game::{HoveredRegion, GameCtx}};

use super::{
    tile::TilePlayer,
    SquareUI, TileUI,
};

pub struct BoardUI<'a> {
    board: &'a Board,
    interactive: bool
}

impl<'a> BoardUI<'a> {
    pub fn new(board: &'a Board) -> Self {
        Self { board, interactive: true }
    }
    
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
}

impl<'a> BoardUI<'a> {
    // TODO: Refactor board to maybe own nothing and pass the whole
    // game object through, since we touch so much of it.
    pub fn render(
        self,
        hand: &Hand,
        board_changes: &HashMap<Coordinate, BoardChange>,
        winner: Option<usize>,
        ctx: &mut GameCtx,
        ui: &mut egui::Ui,
        mapped_board: &MappedBoard,
    ) -> Option<PlayerMessage> {
        let mut msg = None;
        let mut next_selection = None;
        let mut hovered_square = None;

        // TODO: Do something better for this
        let invert = ctx.player_number == 0;

        let game_area = ui.available_rect_before_wrap();
        ui.set_clip_rect(game_area);

        let ((resolved_board_width, resolved_board_height), _, theme) = ctx.theme.calc_rescale(
            &game_area, 
            self.board.width(),
            self.board.height(),
            0.4..2.0,
            (0, 0)
        );
        let theme = theme.rescale(ctx.board_zoom);
        let outer_frame = egui::Frame::none().inner_margin(0.0);

        if !ctx.board_moved {
            let panx = (game_area.width() - resolved_board_width) / 2.0 + game_area.left();
            let pany = (game_area.height() - resolved_board_height) / 2.0 + game_area.top();
            ctx.board_pan = vec2(panx, pany);
        }

        // TODO: Remove this hack, which is currently non-destructive place as the board is the last thing we render.
        // We instead need a way to create a GameCtx scoped to a different theme (or go back to drilling Theme objects down through funcs).
        let prev_theme = ctx.theme.clone();
        ctx.theme = theme;

        let area = egui::Area::new(egui::Id::new("board_layer"))
            .movable(false)
            .order(Order::Background)
            .anchor(Align2::LEFT_TOP, ctx.board_pan);
        let area_id = area.layer();

        let board_frame = area.show(ui.ctx(), |ui| {
            let styles = ui.style_mut();
            styles.spacing.item_spacing = egui::vec2(0.0, 0.0);
            styles.spacing.interact_size = egui::vec2(0.0, 0.0);

            outer_frame.show(ui, |ui| {
                let mut render = |rows: Box<dyn Iterator<Item = (usize, &Vec<Square>)>>| {
                    let mut render_row =
                        |rownum, row: Box<dyn Iterator<Item = (usize, &Square)>>| {
                            ui.horizontal(|ui| {
                                for (colnum, square) in row {
                                    let grid_cell = Rect::from_min_size(ui.next_widget_position(), Vec2::splat(ctx.theme.grid_size));
                                    // An extra row seems to be clipped off the bottom in a normal calculation,
                                    // so we expand each grid cell's check (painting one cell "outside" the screen)
                                    if !ui.is_rect_visible(grid_cell.expand(ctx.theme.grid_size)) {
                                        // Skip all work for board that is offscreen, just move the cursor.
                                        _ = ui.allocate_exact_size(
                                            Vec2::splat(ctx.theme.grid_size),
                                            egui::Sense::hover(),
                                        );
                                        continue;
                                    }

                                    let coord = Coordinate::new(colnum, rownum);
                                    let is_selected = Some(coord) == ctx.selected_square_on_board;
                                    let calc_tile_player = |p: &usize| {
                                        if *p as u64 == ctx.player_number {
                                            TilePlayer::Own
                                        } else {
                                            TilePlayer::Enemy(*p as usize)
                                        }
                                    };


                                    let mut tile = if let Square::Occupied(player, char) = square {
                                        let is_winner = winner == Some(*player);
                                        Some(
                                            TileUI::new(*char, calc_tile_player(player)).selected(is_selected).won(is_winner)
                                        )
                                    } else {
                                        None
                                    };

                                    if let Some(change) = board_changes.get(&coord) {
                                        use Square::*;
                                        use truncate_core::reporting::BoardChangeAction;
                                        tile = match (&change.action, tile) {
                                            (BoardChangeAction::Added, Some(tile)) => Some(tile.added(true)),
                                            (BoardChangeAction::Swapped, Some(tile)) => Some(tile.modified(true)),
                                            (BoardChangeAction::Defeated, None) => 
                                                match change.detail.square {
                                                    Water | Land | Fog | Town{..} | Dock(_) => None,
                                                    Occupied(player, char) => Some((player, char)),
                                                }
                                                .map(
                                                    |(player, char)| {
                                                        TileUI::new(char, calc_tile_player(&player))
                                                            .selected(is_selected)
                                                            .defeated(true)
                                                    },
                                                ),
                                            (BoardChangeAction::Truncated, None) => 
                                                match change.detail.square {
                                                    Water | Land | Fog | Town{..} | Dock(_) => None,
                                                    Occupied(player, char) => Some((player, char)),
                                                }
                                                .map(
                                                    |(player, char)| {
                                                        TileUI::new(char, calc_tile_player(&player))
                                                            .selected(is_selected)
                                                            .truncated(true)
                                                    },
                                                ),
                                            (BoardChangeAction::Exploded, None) =>
                                                match change.detail.square {
                                                    Water | Land | Fog | Town{..} | Dock(_) => None,
                                                    Occupied(player, char) => Some((player, char)),
                                                }
                                                .map(
                                                    |(player, char)| {
                                                        TileUI::new(char, calc_tile_player(&player))
                                                            .selected(is_selected)
                                                            .defeated(true)
                                                    },
                                                ),
                                            (BoardChangeAction::Victorious, Some(tile)) => Some(tile.victor(true)),
                                            (BoardChangeAction::Victorious, None) =>
                                                match change.detail.square {
                                                    Water | Land | Fog | Town{..} | Dock(_) => None,
                                                    Occupied(player, char) => Some((player, char)),
                                                }
                                                .map(
                                                    |(player, char)| {
                                                        TileUI::new(char, calc_tile_player(&player))
                                                            .selected(is_selected)
                                                            .victor(true)
                                                    },
                                                ),
                                            _ => {
                                                eprintln!("Board message received that seems incompatible with the board");
                                                eprintln!("{change}");
                                                eprintln!("{}", self.board);
                                                None
                                            }
                                        }
                                    }

                                    let mut overlay = None;
                                    if let Some(placing_tile) = ctx.selected_tile_in_hand {
                                        if matches!(square, Square::Land) {
                                            overlay = Some(*hand.get(placing_tile).unwrap());
                                        }
                                    } else if let Some(placing_tile) = ctx.selected_square_on_board { // TODO: De-nest
                                        if placing_tile != coord {
                                            if let Square::Occupied(p, _) = square {
                                                if p == &(ctx.player_number as usize) {
                                                    if let Ok(Square::Occupied(_, char)) = self.board.get(placing_tile) {
                                                        overlay = Some(char);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if let Some(squares) = ctx.highlight_squares.as_ref() {
                                        if squares.contains(&coord) && ctx.current_time.subsec_millis() > 500 {
                                            tile = tile.map(|t| t.highlighted(true));
                                        }
                                    }

                                    // TODO: Devise a way to show this tile in the place of the board_selected_tile

                                    let (square_response, outer_rect) = SquareUI::new(coord)
                                        .enabled(matches!(square, Square::Land | Square::Occupied(_, _)))
                                        .empty(matches!(square, Square::Land))
                                        .selected(is_selected)
                                        .overlay(overlay)
                                        .render(ui, ctx, &mapped_board, |ui, ctx| {
                                            if let Some(tile) = tile {
                                                tile.render(Some(coord), ui, ctx, false, None);
                                            }
                                        });
                                    
                                    if !self.interactive {
                                        continue;
                                    }

                                    if matches!(square, Square::Land | Square::Occupied(_, _)) {
                                        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                            let drag_offset = if ctx.is_touch { -50.0 } else { 0.0 };
                                            let drag_pos = pointer_pos + vec2(0.0, drag_offset);

                                            if outer_rect.contains(drag_pos) {
                                                hovered_square = Some(HoveredRegion{
                                                    rect: outer_rect,
                                                    coord: Some(coord)
                                                });
                                            }
                                        }


                                        if square_response.clicked() {
                                            if let Some(tile) = ctx.selected_tile_in_hand {
                                                msg =
                                                    Some(PlayerMessage::Place(coord, *hand.get(tile).unwrap()));
                                                next_selection = Some(None);
                                            } else if is_selected {
                                                next_selection = Some(None);
                                            } else if let Some(selected_coord) = ctx.selected_square_on_board {
                                                // Only try to swap onto a tile, otherwise just deselect
                                                if matches!(square, Square::Occupied(_, _)) {
                                                    msg = Some(PlayerMessage::Swap(coord, selected_coord));
                                                }
                                                next_selection = Some(None);
                                            } else {
                                                // Don't select coordinates that are empty
                                                if matches!(square, Square::Occupied(_, _)) {
                                                    next_selection = Some(Some(coord));
                                                }
                                            }
                                        } else if let Some(tile) = ctx.released_tile {
                                            if tile.1 == coord {
                                                msg = Some(PlayerMessage::Place(coord, *hand.get(tile.0).unwrap()));
                                                next_selection = Some(None);
                                                ctx.released_tile = None;
                                            }
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
            })
        })
        .inner;

        if !self.interactive {
            return None;
        }

        let mut board_pos = board_frame.response.rect.clone();
        let previous_state = (ctx.board_zoom, ctx.board_pan);

        if let Some(hover_pos) = board_frame.response.hover_pos() {
            // Move the drag focus to our board layer if it looks like a drag is starting.
            // NB: This is sensitive to the board being painted _last_ on our screen,
            // such that anything else that should be getting the drag this frame will already
            // exist in the `is_anything_being_dragged` check.
            // (The `layer_id_at` check should avoid this issue in most cases, I imagine)
            if ui.input(|i| i.pointer.any_down() 
            && i.pointer.any_pressed()) 
            && !ui.memory(|mem| mem.is_anything_being_dragged()) {

                if ui.ctx().layer_id_at(hover_pos) == Some(area_id) {
                    ui.memory_mut(|mem| mem.set_dragged_id(area_id.id))
                }
            }
        }

        // Global(ish) interactions
        if let Some(hover_pos) = ui.ctx().pointer_hover_pos() {
            let zoom_delta = ui.input(|i| i.zoom_delta());
            let scroll_delta = ui.input(|i| i.scroll_delta);

            let maybe_zooming = zoom_delta != 1.0;
            let maybe_panning = scroll_delta != Vec2::ZERO;

            let capture_action = maybe_zooming || maybe_panning 
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
                Some(_) => false
            };

            if capture_action {
                // --- Zooming ---
                if zoom_delta != 1.0 {
                    ctx.board_moved = true;
                    
                    ctx.board_zoom *= zoom_delta;
                    let diff = board_pos.size() - board_pos.size() * zoom_delta;
                    board_pos.set_right(board_pos.right() - diff.x);
                    board_pos.set_bottom(board_pos.bottom() - diff.y);

                    // Center the zoom around the cursor
                    let pointer_delta = hover_pos - ctx.board_pan;
                    let zoom_diff = zoom_delta - 1.0;
                    let zoom_pan_delta = pos2(pointer_delta.x * zoom_diff, pointer_delta.y * zoom_diff);
                    ctx.board_pan -= zoom_pan_delta.to_vec2();
                    board_pos = board_pos.translate(-zoom_pan_delta.to_vec2());
                }
                // --- Panning ---
                if scroll_delta != Vec2::ZERO {
                    ctx.board_moved = true;

                    ctx.board_pan += scroll_delta;
                    board_pos = board_pos.translate(scroll_delta);
                }
            }
        }

        // Handle the drag focus in all cases
        // (in case the pointer is now over something else but we are still dragging)
        // (egui handles releasing this drag state when a pointer is up)
        if ui.memory(|mem| mem.is_being_dragged(area_id.id)) {
            let pointer_delta = ui.ctx().input(|i| i.pointer.delta());
            ctx.board_pan += pointer_delta;
            ctx.board_moved = true;
            board_pos = board_pos.translate(pointer_delta);
        }

        // TODO: This is capturing gestures everywhere
        if let Some(touch) = ui.input(|i| i.multi_touch()) {
            ctx.board_zoom *= (touch.zoom_delta - 1.0) * 0.25 + 1.0;
            ctx.board_pan += touch.translation_delta;
            ctx.board_moved = true;
            board_pos = board_pos.translate(touch.translation_delta);
        }


        let buffer = game_area.size() * 0.25;
        let bounce = 0.3;

        let left_overage = game_area.left() - board_pos.right() + buffer.x;
        let right_overage = board_pos.left() - game_area.right() + buffer.x;
        let top_overage = game_area.top() - board_pos.bottom() + buffer.y;
        let bottom_overage = board_pos.top() - game_area.bottom() + buffer.y;

        let mut bounced = false;
        if left_overage > 0.0 {
            ctx.board_pan.x += (left_overage * bounce).max(bounce).min(left_overage);
            bounced = true;
        }
        if right_overage > 0.0 {
            ctx.board_pan.x -= (right_overage * bounce).max(bounce).min(right_overage);
            bounced = true;
        }
        if top_overage > 0.0 {
            ctx.board_pan.y += (top_overage * bounce).max(bounce).min(top_overage);
            bounced = true;
        }
        if bottom_overage > 0.0 {
            ctx.board_pan.y -= (bottom_overage * bounce).max(bounce).min(bottom_overage);
            bounced = true;
        }
        if bounced {
            // Paint at a high FPS while animating the board back to stasis.
            ui.ctx().request_repaint();
        }

        // let resolved_x = (self.board.width() * ctx.theme.grid_size * ctx.board_zoom) ctx.board_pan

        if let Some(new_selection) = next_selection {
            ctx.selected_square_on_board = new_selection;
            ctx.selected_tile_in_hand = None;
        }

        if hovered_square != ctx.hovered_tile_on_board {
            ctx.hovered_tile_on_board = hovered_square;
        }

        ctx.theme = prev_theme;

        msg
    }
}
