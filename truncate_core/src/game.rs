use std::collections::HashMap;

use time::Duration;
use xxhash_rust::xxh3;

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
    pub battle_count: u32,
    pub turn_count: u32,
    pub player_turn_count: Vec<u32>,
    pub recent_changes: Vec<Change>,
    pub started_at: Option<u64>,
    pub game_ends_at: Option<u64>,
    pub next_player: Option<usize>,
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
    pub fn new(width: usize, height: usize, tile_seed: Option<u64>) -> Self {
        let rules = GameRules::default();
        let mut board = Board::new(width, height);
        board.grow();

        let next_player = match &rules.timing {
            rules::Timing::Periodic { .. } => None,
            _ => Some(0),
        };

        Self {
            players: Vec::with_capacity(2),
            board,
            bag: TileBag::new(&rules.tile_distribution, tile_seed),
            judge: Judge::default(),
            battle_count: 0,
            turn_count: 0,
            player_turn_count: Vec::with_capacity(2),
            recent_changes: vec![],
            started_at: None,
            game_ends_at: None,
            next_player,
            winner: None,
            rules,
        }
    }

    pub fn add_player(&mut self, name: String) {
        let time_allowance = match self.rules.timing {
            rules::Timing::PerPlayer {
                time_allowance,
                overtime_rule: _,
            } => Some(Duration::new(time_allowance as i64, 0)),
            rules::Timing::None => None,
            rules::Timing::Periodic { .. } => None,
            _ => unimplemented!(),
        };
        self.players.push(Player::new(
            name,
            self.players.len(),
            self.rules.hand_size,
            &mut self.bag,
            time_allowance,
            GAME_COLORS[self.players.len()],
        ));
        self.player_turn_count.push(0);
    }

    pub fn get_player(&self, player: usize) -> Option<&Player> {
        // TODO: Lookup player by `index` field rather than vec position
        self.players.get(player)
    }

    pub fn start(&mut self) {
        let now = now();
        self.started_at = Some(now);

        match self.rules.timing {
            rules::Timing::PerPlayer { .. } | rules::Timing::None => {
                self.players[self.next_player.unwrap()].turn_starts_no_later_than = Some(now);
                self.players[self.next_player.unwrap()].turn_starts_no_sooner_than = Some(now);
            }
            rules::Timing::Periodic {
                total_time_allowance,
                ..
            } => self.players.iter_mut().for_each(|p| {
                p.turn_starts_no_later_than = Some(now);
                p.turn_starts_no_sooner_than = Some(now);

                self.game_ends_at = Some(now + total_time_allowance as u64);
            }),
            _ => unimplemented!(),
        }
    }

    pub fn any_player_is_overtime(&self) -> Option<usize> {
        let mut most_overtime_player: Option<(Duration, usize)> = None;

        for (player_number, player) in self.players.iter().enumerate() {
            let Some(mut time_remaining) = player.time_remaining else {
                continue;
            };
            if let Some(turn_starts) = player.turn_starts_no_later_than {
                let elapsed_time = now().saturating_sub(turn_starts);
                time_remaining -= Duration::seconds(elapsed_time as i64);
            }

            if !time_remaining.is_positive() {
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

    pub fn game_is_overtime(&self) -> bool {
        let Some(started_at) = self.started_at else {
            return false;
        };

        match &self.rules.timing {
            rules::Timing::Periodic {
                total_time_allowance,
                ..
            } => {
                let elapsed = now() - started_at;
                if elapsed as usize > *total_time_allowance {
                    return true;
                }
            }
            _ => {}
        }

        if let Some(max) = &self.rules.max_turns {
            if self.turn_count as u64 >= *max {
                return true;
            }
        }

        false
    }

    pub fn calculate_game_over(&mut self, current_player: Option<usize>) {
        let overtime_rule = match &self.rules.timing {
            rules::Timing::PerPlayer { overtime_rule, .. } => Some(overtime_rule),
            _ => None,
        };
        if matches!(overtime_rule, Some(OvertimeRule::Elimination)) {
            match self.any_player_is_overtime() {
                Some(overtime_player) => {
                    println!("{overtime_player} is over time!");
                    self.board.defeat_player(overtime_player);
                    self.winner = Some((overtime_player + 1) % 2);
                }
                _ => {}
            }
        }

        if self.game_is_overtime() {
            match &self.rules.win_metric {
                rules::WinMetric::TownProximity => {
                    let mut scores: Vec<_> = (0..self.players.len())
                        .map(|p| self.board.proximity_to_enemy_town(p))
                        .collect();

                    let mut remaining_players: Vec<_> = (0..self.players.len()).collect();

                    // This handles any number of players, returning the player
                    // with the best proximity to some other player
                    while scores.iter().any(|s| !s.is_empty()) {
                        let next_prox: Vec<_> = scores
                            .iter_mut()
                            .map(|scores| scores.pop().unwrap_or(usize::MAX))
                            .collect();

                        println!("Calculating game end promixities: {:?}", next_prox);

                        let best_score = next_prox.iter().min().unwrap();

                        // Players are even at this level
                        if next_prox.iter().max().unwrap() == best_score {
                            continue;
                        }

                        for (player, score) in next_prox.iter().enumerate() {
                            if remaining_players.len() > 1 && score > best_score {
                                // As soon as this player isn't the best at a given distance,
                                // remove them from the contender pool
                                remaining_players.retain(|p| *p != player);
                            }
                        }

                        if remaining_players.len() <= 1 {
                            break;
                        }
                    }

                    let winner = if remaining_players.len() == 1 {
                        remaining_players.pop().unwrap()
                    } else {
                        0 // TODO: We need a draw mechanism
                    };

                    println!("{winner} wins on proximity!");
                    (0..self.players.len())
                        .filter(|p| *p != winner)
                        .for_each(|p| self.board.defeat_player(p));
                    self.winner = Some(winner);
                }
            }
        }

        // If any opponents were blocked out by this turn, they lose
        for (player_index, _player) in self.players.iter().enumerate().filter(|(i, _)| {
            if let Some(p) = current_player {
                *i != p
            } else {
                true
            }
        }) {
            if self.board.playable_positions(player_index).is_empty() {
                self.board.defeat_player(player_index);
                self.winner = Some((player_index + 1) % 2);
            }
        }
    }

    pub fn resign_player(&mut self, resigning_player: usize) {
        self.board.defeat_player(resigning_player);
        self.winner = Some((resigning_player + 1) % 2);
    }

    pub fn play_turn(
        &mut self,
        next_move: Move,
        attacker_dictionary: Option<&WordDict>,
        defender_dictionary: Option<&WordDict>,
        cached_word_judgements: Option<&mut HashMap<String, bool, xxh3::Xxh3Builder>>,
    ) -> Result<Option<usize>, String> {
        if self.winner.is_some() {
            return Err("Game is already over".into());
        }

        let player = match next_move {
            Move::Place { player, .. } => player,
            Move::Swap { player, .. } => player,
        };

        self.calculate_game_over(Some(player));
        if self.winner.is_some() {
            return Ok(self.winner);
        }

        match &self.rules.timing {
            rules::Timing::Periodic { .. } => { /* All players can play */ }
            _ => {
                if player != self.next_player.unwrap() {
                    return Err("Only the next player can play".into());
                }
            }
        }

        if let Some(turn_start) = self.players[player].turn_starts_no_sooner_than {
            if turn_start > now() {
                return Err("Player's turn has not yet started".into());
            }
        } else {
            return Err("Player's turn has not yet started".into());
        }

        self.recent_changes = match self.make_move(
            next_move,
            attacker_dictionary,
            defender_dictionary,
            cached_word_judgements,
        ) {
            Ok(changes) => changes,
            Err(msg) => {
                println!("{}", msg);
                return Err(format!("{msg}")); // TODO: propogate error post polonius
            }
        };

        self.turn_count += 1;
        self.player_turn_count[player] += 1;

        // Check for winning via defeated towns
        if let Some(winner) = Judge::winner(&(self.board)) {
            self.winner = Some(winner);
            return Ok(Some(winner));
        }

        // Check for de-facto winning by blocking all moves
        self.calculate_game_over(Some(player));
        if self.winner.is_some() {
            return Ok(self.winner);
        }

        if let Some(next_player) = self.next_player.as_mut() {
            *next_player = (*next_player + 1) % self.players.len();
        }

        let this_player = &mut self.players[player];
        if let Some(time_remaining) = &mut this_player.time_remaining {
            let turn_duration = now().saturating_sub(
                this_player
                    .turn_starts_no_later_than
                    .or(this_player.turn_starts_no_sooner_than)
                    .expect("Player played without the time running"),
            );

            *time_remaining -= Duration::seconds(turn_duration as i64);

            let overtime_rule = match &self.rules.timing {
                rules::Timing::PerPlayer { overtime_rule, .. } => Some(overtime_rule),
                _ => None,
            };

            match overtime_rule {
                Some(OvertimeRule::Bomb { period }) => {
                    let mut apply_penalties = 0;

                    if time_remaining.is_negative() {
                        // TODO: Make the penalty period an option
                        let total_penalties =
                            1 + (time_remaining.whole_seconds() / -(*period as i64)) as usize; // usize cast as we guaranteed both are negative
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
                _ => {}
            };
        }

        match &self.rules.timing {
            rules::Timing::Periodic { turn_delay, .. } => {
                self.players[player].turn_starts_no_later_than = Some(now() + *turn_delay as u64);
                self.players[player].turn_starts_no_sooner_than = Some(now() + *turn_delay as u64);
            }
            _ => {
                self.players[player].turn_starts_no_later_than = None;
                self.players[player].turn_starts_no_sooner_than = None;

                if self
                    .recent_changes
                    .iter()
                    .any(|c| matches!(c, Change::Battle(_)))
                {
                    self.players[self.next_player.unwrap()].turn_starts_no_sooner_than =
                        Some(now());
                    self.players[self.next_player.unwrap()].turn_starts_no_later_than =
                        Some(now() + self.rules.battle_delay);
                } else {
                    self.players[self.next_player.unwrap()].turn_starts_no_sooner_than =
                        Some(now());
                    self.players[self.next_player.unwrap()].turn_starts_no_later_than = Some(now());
                }
            }
        }

        Ok(None)
    }

    pub fn make_move(
        &mut self,
        game_move: Move,
        attacker_dictionary: Option<&WordDict>,
        defender_dictionary: Option<&WordDict>,
        cached_word_judgements: Option<&mut HashMap<String, bool, xxh3::Xxh3Builder>>,
    ) -> Result<Vec<Change>, GamePlayError> {
        let mut changes = vec![];

        match game_move {
            Move::Place {
                player,
                tile,
                position: player_reported_position,
            } => {
                let position = self.board.map_player_coord_to_game(
                    player,
                    player_reported_position,
                    &self.rules.visibility,
                );

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
                self.resolve_attack(
                    player,
                    position,
                    attacker_dictionary,
                    defender_dictionary,
                    cached_word_judgements,
                    &mut changes,
                );

                self.players[player].swap_count = 0;

                Ok(changes)
            }
            Move::Swap {
                player: player_index,
                positions: player_reported_positions,
            } => {
                let positions = [
                    self.board.map_player_coord_to_game(
                        player_index,
                        player_reported_positions[0],
                        &self.rules.visibility,
                    ),
                    self.board.map_player_coord_to_game(
                        player_index,
                        player_reported_positions[1],
                        &self.rules.visibility,
                    ),
                ];

                let player = &mut self.players[player_index];
                let swap_rules = match &self.rules.swapping {
                    rules::Swapping::Contiguous(rules) => Some(rules),
                    rules::Swapping::Universal(rules) => Some(rules),
                    rules::Swapping::None => None,
                };

                match swap_rules {
                    Some(rules::SwapPenalty::Disallowed { allowed_swaps }) => {
                        let player_swaps = player.swap_count;
                        if player_swaps >= *allowed_swaps {
                            return Err(GamePlayError::TooManySwaps {
                                count: match player_swaps + 1 {
                                    2 => "twice".into(),
                                    n => format!("{n} times"),
                                },
                            });
                        }
                    }
                    _ => {}
                }

                let mut swap_result =
                    self.board
                        .swap(player_index, positions, &self.rules.swapping)?;

                player.swap_count += 1;

                match swap_rules {
                    Some(rules::SwapPenalty::Time {
                        swap_threshold,
                        penalties,
                    }) => {
                        let player_swaps = player.swap_count;

                        if player_swaps > *swap_threshold {
                            let penalty_number = player_swaps - swap_threshold - 1;
                            let penalty =
                                penalties.get(penalty_number).or_else(|| penalties.last());
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
                    Some(rules::SwapPenalty::Disallowed { .. }) => {
                        // Handled before move was made
                    }
                    None => {}
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
        attacker_dictionary: Option<&WordDict>,
        defender_dictionary: Option<&WordDict>,
        cached_word_judgements: Option<&mut HashMap<String, bool, xxh3::Xxh3Builder>>,
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

        if let Some(mut battle) = self.judge.battle(
            attacking_words,
            defending_words,
            &self.rules.battle_rules,
            &self.rules.win_condition,
            attacker_dictionary,
            defender_dictionary,
            cached_word_judgements,
        ) {
            battle.battle_number = Some(self.battle_count);
            self.battle_count += 1;

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
                                _ = self.board.set_square(
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

    pub fn next(&self) -> Option<usize> {
        self.next_player
    }

    pub fn filter_game_to_player(&self, player_index: usize) -> (Board, Vec<Change>) {
        let visible_board =
            self.board
                .filter_to_player(player_index, &self.rules.visibility, &self.winner);
        let visible_changes = reporting::filter_to_player(
            &self.recent_changes,
            &self.board,
            &visible_board,
            player_index,
            &self.rules.visibility,
            &self.winner,
        );
        (visible_board, visible_changes)
    }
}
