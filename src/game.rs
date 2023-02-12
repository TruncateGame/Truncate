use super::board::{Board, Coordinate, Square};
use super::hand::Hands;
use super::judge::Judge;
use super::moves::{Change, Move};

#[derive(Default)]
pub struct Game {
    pub board: Board, // TODO: should these actually be public?
    pub hands: Hands,
    pub judge: Judge,
    pub recent_changes: Vec<Change>,
    next_player: usize,
    winner: Option<usize>,
}

impl Game {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            board: Board::new(width, height),
            hands: Hands::default(),
            judge: Judge::default(),
            recent_changes: vec![],
            next_player: 0,
            winner: None,
        }
    }

    pub fn play_move(&mut self, next_move: Move) -> Result<Option<usize>, &str> {
        if self.winner.is_some() {
            return Err("Game is already over");
        }

        let player = match next_move {
            Move::Place { player, .. } => player,
            Move::Swap { player, .. } => player,
        };
        if player != self.next_player {
            return Err("Only the next player can play");
        }

        self.recent_changes = match self
            .board
            .make_move(next_move, &mut self.hands, &self.judge)
        {
            Ok(changes) => changes,
            Err(msg) => {
                println!("{}", msg);
                return Err("Couldn't make move"); // TODO: propogate error post polonius
            }
        };

        if let Some(winner) = Judge::winner(&(self.board)) {
            self.winner = Some(winner);
            return Ok(Some(winner));
        }

        self.next_player = (self.next_player + 1) % self.board.get_orientations().len(); // TODO: remove this hacky way to get the number of players

        Ok(None)
    }

    pub fn next(&self) -> usize {
        self.next_player
    }
}
