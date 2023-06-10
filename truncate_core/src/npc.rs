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
    reporting::Change,
};

pub struct Arborist {
    assessed: usize,
    prune: bool,
}
impl Arborist {
    fn pruning() -> Self {
        Self {
            assessed: 0,
            prune: true,
        }
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
            f32::MIN,
            f32::MAX,
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
        depth: usize,
        mut alpha: f32,
        mut beta: f32,
        for_player: usize,
        arborist: &mut Arborist,
    ) -> (f32, Option<(Coordinate, char)>) {
        game.instrument_unknown_game_state(for_player);
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
                    depth - 1,
                    alpha,
                    beta,
                    for_player,
                    arborist,
                )
                .0
            };

        if depth == 0 || game.winner.is_some() {
            (game.static_eval(external_dictionary, for_player), None)
        } else if game.next_player == for_player {
            let mut max_score = f32::MIN;
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
            let mut min_score = f32::MAX;
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
        let playable_tiles: Vec<_> = self
            .players
            .get(self.next_player)
            .unwrap()
            .hand
            .iter()
            .cloned()
            .collect();

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

    fn instrument_unknown_game_state(&mut self, evaluation_player: usize) {
        let unknown_player = (evaluation_player + 1) % self.players.len();

        // Prevent the evaluation player from being given new tiles in future turns
        self.players[evaluation_player].hand_capacity = 0;

        // Prevent the NPC from making decisions based on the opponent's tiles,
        // assume all valid plays.
        self.players[unknown_player].hand = Hand("*".chars().collect());
    }
}

// Evaluation functions
impl Game {
    /// Top-most evaluation function for looking at the game and calculating a score
    fn static_eval(&self, _external_dictionary: Option<&WordDict>, for_player: usize) -> f32 {
        // TODO: Replace with a static board evaluation of all words
        // let move_quality =
        //     game.eval_position_quality(for_player, position, external_dictionary.unwrap());

        let our_frontline = self.eval_board_frontline(for_player);
        let our_progress = self.eval_board_positions(for_player) * 0.2;
        let their_progress = self.eval_board_positions((for_player + 1) % self.players.len());

        let our_balance = self.eval_single_player_balance(for_player) * 1.5;

        let win_score = self.eval_win(for_player);

        let total_score = win_score /* + move_quality */ + our_frontline + our_progress
            - their_progress
            - our_balance;

        total_score
    }

    /// How many <player> tiles are there, and how far down the board are they?
    fn eval_board_positions(&self, player: usize) -> f32 {
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
    fn eval_board_frontline(&self, player: usize) -> f32 {
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

    /// How balanced are <player>'s tiles, left to right?
    fn eval_single_player_balance(&self, player: usize) -> f32 {
        let mut left = 0;
        let mut right = 0;
        let mut count = 0;
        let midpoint = self.board.squares[0].len() / 2;

        for row in self.board.squares.iter() {
            for (colnum, sq) in row.iter().enumerate() {
                if matches!(sq, Square::Occupied(p, _) if player == *p) {
                    count += 1;
                    if colnum < midpoint {
                        left += midpoint - colnum;
                    } else if colnum > midpoint {
                        right += colnum - midpoint;
                    }
                }
            }
        }

        if count > 0 {
            left.abs_diff(right) as f32 / count as f32
        } else {
            0.0
        }
    }

    /// Are the words at the proposed position valid? How long and extensible are they?
    fn eval_position_quality(
        &self,
        player: usize,
        position: Coordinate,
        external_dictionary: &WordDict,
    ) -> f32 {
        let (coords, _) = self.board.collect_combanants(player, position);

        if coords.is_empty() {
            return 0.0;
        }

        let words = self
            .board
            .word_strings(&coords)
            .expect("This should have already been a valid turn");
        let num_words = words.len() as f32;

        let score: f32 = words
            .into_iter()
            .map(|word| {
                if let Some(word_data) = external_dictionary.get(&word.to_lowercase()) {
                    word.len() as f32 + (word_data.extensions.min(25) as f32 / 100.0)
                } else {
                    -3 as f32
                }
            })
            .sum();

        score / num_words
    }

    /// Did someone win?
    fn eval_win(&self, player: usize) -> f32 {
        match self.winner {
            Some(p) if p == player => 100000.0,
            Some(_) => -100000.0,
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
        let pruned_best_move = Game::best_move(&game, Some(&dict), 3, Some(&mut pruned_arbor));

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
                  - 3595 possible leaves
                  - 1748 after pruning
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
                  - 3650 possible leaves
                  - 1342 after pruning
                  - Move: Place E at (3, 5)

                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                __ A0 __ __ __
                __ R0 __ __ __
                __ __ T1 __ __
                __ __ A1 E1 __
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
                  - 3768 possible leaves
                  - 2180 after pruning
                  - Move: Place S at (1, 3)

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
                  - 3813 possible leaves
                  - 973 after pruning
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
                  - 3784 possible leaves
                  - 1344 after pruning
                  - Move: Place S at (2, 3)

                ~~ ~~ |0 ~~ ~~
                __ __ __ __ __
                __ __ __ __ __
                __ __ S1 __ __
                Q1 E1 E1 __ __
                __ __ A1 __ __
                R1 I1 T1 __ __
                ~~ ~~ |1 ~~ ~~
                "###);
            });
        }
    }
}
