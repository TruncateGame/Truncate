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
        if let Some((coord, _)) = depot.interactions.selected_square_on_board {
            return coord;
        }
        if let Some((coord, sq)) = depot.interactions.previous_selected_square_on_board {
            depot.interactions.selected_square_on_board = Some((coord.clone(), sq));
            return coord;
        }
        let artifact = board.artifacts.iter().find(|d| {
            board.get(**d).is_ok_and(
                |s| matches!(s, Square::Artifact{player: p, ..} if p == depot.gameplay.player_number as usize),
            )
        });
        let coord = artifact.cloned().unwrap_or_else(|| Coordinate::new(0, 0));
        depot.interactions.selected_square_on_board =
            Some((coord.clone(), board.get(coord).unwrap()));
        coord
    };

    let move_selection = |depot: &mut TruncateDepot, mut movement: [isize; 2]| {
        // If nothing is selected, the first interaction shouldn't move the cursor.
        // At the start of the game, it should select the artifact,
        // and otherwise it should select the previously selected square.
        if depot.interactions.selected_square_on_board.is_none() {
            ensure_board_selection(depot);
            return;
        }

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

        let new_coord = Coordinate {
            x: new_x as usize,
            y: new_y as usize,
        };

        if let Ok(sq) = board.get(new_coord) {
            depot.interactions.selected_square_on_board = Some((new_coord, sq));
            depot.interactions.previous_selected_square_on_board = Some((new_coord, sq));
        }
    };

    ctx.input_mut(|input| {
        if input.consume_key(Modifiers::NONE, Key::Period) {
            if !depot.ui_state.dictionary_open {
                depot.ui_state.dictionary_open = true;
                depot.ui_state.dictionary_opened_by_keyboard = true;
            } else if depot.ui_state.dictionary_opened_by_keyboard {
                depot.ui_state.dictionary_open = false;
                depot.ui_state.dictionary_opened_by_keyboard = false;
            }
        }

        if input.consume_key(Modifiers::NONE, Key::Escape) && depot.ui_state.dictionary_open {
            depot.ui_state.dictionary_open = false;
            depot.ui_state.dictionary_focused = false;
        }

        if depot.ui_state.dictionary_open {
            return;
        }

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

        for c in (b'A'..=b'Z').map(|c| [c]) {
            let letter = std::str::from_utf8(&c).unwrap();
            if input.consume_key(
                Modifiers::NONE,
                Key::from_name(letter).expect("letters should have keys"),
            ) {
                let current_selection = ensure_board_selection(depot);

                msg = Some(PlayerMessage::Place(
                    current_selection,
                    letter.chars().next().unwrap(),
                ))
            }
        }

        if input.consume_key(Modifiers::NONE, Key::Space) {
            let current_selection = ensure_board_selection(depot);
            if matches!(board.get(current_selection), Ok(Square::Occupied { .. })) {
                if let Some((already_selected_tile, _)) = depot.interactions.selected_tile_on_board
                {
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
                    depot.interactions.selected_tile_on_board =
                        Some((current_selection, board.get(current_selection).unwrap()));
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
