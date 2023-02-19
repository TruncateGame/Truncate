use crate::bag::TileBag;

use super::board::{Board, Coordinate, Square};
use super::judge::Judge;
use super::moves::Move;
use super::player::Player;
use super::reporting::{BoardChange, BoardChangeAction, BoardChangeDetail, Change};

#[derive(Default)]
pub struct Game {
    pub players: Vec<Player>,
    pub board: Board, // TODO: should these actually be public?
    pub bag: TileBag,
    pub judge: Judge,
    pub recent_changes: Vec<Change>,
    next_player: usize,
    winner: Option<usize>,
}

impl Game {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            players: Vec::with_capacity(2),
            board: Board::new(width, height),
            bag: TileBag::default(),
            judge: Judge::default(),
            recent_changes: vec![],
            next_player: 0,
            winner: None,
        }
    }

    pub fn add_player(&mut self, name: String) {
        self.players
            .push(Player::new(name, self.players.len(), 7, &mut self.bag));
    }

    pub fn get_player(&self, player: usize) -> Option<&Player> {
        // TODO: Lookup player by `index` field rather than vec position
        self.players.get(player)
    }

    pub fn play_move(&mut self, next_move: Move) -> Result<(Vec<Change>, Option<usize>), String> {
        if self.winner.is_some() {
            return Err("Game is already over".into());
        }

        let player = match next_move {
            Move::Place { player, .. } => player,
            Move::Swap { player, .. } => player,
        };
        if player != self.next_player {
            return Err("Only the next player can play".into());
        }

        self.recent_changes =
            match self
                .board
                .make_move(next_move, &mut self.players, &mut self.bag, &self.judge)
            {
                Ok(changes) => changes,
                Err(msg) => {
                    println!("{}", msg);
                    return Err(format!("Couldn't make move: {msg}")); // TODO: propogate error post polonius
                }
            };

        if let Some(winner) = Judge::winner(&(self.board)) {
            self.winner = Some(winner);
            return Ok((self.recent_changes.clone(), Some(winner)));
        }

        self.next_player = (self.next_player + 1) % self.board.get_orientations().len(); // TODO: remove this hacky way to get the number of players

        Ok((self.recent_changes.clone(), None))
    }

    pub fn next(&self) -> usize {
        self.next_player
    }
}
