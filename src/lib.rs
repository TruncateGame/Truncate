mod bag;
mod board;
mod hand;
mod judge;
mod moves;

use judge::Judge;
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
    pub fn play_move(&mut self, next_move: moves::Move) -> Result<Option<usize>, &str> {
        if self.winner.is_some() {
            return Err("Game is already over");
        }

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

        if self
            .board
            .make_move(next_move, &mut self.hands, &self.judge)
            .is_err()
        {
            return Err("Couldn't make move"); // TODO: propogate error post polonius
        }

        if let Some(winner) = Judge::winner(&(self.board)) {
            self.winner = Some(winner);
            return Ok(Some(winner));
        }

        Ok(None)
    }
}
