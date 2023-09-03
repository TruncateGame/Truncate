use std::{
    collections::{HashMap, HashSet},
    ops::Div,
};

use crate::{
    board::{self, Board, Coordinate, Square},
    game::Game,
    judge::WordDict,
    messages::PlayerMessage,
    moves::Move,
    player::Hand,
    reporting::{BoardChange, Change},
};

mod scoring;

use scoring::BoardScore;

pub struct Arborist {
    assessed: usize,
    prune: bool,
}
impl Arborist {
    pub fn pruning() -> Self {
        Self {
            assessed: 0,
            prune: true,
        }
    }

    pub fn assessed(&self) -> usize {
        self.assessed
    }

    fn exhaustive() -> Self {
        Self {
            assessed: 0,
            prune: false,
        }
    }

    fn prune(&self) -> bool {
        self.prune
    }

    fn tick(&mut self) {
        self.assessed += 1
    }
}

impl Game {
    pub fn best_move(
        game: &Game,
        self_dictionary: Option<&WordDict>,
        opponent_dictionary: Option<&WordDict>,
        depth: usize,
        counter: Option<&mut Arborist>,
    ) -> PlayerMessage {
        let evaluation_player = game.next_player;

        let mut internal_arborist = Arborist::pruning();
        let arborist = counter.unwrap_or_else(|| &mut internal_arborist);

        let (best_score, Some((position, tile))) = Game::minimax(
            game.clone(),
            self_dictionary,
            opponent_dictionary,
            depth,
            depth,
            BoardScore::neg_inf(),
            BoardScore::inf(),
            evaluation_player,
            arborist
        ) else {
            panic!("Couldn't determine a move to play");
        };

        println!("Bot has the hand: {}", game.players[evaluation_player].hand);

        println!("Chosen tree has the score {best_score:#?}");
        if let Some(board) = best_score.board {
            println!("Bot is aiming for the board {board}");
        }

        PlayerMessage::Place(position, tile)
    }

    fn minimax(
        mut game: Game,
        self_dictionary: Option<&WordDict>,
        opponent_dictionary: Option<&WordDict>,
        total_depth: usize,
        depth: usize,
        mut alpha: BoardScore,
        mut beta: BoardScore,
        for_player: usize,
        arborist: &mut Arborist,
    ) -> (BoardScore, Option<(Coordinate, char)>) {
        game.instrument_unknown_game_state(for_player, total_depth, depth);
        let pruning = arborist.prune();

        let mut turn_score =
            |game: &Game, tile: char, position: Coordinate, alpha: BoardScore, beta: BoardScore| {
                arborist.tick();
                let mut next_turn = game.clone();

                let (attacker_dict, defender_dict) = if game.next_player == for_player {
                    (self_dictionary, opponent_dictionary)
                } else {
                    (opponent_dictionary, self_dictionary)
                };

                next_turn
                    .play_turn(
                        Move::Place {
                            player: game.next_player,
                            tile,
                            position,
                        },
                        attacker_dict,
                        defender_dict,
                    )
                    .expect("Should be exploring valid turns");
                Game::minimax(
                    next_turn,
                    self_dictionary,
                    opponent_dictionary,
                    total_depth,
                    depth - 1,
                    alpha,
                    beta,
                    for_player,
                    arborist,
                )
                .0
            };

        if depth == 0 || game.winner.is_some() {
            (game.static_eval(self_dictionary, for_player, depth), None)
        } else if game.next_player == for_player {
            let mut max_score = BoardScore::neg_inf();
            let mut relevant_move = None;

            for (position, tile) in game.possible_moves() {
                let score = turn_score(&game, tile, position, alpha.clone(), beta.clone());

                if score > max_score {
                    max_score = score.clone();
                    relevant_move = Some((position, tile));
                }
                if max_score > alpha {
                    alpha = score;
                }

                if pruning {
                    if beta <= alpha {
                        break;
                    }
                }
            }

            (max_score, relevant_move)
        } else {
            let mut min_score = BoardScore::inf();
            let mut relevant_move = None;

            for (position, tile) in game.possible_moves() {
                let score = turn_score(&game, tile, position, alpha.clone(), beta.clone());

                if score < min_score {
                    min_score = score.clone();
                    relevant_move = Some((position, tile));
                }
                if min_score < beta {
                    beta = score;
                }

                if pruning {
                    if beta <= alpha {
                        break;
                    }
                }
            }

            (min_score, relevant_move)
        }
    }

