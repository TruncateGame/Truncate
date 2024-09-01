use eframe::egui::{self, Key, Modifiers};
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::PlayerMessage,
    player::Hand,
};

use crate::utils::depot::TruncateDepot;

use super::{ensure_board_selection, move_selection};

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
    hands: &Vec<Hand>,
    depot: &mut TruncateDepot,
) -> Option<PlayerMessage> {
    let mut needs_repaint = false;
    let mut msg = None;

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
            move_selection(depot, 0, [0, -1], board);
            needs_repaint = true;
        }
        if input.consume_key(Modifiers::NONE, Key::ArrowRight) {
            move_selection(depot, 0, [1, 0], board);
            needs_repaint = true;
        }
        if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
            move_selection(depot, 0, [0, 1], board);
            needs_repaint = true;
        }
        if input.consume_key(Modifiers::NONE, Key::ArrowLeft) {
            move_selection(depot, 0, [-1, 0], board);
            needs_repaint = true;
        }

        if input.consume_key(Modifiers::NONE, Key::from_name("W").unwrap()) {
            move_selection(depot, 1, [0, 1], board);
            needs_repaint = true;
        }
        if input.consume_key(Modifiers::NONE, Key::from_name("D").unwrap()) {
            move_selection(depot, 1, [-1, 0], board);
            needs_repaint = true;
        }
        if input.consume_key(Modifiers::NONE, Key::from_name("S").unwrap()) {
            move_selection(depot, 1, [0, -1], board);
            needs_repaint = true;
        }
        if input.consume_key(Modifiers::NONE, Key::from_name("A").unwrap()) {
            move_selection(depot, 1, [1, 0], board);
            needs_repaint = true;
        }

        for key in 0..5 {
            if input.consume_key(Modifiers::NONE, NUM_KEYS[key]) {
                let current_selection = ensure_board_selection(depot, 0, board);

                if let Some(char) = hands[0].get(key) {
                    msg = Some(PlayerMessage::Place(current_selection, *char))
                }
            }
        }

        for key in 5..9 {
            if input.consume_key(Modifiers::NONE, NUM_KEYS[key]) {
                let current_selection = ensure_board_selection(depot, 1, board);

                if let Some(char) = hands[1].get(key - 5) {
                    msg = Some(PlayerMessage::Place(current_selection, *char))
                }
            }
        }

        // for c in (b'A'..=b'Z').map(|c| [c]) {
        //     let letter = std::str::from_utf8(&c).unwrap();
        //     if input.consume_key(
        //         Modifiers::NONE,
        //         Key::from_name(letter).expect("letters should have keys"),
        //     ) {
        //         let current_selection = ensure_board_selection(depot, 0, board);

        //         msg = Some(PlayerMessage::Place(
        //             current_selection,
        //             letter.chars().next().unwrap(),
        //         ))
        //     }
        // }

        if input.consume_key(Modifiers::NONE, Key::Space) {
            let current_selection = ensure_board_selection(depot, 0, board);
            if matches!(board.get(current_selection), Ok(Square::Occupied { .. })) {
                if let Some((already_selected_tile, _)) =
                    depot.interactions[0].selected_tile_on_board
                {
                    if already_selected_tile == current_selection {
                        depot.interactions[0].selected_tile_on_board = None;
                    } else {
                        msg = Some(PlayerMessage::Swap(
                            already_selected_tile,
                            current_selection,
                        ));
                        depot.interactions[0].selected_tile_on_board = None;
                    }
                } else {
                    depot.interactions[0].selected_tile_on_board =
                        Some((current_selection, board.get(current_selection).unwrap()));
                }
            } else {
                depot.interactions[0].selected_tile_on_board = None;
            }
        }
    });

    if needs_repaint {
        ctx.request_repaint();
    }

    msg
}
