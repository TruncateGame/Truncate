use time::{Duration, OffsetDateTime};

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
    pub started_at: Option<OffsetDateTime>,
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
            started_at: None,
            next_player: 0,
            winner: None,
        }
    }

    pub fn add_player(&mut self, name: String) {
        self.players.push(Player::new(
            name,
            self.players.len(),
            7,
            &mut self.bag,
            Duration::new(900, 0), // TODO: un-hardcode the duration of turns
        ));
    }

    pub fn get_player(&self, player: usize) -> Option<&Player> {
        // TODO: Lookup player by `index` field rather than vec position
        self.players.get(player)
    }

    pub fn start(&mut self) {
        self.started_at = Some(OffsetDateTime::now_utc());
        // TODO: Lookup player by `index` field rather than vec position
        self.players[self.next_player].turn_starts_at = Some(OffsetDateTime::now_utc());
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
        let turn_duration = OffsetDateTime::now_utc()
            - self.players[player]
                .turn_starts_at
                .expect("Player played without the time running");
        if turn_duration.is_negative() {
            return Err("Player's turn has not yet started".into());
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

        let this_player = &mut self.players[player];
        this_player.time_remaining -= turn_duration;
        let mut apply_penalties = 0;

        if this_player.time_remaining.is_negative() {
            // TODO: Make the penalty period an option
            let total_penalties = 1 + (this_player.time_remaining.whole_seconds() / -60) as usize; // usize cast as we guaranteed both are negative
            println!("Player {player} now has {total_penalties} penalties");
            apply_penalties = total_penalties - this_player.penalties_incurred;
            println!("Player {player} needs {apply_penalties} to be applied");
            this_player.penalties_incurred = total_penalties;
        }

        if apply_penalties > 0 {
            for other_player in &mut self.players {
                if other_player.index == player {
                    continue;
                }
                for _ in 0..apply_penalties {
                    println!("Player {} gets a free tile", other_player.name);
                    self.recent_changes.push(other_player.add_special_tile('*'));
                }
            }
        }

        self.players[player].turn_starts_at = None;
        self.players[self.next_player].turn_starts_at = Some(OffsetDateTime::now_utc());

        Ok((self.recent_changes.clone(), None))
    }

    pub fn next(&self) -> usize {
        self.next_player
    }
}
