use std::{
    collections::{HashMap, HashSet},
    ops::Div,
};

use crate::{
    board::{BoardDistances, Coordinate, Square},
    game::Game,
    judge::WordDict,
    messages::PlayerMessage,
    moves::Move,
    player::Hand,
};

pub mod scoring;

use scoring::BoardScore;
use xxhash_rust::xxh3;

use self::scoring::NPCParams;

pub struct Arborist {
    assessed: usize,
    prune: bool,
    cap: usize,
}
impl Arborist {
    pub fn pruning() -> Self {
        Self {
            assessed: 0,
            prune: true,
            cap: std::usize::MAX,
        }
    }

    pub fn assessed(&self) -> usize {
        self.assessed
    }

    pub fn capped(&mut self, cap: usize) {
        self.cap = cap;
    }

    /// Useful for testing, dead in production code
    #[allow(dead_code)]
    fn exhaustive() -> Self {
        Self {
            assessed: 0,
            prune: false,
            cap: std::usize::MAX,
        }
    }

    fn prune(&self) -> bool {
        self.prune
    }

    fn tick(&mut self) {
        self.assessed += 1
    }
}

pub struct Caches {
    cached_floods: HashMap<Vec<u64>, (BoardDistances, BoardDistances), xxh3::Xxh3Builder>,
    cached_scores: HashMap<(Move, usize), usize, xxh3::Xxh3Builder>,
    cached_words: HashMap<String, bool, xxh3::Xxh3Builder>,
}

impl Caches {
    pub fn new() -> Self {
        Self {
            cached_floods: HashMap::with_hasher(xxh3::Xxh3Builder::new()),
            cached_scores: HashMap::with_hasher(xxh3::Xxh3Builder::new()),
            cached_words: HashMap::with_hasher(xxh3::Xxh3Builder::new()),
        }
    }
}

impl Game {
    pub fn best_move(
        game: &Game,
        self_dictionary: Option<&WordDict>,
        opponent_dictionary: Option<&WordDict>,
        depth: usize,
        counter: Option<&mut Arborist>,
        log: bool,
        npc_params: &NPCParams,
    ) -> (PlayerMessage, BoardScore) {
        let evaluation_player = game
            .next_player
            .expect("Minimax only works in non-periodic playmodes");
        let mut internal_arborist = if npc_params.pruning {
            Arborist::pruning()
        } else {
            Arborist::exhaustive()
        };
        let mut caches = Caches::new();

        let base_score = game.static_eval(
            self_dictionary,
            evaluation_player,
            depth,
            &mut caches,
            npc_params,
        );

        let mut run_mini = |partial_depth: usize, swapping: bool, arborist: &mut Arborist| {
            Game::minimax(
                game.clone(),
                self_dictionary,
                opponent_dictionary,
                partial_depth,
                partial_depth,
                0,
                BoardScore::neg_inf(),
                BoardScore::inf(),
                evaluation_player,
                swapping,
                arborist,
                &mut caches,
                npc_params,
            )
        };

        let mut latest = None;
        let mut looked = 0;
        let arborist = counter.unwrap_or_else(|| &mut internal_arborist);

        let swaplatest = Some(run_mini(1, true, arborist));

        arborist.assessed = 0;
        for d in 1..depth {
            let maybelatest = Some(run_mini(d, false, arborist));
            if latest.is_some() && arborist.assessed > arborist.cap {
                break;
            }
            latest = maybelatest;
            looked = d;
        }

        if arborist.assessed < arborist.cap {
            let maybelatest = Some(run_mini(depth, false, arborist));
            if arborist.assessed < arborist.cap {
                latest = maybelatest;
                looked = depth;
            }
        }

        let Some((mut best_score, Some(mut next_move))) = latest else {
            panic!("Expected a valid position to be playable");
        };

        if log {
            tracing::debug!("- - - - - - - - - - - - - - - - - - - - - - -");
            tracing::debug!(
                "Bot checked {} boards, going to a depth of {looked}",
                arborist.assessed()
            );
            tracing::debug!("Bot has the hand: {}", game.players[evaluation_player].hand);

            tracing::debug!("Chosen tree for {next_move:?} has the score {best_score:#?}");
            if let Some(board) = &best_score.board {
                tracing::debug!("Bot is aiming for the board {board}");
            }
        }

        if let Some((swapscore, Some(swapmove))) = swaplatest {
            if log {
                tracing::debug!("Chosen swap tree for {swapmove:?} has the score {swapscore:#?}");
                if let Some(board) = &swapscore.board {
                    tracing::debug!("Bot is aiming for the board {board}");
                }
                tracing::debug!(
                    "Swap is better than move? {:?}",
                    swapscore.partial_cmp(&best_score)
                );
                tracing::debug!(
                    "Swap is better than base? {:?}",
                    swapscore.partial_cmp(&base_score)
                );
            }

            if swapscore > base_score || swapscore >= best_score {
                best_score = swapscore;
                next_move = swapmove;
            }
        }

        match next_move {
            Move::Place {
                player,
                tile,
                position,
            } => (PlayerMessage::Place(position, tile), best_score),
            Move::Swap { player, positions } => {
                (PlayerMessage::Swap(positions[0], positions[1]), best_score)
            }
        }
    }

