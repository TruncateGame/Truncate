use time::{Duration, OffsetDateTime};

use crate::bag::TileBag;
use crate::board::{Coordinate, Square};
use crate::error::GamePlayError;
use crate::judge::Outcome;
use crate::reporting::{self, BoardChange, BoardChangeAction, BoardChangeDetail};
use crate::rules::{self, GameRules};

use super::board::Board;
use super::judge::Judge;
use super::moves::Move;
use super::player::Player;
use super::reporting::Change;

pub const GAME_COLORS: [(u8, u8, u8); 5] = [
    (80_u8, 167_u8, 232_u8),
    (122_u8, 40_u8, 203_u8),
    (253_u8, 197_u8, 245_u8),
    (230_u8, 63_u8, 56_u8),
    (246_u8, 174_u8, 45_u8),
];

#[derive(Default)]
pub struct Game {
    pub rules: GameRules,
    pub players: Vec<Player>,
    pub board: Board, // TODO: should these actually be public?
    pub bag: TileBag,
    pub judge: Judge,
    pub recent_changes: Vec<Change>,
    pub started_at: Option<OffsetDateTime>,
    pub next_player: usize,
    pub winner: Option<usize>,
}

impl Game {
    pub fn new(width: usize, height: usize, padded: bool) -> Self {
        let rules = GameRules::default();
        Self {
            players: Vec::with_capacity(2),
            board: Board::new(width, height, padded),
            bag: TileBag::new(&rules.tile_distribution),
            judge: Judge::default(),
            recent_changes: vec![],
            started_at: None,
            next_player: 0,
            winner: None,
            rules,
        }
    }

