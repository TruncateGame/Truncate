use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
    player::Hand,
    reporting::BoardChange,
};

use eframe::egui;
use hashbrown::HashMap;

use crate::{theming::mapper::MappedBoard, active_game::{HoveredRegion, GameCtx}};

use super::{
    tile::TilePlayer,
    SquareUI, TileUI,
};

pub struct BoardUI<'a> {
    board: &'a Board,
}

impl<'a> BoardUI<'a> {
    pub fn new(board: &'a Board) -> Self {
        Self { board }
    }
}

impl<'a> BoardUI<'a> {
    // TODO: Refactor board to maybe own nothing and pass the whole
    // game object through, since we touch so much of it.
    pub fn render(
        self,
        hand_released_tile: Option<usize>,
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

        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        let game_area = ui.available_rect_before_wrap();

        let (margin, theme) = ctx.theme.calc_rescale(
            &game_area, 
            self.board.width(),
            self.board.height(),
            0.4..2.0
        );
        let outer_frame = egui::Frame::none().inner_margin(margin);

        // TODO: Remove this hack, which is currently non-destructive place as the board is the last thing we render.
        // We instead need a way to create a GameCtx scoped to a different theme (or go back to drilling Theme objects down through funcs).
        ctx.theme = theme;

        outer_frame.show(ui, |ui| {
            let mut render = |rows: Box<dyn Iterator<Item = (usize, &Vec<Square>)>>| {
                let mut render_row =
                    |rownum, row: Box<dyn Iterator<Item = (usize, &Square)>>| {
                        ui.horizontal(|ui| {
                            for (colnum, square) in row {
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
                                                Water | Land | Town(_) | Dock(_) => None,
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
                                                Water | Land | Town(_) | Dock(_) => None,
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
                                                Water | Land | Town(_) | Dock(_) => None,
                                                Occupied(player, char) => Some((player, char)),
                                            }
                                            .map(
                                                |(player, char)| {
                                                    TileUI::new(char, calc_tile_player(&player))
                                                        .selected(is_selected)
                                                        .defeated(true)
                                                },
                                            ),
                                        (BoardChangeAction::Victorious, Some(tile)) => Some(tile.won(true)),
                                        (BoardChangeAction::Victorious, None) =>
                                            match change.detail.square {
                                                Water | Land | Town(_) | Dock(_) => None,
                                                Occupied(player, char) => Some((player, char)),
                                            }
                                            .map(
                                                |(player, char)| {
                                                    TileUI::new(char, calc_tile_player(&player))
                                                        .selected(is_selected)
                                                        .won(true)
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
                                // TODO: Devise a way to show this tile in the place of the board_selected_tile

                                let mut tile_clicked = false;
                                let (square_response, outer_rect) = SquareUI::new(coord)
                                    .enabled(matches!(square, Square::Land | Square::Occupied(_, _)))
                                    .empty(matches!(square, Square::Land))
                                    .selected(is_selected)
                                    .overlay(overlay)
                                    .render(ui, ctx, &mapped_board, |ui, ctx| {
                                        if let Some(tile) = tile {
                                            tile_clicked = tile.render(Some(coord), ui, ctx, None).clicked();
                                        }
                                    });
                                if matches!(square, Square::Land | Square::Occupied(_, _)) {
                                    if ui.rect_contains_pointer(outer_rect) {
                                        hovered_square = Some(HoveredRegion{
                                            rect: outer_rect,
                                        });
                                    }
                                    if square_response.clicked() || tile_clicked {
                                        if let Some(tile) = ctx.selected_tile_in_hand {
                                            msg =
                                                Some(PlayerMessage::Place(coord, *hand.get(tile).unwrap()));
                                            next_selection = Some(None);
                                        } else if is_selected {
                                            next_selection = Some(None);
                                        } else if let Some(selected_coord) = ctx.selected_square_on_board {
                                            msg = Some(PlayerMessage::Swap(coord, selected_coord));
                                            next_selection = Some(None);
                                        } else {
                                            next_selection = Some(Some(coord));
                                        }
                                    } else if let Some(tile) = hand_released_tile {
                                        if ui.rect_contains_pointer(outer_rect) {
                                            msg = Some(PlayerMessage::Place(coord, *hand.get(tile).unwrap()));
                                            next_selection = Some(None);
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
        });

        if let Some(new_selection) = next_selection {
            ctx.selected_square_on_board = new_selection;
            ctx.selected_tile_in_hand = None;
        }

        if hovered_square != ctx.hovered_tile_on_board {
            ctx.hovered_tile_on_board = hovered_square;
        }

        msg
    }
}
