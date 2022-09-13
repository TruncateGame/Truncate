use std::collections::HashMap;
use std::fmt;

pub struct Board {
    squares: Vec<Vec<Option<Square>>>,
    roots: Vec<Coordinate>,
}

impl Board {
    pub fn new(width: usize, height: usize) -> Self {
        // TODO: is all this internal usize <-> isize conversion worth accepting isize as valid coordinates? Is that only used for simpler traversal algorithms?
        let roots = vec![
            Coordinate {
                x: width as isize / 2 + width as isize % 2 - 1,
                y: 0,
            },
            Coordinate {
                x: width as isize / 2,
                y: height as isize + 1,
            },
        ];

        let mut squares = vec![vec![None; width]]; // Start with an unoccupiable row to house player 1's root
        squares.extend(vec![vec![Some(Square::Empty); width]; height]); // Make the centre of the board empty
        squares.extend(vec![vec![None; width]]); // Add an unoccupiable row to house player 2's root
        squares[roots[0].y as usize][roots[0].x as usize] = Some(Square::Empty); // Create root square
        squares[roots[1].y as usize][roots[1].x as usize] = Some(Square::Empty);

        Board { squares, roots }
    }

    // TODO: generic board constructor that accepts a grid of squares with arbitrary values, as long as:
    //  - the empty squares are fully connected
    //  - there are at least 2 roots
    //  - the roots are at empty squares

    pub fn get(&self, position: Coordinate) -> Result<Square, &str> {
        if position.y < 0 || position.x < 0 {
            return Err("negative coordinates");
        };
        let x = position.x as usize;
        let y = position.y as usize;

        if y >= self.squares.len() {
            Err("y-coordinate is too large for board height") // TODO: specify the coordinate and height
        } else if x >= self.squares[0].len() {
            Err("x-coordinate is too large for board width") // TODO: specify the coordinate and width
        } else {
            match self.squares[y][x] {
                None => Err("Invalid position"),
                Some(square) => Ok(square),
            }
        }
    }

    pub fn set(&mut self, position: Coordinate, player: usize, value: char) -> Result<(), &str> {
        if position.y < 0 || position.x < 0 {
            return Err("negative coordinates");
        };
        let x = position.x as usize;
        let y = position.y as usize;

        if player >= self.roots.len() {
            Err("player does not exist") // TODO: specify the number of players and which player this is
        } else if y >= self.squares.len() {
            Err("y-coordinate is too large for board height") // TODO: specify the coordinate and height
        } else if x >= self.squares[0].len() {
            Err("x-coordinate is too large for board width") // TODO: specify the coordinate and width
        } else {
            match self.squares[y][x] {
                Some(_) => {
                    self.squares[y][x] = Some(Square::Occupied(player, value));
                    Ok(())
                }
                None => Err("Can't set the value of a non-existant square"),
            }
        }
    }