    pub fn add_player(&mut self, name: String) {
        let time_allowance = match self.rules.timing {
            rules::Timing::PerPlayer {
                time_allowance,
                overtime_rule: _,
            } => time_allowance,
            _ => unimplemented!(),
        };
        self.players.push(Player::new(
            name,
            self.players.len(),
            self.rules.hand_size,
            &mut self.bag,
            Duration::new(time_allowance as i64, 0), // TODO: un-hardcode the duration of turns
            GAME_COLORS[self.players.len()],
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

    pub fn play_turn(&mut self, next_move: Move) -> Result<Option<usize>, String> {
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

        self.recent_changes = match self.make_move(next_move) {
            Ok(changes) => changes,
            Err(msg) => {
                println!("{}", msg);
                return Err(format!("Couldn't make move: {msg}")); // TODO: propogate error post polonius
            }
        };

        if let Some(winner) = Judge::winner(&(self.board)) {
            self.winner = Some(winner);
            return Ok(Some(winner));
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
                    self.recent_changes.push(other_player.add_special_tile('¤'));
                }
            }
        }

        self.players[player].turn_starts_at = None;
        self.players[self.next_player].turn_starts_at = Some(OffsetDateTime::now_utc());

        Ok(None)
    }

    pub fn make_move(&mut self, game_move: Move) -> Result<Vec<Change>, GamePlayError> {
        let mut changes = vec![];

        match game_move {
            Move::Place {
                player,
                tile,
                position,
            } => {
                if let Square::Occupied(..) = self.board.get(position)? {
                    return Err(GamePlayError::OccupiedPlace);
                }

                if !self.board.neighbouring_squares(position).iter().any(
                    |&(_, square)| match square {
                        Square::Occupied(p, _) => p == player,
                        Square::Dock(p) => p == player,
                        _ => false,
                    },
                ) {
                    return Err(GamePlayError::NonAdjacentPlace);
                }

                changes.push(self.players[player].use_tile(tile, &mut self.bag)?);
                changes.push(Change::Board(BoardChange {
                    detail: self.board.set(position, player, tile)?,
                    action: BoardChangeAction::Added,
                }));
                self.resolve_attack(player, position, &mut changes);
                Ok(changes)
            }
            Move::Swap { player, positions } => {
                self.board.swap(player, positions, &self.rules.swapping)
            }
        }
    }

    // If any attacking word is invalid, or all defending words are valid and stronger than the longest attacking words
    //   - All attacking words die
    //   - Attacking tiles are truncated
    // Otherwise
    //   - Weak and invalid defending words die
    //   - Any remaining defending letters adjacent to the attacking tile die
    //   - Defending tiles are truncated
    fn resolve_attack(&mut self, player: usize, position: Coordinate, changes: &mut Vec<Change>) {
        let (attackers, defenders) = self.board.collect_combanants(player, position);
        let attacking_words = self
            .board
            .word_strings(&attackers)
            .expect("Words were just found and should be valid");
        let defending_words = self
            .board
            .word_strings(&defenders)
            .expect("Words were just found and should be valid");

        if let Some(battle) =
            self.judge
                .battle(attacking_words, defending_words, &self.rules.battle_rules)
        {
            match battle.outcome.clone() {
                Outcome::DefenderWins => {
                    changes.extend(defenders.iter().flatten().map(|coordinate| {
                        let square = self.board.get(*coordinate).expect("Tile just attacked");
                        Change::Board(BoardChange {
                            detail: BoardChangeDetail {
                                square,
                                coordinate: *coordinate,
                            },
                            action: BoardChangeAction::Victorious,
                        })
                    }));

                    let squares = attackers.into_iter().flat_map(|word| word.into_iter());
                    changes.extend(squares.flat_map(|square| {
                        if let Ok(Square::Occupied(_, letter)) = self.board.get(square) {
                            self.bag.return_tile(letter);
                        }
                        self.board.clear(square).map(|detail| {
                            Change::Board(BoardChange {
                                detail,
                                action: BoardChangeAction::Defeated,
                            })
                        })
                    }));
                }
                Outcome::AttackerWins(losers) => {
                    changes.extend(attackers.iter().flatten().map(|coordinate| {
                        let square = self.board.get(*coordinate).expect("Tile just attacked");
                        Change::Board(BoardChange {
                            detail: BoardChangeDetail {
                                square,
                                coordinate: *coordinate,
                            },
                            action: BoardChangeAction::Victorious,
                        })
                    }));

                    let squares = losers.into_iter().flat_map(|defender_index| {
                        let defender = defenders
                            .get(defender_index)
                            .expect("Losers should only contain valid squares");
                        defender.into_iter()
                    });
                    changes.extend(squares.flat_map(|square| {
                        if let Ok(Square::Occupied(_, letter)) = self.board.get(*square) {
                            self.bag.return_tile(letter);
                        }
                        self.board.clear(*square).map(|detail| {
                            Change::Board(BoardChange {
                                detail,
                                action: BoardChangeAction::Defeated,
                            })
                        })
                    }));

                    // explode adjacent letters belonging to opponents
                    changes.extend(self.board.neighbouring_squares(position).iter().flat_map(
                        |neighbour| {
                            if let (coordinate, Square::Occupied(owner, letter)) = neighbour {
                                if *owner != player {
                                    self.bag.return_tile(*letter);
                                    return self.board.clear(*coordinate).map(|detail| {
                                        Change::Board(BoardChange {
                                            detail,
                                            action: BoardChangeAction::Exploded,
                                        })
                                    });
                                }
                            }
                            None
                        },
                    ));

                    match self.board.get(position) {
                        Ok(Square::Occupied(_, tile)) if tile == '¤' => {
                            changes.push(
                                self.board
                                    .clear(position)
                                    .map(|detail| {
                                        Change::Board(BoardChange {
                                            detail,
                                            action: BoardChangeAction::Exploded,
                                        })
                                    })
                                    .expect("Tile exists and should be removable"),
                            );
                        }
                        _ => {}
                    }
                }
            }
            changes.push(Change::Battle(battle));
        }

        match self.rules.truncation {
            rules::Truncation::Root => {
                changes.extend(self.board.truncate(&mut self.bag).into_iter())
            }
            rules::Truncation::Larger => unimplemented!(),
            rules::Truncation::None => {}
        }
    }

    pub fn next(&self) -> usize {
        self.next_player
    }

    pub fn filter_game_to_player(&self, player_index: usize) -> (Board, Vec<Change>) {
        let visible_board =
            self.board
                .filter_to_player(player_index, &self.rules.visibility, &self.winner);
        let visible_changes = reporting::filter_to_player(
            &self.recent_changes,
            &visible_board,
            player_index,
            &self.rules.visibility,
            &self.winner,
        );
        (visible_board, visible_changes)
    }
}
