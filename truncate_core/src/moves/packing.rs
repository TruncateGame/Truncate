use crate::board::Coordinate;

use super::Move;

fn pack_coord(coord: Coordinate) -> String {
    let x = coord.x.to_string();
    let y = coord.y.to_string();
    let len = x.len().max(y.len());

    format!("{x:0>w$}{y:0>w$}", w = len)
}

fn unpack_coord(packed_coord: &String) -> Result<Coordinate, ()> {
    let (x, y) = packed_coord.split_at(packed_coord.len() / 2);

    Ok(Coordinate {
        x: x.parse().map_err(|_| ())?,
        y: y.parse().map_err(|_| ())?,
    })
}

pub fn pack_moves(moves: &Vec<Move>, player_count: usize) -> String {
    let mut packed = String::with_capacity(moves.len() * 3);

    let mut next_player: usize = 0;

    let incr_player = |p: &mut usize| {
        let r = *p;
        *p = (*p + 1) % player_count;
        r
    };

    if let Some(first_move) = moves.first() {
        next_player = match first_move {
            Move::Place { player, .. } => *player,
            Move::Swap { player, .. } => *player,
        };
        packed.push_str(&format!("[{next_player}]"));
    };

    for m in moves {
        match m {
            Move::Place {
                player,
                tile,
                position,
            } => {
                if *player != next_player {
                    next_player = *player;
                    packed.push_str(&format!("[{player}]"));
                }

                packed.push_str(&pack_coord(*position));
                packed.push(*tile);

                incr_player(&mut next_player);
            }
            Move::Swap {
                player,
                positions: [from, to],
            } => {
                if *player != next_player {
                    next_player = *player;
                    packed.push_str(&format!("[{player}]"));
                }

                packed.push('<');
                packed.push_str(&pack_coord(*from));
                packed.push('/');
                packed.push_str(&pack_coord(*to));
                packed.push('>');

                incr_player(&mut next_player);
            }
        }
    }

    packed
}

pub fn unpack_moves(packed_moves: &String, player_count: usize) -> Result<Vec<Move>, ()> {
    let mut moves = Vec::with_capacity(packed_moves.len() / 3);

    enum State {
        None,
        SetPlayer(String),
        Place(String),
        SwapFrom(String),
        SwapTo(Coordinate, String),
    }

    let mut i = packed_moves.chars();
    let mut state = State::None;
    let mut player = 0;

    let incr_player = |p: &mut usize| {
        let r = *p;
        *p = (*p + 1) % player_count;
        r
    };

    while let Some(c) = i.next() {
        match &mut state {
            State::None => {
                if c.is_numeric() {
                    state = State::Place(c.to_string());
                } else if c == '<' {
                    state = State::SwapFrom(String::new());
                } else if c == '[' {
                    state = State::SetPlayer(String::new());
                } else {
                    return Err(());
                }
            }
            // [4] sets the player for the next move to 4
            State::SetPlayer(s) => {
                if c.is_numeric() {
                    s.push(c);
                } else if c == ']' {
                    player = s.parse().map_err(|_| ())?;
                    state = State::None;
                } else {
                    return Err(());
                }
            }
            // 1204A places tile 'A' at [12, 4]
            State::Place(s) => {
                if c.is_numeric() {
                    s.push(c);
                } else if c.is_alphabetic() {
                    let position = unpack_coord(s)?;
                    moves.push(Move::Place {
                        player: incr_player(&mut player),
                        tile: c,
                        position,
                    });
                    state = State::None;
                } else {
                    return Err(());
                }
            }
            // <34/0118> swaps [3, 4] and [1, 18]
            State::SwapFrom(s) => {
                if c.is_numeric() {
                    s.push(c);
                } else if c == '/' {
                    let coord = unpack_coord(s)?;
                    state = State::SwapTo(coord, String::new());
                } else {
                    return Err(());
                }
            }
            // <34/0118> swaps [3, 4] and [1, 18]
            State::SwapTo(from, s) => {
                if c.is_numeric() {
                    s.push(c);
                } else if c == '>' {
                    let to = unpack_coord(s)?;
                    moves.push(Move::Swap {
                        player: incr_player(&mut player),
                        positions: [*from, to],
                    });
                    state = State::None;
                } else {
                    return Err(());
                }
            }
        }
    }

    Ok(moves)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packing_moves() {
        let moves = vec![
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 12, y: 3 },
            },
            Move::Place {
                player: 1,
                tile: 'B',
                position: Coordinate { x: 1, y: 1 },
            },
            Move::Place {
                player: 0,
                tile: 'J',
                position: Coordinate { x: 1, y: 301 },
            },
            Move::Swap {
                player: 1,
                positions: [Coordinate { x: 1, y: 1 }, Coordinate { x: 10, y: 9 }],
            },
            Move::Place {
                player: 0,
                tile: 'R',
                position: Coordinate { x: 3, y: 3 },
            },
        ];

        let packed = pack_moves(&moves, 2);

        assert_eq!(packed, "[0]1203A11B001301J<11/1009>33R".to_string());

        let unpacked = unpack_moves(&packed, 2);

        assert_eq!(unpacked, Ok(moves));
    }

    #[test]
    fn test_packing_three_players() {
        let moves = vec![
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 12, y: 3 },
            },
            Move::Place {
                player: 1,
                tile: 'B',
                position: Coordinate { x: 1, y: 1 },
            },
            Move::Place {
                player: 2,
                tile: 'J',
                position: Coordinate { x: 1, y: 301 },
            },
            Move::Swap {
                player: 0,
                positions: [Coordinate { x: 1, y: 1 }, Coordinate { x: 10, y: 9 }],
            },
            Move::Place {
                player: 1,
                tile: 'R',
                position: Coordinate { x: 3, y: 3 },
            },
        ];

        let packed = pack_moves(&moves, 3);

        assert_eq!(packed, "[0]1203A11B001301J<11/1009>33R".to_string());

        let unpacked = unpack_moves(&packed, 3);

        assert_eq!(unpacked, Ok(moves));
    }

    #[test]
    fn test_packing_out_of_order_moves() {
        let moves = vec![
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 12, y: 3 },
            },
            Move::Place {
                player: 0,
                tile: 'B',
                position: Coordinate { x: 1, y: 1 },
            },
            Move::Place {
                player: 0,
                tile: 'J',
                position: Coordinate { x: 1, y: 301 },
            },
            Move::Swap {
                player: 1,
                positions: [Coordinate { x: 1, y: 1 }, Coordinate { x: 10, y: 9 }],
            },
            Move::Place {
                player: 0,
                tile: 'E',
                position: Coordinate { x: 2, y: 2 },
            },
            Move::Place {
                player: 1,
                tile: 'R',
                position: Coordinate { x: 3, y: 3 },
            },
            Move::Place {
                player: 9,
                tile: 'X',
                position: Coordinate { x: 0, y: 0 },
            },
        ];

        let packed = pack_moves(&moves, 2);

        assert_eq!(
            packed,
            "[0]1203A[0]11B[0]001301J<11/1009>22E33R[9]00X".to_string()
        );

        let unpacked = unpack_moves(&packed, 2);

        assert_eq!(unpacked, Ok(moves));
    }
}
