use std::collections::HashSet;

use crate::{
    board::{Board, Square},
    game::Game,
    messages::PlayerMessage,
};

impl Game {
    pub fn brute_force(&mut self) -> PlayerMessage {
        let mut playable_squares = HashSet::new();
        for dock in &self.board.docks {
            let sq = self.board.get(*dock).unwrap();
            if !matches!(sq, Square::Dock(p) if p == self.next_player) {
                continue;
            }

            playable_squares.extend(
                self.board
                    .depth_first_search(*dock)
                    .iter()
                    .flat_map(|sq| sq.neighbors_4())
                    .collect::<HashSet<_>>(),
            );
        }

        playable_squares = playable_squares
            .into_iter()
            .filter(|sq| matches!(self.board.get(*sq), Ok(Square::Land)))
            .collect();

        let mut best_move: (f32, PlayerMessage) = (f32::MIN, PlayerMessage::Ping);

        for position in playable_squares {
            for tile in self.players.get(self.next_player).unwrap().hand.iter() {
                self.board
                    .set_square(position, Square::Occupied(self.next_player, *tile))
                    .expect("We should have validated these squares");
                let move_score = self.eval_self_board_progress(self.next_player);
                if best_move.0 < move_score {
                    best_move = (move_score, PlayerMessage::Place(position, *tile));
                }
            }
            self.board.clear(position);
        }

        print!("Best move is {best_move:#?}");

        best_move.1
    }

    pub fn eval_self_board_progress(&self, player: usize) -> f32 {
        let mut score = 0.0;

        for (rownum, row) in self.board.squares.iter().enumerate() {
            let row_score = if player == 0 {
                rownum as f32
            } else {
                (&self.board.squares.len() - rownum) as f32
            };

            for sq in row {
                if matches!(sq, Square::Occupied(p, _) if player == *p) {
                    score += row_score;
                }
            }
        }

        score
    }
}
