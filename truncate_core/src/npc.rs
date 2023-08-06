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
        external_dictionary: Option<&WordDict>,
        depth: usize,
        counter: Option<&mut Arborist>,
    ) -> PlayerMessage {
        let evaluation_player = game.next_player;

        let mut internal_arborist = Arborist::pruning();
        let arborist = counter.unwrap_or_else(|| &mut internal_arborist);

        let (_, Some((position, tile))) = Game::minimax(
            game.clone(),
            external_dictionary,
            depth,
            depth,
            f32::NEG_INFINITY,
            f32::INFINITY,
            evaluation_player,
            arborist
        ) else {
            panic!("Couldn't determine a move to play");
        };

        PlayerMessage::Place(position, tile)
    }

    fn minimax(
        mut game: Game,
        external_dictionary: Option<&WordDict>,
        total_depth: usize,
        depth: usize,
        mut alpha: f32,
        mut beta: f32,
        for_player: usize,
        arborist: &mut Arborist,
    ) -> (f32, Option<(Coordinate, char)>) {
        game.instrument_unknown_game_state(for_player, total_depth, depth);
        let pruning = arborist.prune();

        let mut turn_score =
            |game: &Game, tile: char, position: Coordinate, alpha: f32, beta: f32| {
                arborist.tick();
                let mut next_turn = game.clone();
                next_turn
                    .play_turn(
                        Move::Place {
                            player: game.next_player,
                            tile,
                            position,
                        },
                        external_dictionary,
                    )
                    .expect("Should be exploring valid turns");
                Game::minimax(
                    next_turn,
                    external_dictionary,
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
            (
                game.static_eval(external_dictionary, for_player, depth),
                None,
            )
        } else if game.next_player == for_player {
            let mut max_score = f32::NEG_INFINITY;
            let mut relevant_move = None;

            for (position, tile) in game.possible_moves() {
                let score = turn_score(&game, tile, position, alpha, beta);

                if score > max_score {
                    max_score = score;
                    relevant_move = Some((position, tile));
                }
                alpha = alpha.max(score);

                if pruning {
                    if beta <= alpha {
                        break;
                    }
                }
            }

            (max_score, relevant_move)
        } else {
            let mut min_score = f32::INFINITY;
            let mut relevant_move = None;

            for (position, tile) in game.possible_moves() {
                let score = turn_score(&game, tile, position, alpha, beta);

                if score < min_score {
                    min_score = score;
                    relevant_move = Some((position, tile));
                }
                beta = beta.min(score);

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

// Evaluation functions
impl Game {
    /// Top-most evaluation function for looking at the game and calculating a score
    pub fn static_eval(
        &self,
        external_dictionary: Option<&WordDict>,
        for_player: usize,
        depth: usize,
    ) -> f32 {
        let word_quality = if let Some(external_dictionary) = external_dictionary {
            self.eval_word_quality(external_dictionary, for_player)
        } else {
            0.0
        };

        let our_frontline = self.eval_board_frontline(for_player);
        let their_frontline =
            self.eval_board_frontline((for_player + 1) % self.players.len()) * 2.0;

        let our_progress = self.eval_board_positions(for_player);
        let their_progress = self.eval_board_positions((for_player + 1) % self.players.len()) * 2.0;

        let our_defense = self.eval_defense(for_player) * 1.5;

        let win_score = self.eval_win(for_player, depth);

        let total_score = win_score + word_quality + our_defense + our_frontline + our_progress
            - their_frontline
            - their_progress;

        // println!("win_score{win_score} + word_quality{word_quality} + our_frontline{our_frontline} + our_progress{our_progress} - their_progress{their_progress} - our_balance{our_balance}");

        total_score
    }

    /// How many <player> tiles are there, and how far down the board are they?
    pub fn eval_board_positions(&self, player: usize) -> f32 {
        let mut score = 0.0;

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

        score
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

        score
    }

    pub fn eval_defense(&self, player: usize) -> f32 {
        let towns = self.board.towns.clone();
        let attacker = (player + 1) % self.players.len();
        let max_score = self.board.width() + self.board.height();
        let mut num_towns = 0;

        let score: usize = towns
            .into_iter()
            .filter(|town| matches!(self.board.get(*town), Ok(Square::Town{player: p, ..}) if player == p))
            .map(|town| {
                num_towns += 1;
                self.board
                    .distance_from_attack(town, attacker)
                    .unwrap_or(max_score)
            })
            .sum();

        if num_towns > 0 {
            score as f32 / num_towns as f32
        } else {
            0.0
        }
    }

    pub fn eval_word_quality(&self, external_dictionary: &WordDict, player: usize) -> f32 {
        let mut assessed_tiles: HashSet<Coordinate> = HashSet::new();
        let mut score = 0.0;
        let mut num_words = 0;

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
                        if let Some(word_data) = external_dictionary.get(&word.to_lowercase()) {
                            score += word.len() as f32
                                + (word.len().saturating_sub(2) * 2) as f32
                                + (word_data.extensions.min(25) as f32 / 100.0);

                            score *= word_data.rel_freq;

                            if word.len() == 2 {
                                score -= 2.0;
                            }
                        } else {
                            score -= word.len().pow(2) as f32;
                        }
                    }
                }
            }
        }

        if num_words > 0 {
            score / num_words as f32
        } else {
            0.0
        }
    }

    /// Did someone win?
    pub fn eval_win(&self, player: usize, depth: usize) -> f32 {
        match self.winner {
            Some(p) if p == player => 10000.0 * (depth + 1) as f32,
            Some(_) => -10000.0 * (depth + 1) as f32,
            None => 0.0,
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

    pub static TESTING_DICT: &str = include_str!("../../word_freqs/final_wordlist.txt");

    /// Build an (expensive) word dictionary using the real game data.
    fn dict() -> WordDict {
        let mut valid_words = HashMap::new();
        let lines = TESTING_DICT.lines();

        for line in lines {
            let mut chunks = line.split(' ');
            valid_words.insert(
                chunks.next().unwrap().to_string(),
                WordData {
                    extensions: chunks.next().unwrap().parse().unwrap(),
                    rel_freq: chunks.next().unwrap().parse().unwrap(),
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
        game.play_turn(next_move, Some(dict))
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
        let exhaustive_best_move =
            Game::best_move(&game, Some(&dict), depth, Some(&mut exhaustive_arbor));

        let mut pruned_arbor = Arborist::pruning();
        let pruned_best_move = Game::best_move(&game, Some(&dict), depth, Some(&mut pruned_arbor));

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
            insta::assert_snapshot!(format!("(Total score) A: {} / B: {}", score_a, score_b), @"(Total score) A: 9.227806 / B: 6.5617027");
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
            insta::assert_snapshot!(format!("(Defense score) A: {} / B: {}", score_a, score_b), @"(Defense score) A: 9.333333 / B: 15");
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
                  - 693 after pruning
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
                  - 559 after pruning
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
                  - 798 after pruning
                  - Move: Place A at (1, 3)

                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                __ A0 __ __ __
                __ __ __ __ __
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
                  - 633 after pruning
                  - Move: Place A at (0, 5)

                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                D0 A0 __ __ __
                __ __ __ __ __
                T1 E1 E1 __ __
                A1 __ A1 __ __
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
                  - 443 after pruning
                  - Move: Place T at (2, 3)

                ~~ ~~ |0 ~~ ~~
                __ __ __ __ __
                __ __ __ __ __
                __ __ T1 __ __
                Q1 E1 E1 __ __
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
                  - 5356 after pruning
                  - Move: Place T at (1, 7)

                ~~ ~~ |0 ~~ ~~ ~~ ~~
                __ __ R0 __ __ __ __
                __ __ A0 __ __ __ __
                __ __ O0 S0 __ __ __
                __ __ C0 T0 __ __ __
                __ __ __ A0 __ __ __
                __ __ __ B0 __ __ __
                __ T1 I1 __ __ __ __
                __ __ D1 A1 T1 E1 S1
                __ __ E1 __ __ __ __
                ~~ ~~ |1 ~~ ~~ ~~ ~~
                "###);
            });
        }
    }
}
