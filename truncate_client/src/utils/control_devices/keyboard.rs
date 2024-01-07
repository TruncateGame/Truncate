use eframe::egui::{self, Key, Modifiers};
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
    player::Hand,
};

use crate::utils::depot::TruncateDepot;

/*
TODOS:
- Made this a persistent struct that remembers keys down and performs its own events
*/

const NUM_KEYS: [Key; 9] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
];

pub fn handle_input(
    ctx: &egui::Context,
    board: &Board,
    hand: &Hand,
    depot: &mut TruncateDepot,
) -> Option<PlayerMessage> {
    let mut needs_repaint = false;
    let mut msg = None;

    let ensure_board_selection = |depot: &mut TruncateDepot| {
        if let Some(coord) = depot.interactions.selected_square_on_board {
            return coord;
        }
        let dock = board.docks.iter().find(|d| {
            board.get(**d).is_ok_and(
                |s| matches!(s, Square::Dock(p) if p == depot.gameplay.player_number as usize),
            )
        });
        let coord = dock.cloned().unwrap_or_else(|| Coordinate::new(0, 0));
        depot.interactions.selected_square_on_board = Some(coord.clone());
        coord
    };

    let move_selection = |depot: &mut TruncateDepot, mut movement: [isize; 2]| {
        let current_selection = ensure_board_selection(depot);

        if depot.gameplay.player_number == 0 {
            movement[0] *= -1;
            movement[1] *= -1;
        }

        let mut new_x = (current_selection.x as isize) + movement[0];
        let mut new_y = (current_selection.y as isize) + movement[1];

        new_x = new_x.min(board.width() as isize - 1);
        new_y = new_y.min(board.height() as isize - 1);

        new_x = new_x.max(0);
        new_y = new_y.max(0);

        depot.interactions.selected_square_on_board = Some(Coordinate {
            x: new_x as usize,
            y: new_y as usize,
        });
    };

    ctx.input_mut(|input| {
        if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
            move_selection(depot, [0, -1]);
            needs_repaint = true;
        }
        if input.consume_key(Modifiers::NONE, Key::ArrowRight) {
            move_selection(depot, [1, 0]);
            needs_repaint = true;
        }
        if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
            move_selection(depot, [0, 1]);
            needs_repaint = true;
        }
        if input.consume_key(Modifiers::NONE, Key::ArrowLeft) {
            move_selection(depot, [-1, 0]);
            needs_repaint = true;
        }

        for key in 0..9 {
            if input.consume_key(Modifiers::NONE, NUM_KEYS[key]) {
                let current_selection = ensure_board_selection(depot);

                if let Some(char) = hand.get(key) {
                    msg = Some(PlayerMessage::Place(current_selection, *char))
                }
            }
        }

        if input.consume_key(Modifiers::NONE, Key::Space) {
            let current_selection = ensure_board_selection(depot);
            if matches!(board.get(current_selection), Ok(Square::Occupied(_, _))) {
                if let Some(already_selected_tile) = depot.interactions.selected_tile_on_board {
                    if already_selected_tile == current_selection {
                        depot.interactions.selected_tile_on_board = None;
                    } else {
                        msg = Some(PlayerMessage::Swap(
                            already_selected_tile,
                            current_selection,
                        ));
                        depot.interactions.selected_tile_on_board = None;
                    }
                } else {
                    depot.interactions.selected_tile_on_board = Some(current_selection);
                }
            } else {
                depot.interactions.selected_tile_on_board = None;
            }
        }
    });

    if needs_repaint {
        ctx.request_repaint();
    }

    msg
}
