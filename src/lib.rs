mod bag;
mod board;
mod hand;
mod judge;
mod moves;

use moves::*;

#[derive(Default)]
pub struct Game {
    board: board::Board,
    hands: hand::Hands,
    judge: judge::Judge,
    next_player: usize,
    winner: Option<usize>,
}

impl Game {
    pub fn play_move(&mut self, next_move: moves::Move) -> Result<(), &str> {
        let player = match next_move {
            Move::Place {
                player,
                tile: _,
                position: _,
            } => player,
            Move::Swap {
                player,
                positions: _,
            } => player,
        };
        if player != self.next_player {
            return Err("Only the next player can play");
        }

        self.board
            .make_move(next_move, &mut self.hands, &self.judge)?;
        Ok(())
    }
}