    fn possible_moves(&self) -> Vec<(Coordinate, char)> {
        let mut playable_tiles: Vec<_> = self
            .players
            .get(self.next_player)
            .unwrap()
            .hand
            .iter()
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        playable_tiles.sort();

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

        let mut coords: Vec<_> = playable_squares
            .into_iter()
            .filter(|sq| matches!(self.board.get(*sq), Ok(Square::Land)))
            .flat_map(|sq| playable_tiles.iter().cloned().map(move |t| (sq, t)))
            .collect();

        // TODO: Build move heuristic to deterministically sort these moves by quality
        coords.sort_by(|a, b| {
            if self.next_player == 0 {
                a.0.cmp(&b.0)
            } else {
                b.0.cmp(&a.0)
            }
        });

        coords
    }

    fn instrument_unknown_game_state(
        &mut self,
        evaluation_player: usize,
        total_depth: usize,
        current_depth: usize,
    ) {
        let unknown_player_index = (evaluation_player + 1) % self.players.len();

        let player = &mut self.players[evaluation_player];

        // Remove timing concerns from the simulated turns
        self.rules.battle_delay = 0;

        // Prevent the evaluation player from being given new tiles in future turns
        player.hand_capacity = 0;

        // If we're past the first layer,
        // use a combo tile for the eval player to reduce permutations.
        if current_depth == total_depth - 1 {
            let alias = self.judge.set_alias(player.hand.0.clone());
            // Add enough that using them doesn't cause them to run out.
            player.hand = Hand(vec![alias; current_depth]);
        }

        // Prevent the NPC from making decisions based on the opponent's tiles,
        // assume all valid plays.
        self.players[unknown_player_index].hand = Hand(vec!['*']);
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WordQualityScores {
    word_length: f32,
    word_validity: f32,
    word_extensibility: f32,
}

impl Div<f32> for WordQualityScores {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self {
            word_length: self.word_length / rhs,
            word_validity: self.word_validity / rhs,
            word_extensibility: self.word_extensibility / rhs,
        }
    }
}

// Evaluation functions
impl Game {
    /// Top-most evaluation function for looking at the game and calculating a score
    pub fn static_eval(
        &self,
        external_dictionary: Option<&WordDict>,
        for_player: usize,
        depth: usize,
    ) -> BoardScore {
        let word_quality = if let Some(external_dictionary) = external_dictionary {
            self.eval_word_quality(external_dictionary, for_player)
        } else {
            WordQualityScores::default()
        };
        let for_opponent = (for_player + 1) % self.players.len();

        BoardScore::default()
            .turn_number(depth)
            .word_quality(word_quality)
            .self_frontline(self.eval_board_frontline(for_player))
            .opponent_frontline(self.eval_board_frontline(for_opponent))
            .self_progress(self.eval_board_positions(for_player))
            .opponent_progress(self.eval_board_positions(for_opponent))
            .self_defense(self.eval_defense(for_player))
            .self_win(self.winner == Some(for_player))
            .opponent_win(self.winner == Some(for_opponent))
            .board(self.board.clone())
    }

    /// How many <player> tiles are there, and how far down the board are they?
    pub fn eval_board_positions(&self, player: usize) -> f32 {
        let mut score = 0.0;
        let max_score = (self.board.height() * self.board.width()) as f32;

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

        (score / max_score).min(1.0)
    }

    /// How far forward are our furthest tiles?
    pub fn eval_board_frontline(&self, player: usize) -> f32 {
        let mut score = 0.0;

        for (rownum, row) in self.board.squares.iter().enumerate() {
            let row_score = if player == 0 {
                rownum as f32
            } else {
                (&self.board.squares.len() - rownum) as f32
            };

            for sq in row {
                if matches!(sq, Square::Occupied(p, _) if player == *p) {
                    if row_score > score {
                        score = row_score;
                    }
                }
            }
        }

        score / self.board.squares.len() as f32
    }

    pub fn eval_defense(&self, player: usize) -> f32 {
        let towns = self.board.towns.clone();
        let attacker = (player + 1) % self.players.len();
        let max_score = self.board.width() + self.board.height();

        let mut score = max_score;

        for town in towns.into_iter().filter(
            |town| matches!(self.board.get(*town), Ok(Square::Town{player: p, ..}) if player == p),
        ) {
            let Some(town_dist) = self.board.distance_from_attack(town, attacker) else { continue };
            if town_dist < score {
                score = town_dist;
            }
        }

        score as f32 / max_score as f32
    }

