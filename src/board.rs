use std::fmt;

pub struct Board {
    squares: Vec<Vec<Square>>,
    roots: Vec<Coordinate>,
}

impl Board {
    pub fn new(width: usize, height: usize) -> Self {
        let roots = vec![
            Coordinate {
                x: width / 2 + width % 2 - 1,
                y: 0,
            },
            Coordinate {
                x: width / 2,
                y: height + 1,
            },
        ];

        let mut squares = vec![vec![Square::Dead; width]]; // Start with an unoccupiable row to house player 1's root
        squares.extend(vec![vec![Square::Empty; width]; height]); // Make the centre of the board empty
        squares.extend(vec![vec![Square::Dead; width]]); // Add an unoccupiable row to house player 2's root
        squares[roots[0].y][roots[0].x] = Square::Empty; // Create root square
        squares[roots[1].y][roots[1].x] = Square::Empty;

        Board { squares, roots }
    }

    // TODO: generic board constructor that accepts a grid of squares with arbitrary values, as long as:
    //  - the empty squares are fully connected
    //  - there are at least 2 roots
    //  - the roots are at empty squares

    pub fn get(&self, x: usize, y: usize) -> Result<Square, &str> {
        if y >= self.squares.len() {
            Err("y-coordinate is too large for board height") // TODO: specify the coordinate and height
        } else if x >= self.squares[0].len() {
            Err("x-coordinate is too large for board width") // TODO: specify the coordinate and width
        } else {
            match self.squares[y][x] {
                Square::Empty | Square::Occupied(_, _) => Ok(self.squares[y][x]),
                Square::Dead => Err("dead squares have no values"),
            }
        }
    }

    pub fn set(&mut self, x: usize, y: usize, player: usize, value: char) -> Result<(), &str> {
        if player >= self.roots.len() {
            Err("player does not exist") // TODO: specify the number of players and which player this is
        } else if y >= self.squares.len() {
            Err("y-coordinate is too large for board height") // TODO: specify the coordinate and height
        } else if x >= self.squares[0].len() {
            Err("x-coordinate is too large for board width") // TODO: specify the coordinate and width
        } else {
            match self.squares[y][x] {
                Square::Empty | Square::Occupied(_, _) => {
                    self.squares[y][x] = Square::Occupied(player, value);
                    Ok(())
                }
                Square::Dead => Err("don't try to set the value of a dead square"),
            }
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str = self
            .squares
            .iter()
            .map(|row| {
                row.iter()
                    .map(|sq| sq.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            })
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{}", str)
    }
}

pub struct Coordinate {
    x: usize,
    y: usize,
}

impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Square {
    Dead,
    Empty,
    Occupied(usize, char),
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Square::Dead => write!(f, "*"),
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
            b.get(1, 0),
            Err("x-coordinate is too large for board width")
        );
        assert_eq!(
            b.get(0, 3),
            Err("y-coordinate is too large for board height")
        );

        assert_eq!(
            b.set(1, 0, 0, 'a'),
            Err("x-coordinate is too large for board width")
        );
        assert_eq!(
            b.set(0, 3, 0, 'a'),
            Err("y-coordinate is too large for board height")
        );
    }

    #[test]
    fn getset_errors_for_dead_squares() {
        let mut b = Board::new(2, 1); // Note, height is 3 from home rows
        assert_eq!(b.get(1, 0), Err("dead squares have no values"));
        assert_eq!(b.get(0, 2), Err("dead squares have no values"));

        assert_eq!(
            b.set(1, 0, 0, 'a'),
            Err("don't try to set the value of a dead square")
        );
        assert_eq!(
            b.set(0, 2, 0, 'a'),
            Err("don't try to set the value of a dead square")
        );
    }

    #[test]
    fn getset_handles_empty_squares() {
        let mut b = Board::new(2, 1); // Note, height is 3 from home rows
        assert_eq!(b.get(0, 0), Ok(Square::Empty));
        assert_eq!(b.get(0, 1), Ok(Square::Empty));
        assert_eq!(b.get(1, 1), Ok(Square::Empty));
        assert_eq!(b.get(1, 2), Ok(Square::Empty));

        assert_eq!(b.set(0, 0, 0, 'a'), Ok(()));
        assert_eq!(b.set(0, 1, 0, 'a'), Ok(()));
        assert_eq!(b.set(1, 1, 0, 'a'), Ok(()));
        assert_eq!(b.set(1, 2, 0, 'a'), Ok(()));
    }

    #[test]
    fn set_requires_valid_player() {
        let mut b = Board::new(2, 1);
        assert_eq!(b.set(1, 2, 0, 'a'), Ok(()));
        assert_eq!(b.set(1, 2, 1, 'a'), Ok(()));
        assert_eq!(b.set(1, 2, 2, 'a'), Err("player does not exist"));
        assert_eq!(b.set(1, 2, 3, 'a'), Err("player does not exist"));
        assert_eq!(b.set(1, 2, 100, 'a'), Err("player does not exist"));
    }

    #[test]
    fn set_changes_get() {
        let mut b = Board::new(1, 1); // Note, height is 3 from home rows
        assert_eq!(b.get(0, 0), Ok(Square::Empty));
        assert_eq!(b.set(0, 0, 0, 'a'), Ok(()));
        assert_eq!(b.get(0, 0), Ok(Square::Occupied(0, 'a')));
    }
}
