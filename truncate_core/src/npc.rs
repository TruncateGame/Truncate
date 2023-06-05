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
};

impl Game {
    pub fn brute_force(game: &Game, external_dictionary: Option<&WordDict>) -> PlayerMessage {
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

        let mut best_move: (f32, PlayerMessage) = (f32::MIN, PlayerMessage::Ping);

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

                let our_score = game_clone.eval_board_progress(game.next_player);
                let their_score =
                    game_clone.eval_board_progress((game.next_player + 1) % game.players.len());

                let win_score = game_clone.eval_win(game.next_player);

                let total_score = win_score + move_quality + our_score - their_score;

                if best_move.0 < total_score {
                    best_move = (total_score, PlayerMessage::Place(position, *tile));
                }
            }
        }

        best_move.1
    }

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

    pub fn eval_position_quality(
        &self,
        player: usize,
        position: Coordinate,
        external_dictionary: &WordDict,
    ) -> f32 {
        let (coords, _) = self.board.collect_combanants(player, position);
        let words = self
            .board
            .word_strings(&coords)
            .expect("This should have already been a valid turn");
        let num_words = words.len() as f32;

        let score: f32 = words
            .into_iter()
            .map(|word| {
                if let Some(word_data) = external_dictionary.get(&word.to_lowercase()) {
                    word.len() as f32 + (word_data.extensions.min(100) as f32 / 100.0)
                } else {
                    -3 as f32
                }
            })
            .sum();

        score / num_words
    }

    pub fn eval_win(&self, player: usize) -> f32 {
        if matches!(self.winner, Some(player)) {
            1000.0
        } else {
            0.0
        }
    }
}