    pub fn eval_word_quality(
        &self,
        external_dictionary: &WordDict,
        player: usize,
    ) -> WordQualityScores {
        let mut assessed_tiles: HashSet<Coordinate> = HashSet::new();
        let mut num_words = 0;

        let mut word_scores = WordQualityScores::default();

        for (y, row) in self.board.squares.iter().enumerate() {
            for (x, sq) in row.iter().enumerate() {
                if matches!(sq, Square::Occupied(p, _) if player == *p) {
                    if assessed_tiles.contains(&Coordinate { x, y }) {
                        continue;
                    }

                    let word_coords = self.board.get_words(Coordinate { x, y });
                    assessed_tiles.extend(word_coords.iter().flatten());

                    let words = self
                        .board
                        .word_strings(&word_coords)
                        .expect("There should be words from a tile");

                    num_words += words.len();
                    for word in words {
                        let resolved = self.judge.valid(
                            word,
                            &crate::rules::WinCondition::Elimination,
                            Some(external_dictionary),
                            None,
                        );
                        if let Some(resolved_word) = resolved {
                            if let Some(word_data) =
                                external_dictionary.get(&resolved_word.to_lowercase())
                            {
                                word_scores.word_length +=
                                    (((resolved_word.len() - 1) as f32) / 5.0).min(1.0);

                                word_scores.word_extensibility +=
                                    (word_data.extensions.min(100) as f32) / 100.0;

                                word_scores.word_validity += 1.0;
                            }
                        }
                    }
                }
            }
        }

        if num_words > 0 {
            word_scores / num_words as f32
        } else {
            word_scores
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        bag::TileBag,
        board::Board,
        judge::{Judge, WordData},
        player::Player,
    };

    pub static TESTING_DICT: &str = include_str!("../../../word_freqs/final_wordlist.txt");

    /// Build an (expensive) word dictionary using the real game data.
    fn dict() -> WordDict {
        let mut valid_words = HashMap::new();
        let lines = TESTING_DICT.lines();

        for line in lines {
            let mut chunks = line.split(' ');

            let mut word = chunks.next().unwrap().to_string();
            let objectionable = word.chars().next() == Some('*');
            if objectionable {
                word.remove(0);
            }

            valid_words.insert(
                word,
                WordData {
                    extensions: chunks.next().unwrap().parse().unwrap(),
                    rel_freq: chunks.next().unwrap().parse().unwrap(),
                    objectionable,
                },
            );
        }

        valid_words
    }

    /// Read a PlayerMessage and play the relevant turn on the given game
    fn enact_move(game: &mut Game, msg: PlayerMessage, dict: &WordDict) {
        let Some(next_move) = (match msg {
            PlayerMessage::Place(position, tile) => Some(Move::Place {
                player: game.next_player,
                tile,
                position,
            }),
            PlayerMessage::Swap(from, to) => Some(Move::Swap {
                player: game.next_player,
                positions: [from, to],
            }),
            _ => None,
        }) else {
            panic!("Unhandle-able message");
        };
        game.play_turn(next_move, Some(dict), Some(dict))
            .expect("Move was valid");
    }

    /// Stub out a game for us to test with using the supplied board.
    /// The attacking player will be given the tiles A E T S
    fn test_game(board: &str, hand: &str) -> Game {
        let b = Board::from_string(board);
        let next_player = 1;
        let mut bag = TileBag::default();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b.clone(),
            bag,
            players,
            next_player,
            ..Game::new(3, 1)
        };
        game.players[next_player].hand = Hand(hand.chars().collect());
        game.start();

        game
    }

    /// Run the move algorithm and return the result, along with diagnostics
    /// on how many branches were evaluated.
    fn best_test_move(game: &Game, dict: &WordDict, depth: usize) -> (PlayerMessage, usize, usize) {
        let mut exhaustive_arbor = Arborist::exhaustive();
        let exhaustive_best_move = Game::best_move(
            &game,
            Some(&dict),
            Some(&dict),
            depth,
            Some(&mut exhaustive_arbor),
        );

        let mut pruned_arbor = Arborist::pruning();
        let pruned_best_move = Game::best_move(
            &game,
            Some(&dict),
            Some(&dict),
            depth,
            Some(&mut pruned_arbor),
        );

        assert_eq!(
            pruned_best_move,
            exhaustive_best_move,
            "Pruning should not impact the resolved move for {}",
            game.board.to_string()
        );

        (
            pruned_best_move,
            pruned_arbor.assessed,
            exhaustive_arbor.assessed,
        )
    }

