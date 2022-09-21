use super::board::Board;
use super::hand::Hands;
use super::judge::Judge;
use super::moves::Move;

#[derive(Default)]
pub struct Game {
    pub board: Board, // TODO: should these actually be public?
    pub hands: Hands,
    pub judge: Judge,
    next_player: usize,
    winner: Option<usize>,
}

impl Game {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            board: Board::new(width, height),
            hands: Hands::default(),
            judge: Judge::default(),
            next_player: 0,
            winner: None,
        }
    }

    pub fn play_move(&mut self, next_move: Move) -> Result<Option<usize>, &str> {
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

        if let Err(msg) = self
            .board
            .make_move(next_move, &mut self.hands, &self.judge)
        {
            println!("{}", msg);
            return Err("Couldn't make move"); // TODO: propogate error post polonius
        }

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
