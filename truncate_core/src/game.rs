use std::collections::{HashMap, HashSet};

use time::Duration;

use crate::bag::TileBag;
use crate::board::{Coordinate, Square};
use crate::error::GamePlayError;
use crate::judge::{Outcome, WordDict};
use crate::reporting::{self, BoardChange, BoardChangeAction, BoardChangeDetail, TimeChange};
use crate::rules::{self, GameRules, OvertimeRule};

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

#[derive(Debug, Default, Clone)]
pub struct Game {
    pub rules: GameRules,
    pub players: Vec<Player>,
    pub board: Board, // TODO: should these actually be public?
    pub bag: TileBag,
    pub judge: Judge,
    pub recent_changes: Vec<Change>,
    pub started_at: Option<u64>,
    pub next_player: usize,
    pub winner: Option<usize>,
}

// TODO: Move this to a helper file somewhere
fn now() -> u64 {
    instant::SystemTime::now()
        .duration_since(instant::SystemTime::UNIX_EPOCH)
        .expect("Please don't play Truncate before 1970")
        .as_secs()
}

impl Game {
    pub fn new(width: usize, height: usize) -> Self {
        let rules = GameRules::default();
        Self {
            players: Vec::with_capacity(2),
            board: Board::new(width, height),
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
            Some(Duration::new(time_allowance as i64, 0)),
            GAME_COLORS[self.players.len()],
        ));
    }

    pub fn get_player(&self, player: usize) -> Option<&Player> {
        // TODO: Lookup player by `index` field rather than vec position
        self.players.get(player)
    }

    pub fn start(&mut self) {
        self.started_at = Some(now());
        // TODO: Lookup player by `index` field rather than vec position
        self.players[self.next_player].turn_starts_at = Some(now());
    }

    pub fn any_player_is_overtime(&self) -> Option<usize> {
        let mut most_overtime_player: Option<(Duration, usize)> = None;

        for (player_number, player) in self.players.iter().enumerate() {
            let Some(mut time_remaining) = player.time_remaining else {
                continue;
            };
            if let Some(elapsed_time) = player.turn_starts_at {
                time_remaining -= Duration::seconds(elapsed_time as i64);
            }

            if time_remaining.is_negative() {
                match most_overtime_player {
                    Some((duration, _)) if time_remaining < duration => {
                        most_overtime_player = Some((time_remaining, player_number))
                    }
                    None => most_overtime_player = Some((time_remaining, player_number)),
                    _ => {}
                }
            }
        }

        most_overtime_player.map(|(_, player_number)| player_number)
    }

    pub fn calculate_game_over(&mut self) {
        let overtime_rule = match &self.rules.timing {
            rules::Timing::PerPlayer { overtime_rule, .. } => Some(overtime_rule),
            _ => None,
        };
        if matches!(overtime_rule, Some(OvertimeRule::Elimination)) {
            match self.any_player_is_overtime() {
                Some(overtime_player) => {
                    self.board.defeat_player(overtime_player);
                    self.winner = Some((overtime_player + 1) % 2);
                }
                _ => {}
            }
        }
    }

    pub fn play_turn(
        &mut self,
        next_move: Move,
        external_dictionary: Option<&WordDict>,
    ) -> Result<Option<usize>, String> {
        if self.winner.is_some() {
            return Err("Game is already over".into());
        }

        self.calculate_game_over();
        if self.winner.is_some() {
            return Ok(self.winner);
        }

        let player = match next_move {
            Move::Place { player, .. } => player,
            Move::Swap { player, .. } => player,
        };
        if player != self.next_player {
            return Err("Only the next player can play".into());
        }

        let turn_duration = now().checked_sub(
            self.players[player]
                .turn_starts_at
                .expect("Player played without the time running"),
        );
        let Some(turn_duration) = turn_duration else {
            return Err("Player's turn has not yet started".into());
        };

        self.recent_changes = match self.make_move(next_move, external_dictionary) {
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

        self.next_player = (self.next_player + 1) % self.players.len();

        let this_player = &mut self.players[player];
        if let Some(time_remaining) = &mut this_player.time_remaining {
            *time_remaining -= Duration::seconds(turn_duration as i64);
            let mut apply_penalties = 0;

            if time_remaining.is_negative() {
                // TODO: Make the penalty period an option
                let total_penalties = 1 + (time_remaining.whole_seconds() / -60) as usize; // usize cast as we guaranteed both are negative
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
        }

        self.players[player].turn_starts_at = None;

        if self
            .recent_changes
            .iter()
            .any(|c| matches!(c, Change::Battle(_)))
        {
            self.players[self.next_player].turn_starts_at = Some(now() + self.rules.battle_delay);
        } else {
            self.players[self.next_player].turn_starts_at = Some(now());
        }

        Ok(None)
    }

    pub fn make_move(
        &mut self,
        game_move: Move,
        external_dictionary: Option<&WordDict>,
    ) -> Result<Vec<Change>, GamePlayError> {
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
                self.resolve_attack(player, position, external_dictionary, &mut changes);

                self.players[player].swap_count = 0;

                Ok(changes)
            }
            Move::Swap {
                player: player_index,
                positions,
            } => {
                let mut swap_result =
                    self.board
                        .swap(player_index, positions, &self.rules.swapping)?;
                let player = &mut self.players[player_index];

                player.swap_count += 1;

                if let Some(swap_rules) = match &self.rules.swapping {
                    rules::Swapping::Contiguous(rules) => Some(rules),
                    rules::Swapping::Universal(rules) => Some(rules),
                    rules::Swapping::None => None,
                } {
                    let player_swaps = player.swap_count;
                    if player_swaps > swap_rules.swap_threshold {
                        let penalty_number = player_swaps - swap_rules.swap_threshold - 1;
                        let penalty = swap_rules
                            .penalties
                            .get(penalty_number)
                            .or_else(|| swap_rules.penalties.last());
                        if let (Some(penalty), Some(time_remaining)) =
                            (penalty, &mut player.time_remaining)
                        {
                            let time_change = -(*penalty as isize);
                            *time_remaining += Duration::seconds(time_change as i64);
                            swap_result.push(Change::Time(TimeChange {
                                player: player_index,
                                time_change,
                                reason: format!(
                                    "Lost time for {player_swaps} consecutive swap{}",
                                    if player_swaps == 1 { "" } else { "s" }
                                ),
                            }))
                        }
                    }
                }

                Ok(swap_result)
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
    fn resolve_attack(
        &mut self,
        player: usize,
        position: Coordinate,
        external_dictionary: Option<&WordDict>,
        changes: &mut Vec<Change>,
    ) {
        let (attackers, defenders) = self.board.collect_combanants(player, position);
        let attacking_words = self
            .board
            .word_strings(&attackers)
            .expect("Words were just found and should be valid");
        let defending_words = self
            .board
            .word_strings(&defenders)
            .expect("Words were just found and should be valid");

        if let Some(battle) = self.judge.battle(
            attacking_words,
            defending_words,
            &self.rules.battle_rules,
            &self.rules.win_condition,
            external_dictionary,
        ) {
            match battle.outcome.clone() {
                Outcome::DefenderWins => {
                    let mut all_defenders_are_towns = true;
                    changes.extend(defenders.iter().flatten().map(|coordinate| {
                        let square = self.board.get(*coordinate).expect("Tile just attacked");
                        if matches!(square, Square::Occupied(_, _)) {
                            all_defenders_are_towns = false;
                        }
                        Change::Board(BoardChange {
                            detail: BoardChangeDetail {
                                square,
                                coordinate: *coordinate,
                            },
                            action: BoardChangeAction::Victorious,
                        })
                    }));

                    let mut remove_attackers = true;

                    // When in BeatenByValidity mode, tiles can touch towns without being removed from the board.
                    if matches!(
                        &self.rules.win_condition,
                        rules::WinCondition::Destination {
                            town_defense: rules::TownDefense::BeatenByValidity
                        }
                    ) {
                        remove_attackers = false;
                    }

                    if remove_attackers {
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
                        match self.board.get(*square) {
                            Ok(Square::Occupied(_, letter)) => {
                                self.bag.return_tile(letter);
                            }
                            Ok(Square::Town { player, .. }) => {
                                self.board.set_square(
                                    *square,
                                    Square::Town {
                                        player,
                                        defeated: true,
                                    },
                                );
                            }
                            _ => {}
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
