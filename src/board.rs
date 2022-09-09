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

    pub fn get(&self, x: usize, y: usize) -> Result<char, &str> {
        if y >= self.squares.len() {
            Err("y-coordinate is too large for board height") // TODO: specify the coordinate and height
        } else if x >= self.squares[0].len() {
            Err("x-coordinate is too large for board width") // TODO: specify the coordinate and width
        } else {
            match self.squares[y][x] {
                Square::Empty => Ok('_'),
                Square::Dead => Err("dead square not allowed"),
                Square::Occupied(_player, value) => Ok(value),
            }
        }
    }

    // pub fn set(&self, x: usize)
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

#[derive(Clone)]
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

    fn get_errors_out_of_bounds() {}
    fn get_errors_for_dead_squares() {}
    fn get_returns_empty_squares() {}

    fn set_errors_out_of_bounds() {}
    fn set_errors_for_dead_squares() {}
    fn set_changes_get() {}
}
