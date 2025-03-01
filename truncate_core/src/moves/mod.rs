pub mod packing;

use serde::{Deserialize, Serialize};

use super::board::Coordinate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Move {
    Place {
        player: usize,
        tile: char,
        position: Coordinate,
    },
    Swap {
        player: usize,
        positions: [Coordinate; 2],
    },
}

impl PartialEq for Move {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Place {
                    player: l_player,
                    tile: l_tile,
                    position: l_position,
                },
                Self::Place {
                    player: r_player,
                    tile: r_tile,
                    position: r_position,
                },
            ) => l_player == r_player && l_tile == r_tile && l_position == r_position,
            (
                Self::Swap {
                    player: l_player,
                    positions: l_positions,
                },
                Self::Swap {
                    player: r_player,
                    positions: r_positions,
                },
            ) => {
                l_player == r_player
                    && (l_positions == r_positions
                        || (l_positions[0] == r_positions[1] && l_positions[1] == r_positions[0]))
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bag::TileBag;
    use crate::board::{Board, Coordinate, Square, SquareValidity};
    use crate::error::GamePlayError;
    use crate::game::Game;
    use crate::judge::Judge;
    use crate::player::Player;
    use crate::reporting::*;
    use crate::reporting::{BoardChange, BoardChangeAction};
    use crate::rules::GameRules;

    use super::super::bag::tests as TileUtils;
    use super::*;

    pub fn short_dict() -> Judge {
        Judge::new(vec![
            "BIG".into(),
            "FAT".into(),
            "JOLLY".into(),
            "AND".into(),
            "SILLY".into(),
            "FOLK".into(),
            "ARTS".into(),
        ])
    }
    pub fn b_dict() -> Judge {
        Judge::new(vec!["BIG".into()])
    }

    #[test]
    fn invalid_placement_locations() {
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let position = Coordinate { x: 11, y: 11 };
        let out_of_bounds = Move::Place {
            player: 0,
            tile: 'A',
            position,
        };
        let mut game = Game {
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 3, None, GameRules::generation(0))
        };
        assert_eq!(
            game.make_move(out_of_bounds, None, None, None),
            Err(GamePlayError::OutSideBoardDimensions { position })
        );

        let position = Coordinate { x: 11, y: 1 };
        let out_of_bounds = Move::Place {
            player: 0,
            tile: 'A',
            position,
        };
        assert_eq!(
            game.make_move(out_of_bounds, None, None, None),
            Err(GamePlayError::OutSideBoardDimensions { position })
        );

        let position = Coordinate { x: 2, y: 1 };
        let dead = Move::Place {
            player: 0,
            tile: 'A',
            position,
        };
        assert_eq!(
            game.make_move(dead, None, None, None),
            Err(GamePlayError::InvalidPosition { position })
        );
    }

    #[test]
    fn can_place_and_swap() {
        let mut bag = TileUtils::a_b_bag();
        let players = vec![Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0))];

        let mut game = Game {
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 3, None, GameRules::generation(0))
        };

        // Can't accidentally place beside opponent's artifact on first turn
        assert_eq!(
            game.make_move(
                Move::Place {
                    player: 0,
                    tile: 'A',
                    position: Coordinate { x: 2, y: 5 },
                },
                None,
                None,
                None,
            ),
            Err(GamePlayError::OpponentStartPlace)
        );

        // Places beside artifact
        let changes = game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 3, y: 2 },
            },
            None,
            None,
            None,
        );
        assert_eq!(
            changes.clone().map(|c| {
                c.into_iter()
                    .filter(|c| matches!(c, Change::Board(_)))
                    .collect::<Vec<_>>()
            }),
            Ok(vec![Change::Board(BoardChange {
                detail: BoardChangeDetail {
                    square: Square::Occupied {
                        player: 0,
                        tile: 'A',
                        validity: SquareValidity::Unknown,
                        foggy: false
                    },
                    coordinate: Coordinate { x: 3, y: 2 },
                },
                action: BoardChangeAction::Added
            })])
        );
        assert_eq!(
            changes.map(|c| {
                c.into_iter()
                    .filter_map(|c| {
                        if let Change::Hand(c) = c {
                            // TODO: skipping test for c.added since it is random
                            Some((c.player, c.removed))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            }),
            Ok(vec![(0, vec!['A'])])
        );

        // Can't place on the same place again
        assert_eq!(
            game.make_move(
                Move::Place {
                    player: 0,
                    tile: 'B',
                    position: Coordinate { x: 3, y: 2 }
                },
                None,
                None,
                None
            ),
            Err(GamePlayError::OccupiedPlace)
        );

        // Can't place at a diagonal
        assert_eq!(
            game.make_move(
                Move::Place {
                    player: 0,
                    tile: 'B',
                    position: Coordinate { x: 4, y: 3 }
                },
                None,
                None,
                None
            ),
            Err(GamePlayError::NonAdjacentPlace)
        );

        // Can place directly above
        assert_eq!(
            game.make_move(
                Move::Place {
                    player: 0,
                    tile: 'B',
                    position: Coordinate { x: 3, y: 3 }
                },
                None,
                None,
                None
            )
            .map(|c| {
                c.into_iter()
                    .filter(|c| matches!(c, Change::Board(_)))
                    .collect::<Vec<_>>()
            }),
            Ok(vec![Change::Board(BoardChange {
                detail: BoardChangeDetail {
                    square: Square::Occupied {
                        player: 0,
                        tile: 'B',
                        validity: SquareValidity::Unknown,
                        foggy: false
                    },
                    coordinate: Coordinate { x: 3, y: 3 },
                },
                action: BoardChangeAction::Added
            })])
        );

        // Can't place on the same place again
        assert_eq!(
            game.make_move(
                Move::Place {
                    player: 0,
                    tile: 'B',
                    position: Coordinate { x: 3, y: 3 }
                },
                None,
                None,
                None
            ),
            Err(GamePlayError::OccupiedPlace)
        );

        // Can swap
        assert_eq!(
            game.make_move(
                Move::Swap {
                    player: 0,
                    positions: [Coordinate { x: 3, y: 2 }, Coordinate { x: 3, y: 3 }]
                },
                None,
                None,
                None
            ),
            Ok(vec![
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied {
                            player: 0,
                            tile: 'B',
                            validity: SquareValidity::Unknown,
                            foggy: false
                        },
                        coordinate: Coordinate { x: 3, y: 2 },
                    },
                    action: BoardChangeAction::Swapped
                }),
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied {
                            player: 0,
                            tile: 'A',
                            validity: SquareValidity::Unknown,
                            foggy: false
                        },
                        coordinate: Coordinate { x: 3, y: 3 },
                    },
                    action: BoardChangeAction::Swapped
                })
            ])
        );
    }

    #[test]
    fn invalid_player_or_tile() {
        let mut bag = TileBag::latest(None).1;
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 3, None, GameRules::generation(0))
        };

        assert_eq!(
            game.make_move(
                Move::Place {
                    player: 2,
                    tile: 'A',
                    position: Coordinate { x: 3, y: 3 }
                },
                None,
                None,
                None
            ),
            Err(GamePlayError::NonExistentPlayer { index: 2 })
        );

        assert_eq!(
            game.make_move(
                Move::Place {
                    player: 0,
                    tile: '&',
                    position: Coordinate { x: 2, y: 1 }
                },
                None,
                None,
                None
            ),
            Err(GamePlayError::PlayerDoesNotHaveTile {
                player: 0,
                tile: '&'
            })
        );
    }

    #[test]
    fn collect_combanants() {
        let middle = Coordinate { x: 2, y: 2 };

        let left_defender: Vec<Coordinate> = (2..=4).map(|y| Coordinate { x: 1, y }).collect();
        let right_defender: Vec<Coordinate> = (2..=4).map(|y| Coordinate { x: 3, y }).collect();
        let middle_defender: Vec<Coordinate> = (3..=4).map(|y| Coordinate { x: 2, y }).collect();
        let middle_attacker: Vec<Coordinate> =
            (0..=2).rev().map(|y| Coordinate { x: 2, y }).collect();
        let left_attacker: Vec<Coordinate> =
            (0..=2).rev().map(|x| Coordinate { x, y: 2 }).collect();
        let cross_defender: Vec<Coordinate> = (1..=3).map(|x| Coordinate { x, y: 3 }).collect();
        let short_cross_defender: Vec<Coordinate> =
            (2..=3).map(|x| Coordinate { x, y: 3 }).collect();

        // There are at most 4 squares contributing combatants.
        // Either 1 attacker with 1, 2, or 3 defenders
        // 2 attackers with 1 or 2 defenders
        // Note, 3 attackers are impossible because the letter being placed will combine two of the words into one

        // 1v1
        let mut one_v_one = Board::from_string(
            "__ |0 M0 __ __\n\
             __ __ D0 __ __\n\
             __ __ __ __ __\n\
             __ __ M1 __ __\n\
             __ __ D1 |1 __",
        );
        one_v_one
            .set(middle, 0, 'A', Some(&short_dict().builtin_dictionary))
            .unwrap();

        assert_eq!(
            one_v_one.collect_combanants(0, middle, &GameRules::generation(0)),
            (vec![middle_attacker.clone()], vec![middle_defender.clone()])
        );

        // 1v2
        let mut one_v_two = Board::from_string(
            "__ |0 M0 __ __\n\
             __ __ D0 __ __\n\
             __ L1 __ R1 __\n\
             __ F1 __ T1 __\n\
             __ D1 R1 D1 |1",
        );
        one_v_two
            .set(middle, 0, 'A', Some(&short_dict().builtin_dictionary))
            .unwrap();

        assert_eq!(
            one_v_two.collect_combanants(0, middle, &GameRules::generation(0)),
            (
                vec![middle_attacker.clone()],
                vec![right_defender.clone(), left_defender.clone()],
            )
        );

        // 1v3
        let mut one_v_three = Board::from_string(
            "__ |0 M0 __ __\n\
             __ __ D0 __ __\n\
             __ L1 __ R1 __\n\
             __ F1 M1 T1 __\n\
             __ D1 D1 D1 |1",
        );
        one_v_three
            .set(middle, 0, 'A', Some(&short_dict().builtin_dictionary))
            .unwrap();

        assert_eq!(
            one_v_three.collect_combanants(0, middle, &GameRules::generation(0)),
            (
                vec![middle_attacker.clone()],
                vec![
                    right_defender.clone(),
                    middle_defender.clone(),
                    cross_defender.clone(),
                    left_defender.clone(),
                ]
            )
        );

        // 2v2
        let mut two_v_two = Board::from_string(
            "X0 X0 M0 |0 __\n\
             X0 __ D0 __ __\n\
             L0 F0 __ R1 __\n\
             __ __ M1 T1 __\n\
             __ __ D1 D1 |1",
        );
        two_v_two
            .set(middle, 0, 'A', Some(&short_dict().builtin_dictionary))
            .unwrap();
        assert_eq!(
            two_v_two.collect_combanants(0, middle, &GameRules::generation(0)),
            (
                vec![middle_attacker, left_attacker],
                vec![right_defender, middle_defender, short_cross_defender],
            )
        );
    }

    #[test]
    fn collect_symbolic_combanants() {
        let c = |x: usize, y: usize| Coordinate::new(x, y);
        let mut board = Board::from_string(
            "__ __ #0 __ __\n\
             __ __ D0 A0 __\n\
             __ |0 __ #0 __\n\
             __ __ M1 __ __\n\
             __ __ D1 |1 __",
        );
        board
            .set(c(2, 2), 1, 'A', Some(&short_dict().builtin_dictionary))
            .unwrap();

        assert_eq!(
            board.collect_combanants(1, c(2, 2), &GameRules::generation(2)),
            (
                vec![vec![c(2, 2), c(2, 3), c(2, 4)]],
                vec![vec![c(3, 1), c(2, 1)], vec![c(3, 2)], vec![c(1, 2)]],
            )
        );
    }

    #[test]
    fn resolve_successful_attack() {
        let b = Board::from_string(
            "__ S0 X0 |0 __\n\
             __ T0 __ __ __\n\
             __ R0 __ __ __\n\
             __ __ I1 __ __\n\
             __ __ T1 |1 __",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(1, 1, None, GameRules::generation(0))
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            game.board.to_string(),
            "__ S0 X0 |0 __\n\
             __ T0 __ __ __\n\
             __ R0 __ __ __\n\
             __ A0 __ __ __\n\
             __ __ __ |1 __",
        )
    }

    #[test]
    fn resolve_failed_attack() {
        let b = Board::from_string(
            "__ X0 X0 |0 __\n\
             __ T0 __ __ __\n\
             __ R0 __ __ __\n\
             __ __ I1 __ __\n\
             __ __ T1 |1 __",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 1, None, GameRules::generation(0))
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            game.board.to_string(),
            "__ __ X0 |0 __\n\
             __ __ __ __ __\n\
             __ __ __ __ __\n\
             __ __ I1 __ __\n\
             __ __ T1 |1 __",
        )
    }

    #[test]
    fn resolve_truncation() {
        let b = Board::from_string(
            "__ S0 X0 |0 __\n\
             __ T0 __ __ __\n\
             __ R0 __ X1 __\n\
             __ __ B1 X1 __\n\
             __ __ I1 __ __\n\
             __ __ G1 |1 __",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut test_bag = TileUtils::trivial_bag();
        let test_players = vec![
            Player::new("A".into(), 0, 7, &mut test_bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut test_bag, None, (0, 0, 0)),
        ];

        assert_eq!(players, test_players);
        assert_eq!(bag, test_bag);

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 1, None, GameRules::generation(0))
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
            None,
            None,
            None,
        )
        .unwrap();

        for letter in ['B', 'X', 'X'] {
            test_bag.return_tile(letter);
        }
        assert_eq!(game.bag, test_bag);

        assert_eq!(
            game.board.to_string(),
            "__ S0 X0 |0 __\n\
             __ T0 __ __ __\n\
             __ R0 __ __ __\n\
             __ A0 __ __ __\n\
             __ __ I1 __ __\n\
             __ __ G1 |1 __",
        );
    }

    #[test]
    fn resolve_explosion() {
        let b = Board::from_string(
            "__ __ S0 |0 __\n\
             __ __ T0 __ __\n\
             __ __ R0 __ __\n\
             __ B1 __ X1 __\n\
             __ I1 __ X1 __\n\
             __ G1 X1 X1 __\n\
             ~~ ~~ |1 ~~ ~~",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 1, None, GameRules::generation(0))
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 2, y: 3 },
            },
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            game.board.to_string(),
            "__ __ S0 |0 __\n\
             __ __ T0 __ __\n\
             __ __ R0 __ __\n\
             __ __ A0 __ __\n\
             __ I1 __ __ __\n\
             __ G1 X1 __ __\n\
             ~~ ~~ |1 ~~ ~~",
        );
    }

    #[test]
    fn resolve_win() {
        let b = Board::from_string(
            "__ __ S0 |0 __\n\
             __ __ T0 __ __\n\
             __ A0 R0 __ __\n\
             D0 B0 __ X1 __\n\
             N0 __ __ X1 __\n\
             __ __ X1 X1 __\n\
             #1 #1 |1 #1 #1",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 1, None, GameRules::generation(0))
        };
        game.start();

        _ = game.play_turn(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 0, y: 5 },
            },
            None,
            None,
            None,
        );

        assert_eq!(
            game.board.to_string(),
            "__ __ S0 |0 __\n\
             __ __ T0 __ __\n\
             __ A0 R0 __ __\n\
             D0 B0 __ X1 __\n\
             N0 __ __ X1 __\n\
             A0 __ X1 X1 __\n\
             ⊭1 #1 |1 #1 #1",
        );
        assert_eq!(
            game.board.get(Coordinate { x: 0, y: 6 }).unwrap(),
            Square::Town {
                player: 1,
                defeated: true,
                foggy: false
            }
        );
        assert_eq!(game.winner, Some(0));
    }

    #[test]
    fn resolve_failed_win() {
        let b = Board::from_string(
            "__ __ S0 |0 __\n\
             __ __ T0 __ __\n\
             __ A0 R0 __ __\n\
             G0 B0 __ X1 __\n\
             N0 __ __ X1 __\n\
             __ __ X1 X1 __\n\
             #1 #1 |1 #1 #1",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 1, None, GameRules::generation(0))
        };
        game.start();

        _ = game.play_turn(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 0, y: 5 },
            },
            None,
            None,
            None,
        );

        assert_eq!(
            game.board.to_string(),
            "__ __ S0 |0 __\n\
             __ __ T0 __ __\n\
             __ A0 R0 __ __\n\
             __ B0 __ X1 __\n\
             __ __ __ X1 __\n\
             __ __ X1 X1 __\n\
             #1 #1 |1 #1 #1",
        );
        assert_eq!(
            game.board.get(Coordinate { x: 0, y: 6 }).unwrap(),
            Square::Town {
                player: 1,
                defeated: false,
                foggy: false
            }
        );
        assert_eq!(game.winner, None);
    }

    #[test]
    fn resolve_failed_win_due_to_battle() {
        let b = Board::from_string(
            "__ __ S0 |0 __\n\
             __ __ T0 __ __\n\
             __ A0 R0 __ __\n\
             D0 B0 __ X1 __\n\
             N0 __ __ X1 __\n\
             __ B1 I1 G1 __\n\
             #1 #1 |1 #1 #1",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 1, None, GameRules::generation(0))
        };
        game.start();

        _ = game.play_turn(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 0, y: 5 },
            },
            None,
            None,
            None,
        );

        assert_eq!(
            game.board.to_string(),
            "__ __ S0 |0 __\n\
             __ __ T0 __ __\n\
             __ A0 R0 __ __\n\
             __ B0 __ X1 __\n\
             __ __ __ X1 __\n\
             __ B1 I1 G1 __\n\
             #1 #1 |1 #1 #1",
        );
        assert_eq!(
            game.board.get(Coordinate { x: 0, y: 6 }).unwrap(),
            Square::Town {
                player: 1,
                defeated: false,
                foggy: false
            }
        );
        assert_eq!(game.winner, None);
    }

    #[test]
    fn resolve_win_after_battle() {
        let b = Board::from_string(
            "__ __ S0 |0 __\n\
             __ __ T0 __ __\n\
             __ A0 R0 __ __\n\
             D0 B0 __ X1 __\n\
             N0 __ __ X1 __\n\
             __ X1 I1 G1 __\n\
             #1 #1 |1 #1 #1",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 1, None, GameRules::generation(0))
        };
        game.start();

        _ = game.play_turn(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 0, y: 5 },
            },
            None,
            None,
            None,
        );

        assert_eq!(
            game.board.to_string(),
            "__ __ S0 |0 __\n\
             __ __ T0 __ __\n\
             __ A0 R0 __ __\n\
             D0 B0 __ __ __\n\
             N0 __ __ __ __\n\
             A0 __ __ __ __\n\
             ⊭1 #1 |1 #1 #1",
        );
        assert_eq!(
            game.board.get(Coordinate { x: 0, y: 6 }).unwrap(),
            Square::Town {
                player: 1,
                defeated: true,
                foggy: false
            }
        );
        assert_eq!(game.winner, Some(0));
    }

    #[test]
    fn resolve_win_via_explosion() {
        let b = Board::from_string(
            "__ __ __ __ S0 |0 __\n\
             __ __ __ __ T0 __ __\n\
             __ __ __ A0 R0 __ __\n\
             __ __ D0 B0 __ X1 __\n\
             __ __ N0 __ __ X1 __\n\
             X1 X1 __ B1 I1 G1 __\n\
             |1 #1 #1 #1 |1 #1 #1",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 1, None, GameRules::generation(0))
        };
        game.start();

        _ = game.play_turn(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 2, y: 5 },
            },
            None,
            None,
            None,
        );

        assert_eq!(
            game.board.to_string(),
            "__ __ __ __ S0 |0 __\n\
             __ __ __ __ T0 __ __\n\
             __ __ __ A0 R0 __ __\n\
             __ __ D0 B0 __ X1 __\n\
             __ __ N0 __ __ X1 __\n\
             __ __ A0 __ I1 G1 __\n\
             |1 #1 ⊭1 #1 |1 #1 #1",
        );
        assert_eq!(
            game.board.get(Coordinate { x: 2, y: 6 }).unwrap(),
            Square::Town {
                player: 1,
                defeated: true,
                foggy: false
            }
        );
        assert_eq!(game.winner, Some(0));
    }

    #[test]
    fn resolve_win_via_blocking() {
        let b = Board::from_string(
            "__ __ __ __ S0 |0 __\n\
             #1 __ __ __ T0 __ __\n\
             ~~ __ __ A0 R0 __ __\n\
             |1 __ D0 B0 __ __ __\n\
             ~~ __ __ __ __ __ __\n\
             #1 __ __ __ __ __ __",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            turn_count: 0,
            ..Game::new_legacy(3, 1, None, GameRules::generation(0))
        };
        game.start();

        _ = game.play_turn(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
            None,
            None,
            None,
        );

        assert_eq!(
            game.board.to_string(),
            "__ __ __ __ S0 |0 __\n\
             ⊭1 __ __ __ T0 __ __\n\
             ~~ __ __ A0 R0 __ __\n\
             |1 A0 D0 B0 __ __ __\n\
             ~~ __ __ __ __ __ __\n\
             ⊭1 __ __ __ __ __ __",
        );
        assert_eq!(
            game.board.get(Coordinate { x: 0, y: 1 }).unwrap(),
            Square::Town {
                player: 1,
                defeated: true,
                foggy: false
            }
        );
        assert_eq!(game.winner, Some(0));
    }

    #[test]
    fn resolve_noop() {
        let b = Board::from_string(
            "~~ |0 __ ~~ ~~\n\
             __ __ __ __ __\n\
             __ __ __ __ __\n\
             __ __ __ __ __\n\
             ~~ ~~ T1 |1 ~~",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(3, 1, None, GameRules::generation(0))
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 2, y: 0 },
            },
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            game.board.to_string(),
            "~~ |0 A0 ~~ ~~\n\
             __ __ __ __ __\n\
             __ __ __ __ __\n\
             __ __ __ __ __\n\
             ~~ ~~ T1 |1 ~~",
        )
    }

    #[test]
    fn resolve_with_smaller_attack_dict() {
        let b = Board::from_string(
            "__ S0 X0 |0 __\n\
             __ T0 __ __ __\n\
             __ R0 __ __ __\n\
             __ __ A1 __ __\n\
             __ __ R1 __ __\n\
             __ __ T1 __ __\n\
             __ __ S1 |1 __",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(1, 1, None, GameRules::generation(0))
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
            Some(&b_dict().builtin_dictionary),
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            game.board.to_string(),
            "__ __ X0 |0 __\n\
             __ __ __ __ __\n\
             __ __ __ __ __\n\
             __ __ A1 __ __\n\
             __ __ R1 __ __\n\
             __ __ T1 __ __\n\
             __ __ S1 |1 __",
        )
    }

    #[test]
    fn resolve_with_smaller_defense_dict() {
        let b = Board::from_string(
            "__ S0 X0 |0 __\n\
             __ T0 __ __ __\n\
             __ R0 __ __ __\n\
             __ __ A1 __ __\n\
             __ __ R1 __ __\n\
             __ __ T1 __ __\n\
             __ __ S1 |1 __",
        );
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            player_turn_count: vec![0, 0],
            judge: short_dict(),
            ..Game::new_legacy(1, 1, None, GameRules::generation(0))
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
            None,
            Some(&b_dict().builtin_dictionary),
            None,
        )
        .unwrap();

        assert_eq!(
            game.board.to_string(),
            "__ S0 X0 |0 __\n\
             __ T0 __ __ __\n\
             __ R0 __ __ __\n\
             __ A0 __ __ __\n\
             __ __ __ __ __\n\
             __ __ __ __ __\n\
             __ __ __ |1 __",
        )
    }
}