    pub fn neighbouring_squares(&self, position: Coordinate) -> HashMap<Coordinate, Square> {
        // TODO: does this reinitialise every time even though it's a constant? Or is it compiled into the program?
        const deltas: [(isize, isize); 8] = [
            (0, 1),
            (0, -1),
            (1, 0),
            (1, 1),
            (1, -1),
            (-1, 0),
            (-1, 1),
            (-1, -1),
        ];

        let mut neighbours = HashMap::new();
        for delta in deltas {
            let neighbour_coordinate = Coordinate {
                x: position.x + delta.0,
                y: position.y + delta.1,
            };
            match self.get(neighbour_coordinate) {
                Err(_) => {
                    continue; // Skips invalid squares
                }
                Ok(square) => {
                    neighbours.insert(neighbour_coordinate, square);
                }
            }
        }
        neighbours
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str = self
            .squares
            .iter()
            .map(|row| {
                row.iter()
                    .map(|opt| match opt {
                        Some(sq) => sq.to_string(),
                        None => "*".to_string(),
                    })
                    .collect::<Vec<String>>()
                    .join(" ")
            })
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{}", str)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Coordinate {
    pub x: isize,
    pub y: isize,
}

impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Square {
    Empty,
    Occupied(usize, char),
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Square::Empty => write!(f, "_"),
            Square::Occupied(player, tile) => write!(f, "{}", tile),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_default_boards() {
        assert_eq!(
            Board::new(3, 1).to_string(),
            "* _ *
_ _ _
* _ *"
        );

        assert_eq!(
            Board::new(3, 2).to_string(),
            "* _ *
_ _ _
_ _ _
* _ *"
        );

        assert_eq!(
            Board::new(2, 1).to_string(),
            "_ *
_ _
* _"
        );

        assert_eq!(
            Board::new(5, 1).to_string(),
            "* * _ * *
_ _ _ _ _
* * _ * *"
        );

        assert_eq!(
            Board::new(6, 1).to_string(),
            "* * _ * * *
_ _ _ _ _ _
* * * _ * *"
        );
    }

    #[test]
    fn getset_errors_out_of_bounds() {
        let mut b = Board::new(1, 1); // Note, height is 3 from home rows
        assert_eq!(
            b.get(Coordinate { x: 1, y: 0 }),
            Err("x-coordinate is too large for board width")
        );
        assert_eq!(
            b.get(Coordinate { x: 0, y: 3 }),
            Err("y-coordinate is too large for board height")
        );

        assert_eq!(
            b.set(Coordinate { x: 1, y: 0 }, 0, 'a'),
            Err("x-coordinate is too large for board width")
        );
        assert_eq!(
            b.set(Coordinate { x: 0, y: 3 }, 0, 'a'),
            Err("y-coordinate is too large for board height")
        );
    }

    #[test]
    fn getset_errors_for_dead_squares() {
        let mut b = Board::new(2, 1); // Note, height is 3 from home rows
        assert_eq!(b.get(Coordinate { x: 1, y: 0 }), Err("Invalid position"));
        assert_eq!(b.get(Coordinate { x: 0, y: 2 }), Err("Invalid position"));

        assert_eq!(
            b.set(Coordinate { x: 1, y: 0 }, 0, 'a'),
            Err("Can't set the value of a non-existant square")
        );
        assert_eq!(
            b.set(Coordinate { x: 0, y: 2 }, 0, 'a'),
            Err("Can't set the value of a non-existant square")
        );
    }

    #[test]
    fn getset_handles_empty_squares() {
        let mut b = Board::new(2, 1); // Note, height is 3 from home rows
        assert_eq!(b.get(Coordinate { x: 0, y: 0 }), Ok(Square::Empty));
        assert_eq!(b.get(Coordinate { x: 0, y: 1 }), Ok(Square::Empty));
        assert_eq!(b.get(Coordinate { x: 1, y: 1 }), Ok(Square::Empty));
        assert_eq!(b.get(Coordinate { x: 1, y: 2 }), Ok(Square::Empty));

        assert_eq!(b.set(Coordinate { x: 0, y: 0 }, 0, 'a'), Ok(()));
        assert_eq!(b.set(Coordinate { x: 0, y: 1 }, 0, 'a'), Ok(()));
        assert_eq!(b.set(Coordinate { x: 1, y: 1 }, 0, 'a'), Ok(()));
        assert_eq!(b.set(Coordinate { x: 1, y: 2 }, 0, 'a'), Ok(()));
    }

    #[test]
    fn set_requires_valid_player() {
        let mut b = Board::new(2, 1);
        assert_eq!(b.set(Coordinate { x: 1, y: 2 }, 0, 'a'), Ok(()));
        assert_eq!(b.set(Coordinate { x: 1, y: 2 }, 1, 'a'), Ok(()));
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 2, 'a'),
            Err("player does not exist")
        );
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 3, 'a'),
            Err("player does not exist")
        );
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 100, 'a'),
            Err("player does not exist")
        );
    }

    #[test]
    fn set_changes_get() {
        let mut b = Board::new(1, 1); // Note, height is 3 from home rows
        assert_eq!(b.get(Coordinate { x: 0, y: 0 }), Ok(Square::Empty));
        assert_eq!(b.set(Coordinate { x: 0, y: 0 }, 0, 'a'), Ok(()));
        assert_eq!(
            b.get(Coordinate { x: 0, y: 0 }),
            Ok(Square::Occupied(0, 'a'))
        );
    }

    #[test]
    fn get_neighbours() {
        // (0,0) (1,0) (2,0)
        // (0,1) (1,1) (2,1)
        // (0,2) (1,2) (2,2)
        // (0,3) (1,3) (2,3)
        // (0,4) (1,4) (2,4)
        let mut b = Board::new(3, 3);

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 0, y: 0 }),
            HashMap::from([
                (Coordinate { x: 0, y: 1 }, Square::Empty),
                (Coordinate { x: 1, y: 0 }, Square::Empty),
                (Coordinate { x: 1, y: 1 }, Square::Empty),
            ])
        );

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 1, y: 0 }),
            HashMap::from([
                (Coordinate { x: 0, y: 1 }, Square::Empty),
                (Coordinate { x: 1, y: 1 }, Square::Empty),
                (Coordinate { x: 2, y: 1 }, Square::Empty),
            ])
        );

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 1, y: 2 }),
            HashMap::from([
                (Coordinate { x: 0, y: 1 }, Square::Empty),
                (Coordinate { x: 1, y: 1 }, Square::Empty),
                (Coordinate { x: 2, y: 1 }, Square::Empty),
                (Coordinate { x: 0, y: 2 }, Square::Empty),
                (Coordinate { x: 2, y: 2 }, Square::Empty),
                (Coordinate { x: 0, y: 3 }, Square::Empty),
                (Coordinate { x: 1, y: 3 }, Square::Empty),
                (Coordinate { x: 2, y: 3 }, Square::Empty),
            ])
        );

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 1, y: 4 }),
            HashMap::from([
                (Coordinate { x: 0, y: 3 }, Square::Empty),
                (Coordinate { x: 1, y: 3 }, Square::Empty),
                (Coordinate { x: 2, y: 3 }, Square::Empty),
            ])
        );
    }
}
