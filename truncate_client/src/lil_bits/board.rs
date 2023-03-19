use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
    player::Hand,
    reporting::BoardChange,
};

use eframe::egui;
use hashbrown::HashMap;

use crate::{theming::Theme, active_game::HoveredRegion};

use super::{
    tile::{TilePlayer},
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
        hand_selected_tile: Option<usize>,
        hand_released_tile: Option<usize>,
        board_selected_tile: Option<Coordinate>,
        hand: &Hand,
        board_changes: &HashMap<Coordinate, BoardChange>,
        player: u64,
        invert: bool, // TODO: Transpose to any rotation
        ui: &mut egui::Ui,
        theme: &Theme,
    ) -> (Option<Option<Coordinate>>, Option<PlayerMessage>, Option<HoveredRegion>) {
        let mut msg = None;
        let mut next_selection = None;
        let mut hovered_square = None;

        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        let game_area = ui.available_rect_before_wrap();

        let (margin, theme) = theme.calc_rescale(
            &game_area, 
            self.board.width(),
            self.board.height(),
            0.4..2.0
        );
        let outer_frame = egui::Frame::none().inner_margin(margin);

        outer_frame.show(ui, |ui| {
            let mut render = |rows: Box<dyn Iterator<Item = (usize, &Vec<Option<Square>>)>>| {
                let mut render_row =
                    |rownum, row: Box<dyn Iterator<Item = (usize, &Option<Square>)>>| {
                        ui.horizontal(|ui| {
                            for (colnum, square) in row {
                                let coord = Coordinate::new(colnum, rownum);
                                let is_root = self.board.roots.contains(&coord);
                                let is_selected = Some(coord) == board_selected_tile;
                                let tile_player = |p: &usize| {
                                    if *p as u64 == player {
                                        TilePlayer::Own
                                    } else {
                                        TilePlayer::Enemy
                                    }
                                };

                                let mut tile = if let Some(Square::Occupied(player, char)) = square {
                                    Some(TileUI::new(*char, tile_player(player)).selected(is_selected))
                                } else {
                                    None
                                };

                                if let Some(change) = board_changes.get(&coord) {
                                    use truncate_core::reporting::BoardChangeAction;
                                    tile = match (&change.action, tile) {
                                        (BoardChangeAction::Added, Some(tile)) => Some(tile.added(true)),
                                        (BoardChangeAction::Swapped, Some(tile)) => Some(tile.modified(true)),
                                        (BoardChangeAction::Defeated, None) => 
                                            match change.detail.square {
                                                Square::Empty => None,
                                                Square::Occupied(player, char) => Some((player, char)),
                                            }
                                            .map(
                                                |(player, char)| {
                                                    TileUI::new(char, tile_player(&player))
                                                        .selected(is_selected)
                                                        .defeated(true)
                                                },
                                            ),
                                        (BoardChangeAction::Truncated, None) => 
                                            match change.detail.square {
                                                Square::Empty => None,
                                                Square::Occupied(player, char) => Some((player, char)),
                                            }
                                            .map(
                                                |(player, char)| {
                                                    TileUI::new(char, tile_player(&player))
                                                        .selected(is_selected)
                                                        .truncated(true)
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
                                if let Some(placing_tile) = hand_selected_tile {
                                    if matches!(square, Some(Square::Empty)) {
                                        overlay = Some(*hand.get(placing_tile).unwrap());
                                    }
                                } else if let Some(placing_tile) = board_selected_tile { // TODO: De-nest
                                    if placing_tile != coord {
                                        if let Some(Square::Occupied(p, _)) = square {
                                            if p == &(player as usize) {
                                                if let Ok(Square::Occupied(_, char)) = self.board.get(placing_tile) {
                                                    overlay = Some(char);
                                                }
                                            }
                                        }
                                    }
                                }
                                // TODO: Devise a way to show this tile in the place of the board_selected_tile

                                let mut tile_clicked = false;
                                let (square_response, outer_rect) = SquareUI::new()
                                    .enabled(square.is_some())
                                    .empty(matches!(square, Some(Square::Empty)))
                                    .root(is_root)
                                    .selected(is_selected)
                                    .overlay(overlay)
                                    .render(ui, &theme, |ui, theme| {
                                        if let Some(tile) = tile {
                                            tile_clicked = tile.render(ui, theme).clicked();
                                        }
                                    });
                                if square.is_some() {
                                    if ui.rect_contains_pointer(outer_rect) {
                                        hovered_square = Some(HoveredRegion{
                                            rect: outer_rect,
                                            engaged: ui.rect_contains_pointer(square_response.rect),
                                        });
                                    }
                                    if square_response.clicked() || tile_clicked {
                                        if let Some(tile) = hand_selected_tile {
                                            msg =
                                                Some(PlayerMessage::Place(coord, *hand.get(tile).unwrap()));
                                            next_selection = Some(None);
                                        } else if is_selected {
                                            next_selection = Some(None);
                                        } else if let Some(selected_coord) = board_selected_tile {
                                            msg = Some(PlayerMessage::Swap(coord, selected_coord));
                                            next_selection = Some(None);
                                        } else {
                                            next_selection = Some(Some(coord));
                                        }
                                    } else if let Some(tile) = hand_released_tile {
                                        if square_response.hovered() {
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
        (next_selection, msg, hovered_square)
    }
}
