use anyhow::Result;
use std::collections::HashSet;
use std::fmt;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::error::GamePlayError;
use crate::moves::{Change, ChangeAction, ChangeDetail};

#[derive(EnumIter, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    South,
    East,
    North,
    West,
}

impl Direction {
    // Returns whether vertical words should be read from top to bottom if played by a player on this side of the board
    fn read_top_to_bottom(self) -> bool {
        matches!(self, Direction::South) || matches!(self, Direction::West)
    }

    // Returns whether horizontal words should be read from left to right if played by a player on this side of the board
    fn read_left_to_right(self) -> bool {
        matches!(self, Direction::South) || matches!(self, Direction::East)
    }

    pub fn opposite(self) -> Self {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Board {
    squares: Vec<Vec<Option<Square>>>,
    roots: Vec<Coordinate>,
    orientations: Vec<Direction>, // The side of the board that the player is sitting at, and the direction that their vertical words go in
}

impl Board {
    pub fn new(width: usize, height: usize) -> Self {
        // TODO: is all this internal usize <-> isize conversion worth accepting isize as valid coordinates? Is that only used for simpler traversal algorithms?
        // TODO: resolve discrepancy between width parameter, and the actual width of the board (which is returned by self.width()) where `actual == width + 2` because of the extra home rows.
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

        let mut squares = vec![vec![None; width]]; // Start with an unoccupiable row to house player 1's root
        squares.extend(vec![vec![Some(Square::Empty); width]; height]); // Make the centre of the board empty
        squares.extend(vec![vec![None; width]]); // Add an unoccupiable row to house player 2's root
        squares[roots[0].y][roots[0].x] = Some(Square::Empty); // Create root square
        squares[roots[1].y][roots[1].x] = Some(Square::Empty);

        Board {
            squares,
            roots,
            orientations: vec![Direction::North, Direction::South],
        }
    }

    // TODO: generic board constructor that accepts a grid of squares with arbitrary values, as long as:
    //  - the empty squares are fully connected
    //  - there are at least 2 roots
    //  - the roots are at empty squares

    pub fn get(&self, position: Coordinate) -> Result<Square, GamePlayError> {
        match self
            .squares
            .get(position.y)
            .and_then(|row| row.get(position.x))
        {
            Some(Some(square)) => Ok(*square),
            Some(None) => Err(GamePlayError::InvalidPosition { position }),
            None => Err(GamePlayError::OutSideBoardDimensions { position }),
        }
    }

    pub fn set(
        &mut self,
        position: Coordinate,
        player: usize,
        value: char,
    ) -> Result<ChangeDetail, GamePlayError> {
        if self.roots.get(player).is_none() {
            return Err(GamePlayError::NonExistentPlayer { index: player });
        }

        match self
            .squares
            .get_mut(position.y)
            .and_then(|row| row.get_mut(position.x))
        {
            Some(Some(square)) => {
                *square = Square::Occupied(player, value);
                Ok(ChangeDetail {
                    square: square.to_owned(),
                    coordinate: position,
                })
            }
            Some(None) => Err(GamePlayError::InvalidPosition { position }),
            None => Err(GamePlayError::OutSideBoardDimensions { position }),
        }
    }

    // TODO: safety on index access like get and set - ideally combine error checking for all 3
    pub fn clear(&mut self, position: Coordinate) -> Option<ChangeDetail> {
        if let Some(pos) = self
            .squares
            .get_mut(position.y as usize)
            .and_then(|y| y.get_mut(position.x as usize))
        {
            match pos.replace(Square::Empty) {
                Some(square) if matches!(square, Square::Occupied(_, _)) => {
                    return Some(ChangeDetail {
                        square,
                        coordinate: position,
                    })
                }
                _ => return None,
            }
        }
        None
    }

    pub fn truncate(&mut self) -> Vec<Change> {
        let mut attatched = HashSet::new();
        for root in self.roots.iter() {
            let tree = self.depth_first_search(*root);
            for bit in tree {
                attatched.insert(bit);
            }
        }

        let rows = self.height();
        let cols = self.width();
        let squares = (0..rows).flat_map(|y| (0..cols).zip(std::iter::repeat(y)));

        squares
            .flat_map(|(x, y)| {
                let c = Coordinate { x, y };
                if !attatched.contains(&c) {
                    self.clear(c).map(|detail| Change {
                        detail,
                        change: ChangeAction::Truncated,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_root(&self, player: usize) -> Result<Coordinate, GamePlayError> {
        match self.roots.get(player) {
            Some(player) => Ok(*player),
            None => Err(GamePlayError::NonExistentPlayer { index: player }),
        }
    }

    pub fn neighbouring_squares(&self, position: Coordinate) -> Vec<(Coordinate, Square)> {
        let mut neighbours = Vec::new();
        for delta in Direction::iter() {
            let neighbour_coordinate = position.add(delta);
            if let Ok(square) = self.get(neighbour_coordinate) {
                neighbours.push((neighbour_coordinate, square));
            };
        }
        neighbours
    }

    // TODO: return iterator or rename since it doesn't matter that this is depth first when we return a HashSet
    fn depth_first_search(&self, position: Coordinate) -> HashSet<Coordinate> {
        let mut set = HashSet::new();

        let player = if let Ok(Square::Occupied(player, _)) = self.get(position) {
            player
        } else {
            return set;
        };
        let mut stack = vec![position]; // TODO: consider more efficient stack type

        while let Some(current) = stack.pop() {
            set.insert(current);
            for (position, square) in self.neighbouring_squares(current) {
                // Put the neighbour in the set if it is occupied by the current player
                if let Square::Occupied(neighbours_player, _) = square {
                    if !set.contains(&position) && player == neighbours_player {
                        stack.push(position);
                    }
                }
            }
        }

        set
    }

    pub fn swap(
        &mut self,
        player: usize,
        positions: [Coordinate; 2],
    ) -> Result<Vec<Change>, GamePlayError> {
        if positions[0] == positions[1] {
            return Err(GamePlayError::SelfSwap);
        }

        let mut tiles = ['&'; 2];
        for (i, pos) in positions.iter().enumerate() {
            match self.get(*pos)? {
                Square::Empty => return Err(GamePlayError::UnoccupiedSwap),
                Square::Occupied(owner, tile) => {
                    if owner != player {
                        return Err(GamePlayError::UnownedSwap);
                    }
                    tiles[i] = tile;
                }
            };
        }

        Ok(vec![
            Change {
                detail: self.set(positions[0], player, tiles[1])?,
                change: ChangeAction::Swapped,
            },
            Change {
                detail: self.set(positions[1], player, tiles[0])?,
                change: ChangeAction::Swapped,
            },
        ])
    }

    pub fn get_words(&self, position: Coordinate) -> Vec<Vec<Coordinate>> {
        let mut words = Vec::new();
        let mut owner = None;

        for (i, direction) in Direction::iter().enumerate() {
            let mut word = Vec::new();
            let mut location = position;

            'wordbuilder: loop {
                if let Ok(Square::Occupied(player, _)) = self.get(location) {
                    if owner == None {
                        owner = Some(player);
                    }

                    if owner != Some(player) {
                        break 'wordbuilder; // Word ends at other players' letters
                    }

                    word.push(location);
                } else {
                    break 'wordbuilder; // Word ends at the edge of the board or empty squares
                }
                location = location.add(direction);
            }
            if i < 2 {
                words.push(word);
            } else {
                // Combine North/South and East/West words
                word.reverse();
                if word.len() > 0 {
                    if words[i - 2].len() > 0 {
                        words[i - 2].splice(0..1, word);
                        // Prepend and remove repeated letter
                    } else {
                        words[i - 2] = word;
                    }
                }
            }
        }

        // Reverse words based on the player's orientation
        if let Some(owner) = owner {
            let orientation = self.orientations[owner];
            if !orientation.read_top_to_bottom() {
                words[0].reverse();
            }
            if !orientation.read_left_to_right() {
                words[1].reverse();
            }
        }

        // 1 letter words don't count expect when there's only one tile, in which case it does count as a word
        if words.iter().filter(|w| w.len() == 1).count() != 2 {
            for i in (0..=1).rev() {
                // TODO: use filter
                if words[i].len() <= 1 {
                    words.remove(i);
                }
            }
        }

        words
    }

    pub fn word_strings(
        &self,
        coordinates: &Vec<Vec<Coordinate>>,
    ) -> Result<Vec<String>, GamePlayError> {
        let mut err = None; // TODO: is this a reasonable error handling method? We can't return an Err from the function from within the closure passed to map.
        let strings = coordinates
            .iter()
            .map(|word| {
                word.iter()
                    .map(|&square| match self.get(square) {
                        Ok(sq) => match sq {
                            Square::Empty => {
                                err = Some(GamePlayError::EmptySquareInWord);
                                '_'
                            }
                            Square::Occupied(_, letter) => letter,
                        },
                        Err(e) => {
                            err = Some(e);
                            '_'
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<String>>();

        if let Some(err_string) = err {
            Err(err_string)
        } else {
            Ok(strings)
        }
    }

    pub fn get_orientations(&self) -> &Vec<Direction> {
        &self.orientations
    }

    pub fn width(&self) -> usize {
        self.squares[0].len()
    }

    pub fn height(&self) -> usize {
        self.squares.len()
    }

    // Get the row just beside the edge
    pub fn get_near_edge(&self, side: Direction) -> Vec<Coordinate> {
        match side {
            Direction::North => (0..self.width()).map(|x| Coordinate { x, y: 1 }).collect(),
            Direction::South => (0..self.width())
                .map(|x| Coordinate {
                    x,
                    y: (self.height() - 2),
                })
                .collect(),
            Direction::East => (0..self.width())
                .map(|y| Coordinate {
                    x: (self.width() - 2),
                    y,
                })
                .collect(),
            Direction::West => (0..self.width()).map(|y| Coordinate { x: 1, y }).collect(),
        }
    }

    pub fn render_squares<F: Fn(&Square) -> String, G: Fn(usize, &String) -> String>(
        &self,
        square_renderer: F,
        line_transform: G,
    ) -> String {
        self.squares
            .iter()
            .map(|row| {
                row.iter()
                    .map(|opt| match opt {
                        Some(sq) => square_renderer(sq),
                        None => " ".to_string(),
                    })
                    .collect::<Vec<String>>()
                    .join(" ")
            })
            .collect::<Vec<String>>()
            .iter()
            .enumerate()
            .map(|(line_number, line)| line_transform(line_number, line))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new(9, 9)
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.render_squares(|sq| sq.to_string(), |_, s| s.clone()) // TODO: remove this clone
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Coordinate {
    pub x: usize,
    pub y: usize,
}

impl Coordinate {
    fn add(self, direction: Direction) -> Coordinate {
        match direction {
            Direction::North => Coordinate {
                x: self.x,
                y: usize::wrapping_sub(self.y, 1), // We use the computer graphics convention of (0,0) in the top left
            },
            Direction::South => Coordinate {
                x: self.x,
                y: self.y + 1,
            },
            Direction::East => Coordinate {
                x: self.x + 1,
                y: self.y,
            },
            Direction::West => Coordinate {
                x: usize::wrapping_sub(self.x, 1),
                y: self.y,
            },
        }
    }
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
            Square::Occupied(_, tile) => write!(f, "{}", tile),
        }
    }
}

impl Square {
    pub fn to_oriented_string(&self, orientations: &Vec<Direction>) -> String {
        match &self {
            Square::Empty => String::from("_"),
            Square::Occupied(player, tile) => {
                if *(orientations
                    .get(*player)
                    .expect("Should only pass valid players"))
                    == Direction::North
                {
                    Self::flip(tile).to_string()
                } else {
                    tile.to_string()
                }
            }
        }
    }

    fn flip(character: &char) -> char {
        const FLIPPED: [char; 26] = [
            // TODO: does this recompute every time, or is it created at compile time since it's a const?
            '∀', 'ꓭ', 'Ͻ', 'ᗡ', 'Ǝ', 'ᖵ', '⅁', 'H', 'I', 'ᒋ', 'ꓘ', '⅂', 'ꟽ', 'N', 'O', 'Ԁ', 'Ꝺ',
            'ꓤ', 'S', 'ꓕ', 'Ո', 'Ʌ', 'Ϻ', 'X', '⅄', 'Z',
        ];
        FLIPPED[(*character as usize) - 65]
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn makes_default_boards() {
        assert_eq!(
            Board::new(3, 1).to_string(),
            ["  _  ", "_ _ _", "  _  "].join("\n")
        );

        assert_eq!(
            Board::new(3, 2).to_string(),
            ["  _  ", "_ _ _", "_ _ _", "  _  "].join("\n")
        );

        assert_eq!(
            Board::new(2, 1).to_string(),
            ["_  ", "_ _", "  _"].join("\n")
        );

        assert_eq!(
            Board::new(5, 1).to_string(),
            ["    _    ", "_ _ _ _ _", "    _    "].join("\n")
        );

        assert_eq!(
            Board::new(6, 1).to_string(),
            ["    _      ", "_ _ _ _ _ _", "      _    "].join("\n")
        );
    }

    #[test]
    fn width_height() {
        let b = Board::new(6, 1);
        assert_eq!(b.width(), 6);
        assert_eq!(b.height(), 3);
    }

    #[test]
    fn getset_errors_out_of_bounds() {
        let mut b = Board::new(1, 1); // Note, height is 3 from home rows
        for position in [Coordinate { x: 1, y: 0 }, Coordinate { x: 0, y: 3 }] {
            assert_eq!(
                b.get(position),
                Err(GamePlayError::OutSideBoardDimensions { position })
            );
            assert_eq!(
                b.set(position, 0, 'a'),
                Err(GamePlayError::OutSideBoardDimensions { position })
            );
        }
    }

    #[test]
    fn getset_errors_for_dead_squares() {
        let mut b = Board::new(2, 1); // Note, height is 3 from home rows

        for position in [Coordinate { x: 1, y: 0 }, Coordinate { x: 0, y: 2 }] {
            assert_eq!(
                b.get(position),
                Err(GamePlayError::InvalidPosition { position })
            );
            assert_eq!(
                b.set(position, 0, 'a'),
                Err(GamePlayError::InvalidPosition { position })
            );
        }
    }

    #[test]
    fn getset_handles_empty_squares() {
        let mut b = Board::new(2, 1); // Note, height is 3 from home rows
        assert_eq!(b.get(Coordinate { x: 0, y: 0 }), Ok(Square::Empty));
        assert_eq!(b.get(Coordinate { x: 0, y: 1 }), Ok(Square::Empty));
        assert_eq!(b.get(Coordinate { x: 1, y: 1 }), Ok(Square::Empty));
        assert_eq!(b.get(Coordinate { x: 1, y: 2 }), Ok(Square::Empty));

        assert_eq!(
            b.set(Coordinate { x: 0, y: 0 }, 0, 'a'),
            Ok(ChangeDetail {
                square: Square::Occupied(0, 'a'),
                coordinate: Coordinate { x: 0, y: 0 },
            })
        );
        assert_eq!(
            b.set(Coordinate { x: 0, y: 1 }, 0, 'a'),
            Ok(ChangeDetail {
                square: Square::Occupied(0, 'a'),
                coordinate: Coordinate { x: 0, y: 1 },
            })
        );
        assert_eq!(
            b.set(Coordinate { x: 1, y: 1 }, 0, 'a'),
            Ok(ChangeDetail {
                square: Square::Occupied(0, 'a'),
                coordinate: Coordinate { x: 1, y: 1 },
            })
        );
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 0, 'a'),
            Ok(ChangeDetail {
                square: Square::Occupied(0, 'a'),
                coordinate: Coordinate { x: 1, y: 2 },
            })
        );
    }

    #[test]
    fn set_requires_valid_player() {
        let mut b = Board::new(2, 1);
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 0, 'a'),
            Ok(ChangeDetail {
                square: Square::Occupied(0, 'a'),
                coordinate: Coordinate { x: 1, y: 2 },
            })
        );
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 1, 'a'),
            Ok(ChangeDetail {
                square: Square::Occupied(1, 'a'),
                coordinate: Coordinate { x: 1, y: 2 },
            })
        );
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 2, 'a'),
            Err(GamePlayError::NonExistentPlayer { index: 2 })
        );
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 3, 'a'),
            Err(GamePlayError::NonExistentPlayer { index: 3 })
        );
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 100, 'a'),
            Err(GamePlayError::NonExistentPlayer { index: 100 })
        );
    }

    #[test]
    fn set_changes_get() {
        let mut b = Board::new(1, 1); // Note, height is 3 from home rows
        assert_eq!(b.get(Coordinate { x: 0, y: 0 }), Ok(Square::Empty));
        assert_eq!(
            b.set(Coordinate { x: 0, y: 0 }, 0, 'a'),
            Ok(ChangeDetail {
                square: Square::Occupied(0, 'a'),
                coordinate: Coordinate { x: 0, y: 0 },
            })
        );
        assert_eq!(
            b.get(Coordinate { x: 0, y: 0 }),
            Ok(Square::Occupied(0, 'a'))
        );
    }

    #[test]
    fn depth_first_search() {
        let mut b = Board::new(3, 1);

        // Create a connected tree
        let parts = [
            Coordinate { x: 2, y: 1 },
            Coordinate { x: 1, y: 1 },
            Coordinate { x: 1, y: 0 },
            Coordinate { x: 0, y: 1 },
        ];
        let parts_set = HashSet::from(parts);
        for part in parts {
            assert_eq!(
                b.set(part, 0, 'a'),
                Ok(ChangeDetail {
                    square: Square::Occupied(0, 'a'),
                    coordinate: part,
                })
            );
        }

        // The tree should be returned no matter where in the tree we start DFS from
        for part in parts {
            assert!(b.depth_first_search(part).is_subset(&parts_set));
            assert!(b.depth_first_search(part).is_superset(&parts_set));
        }

        // Set the remaining unoccupied square on the board to be occupied by another player
        let other = Coordinate { x: 1, y: 2 };
        // WHen unoccupied it should give the empty set, when occupied, just itself
        assert!(b.depth_first_search(other).iter().eq([].iter()));
        assert_eq!(
            b.set(other, 1, 'a'),
            Ok(ChangeDetail {
                square: Square::Occupied(1, 'a'),
                coordinate: other,
            })
        );
        assert!(b.depth_first_search(other).iter().eq([other].iter()));

        // The result of DFS on the main tree should not have changed
        for part in parts {
            assert!(b.depth_first_search(part).is_subset(&parts_set));
            assert!(b.depth_first_search(part).is_superset(&parts_set));
        }
    }

    #[test]
    fn get_neighbours() {
        // (0,0) (1,0) (2,0)
        // (0,1) (1,1) (2,1)
        // (0,2) (1,2) (2,2)
        // (0,3) (1,3) (2,3)
        // (0,4) (1,4) (2,4)
        let b = Board::new(3, 3);

        assert_eq!(
            // TODO: should we allow you to find neighbours of an invalid square?
            b.neighbouring_squares(Coordinate { x: 0, y: 0 }),
            [
                (Coordinate { x: 0, y: 1 }, Square::Empty),
                (Coordinate { x: 1, y: 0 }, Square::Empty),
            ]
        );

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 1, y: 0 }),
            [(Coordinate { x: 1, y: 1 }, Square::Empty),]
        );

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 1, y: 2 }),
            [
                (Coordinate { x: 1, y: 3 }, Square::Empty),
                (Coordinate { x: 2, y: 2 }, Square::Empty),
                (Coordinate { x: 1, y: 1 }, Square::Empty),
                (Coordinate { x: 0, y: 2 }, Square::Empty),
            ]
        );

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 1, y: 4 }),
            [(Coordinate { x: 1, y: 3 }, Square::Empty),]
        );
    }

    #[test]
    fn swap() {
        let mut b = Board::new(3, 1);
        let c0_1 = Coordinate { x: 0, y: 1 };
        let c1_1 = Coordinate { x: 1, y: 1 };
        let c2_1 = Coordinate { x: 2, y: 1 };
        assert_eq!(
            b.set(c0_1, 0, 'a'),
            Ok(ChangeDetail {
                square: Square::Occupied(0, 'a'),
                coordinate: c0_1,
            })
        );
        assert_eq!(
            b.set(c1_1, 0, 'b'),
            Ok(ChangeDetail {
                square: Square::Occupied(0, 'b'),
                coordinate: c1_1,
            })
        );
        assert_eq!(
            b.set(c2_1, 1, 'c'),
            Ok(ChangeDetail {
                square: Square::Occupied(1, 'c'),
                coordinate: c2_1,
            })
        );

        assert_eq!(b.get(c0_1), Ok(Square::Occupied(0, 'a')));
        assert_eq!(b.get(c1_1), Ok(Square::Occupied(0, 'b')));
        assert_eq!(
            b.swap(0, [c0_1, c1_1]),
            Ok(vec![
                Change {
                    detail: ChangeDetail {
                        square: Square::Occupied(0, 'b'),
                        coordinate: c0_1,
                    },
                    change: ChangeAction::Swapped
                },
                Change {
                    detail: ChangeDetail {
                        square: Square::Occupied(0, 'a'),
                        coordinate: c1_1,
                    },
                    change: ChangeAction::Swapped
                }
            ])
        );
        assert_eq!(b.get(c0_1), Ok(Square::Occupied(0, 'b')));
        assert_eq!(b.get(c1_1), Ok(Square::Occupied(0, 'a')));
        assert_eq!(b.swap(0, [c0_1, c0_1]), Err(GamePlayError::SelfSwap));
        assert_eq!(b.swap(0, [c0_1, c2_1]), Err(GamePlayError::UnownedSwap));
        assert_eq!(b.swap(1, [c0_1, c1_1]), Err(GamePlayError::UnownedSwap));
    }

    #[test]
    fn get_words() {
        // Should return an empty list of words for all points on an empty board, and for positions off the board
        let empty: Vec<Vec<Coordinate>> = vec![];
        let b = Board::default();
        for x in 0..12 {
            for y in 0..12 {
                assert_eq!(
                    b.get_words(Coordinate {
                        x: usize::wrapping_sub(x, 2),
                        y: usize::wrapping_sub(y, 2)
                    }),
                    empty
                );
            }
        }

        // Gets two words in the middle of a cross
        let b = from_string(
            [
                "_ _ C _ _",
                "_ _ R _ _",
                "S W O R D",
                "_ _ S _ _",
                "_ _ S _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 0, y: 0 }],
            vec![Direction::South],
        )
        .unwrap();
        let cross = ([0, 1, 2, 3, 4]).map(|y| Coordinate { x: 2, y }); // TODO: range
        let sword = ([0, 1, 2, 3, 4]).map(|x| Coordinate { x, y: 2 }); // TODO: range
        assert_eq!(b.get_words(Coordinate { x: 2, y: 2 }), vec![cross, sword]);

        let just_cross = ([0, 1, 3, 4]).map(|y| Coordinate { x: 2, y });
        for square in just_cross {
            assert_eq!(b.get_words(square), vec![cross]);
        }

        let just_sword = ([0, 1, 3, 4]).map(|x| Coordinate { x, y: 2 });
        for square in just_sword {
            assert_eq!(b.get_words(square), vec![sword]);
        }

        // Doesn't cross other players
        let mut b = from_string(
            [
                "_ _ C _ _",
                "_ _ R _ _",
                "_ _ O _ _",
                "_ _ S _ _",
                "_ _ S _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 0, y: 0 }, Coordinate { x: 4, y: 4 }],
            vec![Direction::South, Direction::North],
        )
        .unwrap();
        assert_eq!(
            b.get(Coordinate { x: 2, y: 4 }),
            Ok(Square::Occupied(0, 'S'))
        );
        assert_eq!(
            b.set(Coordinate { x: 3, y: 4 }, 1, 'O'),
            Ok(ChangeDetail {
                square: Square::Occupied(1, 'O'),
                coordinate: Coordinate { x: 3, y: 4 },
            })
        );
        assert_eq!(b.get_words(Coordinate { x: 2, y: 4 }), vec![cross]); // TODO: check coordinates
    }

    #[test]
    fn get_words_orientations() {
        let corners = vec![
            Coordinate { x: 0, y: 0 }, // Anti-clockwise from the top left
            Coordinate { x: 0, y: 6 },
            Coordinate { x: 6, y: 6 },
            Coordinate { x: 6, y: 0 },
        ];
        let cc = corners.clone();

        let b = from_string(
            [
                "N E Z _ G A N",
                "A _ _ _ _ _ E",
                "G _ _ _ _ _ Z",
                "_ _ _ _ _ _ _",
                "Z _ _ _ _ _ G",
                "E _ _ _ _ _ A",
                "N A G _ Z E N",
            ]
            .join("\n"),
            corners,
            vec![
                Direction::West,
                Direction::South,
                Direction::East,
                Direction::North,
            ],
        )
        .unwrap();

        for corner in cc {
            let mut words = b.word_strings(&b.get_words(corner)).unwrap();
            words.sort();
            assert_eq!(words, vec!["NAG", "ZEN"]);
        }
    }

    #[test]
    fn get_near_edge() {
        let b = Board::new(3, 1);
        assert_eq!(
            b.get_near_edge(Direction::North),
            vec![
                Coordinate { x: 0, y: 1 },
                Coordinate { x: 1, y: 1 },
                Coordinate { x: 2, y: 1 }
            ]
        );

        assert_eq!(
            b.get_near_edge(Direction::South),
            vec![
                Coordinate { x: 0, y: 1 },
                Coordinate { x: 1, y: 1 },
                Coordinate { x: 2, y: 1 }
            ]
        );
        assert_eq!(
            b.get_near_edge(Direction::East),
            vec![
                Coordinate { y: 0, x: 1 },
                Coordinate { y: 1, x: 1 },
                Coordinate { y: 2, x: 1 }
            ]
        );

        assert_eq!(
            b.get_near_edge(Direction::West),
            vec![
                Coordinate { y: 0, x: 1 },
                Coordinate { y: 1, x: 1 },
                Coordinate { y: 2, x: 1 }
            ]
        );
    }

    #[test]
    fn flipped() {
        assert_eq!(Square::flip(&'A'), '∀');
        assert_eq!(Square::flip(&'J'), 'ᒋ');
        assert_eq!(Square::flip(&'Z'), 'Z');
    }

    pub fn from_string<'a>(
        s: String,
        roots: Vec<Coordinate>,
        orientations: Vec<Direction>,
    ) -> Result<Board, &'a str> {
        if roots.len() != orientations.len() {
            return Err("Every player needs a root and orientation");
        }

        // Transform string into a board
        let mut squares: Vec<Vec<Option<Square>>> = vec![];
        for line in s.split('\n') {
            if line.chars().skip(1).step_by(2).any(|letter| letter != ' ') {
                return Err("board strings should have spaces to separate each tile");
            }

            squares.push(
                line.chars()
                    .step_by(2)
                    .map(|letter| match letter {
                        ' ' => None,
                        '_' => Some(Square::Empty),
                        letter => Some(Square::Occupied(0, letter)),
                    })
                    .collect(),
            );
        }

        // Make sure the board is an valid non-jagged grid
        if squares
            .iter()
            .skip(1)
            .any(|line| line.len() != squares[0].len())
        {
            return Err("Unequal line lengths");
        }

        // Make sure letters connected to players' roots are owned by the player
        let r = roots.clone(); // TODO: remove hack
        let mut board = Board {
            roots,
            squares,
            orientations,
        };
        for (player, root) in r.iter().enumerate() {
            if player != 0 {
                // All tiles are already owned by the first player by default
                for square in board.depth_first_search(*root).iter() {
                    if let Ok(Square::Occupied(_, value)) = board.get(*square) {
                        board.set(*square, player, value).expect(
                            "A coordinate returned from a DFS should always be valid and settable",
                        );
                    }
                }
            }
        }

        Ok(board)
    }

    #[test]
    fn from_string_fails() {
        assert_eq!(
            from_string(
                ["AXB"].join("\n"),
                vec![Coordinate { x: 0, y: 0 }],
                vec![Direction::North],
            ),
            Err("board strings should have spaces to separate each tile")
        );
    }

    #[test]
    fn from_string_succeeds() {
        // Checks that our default boards come are the same after being stringified and parsed
        let boards = [Board::default(), Board::new(34, 28)];
        for b in boards {
            assert_eq!(
                from_string(b.to_string(), b.roots.clone(), b.orientations.clone()),
                Ok(b)
            );
        }

        // Checks that various strings are the same when parsed and stringified
        let strings = [
            ["_ _ _", "_   _", "_ _ _"].join("\n"),
            ["_ X _", "_   A", "V _ _"].join("\n"),
            ["_ X _ _", "_ B A _", "V _ _ _", "  _ J _"].join("\n"),
        ];
        for s in strings {
            let s1 = s.clone();
            assert_eq!(
                from_string(s, vec![Coordinate { x: 0, y: 0 }], vec![Direction::South])
                    .unwrap()
                    .to_string(),
                s1
            );
        }

        // Checks that various complex boards have the correct players assigned to them
        // Donut board
        let top_left = Coordinate { x: 0, y: 0 };
        let top_right = Coordinate { x: 4, y: 0 };
        let bottom_left = Coordinate { x: 0, y: 4 };
        let bottom_right = Coordinate { x: 4, y: 4 };
        let dangling = Coordinate { x: 2, y: 3 };
        let hole = Coordinate { x: 2, y: 2 };
        let donut = from_string(
            [
                "A _ _ _ B",
                "_ _ _ _ _",
                "_ _   _ _",
                "_ _ D _ _",
                "C _ _ _ _",
            ]
            .join("\n"),
            vec![top_left, top_right, bottom_left, bottom_right],
            vec![Direction::North; 4],
        )
        .unwrap();
        assert_eq!(donut.get(top_left), Ok(Square::Occupied(0, 'A')));
        assert_eq!(donut.get(top_right), Ok(Square::Occupied(1, 'B')));
        assert_eq!(donut.get(bottom_left), Ok(Square::Occupied(2, 'C')));
        assert_eq!(
            donut.get(hole),
            Err(GamePlayError::InvalidPosition { position: hole })
        );
        assert_eq!(donut.get(dangling), Ok(Square::Occupied(0, 'D')));
        assert_eq!(donut.get(Coordinate { x: 1, y: 1 }), Ok(Square::Empty));

        // Complex trees
        let player_1 = [
            Coordinate { x: 2, y: 0 }, // First row
            Coordinate { x: 0, y: 1 }, // Second row
            Coordinate { x: 1, y: 1 },
            Coordinate { x: 2, y: 1 },
            Coordinate { x: 3, y: 1 },
            Coordinate { x: 4, y: 1 },
            Coordinate { x: 0, y: 2 }, // Third row
            Coordinate { x: 0, y: 3 }, // Fourth row
            Coordinate { x: 0, y: 4 }, // Fifth row
            Coordinate { x: 1, y: 4 },
            Coordinate { x: 0, y: 5 }, // Sixth row
        ];
        let player_2 = [
            Coordinate { x: 2, y: 6 }, // Seventh row
            Coordinate { x: 2, y: 5 }, // Sixth row
            Coordinate { x: 3, y: 5 },
            Coordinate { x: 3, y: 4 }, // Fifth row
            Coordinate { x: 2, y: 3 }, // Fourth row
            Coordinate { x: 3, y: 3 },
            Coordinate { x: 4, y: 3 },
        ];
        let complex_tree = from_string(
            [
                "    A    ",
                "A A A A A",
                "A _ _ _ _",
                "A _ B B B",
                "A A _ B _",
                "A _ B B _",
                "    B    ",
            ]
            .join("\n"),
            vec![player_1[0], player_2[0]],
            vec![Direction::North; 2],
        )
        .unwrap();

        for square in player_1 {
            assert_eq!(complex_tree.get(square), Ok(Square::Occupied(0, 'A')));
        }
        for square in player_2 {
            assert_eq!(complex_tree.get(square), Ok(Square::Occupied(1, 'B')));
        }
    }
}