    fn eval_npc_result<'a>(
        hand: &str,
        initial_board: &'a str,
        depth: usize,
        dict: &WordDict,
    ) -> (&'a str, String) {
        let mut game = test_game(initial_board, hand);
        let (best_move, pruned_checks, total_checks) = best_test_move(&game, &dict, depth);

        enact_move(&mut game, best_move.clone(), &dict);

        let next_board = game.board.to_string();
        (
            initial_board,
            format!(
                "Evaluating:\n  - {} possible leaves\n  - {} after pruning\n  - Move: {best_move}\n\n{next_board}",
                total_checks, pruned_checks
            ),
        )
    }

    #[test]
    fn generic_scoring_tests() {
        let dict = dict();

        let game_a = test_game(
            r###"
            ~~ ~~ ~~ |0 ~~ ~~ ~~
            __ __ S0 O0 __ __ __
            __ __ T0 __ __ __ __
            __ __ R0 __ __ __ __
            __ __ __ T1 __ H1 __
            __ __ __ A1 __ A1 __
            __ __ __ R1 A1 T1 __
            ~~ ~~ ~~ |1 ~~ ~~ ~~
            "###,
            "A",
        );
        let score_a = game_a.static_eval(Some(&dict), 1, 1);
        let game_b = test_game(
            r###"
            ~~ ~~ ~~ |0 ~~ ~~ ~~
            __ __ S0 O0 __ __ __
            __ __ T0 __ __ __ __
            __ __ R0 __ __ __ __
            __ __ __ T1 __ __ __
            __ __ __ A1 __ __ R1
            __ __ __ R1 A1 T1 E1
            ~~ ~~ ~~ |1 ~~ ~~ ~~
            "###,
            "A",
        );
        let score_b = game_b.static_eval(Some(&dict), 1, 1);

        insta::with_settings!({
            description => format!("Game A:\n{}\n\nGame B:\n{}", game_a.board.to_string(), game_b.board.to_string()),
            omit_expression => true
        }, {
            insta::assert_snapshot!(format!("(Total score) A: {:#?} / B: {:#?}", score_a, score_b), @r###"
            (Total score) A: BoardScore {
                infinity: false,
                neg_infinity: false,
                turn_number: 1,
                word_quality: WordQualityScores {
                    word_length: 0.4,
                    word_validity: 1.0,
                    word_extensibility: 1.0,
                },
                self_frontline: 0.5,
                opponent_frontline: 0.375,
                self_progress: 0.35714287,
                opponent_progress: 0.125,
                self_defense: 1.0,
                self_win: false,
                opponent_win: false,
            } / B: BoardScore {
                infinity: false,
                neg_infinity: false,
                turn_number: 1,
                word_quality: WordQualityScores {
                    word_length: 0.4,
                    word_validity: 1.0,
                    word_extensibility: 1.0,
                },
                self_frontline: 0.5,
                opponent_frontline: 0.375,
                self_progress: 0.32142857,
                opponent_progress: 0.125,
                self_defense: 1.0,
                self_win: false,
                opponent_win: false,
            }
            "###);
        });
    }

    #[test]
    fn defense_scoring_tests() {
        let game_a = test_game(
            r###"
            ~~ ~~ ~~ |0 ~~ ~~ ~~
            __ __ S0 O0 __ __ __
            __ __ __ __ __ __ __
            __ __ __ __ __ __ __
            __ __ A1 T1 __ H1 __
            __ __ __ A1 __ A1 __
            #1 #1 __ R1 A1 T1 #1
            ~~ ~~ ~~ |1 ~~ ~~ ~~
            "###,
            "A",
        );
        let score_a = game_a.eval_defense(1);
        let game_b = test_game(
            r###"
            ~~ ~~ ~~ |0 ~~ ~~ ~~
            __ __ S0 O0 __ __ __
            __ __ __ __ __ __ __
            __ __ __ __ __ __ __
            __ H1 A1 T1 __ H1 __
            __ __ __ A1 __ A1 __
            #1 #1 __ R1 A1 T1 #1
            ~~ ~~ ~~ |1 ~~ ~~ ~~
            "###,
            "A",
        );
        let score_b = game_b.eval_defense(1);

        insta::with_settings!({
            description => format!("Game A:\n{}\n\nGame B:\n{}", game_a.board.to_string(), game_b.board.to_string()),
            omit_expression => true
        }, {
            insta::assert_snapshot!(format!("(Defense score) A: {} / B: {}", score_a, score_b), @"(Defense score) A: 0.4 / B: 1");
        });
    }

    #[test]
    fn generic_npc_tests() {
        let dict = dict();

        /*
         * For all tests below, we are simulating the best move for player 1
         */

        /* - - - - - - - - - - - - - - - - - */

        {
            let (board, result) = eval_npc_result(
                "SEAT",
                r###"
                ~~ ~~ |0 ~~ ~~
                __ S0 O0 __ __
                __ T0 __ __ __
                __ R0 __ __ __
                __ __ T1 __ __
                __ __ A1 __ __
                __ __ R1 __ __
                ~~ ~~ |1 ~~ ~~
                "###,
                3,
                &dict,
            );

            insta::with_settings!({
                description => board,
                omit_expression => true
            }, {
                insta::assert_snapshot!(result, @r###"
                Evaluating:
                  - 1337 possible leaves
                  - 478 after pruning
                  - Move: Place S at (2, 3)

                ~~ ~~ |0 ~~ ~~
                __ __ O0 __ __
                __ __ __ __ __
                __ __ S1 __ __
                __ __ T1 __ __
                __ __ A1 __ __
                __ __ R1 __ __
                ~~ ~~ |1 ~~ ~~
                "###);
            });
        }

        /* - - - - - - - - - - - - - - - - - */

        {
            let (board, result) = eval_npc_result(
                "SEAT",
                r###"
                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                __ A0 __ __ __
                __ R0 __ __ __
                __ __ T1 __ __
                __ __ A1 __ __
                __ __ R1 __ __
                ~~ ~~ |1 ~~ ~~
                "###,
                3,
                &dict,
            );

            insta::with_settings!({
                description => board,
                omit_expression => true
            }, {
                insta::assert_snapshot!(result, @r###"
                Evaluating:
                  - 1366 possible leaves
                  - 404 after pruning
                  - Move: Place A at (3, 5)

                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                __ A0 __ __ __
                __ R0 __ __ __
                __ __ T1 __ __
                __ __ A1 A1 __
                __ __ R1 __ __
                ~~ ~~ |1 ~~ ~~
                "###);
            });
        }

        /* - - - - - - - - - - - - - - - - - */

        {
            let (board, result) = eval_npc_result(
                "SEAT",
                r###"
                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                __ A0 __ __ __
                __ __ __ __ __
                __ X1 T1 __ __
                __ __ A1 __ __
                __ __ R1 __ __
                ~~ ~~ |1 ~~ ~~
                "###,
                3,
                &dict,
            );

            insta::with_settings!({
                description => board,
                omit_expression => true
            }, {
                insta::assert_snapshot!(result, @r###"
                Evaluating:
                  - 1384 possible leaves
                  - 760 after pruning
                  - Move: Place A at (3, 4)

                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                __ A0 __ __ __
                __ __ __ __ __
                __ X1 T1 A1 __
                __ __ A1 __ __
                __ __ R1 __ __
                ~~ ~~ |1 ~~ ~~
                "###);
            });
        }

        /* - - - - - - - - - - - - - - - - - */

        {
            let (board, result) = eval_npc_result(
                "SEAT",
                r###"
                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                D0 A0 __ __ __
                __ __ __ __ __
                T1 E1 E1 __ __
                __ __ A1 __ __
                R1 I1 T1 __ __
                ~~ ~~ |1 ~~ ~~
                "###,
                3,
                &dict,
            );

            insta::with_settings!({
                description => board,
                omit_expression => true
            }, {
                insta::assert_snapshot!(result, @r###"
                Evaluating:
                  - 1399 possible leaves
                  - 379 after pruning
                  - Move: Place S at (3, 4)

                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                D0 A0 __ __ __
                __ __ __ __ __
                T1 E1 E1 S1 __
                __ __ A1 __ __
                R1 I1 T1 __ __
                ~~ ~~ |1 ~~ ~~
                "###);
            });
        }

        /* - - - - - - - - - - - - - - - - - */

        {
            let (board, result) = eval_npc_result(
                "SEAT",
                r###"
                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                D0 A0 Q0 __ __
                __ __ __ __ __
                Q1 E1 E1 __ __
                __ __ A1 __ __
                R1 I1 T1 __ __
                ~~ ~~ |1 ~~ ~~
                "###,
                3,
                &dict,
            );

            insta::with_settings!({
                description => board,
                omit_expression => true
            }, {
                insta::assert_snapshot!(result, @r###"
                Evaluating:
                  - 1400 possible leaves
                  - 477 after pruning
                  - Move: Place S at (1, 3)

                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                D0 A0 Q0 __ __
                __ __ __ __ __
                __ __ E1 __ __
                __ __ A1 __ __
                R1 I1 T1 __ __
                ~~ ~~ |1 ~~ ~~
                "###);
            });
        }

        /* - - - - - - - - - - - - - - - - - */

        {
            let (board, result) = eval_npc_result(
                "MATERSK",
                r###"
                ~~ ~~ |0 ~~ ~~ ~~ ~~
                __ __ R0 __ __ __ __
                __ __ A0 __ __ __ __
                __ __ O0 S0 __ __ __
                __ __ C0 T0 __ __ __
                __ __ __ A0 __ __ __
                __ __ __ B0 __ __ __
                __ __ I1 __ __ __ __
                __ __ D1 A1 T1 E1 S1
                __ __ E1 __ __ __ __
                ~~ ~~ |1 ~~ ~~ ~~ ~~
                "###,
                3,
                &dict,
            );

            insta::with_settings!({
                description => board,
                omit_expression => true
            }, {
                insta::assert_snapshot!(result, @r###"
                Evaluating:
                  - 12345 possible leaves
                  - 2550 after pruning
                  - Move: Place A at (6, 7)

                ~~ ~~ |0 ~~ ~~ ~~ ~~
                __ __ R0 __ __ __ __
                __ __ A0 __ __ __ __
                __ __ O0 S0 __ __ __
                __ __ C0 T0 __ __ __
                __ __ __ A0 __ __ __
                __ __ __ B0 __ __ __
                __ __ I1 __ __ __ A1
                __ __ D1 A1 T1 E1 S1
                __ __ E1 __ __ __ __
                ~~ ~~ |1 ~~ ~~ ~~ ~~
                "###);
            });
        }

        /* - - - - - - - - - - - - - - - - - */

        {
            let (board, result) = eval_npc_result(
                "LDDEUQU",
                r###"
                ~~ ~~ ~~ ~~ ~~ |0 ~~ ~~ ~~ ~~ ~~
                ~~ #0 #0 #0 #0 E0 #0 #0 #0 #0 ~~
                ~~ __ __ __ __ N0 __ __ __ __ ~~
                ~~ __ __ __ __ I0 __ __ __ __ ~~
                ~~ __ __ __ __ __ __ __ __ __ ~~
                ~~ __ __ __ __ __ __ __ __ __ ~~
                ~~ __ __ __ __ __ __ __ __ __ ~~
                ~~ __ __ __ __ N1 __ __ __ __ ~~
                ~~ __ __ __ __ E1 __ __ __ __ ~~
                ~~ #1 #1 #1 #1 E1 #1 #1 #1 #1 ~~
                ~~ ~~ ~~ ~~ ~~ |1 ~~ ~~ ~~ ~~ ~~
                "###,
                4,
                &dict,
            );

            insta::with_settings!({
                description => board,
                omit_expression => true
            }, {
                insta::assert_snapshot!(result, @r###"
                Evaluating:
                  - 5080 possible leaves
                  - 1101 after pruning
                  - Move: Place E at (6, 7)

                ~~ ~~ ~~ ~~ ~~ |0 ~~ ~~ ~~ ~~ ~~
                ~~ #0 #0 #0 #0 E0 #0 #0 #0 #0 ~~
                ~~ __ __ __ __ N0 __ __ __ __ ~~
                ~~ __ __ __ __ I0 __ __ __ __ ~~
                ~~ __ __ __ __ __ __ __ __ __ ~~
                ~~ __ __ __ __ __ __ __ __ __ ~~
                ~~ __ __ __ __ __ __ __ __ __ ~~
                ~~ __ __ __ __ N1 E1 __ __ __ ~~
                ~~ __ __ __ __ E1 __ __ __ __ ~~
                ~~ #1 #1 #1 #1 E1 #1 #1 #1 #1 ~~
                ~~ ~~ ~~ ~~ ~~ |1 ~~ ~~ ~~ ~~ ~~
                "###);
            });
        }
    }
}
