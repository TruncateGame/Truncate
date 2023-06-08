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
};

impl Game {
    pub fn brute_force(game: &Game, external_dictionary: Option<&WordDict>) -> PlayerMessage {
        let evaluation_player = game.next_player;

        let mut best_moves = Game::find_best_moves(game.to_owned(), external_dictionary, 5);

        for potential_move in best_moves.iter_mut() {
            // Winning move
            if potential_move.0 >= 5000.0 {
                return potential_move.1.clone();
            }

            let PlayerMessage::Place(position, tile) = potential_move.1 else {
                continue;
            };

            let mut thisgame = game.to_owned();
            thisgame.instrument_unknown_game_state(evaluation_player);
            thisgame
                .play_turn(
                    Move::Place {
                        player: thisgame.next_player,
                        tile,
                        position,
                    },
                    external_dictionary,
                )
                .expect("Potential move was valid");

            if let Some(best_opponent_move) =
                Game::find_best_moves(thisgame.clone(), external_dictionary, 1).first()
            {
                let PlayerMessage::Place(position, tile) = best_opponent_move.1 else {
                    continue;
                };

                thisgame
                    .play_turn(
                        Move::Place {
                            player: thisgame.next_player,
                            tile,
                            position,
                        },
                        external_dictionary,
                    )
                    .expect("Potential move was valid");

                if let Some(best_subsequent_move) =
                    Game::find_best_moves(thisgame.clone(), external_dictionary, 1).first()
                {
                    potential_move.0 = best_subsequent_move.0;
                }
            }
        }

        best_moves.sort_by(|(a, _), (b, _)| b.partial_cmp(a).unwrap());

        if let Some((_, best_move)) = best_moves.into_iter().next() {
            best_move
        } else {
            panic!("NPC Couldn't perform any move!");
        }
    }

    // Finds the best N moves for whoever the next player is
    pub fn find_best_moves(
        game: Game,
        external_dictionary: Option<&WordDict>,
        num_moves: usize,
    ) -> Vec<(f32, PlayerMessage)> {
        let mut playable_squares = HashSet::new();
        for dock in &game.board.docks {
            let sq = game.board.get(*dock).unwrap();
            if !matches!(sq, Square::Dock(p) if p == game.next_player) {
                continue;
            }

            playable_squares.extend(
                game.board
                    .depth_first_search(*dock)
                    .iter()
                    .flat_map(|sq| sq.neighbors_4())
                    .collect::<HashSet<_>>(),
            );
        }

        playable_squares = playable_squares
            .into_iter()
            .filter(|sq| matches!(game.board.get(*sq), Ok(Square::Land)))
            .collect();

        let mut moves: Vec<(f32, PlayerMessage)> = vec![];

        let playable_tiles: Vec<_> = game
            .players
            .get(game.next_player)
            .unwrap()
            .hand
            .iter()
            .cloned()
            .collect();

        for position in playable_squares {
            for tile in &playable_tiles {
                let mut game_clone = game.to_owned();

                if game_clone
                    .play_turn(
                        Move::Place {
                            player: game.next_player,
                            tile: *tile,
                            position,
                        },
                        external_dictionary,
                    )
                    .is_err()
                {
                    continue;
                }

                let move_quality = game_clone.eval_position_quality(
                    game.next_player,
                    position,
                    external_dictionary.unwrap(),
                );

                let our_progress = game_clone.eval_board_progress(game.next_player);
                let their_progress =
                    game_clone.eval_board_progress((game.next_player + 1) % game.players.len());

                let our_balance = game_clone.eval_single_player_balance(game.next_player) * 2.0;

                let win_score = game_clone.eval_win(game.next_player);

                let total_score =
                    win_score + move_quality + our_progress - their_progress - our_balance;

                moves.push((total_score, PlayerMessage::Place(position, *tile)));
            }
        }

        if !moves.is_empty() {
            moves.sort_by(|(a, _), (b, _)| b.partial_cmp(a).unwrap());

            println!("{moves:#?}");

            let num_moves = moves.len().max(num_moves);
            moves[0..num_moves].to_vec()
        } else {
            vec![]
        }
    }

    pub fn instrument_unknown_game_state(&mut self, evaluation_player: usize) {
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
    pub fn eval_board_progress(&self, player: usize) -> f32 {
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

    pub fn eval_single_player_balance(&self, player: usize) -> f32 {
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

    pub fn eval_position_quality(
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

    pub fn eval_win(&self, player: usize) -> f32 {
        if matches!(self.winner, Some(p) if p == player) {
            100000.0
        } else {
            0.0
        }
    }
}
