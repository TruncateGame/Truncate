use truncate_core::board::{Board, Coordinate, Square};

use super::depot::TruncateDepot;

#[cfg(not(target_arch = "wasm32"))]
pub mod gamepad;
pub mod keyboard;

fn ensure_board_selection(
    depot: &mut TruncateDepot,
    local_player_index: usize,
    board: &Board,
) -> Coordinate {
    if let Some((coord, _)) = depot.interactions[local_player_index].selected_square_on_board {
        return coord;
    }
    if let Some((coord, sq)) =
        depot.interactions[local_player_index].previous_selected_square_on_board
    {
        depot.interactions[local_player_index].selected_square_on_board = Some((coord.clone(), sq));
        return coord;
    }
    let dock = board.docks.iter().find(|d| {
        board.get(**d).is_ok_and(
            |s| matches!(s, Square::Dock{ player, .. } if player == depot.gameplay.player_numbers[local_player_index] as usize),
        )
    });
    let coord = dock.cloned().unwrap_or_else(|| Coordinate::new(0, 0));
    depot.interactions[local_player_index].selected_square_on_board =
        Some((coord.clone(), board.get(coord).unwrap()));
    coord
}

fn move_selection(
    depot: &mut TruncateDepot,
    local_player_index: usize,
    mut movement: [isize; 2],
    board: &Board,
) {
    // If nothing is selected, the first interaction shouldn't move the cursor.
    // At the start of the game, it should select the dock,
    // and otherwise it should select the previously selected square.
    if depot.interactions[local_player_index]
        .selected_square_on_board
        .is_none()
    {
        ensure_board_selection(depot, local_player_index, board);
        return;
    }

    let current_selection = ensure_board_selection(depot, local_player_index, board);

    if depot.gameplay.player_numbers[local_player_index] == 0 {
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
        depot.interactions[local_player_index].selected_square_on_board = Some((new_coord, sq));
        depot.interactions[local_player_index].previous_selected_square_on_board =
            Some((new_coord, sq));
    }
}
