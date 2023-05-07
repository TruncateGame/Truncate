use super::board::{Board, Coordinate, Square};
use super::judge::{Judge, Outcome};
use super::reporting::{BoardChange, BoardChangeAction};
use crate::bag::TileBag;
use crate::error::GamePlayError;
use crate::player::Player;
use crate::reporting::Change;

#[derive(PartialEq)]
pub enum Move {
    // TODO: make Move a struct and make player a top level property of it
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

#[cfg(test)]
mod tests {
    use time::Duration;

    use crate::board::{tests as BoardUtils, Direction};
    use crate::game::Game;
    use crate::reporting::*;

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
        ]) // TODO: Collins 2018 list
    }

    #[test]
    fn invalid_placement_locations() {
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let position = Coordinate { x: 10, y: 10 };
        let out_of_bounds = Move::Place {
            player: 0,
            tile: 'A',
            position,
        };
        let mut game = Game {
            bag,
            players,
            judge: short_dict(),
            ..Game::new(3, 3)
        };
        assert_eq!(
            game.make_move(out_of_bounds, None),
            Err(GamePlayError::OutSideBoardDimensions { position })
        );

        let position = Coordinate { x: 10, y: 0 };
        let out_of_bounds = Move::Place {
            player: 0,
            tile: 'A',
            position,
        };
        assert_eq!(
            game.make_move(out_of_bounds, None),
            Err(GamePlayError::OutSideBoardDimensions { position })
        );

        let position = Coordinate { x: 1, y: 0 };
        let dead = Move::Place {
            player: 0,
            tile: 'A',
            position,
        };
        assert_eq!(
            game.make_move(dead, None),
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
            judge: short_dict(),
            ..Game::new(3, 3)
        };

        // Places beside dock
        let changes = game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 2, y: 1 },
            },
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
                    square: Square::Occupied(0, 'A'),
                    coordinate: Coordinate { x: 2, y: 1 },
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
                    position: Coordinate { x: 2, y: 1 }
                },
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
                    position: Coordinate { x: 3, y: 2 }
                },
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
                    position: Coordinate { x: 2, y: 2 }
                },
                None
            )
            .map(|c| {
                c.into_iter()
                    .filter(|c| matches!(c, Change::Board(_)))
                    .collect::<Vec<_>>()
            }),
            Ok(vec![Change::Board(BoardChange {
                detail: BoardChangeDetail {
                    square: Square::Occupied(0, 'B'),
                    coordinate: Coordinate { x: 2, y: 2 },
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
                    position: Coordinate { x: 2, y: 2 }
                },
                None
            ),
            Err(GamePlayError::OccupiedPlace)
        );

        // Can swap
        assert_eq!(
            game.make_move(
                Move::Swap {
                    player: 0,
                    positions: [Coordinate { x: 2, y: 1 }, Coordinate { x: 2, y: 2 }]
                },
                None
            ),
            Ok(vec![
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied(0, 'B'),
                        coordinate: Coordinate { x: 2, y: 1 },
                    },
                    action: BoardChangeAction::Swapped
                }),
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied(0, 'A'),
                        coordinate: Coordinate { x: 2, y: 2 },
                    },
                    action: BoardChangeAction::Swapped
                })
            ])
        );
    }

    #[test]
    fn invalid_player_or_tile() {
        let mut bag = TileBag::default();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
        ];

        let mut game = Game {
            bag,
            players,
            judge: short_dict(),
            ..Game::new(3, 3)
        };

        assert_eq!(
            game.make_move(
                Move::Place {
                    player: 2,
                    tile: 'A',
                    position: Coordinate { x: 2, y: 2 }
                },
                None
            ),
            Err(GamePlayError::NonAdjacentPlace)
        );

        assert_eq!(
            game.make_move(
                Move::Place {
                    player: 0,
                    tile: '&',
                    position: Coordinate { x: 1, y: 0 }
                },
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
        one_v_one.set(middle, 0, 'A').unwrap();

        assert_eq!(
            one_v_one.collect_combanants(0, middle),
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
        one_v_two.set(middle, 0, 'A').unwrap();

        assert_eq!(
            one_v_two.collect_combanants(0, middle),
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
        one_v_three.set(middle, 0, 'A').unwrap();

        assert_eq!(
            one_v_three.collect_combanants(0, middle),
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
        two_v_two.set(middle, 0, 'A').unwrap();
        assert_eq!(
            two_v_two.collect_combanants(0, middle),
            (
                vec![middle_attacker, left_attacker],
                vec![right_defender, middle_defender, short_cross_defender],
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
            judge: short_dict(),
            ..Game::new(1, 1)
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
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
            judge: short_dict(),
            ..Game::new(3, 1)
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
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
            judge: short_dict(),
            ..Game::new(3, 1)
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
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
            judge: short_dict(),
            ..Game::new(3, 1)
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 2, y: 3 },
            },
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
            judge: short_dict(),
            ..Game::new(3, 1)
        };

        game.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 2, y: 0 },
            },
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
}