    fn minimax(
        mut game: Game,
        self_dictionary: Option<&WordDict>,
        opponent_dictionary: Option<&WordDict>,
        total_depth: usize,
        depth: usize,
        layer: usize,
        mut alpha: BoardScore,
        mut beta: BoardScore,
        for_player: usize,
        swapping: bool,
        arborist: &mut Arborist,
        caches: &mut Caches,
        npc_params: &NPCParams,
    ) -> (BoardScore, Option<Move>) {
        game.instrument_unknown_game_state(for_player, total_depth, depth);
        let pruning = arborist.prune();

        if depth == 0 || game.winner.is_some() {
            return (
                game.static_eval(self_dictionary, for_player, depth, caches, npc_params),
                None,
            );
        }

        let mut possible_moves = if swapping {
            game.possible_swaps()
        } else {
            game.possible_moves()
        };

        possible_moves.sort_by_cached_key(|m| {
            std::usize::MAX
                - caches
                    .cached_scores
                    .get(&(m.clone(), layer))
                    .unwrap_or(&std::usize::MAX)
        });

        let mut turn_score = |game: &Game, next_move: Move, alpha: BoardScore, beta: BoardScore| {
            arborist.tick();
            if arborist.assessed > arborist.cap {
                return None;
            }
            let mut next_turn = game.clone();

            let next_player = game.next_player.unwrap();

            let (attacker_dict, defender_dict) = if next_player == for_player {
                (self_dictionary, opponent_dictionary)
            } else {
                (opponent_dictionary, self_dictionary)
            };

            let is_players_turn = next_player == for_player;

            if next_turn
                .play_turn(
                    next_move.clone(),
                    attacker_dict,
                    defender_dict,
                    Some(&mut caches.cached_words),
                )
                .is_err()
            {
                return None;
            }
            let score = Game::minimax(
                next_turn,
                self_dictionary,
                opponent_dictionary,
                total_depth,
                depth - 1,
                layer + 1,
                alpha,
                beta,
                for_player,
                swapping,
                arborist,
                caches,
                npc_params,
            )
            .0;

            if is_players_turn {
                caches
                    .cached_scores
                    .insert((next_move, layer), score.usize_rank());
            } else {
                caches
                    .cached_scores
                    .insert((next_move, layer), std::usize::MAX - score.usize_rank());
            }

            Some(score)
        };

        if game.next_player.unwrap() == for_player {
            let mut max_score = BoardScore::neg_inf();
            let mut relevant_move = None;

            for next_move in possible_moves {
                let Some(score) = turn_score(&game, next_move.clone(), alpha.clone(), beta.clone())
                else {
                    break;
                };

                if score > max_score {
                    max_score = score.clone();
                    relevant_move = Some(next_move);
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

            for next_move in possible_moves {
                let Some(score) = turn_score(&game, next_move.clone(), alpha.clone(), beta.clone())
                else {
                    break;
                };

                if score < min_score {
                    min_score = score.clone();
                    relevant_move = Some(next_move);
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

    fn possible_moves(&self) -> Vec<Move> {
        let mut playable_tiles: Vec<_> = self
            .players
            .get(self.next_player.unwrap())
            .unwrap()
            .hand
            .iter()
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        playable_tiles.sort();

        let for_player = self.next_player.unwrap();
        let playable_squares = self
            .board
            .playable_positions(for_player, &self.rules.truncation);

        let mut coords: Vec<_> = playable_squares
            .into_iter()
            .flat_map(|sq| playable_tiles.iter().cloned().map(move |t| (sq, t)))
            .collect();

        // TODO: Build move heuristic to deterministically sort these moves by quality
        coords.sort_by(|a, b| {
            if self.next_player.unwrap() == 0 {
                a.0.cmp(&b.0)
            } else {
                b.0.cmp(&a.0)
            }
        });

        coords
            .into_iter()
            .map(|(position, tile)| Move::Place {
                player: for_player,
                tile,
                position,
            })
            .collect()
    }

    fn possible_swaps(&self) -> Vec<Move> {
        let for_player = self.next_player.unwrap();
        let playable_clusters = self.board.swappable_positions(for_player);

        let mut playable_swaps = vec![];

        for cluster in playable_clusters {
            for (a_pos, a_tile) in &cluster {
                for (b_pos, b_tile) in &cluster {
                    if a_tile == b_tile {
                        continue;
                    }
                    playable_swaps.push(Move::Swap {
                        player: for_player,
                        positions: [*a_pos, *b_pos],
                    });
                }
            }
        }

        playable_swaps
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
        // use a combo tile for the eval player, to reduce permutations.
        if current_depth + 1 == total_depth {
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

pub enum DefenceEvalType {
    Attackable,
    Direct,
}

// Evaluation functions
impl Game {
    /// Top-most evaluation function for looking at the game and calculating a score
    pub fn static_eval(
        &self,
        external_dictionary: Option<&WordDict>,
        for_player: usize,
        depth: usize,
        caches: &mut Caches,
        npc_params: &NPCParams,
    ) -> BoardScore {
        let word_quality = if let Some(external_dictionary) = external_dictionary {
            self.eval_word_quality(external_dictionary, for_player, caches)
        } else {
            WordQualityScores::default()
        };
        let for_opponent = (for_player + 1) % self.players.len();

        let shape = self.board.get_shape();
        let (self_attack_distances, opponent_attack_distances) =
            if let Some(res) = caches.cached_floods.get(&shape) {
                res
            } else {
                let self_attack_distances = self.board.flood_fill_attacks(for_player);
                let opponent_attack_distances = self.board.flood_fill_attacks(for_opponent);
                caches.cached_floods.insert(
                    shape.clone(),
                    (self_attack_distances, opponent_attack_distances),
                );
                caches.cached_floods.get(&shape).unwrap()
            };

        let mut score = BoardScore::default()
            .npc_params(*npc_params)
            .turn_number(depth)
            .word_quality(word_quality)
            .raced_defense(self.eval_min_raced_distance_to_towns(
                &opponent_attack_distances,
                &self_attack_distances,
                for_player,
            ))
            .raced_attack(
                1.0 - self.eval_min_raced_distance_to_towns(
                    &self_attack_distances,
                    &opponent_attack_distances,
                    for_opponent,
                ),
            )
            .self_defense(self.eval_min_distance_to_towns(
                &opponent_attack_distances,
                for_player,
                DefenceEvalType::Attackable,
            ))
            .self_attack(
                1.0 - self.eval_min_distance_to_towns(
                    &self_attack_distances,
                    for_opponent,
                    DefenceEvalType::Attackable,
                ),
            )
            .direct_defence(self.eval_min_distance_to_towns(
                &opponent_attack_distances,
                for_player,
                DefenceEvalType::Direct,
            ))
            .direct_attack(
                1.0 - self.eval_min_distance_to_towns(
                    &self_attack_distances,
                    for_opponent,
                    DefenceEvalType::Direct,
                ),
            )
            .self_win(self.winner == Some(for_player))
            .opponent_win(self.winner == Some(for_opponent));

        score = score.board(self.board.clone());

        score
    }

    pub fn eval_min_distance_to_towns(
        &self,
        distances: &BoardDistances,
        defender: usize,
        defence_type: DefenceEvalType,
    ) -> f32 {
        let towns = self.board.towns.clone();
        let max_score = self.board.width() + self.board.height();

        let defense_towns = towns
            .into_iter()
            .filter(
                |town_pt| matches!(self.board.get(*town_pt), Ok(Square::Town{player: p, ..}) if defender == p),
            ).collect::<Vec<_>>();

        let score = defense_towns
            .iter()
            .map(|town_pt| match defence_type {
                DefenceEvalType::Attackable => {
                    distances.attackable_distance(town_pt).unwrap_or(max_score)
                }
                DefenceEvalType::Direct => distances.direct_distance(town_pt).unwrap_or(max_score),
            })
            .min();

        (score.unwrap_or(max_score) as f32) / (max_score as f32)
    }

    pub fn eval_min_raced_distance_to_towns(
        &self,
        attackers_tiles: &BoardDistances,
        defenders_tiles: &BoardDistances,
        defender: usize,
    ) -> f32 {
        let towns = self.board.towns.clone();
        let max_score = self.board.width() + self.board.height();

        let defense_towns = towns
            .into_iter()
            .filter(
                |town_pt| matches!(self.board.get(*town_pt), Ok(Square::Town{player: p, ..}) if defender == p),
            ).collect::<Vec<_>>();

        let score = defense_towns
            .iter()
            .map(|town_pt| {
                let mut can_defend_in = defenders_tiles
                    .attackable_distance(town_pt)
                    .unwrap_or(max_score);
                let mut can_attack_in = attackers_tiles
                    .attackable_distance(town_pt)
                    .unwrap_or(max_score);

                // Fudge the numbers so that even races look bad for the defender
                can_defend_in += 2;
                can_attack_in = can_attack_in.saturating_sub(2);

                can_defend_in.saturating_sub(can_attack_in)
            })
            .max();

        ((max_score as f32) - score.unwrap_or(max_score) as f32) / (max_score as f32)
    }

    pub fn eval_word_quality(
        &self,
        external_dictionary: &WordDict,
        player: usize,
        caches: &mut Caches,
    ) -> WordQualityScores {
        let mut assessed_tiles: HashSet<Coordinate> = HashSet::new();
        let mut num_words = 0;

        let mut word_scores = WordQualityScores::default();

        for (y, row) in self.board.squares.iter().enumerate() {
            for (x, sq) in row.iter().enumerate() {
                if matches!(sq, Square::Occupied{ player: p, ..} if player == *p) {
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
                            &mut Some(&mut caches.cached_words),
                        );
                        if let Some(resolved_word) = resolved {
                            if let Some(word_data) =
                                external_dictionary.get(&resolved_word.to_lowercase())
                            {
                                word_scores.word_length +=
                                    (((resolved_word.len() - 1) as f32) / 5.0).min(1.0);

                                word_scores.word_extensibility +=
                                    (word_data.extensions as f32).sqrt().min(100.0) / 100.0;

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

    use crate::{bag::TileBag, board::Board, judge::WordData, player::Player, rules::GameRules};

    pub static TESTING_DICT: &str = include_str!("../../../dict_builder/final_wordlist.txt");

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
                player: game.next_player.unwrap(),
                tile,
                position,
            }),
            PlayerMessage::Swap(from, to) => Some(Move::Swap {
                player: game.next_player.unwrap(),
                positions: [from, to],
            }),
            _ => None,
        }) else {
            panic!("Unhandle-able message");
        };
        game.play_turn(next_move, Some(dict), Some(dict), None)
            .expect("Move was valid");
    }

    /// Stub out a game for us to test with using the supplied board.
    /// The attacking player will be given the tiles A E T S
    fn test_game(board: &str, hand: &str) -> Game {
        let b = Board::from_string(board);
        let next_player = 1;
        let mut bag = TileBag::latest(None).1;
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b.clone(),
            bag,
            players,
            player_turn_count: vec![0, 0],
            next_player: Some(next_player),
            ..Game::new_legacy(3, 1, None, GameRules::generation(0)) // TODO: update snapshots to rules v1
        };
        game.players[next_player].hand = Hand(hand.chars().collect());
        game.start();

        game
    }

    /// Run the move algorithm and return the result, along with diagnostics
    /// on how many branches were evaluated.
    fn best_test_move(game: &Game, dict: &WordDict, depth: usize) -> (PlayerMessage, usize, usize) {
        let mut exhaustive_arbor = Arborist::exhaustive();
        let (exhaustive_best_move, _) = Game::best_move(
            &game,
            Some(&dict),
            Some(&dict),
            depth,
            Some(&mut exhaustive_arbor),
            false,
            &NPCParams::default(),
        );

        let mut pruned_arbor = Arborist::pruning();
        let (pruned_best_move, _) = Game::best_move(
            &game,
            Some(&dict),
            Some(&dict),
            depth,
            Some(&mut pruned_arbor),
            false,
            &NPCParams::default(),
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
        let score_a =
            game_a.static_eval(Some(&dict), 1, 1, &mut Caches::new(), &NPCParams::default());
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
        let score_b =
            game_b.static_eval(Some(&dict), 1, 1, &mut Caches::new(), &NPCParams::default());

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
                    word_extensibility: 0.4700235,
                },
                raced_defense: 0.0,
                raced_attack: 1.0,
                self_defense: 1.0,
                self_attack: 0.0,
                direct_defence: 1.0,
                direct_attack: 0.0,
                self_win: false,
                opponent_win: false,
            } / B: BoardScore {
                infinity: false,
                neg_infinity: false,
                turn_number: 1,
                word_quality: WordQualityScores {
                    word_length: 0.4,
                    word_validity: 1.0,
                    word_extensibility: 0.5438283,
                },
                raced_defense: 0.0,
                raced_attack: 1.0,
                self_defense: 1.0,
                self_attack: 0.0,
                direct_defence: 1.0,
                direct_attack: 0.0,
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
        let dists = game_a.board.flood_fill_attacks(0);
        let score_a = game_a.eval_min_distance_to_towns(&dists, 1, DefenceEvalType::Attackable);
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
        let dists = game_b.board.flood_fill_attacks(0);
        let score_b = game_b.eval_min_distance_to_towns(&dists, 1, DefenceEvalType::Attackable);

        insta::with_settings!({
            description => format!("Game A:\n{}\n\nGame B:\n{}", game_a.board.to_string(), game_b.board.to_string()),
            omit_expression => true
        }, {
            insta::assert_snapshot!(format!("(Defense score) A: {} / B: {}", score_a, score_b), @"(Defense score) A: 0.4 / B: 1");
        });
    }

    #[test]
    fn defense_racing_tests() {
        {
            let mostly_defended = test_game(
                r###"
            ~~ ~~ ~~ |0 ~~ ~~ ~~
            __ __ S0 O0 __ __ __
            __ __ __ __ __ __ __
            __ __ __ __ __ __ __
            __ __ __ __ __ __ __
            __ __ __ __ __ __ __
            __ __ A1 T1 __ H1 __
            __ __ __ A1 __ A1 __
            #1 #1 __ R1 A1 T1 #1
            ~~ ~~ ~~ |1 ~~ ~~ ~~
            "###,
                "A",
            );
            let opponent_dists = mostly_defended.board.flood_fill_attacks(0);
            let own_dists = mostly_defended.board.flood_fill_attacks(1);
            assert_eq!(
                opponent_dists.attackable_distance(&Coordinate { x: 0, y: 8 }),
                Some(8)
            );
            assert_eq!(
                own_dists.attackable_distance(&Coordinate { x: 0, y: 8 }),
                Some(3)
            );
            let defense_score =
                mostly_defended.eval_min_raced_distance_to_towns(&opponent_dists, &own_dists, 1);

            assert_eq!(defense_score, 1.0);
        }

        {
            let even_race = test_game(
                r###"
            ~~ ~~ ~~ |0 ~~ ~~ ~~
            __ __ S0 O0 __ __ __
            __ __ __ __ __ __ __
            #1 __ __ __ __ __ __
            __ __ __ __ __ __ __
            __ __ A1 T1 __ H1 __
            __ __ __ A1 __ A1 __
            #1 #1 __ R1 A1 T1 #1
            ~~ ~~ ~~ |1 ~~ ~~ ~~
            "###,
                "A",
            );
            let opponent_dists = even_race.board.flood_fill_attacks(0);
            let own_dists = even_race.board.flood_fill_attacks(1);
            assert_eq!(
                opponent_dists.attackable_distance(&Coordinate { x: 0, y: 3 }),
                Some(3)
            );
            assert_eq!(
                own_dists.attackable_distance(&Coordinate { x: 0, y: 3 }),
                Some(3)
            );
            let defense_score =
                even_race.eval_min_raced_distance_to_towns(&opponent_dists, &own_dists, 1);

            let expected_max = (even_race.board.width() + even_race.board.height()) as f32;
            let expected_score = (expected_max - 4.0) / expected_max;

            assert_eq!(defense_score, expected_score);
        }
    }

    #[test]
    fn test_npc_determinism() {
        let dict = dict();

        let eval = || {
            let game = test_game(
                r###"
                ~~ ~~ |0 ~~
                ~~ S0 O0 ~~
                ~~ T0 A0 Y0
                ~~ A0 ~~ ~~
                ~~ R0 __ ~~
                ~~ __ A1 |1
                ~~ ~~ |1 ~~
                ~~ ~~ ~~ ~~
                "###,
                "XZF",
            );
            let mut pruned_arbor = Arborist::pruning();
            let (pruned_best_move, _) = Game::best_move(
                &game,
                Some(&dict),
                Some(&dict),
                2,
                Some(&mut pruned_arbor),
                false,
                &NPCParams::default(),
            );

            pruned_best_move
        };

        let base = eval();
        println!("{:#?}", base);
        for _ in 0..200 {
            assert_eq!(eval(), base);
        }
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
                  - 1592 possible leaves
                  - 438 after pruning
                  - Move: Place S at (3, 5)

                ~~ ~~ |0 ~~ ~~
                __ S0 O0 __ __
                __ T0 __ __ __
                __ R0 __ __ __
                __ __ T1 __ __
                __ __ A1 S1 __
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
                  - 1618 possible leaves
                  - 415 after pruning
                  - Move: Place S at (3, 5)

                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                __ A0 __ __ __
                __ R0 __ __ __
                __ __ T1 __ __
                __ __ A1 S1 __
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
                  - 1608 possible leaves
                  - 450 after pruning
                  - Move: Place S at (3, 5)

                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                __ A0 __ __ __
                __ __ __ __ __
                __ X1 T1 __ __
                __ __ A1 S1 __
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
                  - 1611 possible leaves
                  - 481 after pruning
                  - Move: Place T at (1, 5)

                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                D0 A0 __ __ __
                __ __ __ __ __
                T1 E1 E1 __ __
                __ T1 A1 __ __
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
                  - 1656 possible leaves
                  - 455 after pruning
                  - Move: Place E at (3, 6)

                ~~ ~~ |0 ~~ ~~
                __ T0 O0 __ __
                D0 A0 Q0 __ __
                __ __ __ __ __
                Q1 E1 E1 __ __
                __ __ A1 __ __
                R1 I1 T1 E1 __
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
                  - 13594 possible leaves
                  - 1671 after pruning
                  - Move: Place S at (3, 9)

                ~~ ~~ |0 ~~ ~~ ~~ ~~
                __ __ R0 __ __ __ __
                __ __ A0 __ __ __ __
                __ __ O0 S0 __ __ __
                __ __ C0 T0 __ __ __
                __ __ __ A0 __ __ __
                __ __ __ B0 __ __ __
                __ __ I1 __ __ __ __
                __ __ D1 A1 T1 E1 S1
                __ __ E1 S1 __ __ __
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
                  - 6130 possible leaves
                  - 802 after pruning
                  - Move: Place E at (4, 7)

                ~~ ~~ ~~ ~~ ~~ |0 ~~ ~~ ~~ ~~ ~~
                ~~ #0 #0 #0 #0 E0 #0 #0 #0 #0 ~~
                ~~ __ __ __ __ N0 __ __ __ __ ~~
                ~~ __ __ __ __ I0 __ __ __ __ ~~
                ~~ __ __ __ __ __ __ __ __ __ ~~
                ~~ __ __ __ __ __ __ __ __ __ ~~
                ~~ __ __ __ __ __ __ __ __ __ ~~
                ~~ __ __ __ E1 N1 __ __ __ __ ~~
                ~~ __ __ __ __ E1 __ __ __ __ ~~
                ~~ #1 #1 #1 #1 E1 #1 #1 #1 #1 ~~
                ~~ ~~ ~~ ~~ ~~ |1 ~~ ~~ ~~ ~~ ~~
                "###);
            });
        }
    }

    #[test]
    fn npc_winning_tests() {
        let dict = dict();

        /* - - - - - - - - - - - - - - - - - */

        {
            let (board, result) = eval_npc_result(
                "A",
                r###"
                ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~
                ~~ ~~ ~~ ~~ __ ~~ __ ~~ ~~ ~~ ~~
                ~~ ~~ |0 __ __ I1 __ __ ~~ ~~ ~~
                ~~ #0 __ S1 __ O1 __ __ __ ~~ ~~
                ~~ ~~ __ U1 T1 S1 __ ~~ __ ~~ ~~
                ~~ __ G1 N1 U1 __ __ __ __ __ ~~
                ~~ Y1 U1 __ S1 I1 B1 __ ~~ ~~ ~~
                ~~ E1 ~~ __ H1 O1 L1 D1 #1 __ ~~
                ~~ ~~ E1 L1 __ __ A1 A1 ~~ ~~ ~~
                ~~ ~~ S1 E1 R1 E1 |1 #1 ~~ ~~ ~~
                ~~ ~~ T1 A1 ~~ ~~ ~~ ~~ ~~ ~~ ~~
                ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~
                "###,
                2,
                &dict,
            );

            insta::with_settings!({
                description => board,
                omit_expression => true
            }, {
                insta::assert_snapshot!(result, @r###"
                Evaluating:
                  - 61 possible leaves
                  - 47 after pruning
                  - Move: Place A at (2, 3)

                ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~
                ~~ ~~ ~~ ~~ __ ~~ __ ~~ ~~ ~~ ~~
                ~~ ~~ |0 __ __ I1 __ __ ~~ ~~ ~~
                ~~ ⊭0 A1 S1 __ O1 __ __ __ ~~ ~~
                ~~ ~~ __ U1 T1 S1 __ ~~ __ ~~ ~~
                ~~ __ G1 N1 U1 __ __ __ __ __ ~~
                ~~ Y1 U1 __ S1 I1 B1 __ ~~ ~~ ~~
                ~~ E1 ~~ __ H1 O1 L1 D1 #1 __ ~~
                ~~ ~~ E1 L1 __ __ A1 A1 ~~ ~~ ~~
                ~~ ~~ S1 E1 R1 E1 |1 #1 ~~ ~~ ~~
                ~~ ~~ T1 A1 ~~ ~~ ~~ ~~ ~~ ~~ ~~
                ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~ ~~
                "###);
            });
        }
    }
}
