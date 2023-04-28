use super::board::{Board, Coordinate, Square};
use super::judge::{Judge, Outcome};
use super::reporting::{BoardChange, BoardChangeAction};
use crate::bag::TileBag;
use crate::error::GamePlayError;
use crate::player::Player;
use crate::reporting::Change;

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
        Judge::new(vec!["BIG", "FAT", "JOLLY", "AND", "SILLY", "FOLK", "ARTS"]) // TODO: Collins 2018 list
    }

    #[test]
    fn invalid_placement_locations() {
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
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
            ..Game::new(3, 1, false)
        };
        assert_eq!(
            game.make_move(out_of_bounds),
            Err(GamePlayError::OutSideBoardDimensions { position })
        );

        let position = Coordinate { x: 10, y: 0 };
        let out_of_bounds = Move::Place {
            player: 0,
            tile: 'A',
            position,
        };
        assert_eq!(
            game.make_move(out_of_bounds),
            Err(GamePlayError::OutSideBoardDimensions { position })
        );

        let position = Coordinate { x: 0, y: 0 };
        let dead = Move::Place {
            player: 0,
            tile: 'A',
            position,
        };
        assert_eq!(
            game.make_move(dead),
            Err(GamePlayError::InvalidPosition { position })
        );
    }

    #[test]
    fn can_place_and_swap() {
        let mut bag = TileUtils::a_b_bag();
        let players = vec![Player::new(
            "A".into(),
            0,
            7,
            &mut bag,
            Duration::new(60, 0),
            (0, 0, 0),
        )];

        let mut game = Game {
            bag,
            players,
            judge: short_dict(),
            ..Game::new(3, 1, false)
        };

        // Places on the root
        let changes = game.make_move(Move::Place {
            player: 0,
            tile: 'A',
            position: Coordinate { x: 1, y: 0 },
        });
        assert_eq!(
            changes.clone().map(|c| {
                c.into_iter()
                    .filter(|c| matches!(c, Change::Board(_)))
                    .collect::<Vec<_>>()
            }),
            Ok(vec![Change::Board(BoardChange {
                detail: BoardChangeDetail {
                    square: Square::Occupied(0, 'A'),
                    coordinate: Coordinate { x: 1, y: 0 },
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
            game.make_move(Move::Place {
                player: 0,
                tile: 'B',
                position: Coordinate { x: 1, y: 0 }
            },),
            Err(GamePlayError::OccupiedPlace)
        );

        // Can't place at a diagonal
        assert_eq!(
            game.make_move(Move::Place {
                player: 0,
                tile: 'B',
                position: Coordinate { x: 0, y: 1 }
            },),
            Err(GamePlayError::NonAdjacentPlace)
        );

        // Can place directly above
        assert_eq!(
            game.make_move(Move::Place {
                player: 0,
                tile: 'B',
                position: Coordinate { x: 1, y: 1 }
            },)
                .map(|c| {
                    c.into_iter()
                        .filter(|c| matches!(c, Change::Board(_)))
                        .collect::<Vec<_>>()
                }),
            Ok(vec![Change::Board(BoardChange {
                detail: BoardChangeDetail {
                    square: Square::Occupied(0, 'B'),
                    coordinate: Coordinate { x: 1, y: 1 },
                },
                action: BoardChangeAction::Added
            })])
        );

        // Can't place on the same place again
        assert_eq!(
            game.make_move(Move::Place {
                player: 0,
                tile: 'B',
                position: Coordinate { x: 1, y: 1 }
            },),
            Err(GamePlayError::OccupiedPlace)
        );

        // Can swap
        assert_eq!(
            game.make_move(Move::Swap {
                player: 0,
                positions: [Coordinate { x: 1, y: 1 }, Coordinate { x: 1, y: 0 }]
            },),
            Ok(vec![
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied(0, 'A'),
                        coordinate: Coordinate { x: 1, y: 1 },
                    },
                    action: BoardChangeAction::Swapped
                }),
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied(0, 'B'),
                        coordinate: Coordinate { x: 1, y: 0 },
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
            Player::new("A".into(), 0, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
        ];

        let mut game = Game {
            bag,
            players,
            judge: short_dict(),
            ..Game::new(3, 1, false)
        };

        assert_eq!(
            game.make_move(Move::Place {
                player: 2,
                tile: 'A',
                position: Coordinate { x: 1, y: 0 }
            },),
            Err(GamePlayError::NonExistentPlayer { index: 2 })
        );

        assert_eq!(
            game.make_move(Move::Place {
                player: 0,
                tile: '&',
                position: Coordinate { x: 1, y: 0 }
            },),
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
        let mut one_v_one = BoardUtils::from_string(
            [
                "_ _ M _ _",
                "_ _ D _ _",
                "_ _ _ _ _",
                "_ _ M _ _",
                "_ _ D _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        one_v_one.set(middle, 0, 'A').unwrap();

        assert_eq!(
            one_v_one.collect_combanants(0, middle),
            (vec![middle_attacker.clone()], vec![middle_defender.clone()])
        );

        // 1v2
        let mut one_v_two = BoardUtils::from_string(
            [
                "_ _ M _ _",
                "_ _ D _ _",
                "_ L _ R _",
                "_ F _ T _",
                "_ D R D _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        one_v_two.set(middle, 0, 'A').unwrap();

        assert_eq!(
            one_v_two.collect_combanants(0, middle),
            (
                vec![middle_attacker.clone()],
                vec![right_defender.clone(), left_defender.clone()],
            )
        );

        // 1v3
        let mut one_v_three = BoardUtils::from_string(
            [
                "_ _ M _ _",
                "_ _ D _ _",
                "_ L _ R _",
                "_ F M T _",
                "_ D D D _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
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
        let mut two_v_two = BoardUtils::from_string(
            [
                "X X M _ _",
                "X _ D _ _",
                "L F _ R _",
                "_ _ M T _",
                "_ _ D D _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
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
        let mut b = BoardUtils::from_string(
            [
                "_ S X _ _",
                "_ T _ _ _",
                "_ R _ _ _",
                "_ _ I _ _",
                "_ _ T _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            judge: short_dict(),
            ..Game::new(1, 1, false)
        };

        game.make_move(Move::Place {
            player: 0,
            tile: 'A',
            position: Coordinate { x: 1, y: 3 },
        })
        .unwrap();

        assert_eq!(
            game.board.to_string(),
            [
                "_ S X _ _",
                "_ T _ _ _",
                "_ R _ _ _",
                "_ A _ _ _",
                "_ _ _ _ _",
                "Roots: (2, 0) / (2, 4)"
            ]
            .join("\n"),
        )
    }

    #[test]
    fn resolve_failed_attack() {
        let mut b = BoardUtils::from_string(
            [
                "_ X X _ _",
                "_ T _ _ _",
                "_ R _ _ _",
                "_ _ I _ _",
                "_ _ T _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            judge: short_dict(),
            ..Game::new(3, 1, false)
        };

        game.make_move(Move::Place {
            player: 0,
            tile: 'A',
            position: Coordinate { x: 1, y: 3 },
        })
        .unwrap();

        assert_eq!(
            game.board.to_string(),
            [
                "_ _ X _ _",
                "_ _ _ _ _",
                "_ _ _ _ _",
                "_ _ I _ _",
                "_ _ T _ _",
                "Roots: (2, 0) / (2, 4)",
            ]
            .join("\n"),
        )
    }

    #[test]
    fn resolve_truncation() {
        let mut b = BoardUtils::from_string(
            [
                "_ S X _ _",
                "_ T _ _ _",
                "_ R _ X _",
                "_ _ B X _",
                "_ _ I _ _",
                "_ _ G _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 5 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
        ];

        let mut test_bag = TileUtils::trivial_bag();
        let test_players = vec![
            Player::new(
                "A".into(),
                0,
                7,
                &mut test_bag,
                Duration::new(60, 0),
                (0, 0, 0),
            ),
            Player::new(
                "B".into(),
                1,
                7,
                &mut test_bag,
                Duration::new(60, 0),
                (0, 0, 0),
            ),
        ];

        assert_eq!(players, test_players);
        assert_eq!(bag, test_bag);

        let mut game = Game {
            board: b,
            bag,
            players,
            judge: short_dict(),
            ..Game::new(3, 1, false)
        };

        game.make_move(Move::Place {
            player: 0,
            tile: 'A',
            position: Coordinate { x: 1, y: 3 },
        })
        .unwrap();

        for letter in ['B', 'X', 'X'] {
            test_bag.return_tile(letter);
        }
        assert_eq!(game.bag, test_bag);

        assert_eq!(
            game.board.to_string(),
            [
                "_ S X _ _",
                "_ T _ _ _",
                "_ R _ _ _",
                "_ A _ _ _",
                "_ _ I _ _",
                "_ _ G _ _",
                "Roots: (2, 0) / (2, 5)",
            ]
            .join("\n"),
        );
    }

    #[test]
    fn resolve_explosion() {
        let mut b = BoardUtils::from_string(
            [
                "_ _ S _ _",
                "_ _ T _ _",
                "_ _ R _ _",
                "_ B _ X _",
                "_ I _ X _",
                "_ G X X _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 5 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            judge: short_dict(),
            ..Game::new(3, 1, false)
        };

        game.make_move(Move::Place {
            player: 0,
            tile: 'A',
            position: Coordinate { x: 2, y: 3 },
        })
        .unwrap();

        assert_eq!(
            game.board.to_string(),
            [
                "_ _ S _ _",
                "_ _ T _ _",
                "_ _ R _ _",
                "_ _ A _ _",
                "_ I _ _ _",
                "_ G X _ _",
                "Roots: (2, 0) / (2, 5)",
            ]
            .join("\n"),
        );
    }

    #[test]
    fn resolve_noop() {
        let mut b = BoardUtils::from_string(
            [
                "    _    ",
                "_ _ _ _ _",
                "_ _ _ _ _",
                "_ _ _ _ _",
                "    T    ",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        let mut bag = TileUtils::trivial_bag();
        let players = vec![
            Player::new("A".into(), 0, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
            Player::new("B".into(), 1, 7, &mut bag, Duration::new(60, 0), (0, 0, 0)),
        ];

        let mut game = Game {
            board: b,
            bag,
            players,
            judge: short_dict(),
            ..Game::new(3, 1, false)
        };

        game.make_move(Move::Place {
            player: 0,
            tile: 'A',
            position: Coordinate { x: 2, y: 0 },
        })
        .unwrap();

        assert_eq!(
            game.board.to_string(),
            [
                "    A    ",
                "_ _ _ _ _",
                "_ _ _ _ _",
                "_ _ _ _ _",
                "    T    ",
                "Roots: (2, 0) / (2, 4)",
            ]
            .join("\n"),
        )
    }
}
