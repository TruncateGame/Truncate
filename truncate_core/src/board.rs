use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::slice::Iter;

use super::reporting::{BoardChange, BoardChangeAction, BoardChangeDetail};
use crate::bag::TileBag;
use crate::error::GamePlayError;
use crate::judge::WordDict;
use crate::reporting::Change;
use crate::rules;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    NorthWest,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
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
        use Direction::*;

        match self {
            NorthWest => SouthEast,
            North => South,
            NorthEast => SouthWest,
            East => West,
            SouthEast => NorthWest,
            South => North,
            SouthWest => NorthEast,
            West => East,
        }
    }
}

struct RedundantEdges {
    top: usize,
    right: usize,
    bottom: usize,
    left: usize,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Board {
    pub squares: Vec<Vec<Square>>,
    pub docks: Vec<Coordinate>,
    pub towns: Vec<Coordinate>,
    orientations: Vec<Direction>, // The side of the board that the player is sitting at, and the direction that their vertical words go in
                                  // TODO: Move orientations off the Board and have them tagged against specific players
}

// TODO: provide a way to validate the board
//  - the empty squares are fully connected
//  - there are at least 2 roots
//  - the roots are at empty squares

impl Board {
    pub fn new(land_width: usize, land_height: usize) -> Self {
        // TODO: resolve discrepancy between width parameter, and the actual width of the board (which is returned by self.width()) where `actual == width + 2` because of the extra home rows.
        // let roots = vec![
        //     Coordinate {
        //         x: land_width / 2 + land_width % 2 - 1,
        //         y: 0,
        //     },
        //     Coordinate {
        //         x: land_width / 2,
        //         y: land_height + 1,
        //     },
        // ];

        // Final board should have a ring of water around the land, in which to place the docks
        let board_width = land_width + 2;
        let board_height = land_height + 2;

        // Create a slice of land with water on the edges
        let mut land_row = vec![Square::Land; land_width];
        land_row.insert(0, Square::Water);
        land_row.push(Square::Water);

        let mut squares = vec![vec![Square::Water; board_width]]; // Start with our north row of water
        squares.extend(vec![land_row.clone(); land_height]); // Build out the centre land of the board
        squares.extend(vec![vec![Square::Water; board_width]]); // Finish with a south row of water

        let mut board = Board {
            squares,
            docks: vec![],
            towns: vec![], // TODO: populate
            orientations: vec![Direction::North, Direction::South],
        };

        let dock_x = board_width / 2;

        let north_towns = (1..=land_width)
            .filter(|x| *x != dock_x)
            .map(|x| Coordinate { x, y: 1 });
        for town in north_towns {
            board
                .set_square(
                    town,
                    Square::Town {
                        player: 0,
                        defeated: false,
                    },
                )
                .expect("Town square should exist on the land");
        }
        // North dock
        board
            .set_square(Coordinate { x: dock_x, y: 0 }, Square::Dock(0))
            .expect("Dock square should exist in the sea");

        let south_towns = (1..=land_width)
            .filter(|x| *x != dock_x)
            .map(|x| Coordinate {
                x,
                y: board_height - 2,
            });
        for town in south_towns {
            board
                .set_square(
                    town,
                    Square::Town {
                        player: 1,
                        defeated: false,
                    },
                )
                .expect("Town square should exist on the land");
        }
        // South dock
        board
            .set_square(
                Coordinate {
                    x: dock_x,
                    y: board_height - 1,
                },
                Square::Dock(1),
            )
            .expect("Dock square should exist in the sea");

        board.cache_special_squares();

        board
    }

    pub fn get_orientations(&self) -> &Vec<Direction> {
        &self.orientations
    }

    pub fn land_width(&self) -> usize {
        unimplemented!("Need to calculate the playable dimensions")
    }

    pub fn land_height(&self) -> usize {
        unimplemented!("Need to calculate the playable dimensions")
    }

    pub fn width(&self) -> usize {
        self.squares[0].len()
    }

    pub fn height(&self) -> usize {
        self.squares.len()
    }

    pub fn towns(&self) -> Iter<Coordinate> {
        self.towns.iter()
    }

    /// Adds water to all edges of the board
    pub fn grow(&mut self) {
        for row in &mut self.squares {
            row.insert(0, Square::Water);
            row.push(Square::Water);
        }

        self.squares.insert(0, vec![Square::Water; self.width()]);
        self.squares.push(vec![Square::Water; self.width()]);

        self.cache_special_squares();
    }

    /// Returns the number of rows/columns
    fn redundant_edges(&self) -> RedundantEdges {
        let top = self
            .squares
            .iter()
            .position(|row| {
                row.iter()
                    .any(|s| !matches!(s, Square::Water | Square::Fog | Square::Dock(_)))
            })
            .unwrap_or_default()
            .saturating_sub(1);

        let bottom = self
            .squares
            .iter()
            .rev()
            .position(|row| {
                row.iter()
                    .any(|s| !matches!(s, Square::Water | Square::Fog | Square::Dock(_)))
            })
            .unwrap_or_default()
            .saturating_sub(1);

        let left = (0..self.width())
            .position(|i| {
                self.squares
                    .iter()
                    .any(|row| !matches!(row[i], Square::Water | Square::Fog | Square::Dock(_)))
            })
            .unwrap_or_default()
            .saturating_sub(1);

        let right = (0..self.width())
            .rev()
            .position(|i| {
                self.squares
                    .iter()
                    .any(|row| !matches!(row[i], Square::Water | Square::Fog | Square::Dock(_)))
            })
            .unwrap_or_default()
            .saturating_sub(1);

        RedundantEdges {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Trims edges containing only empty squares
    pub fn trim(&mut self) {
        let trim = self.redundant_edges();

        for _ in 0..trim.top {
            self.squares.remove(0);
        }
        for _ in 0..trim.bottom {
            self.squares.remove(self.height() - 1);
        }
        for row in &mut self.squares {
            for _ in 0..trim.left {
                row.remove(0);
            }
            for _ in 0..trim.right {
                row.remove(row.len() - 1);
            }
        }
        self.cache_special_squares();
    }

    pub fn cache_special_squares(&mut self) {
        let rows = self.height();
        let cols = self.width();
        // TODO: Implement iterators for board and pull this out
        let coords = (0..rows)
            .flat_map(|y| (0..cols).zip(std::iter::repeat(y)))
            .map(|(x, y)| Coordinate { x, y });

        self.docks.clear();
        self.towns.clear();

        for coord in coords {
            match self.get(coord) {
                Ok(Square::Water | Square::Land | Square::Occupied { .. } | Square::Fog) => {}
                Ok(Square::Town { .. }) => self.towns.push(coord),
                Ok(Square::Dock(_)) => self.docks.push(coord),
                Err(e) => {
                    unreachable!(
                        "Iterating over the board should not return invalid positions: {e}"
                    )
                }
            }
        }
    }

    pub fn get(&self, position: Coordinate) -> Result<Square, GamePlayError> {
        match self
            .squares
            .get(position.y)
            .and_then(|row| row.get(position.x))
        {
            Some(square) => Ok(*square),
            None => Err(GamePlayError::OutSideBoardDimensions { position }),
        }
    }

    pub fn get_mut<'a>(
        &'a mut self,
        position: Coordinate,
    ) -> Result<&'a mut Square, GamePlayError> {
        match self
            .squares
            .get_mut(position.y)
            .and_then(|row| row.get_mut(position.x))
        {
            Some(square) => Ok(square),
            None => Err(GamePlayError::OutSideBoardDimensions { position }),
        }
    }

    pub fn set_square(
        &mut self,
        position: Coordinate,
        new_square: Square,
    ) -> Result<(), GamePlayError> {
        let square = self
            .squares
            .get_mut(position.y)
            .and_then(|row| row.get_mut(position.x));

        let Some(square) = square else {
            return Err(GamePlayError::OutSideBoardDimensions { position });
        };

        *square = new_square;

        Ok(())
    }

    pub fn set(
        &mut self,
        position: Coordinate,
        player: usize,
        tile: char,
        ref_dict: Option<&WordDict>,
    ) -> Result<BoardChangeDetail, GamePlayError> {
        if self.docks.get(player).is_none() {
            return Err(GamePlayError::NonExistentPlayer { index: player });
        }

        match self
            .squares
            .get_mut(position.y)
            .and_then(|row| row.get_mut(position.x))
        {
            Some(square) if matches!(square, Square::Land | Square::Occupied { .. }) => {
                *square = Square::Occupied {
                    player,
                    tile,
                    validity: SquareValidity::Unknown,
                };
                Ok(())
            }
            Some(_) => Err(GamePlayError::InvalidPosition { position }),
            None => Err(GamePlayError::OutSideBoardDimensions { position }),
        }?;

        self.mark_validity(position, ref_dict);

        Ok(BoardChangeDetail {
            square: self.get(position).unwrap().clone(),
            coordinate: position,
        })
    }

    pub fn swap(
        &mut self,
        player: usize,
        positions: [Coordinate; 2],
        swap_rules: &rules::Swapping,
        ref_dict: Option<&WordDict>,
    ) -> Result<Vec<Change>, GamePlayError> {
        if positions[0] == positions[1] {
            return Err(GamePlayError::SelfSwap);
        }

        let mut tiles = ['&'; 2];
        for (i, pos) in positions.iter().enumerate() {
            use Square::*;
            match self.get(*pos)? {
                Occupied {
                    player: owner,
                    tile,
                    validity: _,
                } => {
                    if owner != player {
                        return Err(GamePlayError::UnownedSwap);
                    }
                    tiles[i] = tile;
                }
                Water | Land | Fog | Town { .. } | Dock(_) => {
                    return Err(GamePlayError::UnoccupiedSwap)
                }
            };
        }

        if tiles[0] == tiles[1] {
            return Err(GamePlayError::NoopSwap);
        }

        match swap_rules {
            rules::Swapping::Contiguous(_) => {
                if self
                    .depth_first_search(positions[0])
                    .get(&positions[1])
                    .is_none()
                {
                    return Err(GamePlayError::DisjointSwap);
                }
            }
            rules::Swapping::Universal(_) => { /* All swaps are allowed */ }
            rules::Swapping::None => {
                return Err(GamePlayError::NoSwapping);
            }
        }

        Ok(vec![
            Change::Board(BoardChange {
                detail: self.set(positions[0], player, tiles[1], ref_dict)?,
                action: BoardChangeAction::Swapped,
            }),
            Change::Board(BoardChange {
                detail: self.set(positions[1], player, tiles[0], ref_dict)?,
                action: BoardChangeAction::Swapped,
            }),
        ])
    }

    // TODO: safety on index access like get and set - ideally combine error checking for all 3
    pub fn clear(
        &mut self,
        position: Coordinate,
        ref_dict: Option<&WordDict>,
    ) -> Option<BoardChangeDetail> {
        if let Some(square) = self
            .squares
            .get_mut(position.y as usize)
            .and_then(|y| y.get_mut(position.x as usize))
        {
            if matches!(square, Square::Occupied { .. }) {
                let change = Some(BoardChangeDetail {
                    square: *square,
                    coordinate: position,
                });
                *square = Square::Land;

                self.neighbouring_squares(position)
                    .into_iter()
                    .filter(|(_, s)| matches!(s, Square::Occupied { .. }))
                    .for_each(|(c, _)| self.mark_validity(c, ref_dict));

                return change;
            }
        }
        None
    }

    pub fn reset(&mut self) {
        let rows = self.height();
        let cols = self.width();
        // TODO: Implement iterators for board and pull this out
        let coords = (0..rows)
            .flat_map(|y| (0..cols).zip(std::iter::repeat(y)))
            .map(|(x, y)| Coordinate { x, y });

        for coord in coords {
            let Ok(sq) = self.get_mut(coord) else {
                unreachable!("Iterating over the board should not return invalid positions");
            };
            match sq {
                Square::Occupied { .. } => *sq = Square::Land,
                Square::Town { player, .. } => {
                    *sq = Square::Town {
                        player: player.clone(),
                        defeated: false,
                    }
                }
                _ => {}
            }
        }
    }

    pub fn defeat_player(&mut self, player_to_defeat: usize) {
        let towns = self.towns.clone();
        for town in towns {
            let Ok(sq) = self.get_mut(town) else {
                continue;
            };
            match sq {
                Square::Town { player, .. } if *player == player_to_defeat => {
                    *sq = Square::Town {
                        player: player_to_defeat,
                        defeated: true,
                    }
                }
                _ => {}
            }
        }
    }

    pub fn neighbouring_squares(&self, position: Coordinate) -> Vec<(Coordinate, Square)> {
        position
            .neighbors_4()
            .into_iter()
            .filter_map(|pos| {
                if let Ok(square) = self.get(pos) {
                    Some((pos, square))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Board {
    pub fn mark_all_validity(&mut self, ref_dict: Option<&WordDict>) {
        let rows = self.height();
        let cols = self.width();
        let squares = (0..rows).flat_map(|y| (0..cols).zip(std::iter::repeat(y)));

        for (x, y) in squares {
            self.mark_validity(Coordinate::new(x, y), ref_dict);
        }
    }

    pub fn mark_validity(&mut self, modified_position: Coordinate, ref_dict: Option<&WordDict>) {
        let coords = self.get_words(modified_position);
        let Ok(words) = self.word_strings(&coords) else {
            return;
        };

        let Some(ref_dict) = ref_dict else {
            return;
        };

        for (coords, word) in coords.into_iter().zip(words.into_iter()) {
            // TODO: Use the full judge here to handle, e.g., wildcards
            let main_word_valid = ref_dict.contains_key(&word.to_ascii_lowercase());
            let ideal_validity = if main_word_valid {
                SquareValidity::Valid
            } else {
                SquareValidity::Invalid
            };

            for coord in coords {
                let nested_coords = self.get_words(coord);
                let mut square_validity = ideal_validity;
                // For the tiles in the two possible "main" words,
                // we also need to assess the other words they're a part of
                if nested_coords.len() > 1 {
                    let Ok(words) = self.word_strings(&nested_coords) else {
                        return;
                    };
                    let valid_words: Vec<_> = words
                        .into_iter()
                        .map(|w| ref_dict.contains_key(&w.to_ascii_lowercase()))
                        .collect();
                    if main_word_valid && valid_words.contains(&false) {
                        square_validity = SquareValidity::Partial;
                    }
                    if !main_word_valid && valid_words.contains(&true) {
                        square_validity = SquareValidity::Partial;
                    }
                }

                match self.get_mut(coord) {
                    Ok(Square::Occupied { validity, .. }) => *validity = square_validity,
                    _ => {}
                }
            }
        }
    }

    pub fn truncate(&mut self, bag: &mut TileBag, ref_dict: Option<&WordDict>) -> Vec<Change> {
        let mut attatched = HashSet::new();
        for root in self.docks.iter() {
            attatched.extend(self.depth_first_search(*root));
        }

        let rows = self.height();
        let cols = self.width();
        let squares = (0..rows).flat_map(|y| (0..cols).zip(std::iter::repeat(y)));

        squares
            .flat_map(|(x, y)| {
                let c = Coordinate { x, y };
                if !attatched.contains(&c) {
                    if let Ok(Square::Occupied { tile, .. }) = self.get(c) {
                        bag.return_tile(tile);
                    }
                    self.clear(c, ref_dict).map(|detail| {
                        Change::Board(BoardChange {
                            detail,
                            action: BoardChangeAction::Truncated,
                        })
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    // TODO: return iterator or rename since it doesn't matter that this is depth first when we return a HashSet
    pub fn depth_first_search(&self, position: Coordinate) -> HashSet<Coordinate> {
        let mut visited = HashSet::new();

        fn dfs(b: &Board, position: Coordinate, visited: &mut HashSet<Coordinate>) {
            let player = match b.get(position) {
                Ok(Square::Occupied { player, .. }) => Some(player),
                Ok(Square::Dock(player)) => Some(player),
                _ => None,
            };
            if let Some(player) = player {
                visited.insert(position);
                for (position, square) in b.neighbouring_squares(position) {
                    if let Square::Occupied {
                        player: neighbours_player,
                        ..
                    } = square
                    {
                        if !visited.contains(&position) && player == neighbours_player {
                            dfs(b, position, visited);
                        };
                    }
                }
            }
        }

        dfs(self, position, &mut visited);
        visited
    }

    pub fn flood_fill(&self, starting_pos: &Coordinate) -> BoardDistances {
        let mut distances = BoardDistances::new(self);
        let attacker = self
            .get(*starting_pos)
            .ok()
            .map(|sq| match sq {
                Square::Occupied { player, .. } => Some(player),
                Square::Dock(player) => Some(player),
                _ => None,
            })
            .flatten();

        let adjacent_to_opponent = |sqs: &Vec<(Coordinate, Square)>| {
            sqs.iter().any(|(_, n)| match n {
                Square::Occupied { player, .. } if Some(*player) != attacker => true,
                Square::Town { player, .. } if Some(*player) != attacker => true,
                _ => false,
            })
        };

        distances.set_attackable(starting_pos, 0);
        let initial_neighbors = self.neighbouring_squares(*starting_pos);
        let mut attackable_pts: VecDeque<_> = initial_neighbors.iter().map(|n| (n.0, 0)).collect();
        let mut direct_pts: VecDeque<(Coordinate, usize)> = VecDeque::new();

        while !attackable_pts.is_empty() {
            let (pt, dist) = attackable_pts.pop_front().unwrap();

            match distances.attackable_distance_mut(&pt) {
                Some(Some(visited_dist)) => {
                    if *visited_dist > dist {
                        // We have now found a better path to this point, so we will reprocess it
                        *visited_dist = dist;
                    } else {
                        // We have previously found a better (or equal) path to this point, move to the next
                        continue;
                    }
                }
                _ => {
                    distances.set_attackable(&pt, dist);
                }
            }

            match self.get(pt) {
                Ok(Square::Occupied { player, .. }) if Some(player) == attacker => {
                    let neighbors = self.neighbouring_squares(pt);

                    // We found another one of our tiles — search its neighbors with a new starting distance
                    attackable_pts.extend(neighbors.iter().map(|n| (n.0, 0)));
                    distances.set_attackable(&pt, 0);
                }
                Ok(Square::Land) => {
                    let neighbors = self.neighbouring_squares(pt);

                    if adjacent_to_opponent(&neighbors) {
                        // This tile is touching the opponent.
                        // We don't want to flood fill any more adjacent land since we
                        // can't play _through_ this tile, but we do want to visit any
                        // adjacent towns and tiles since they would be attacked by playing here.
                        attackable_pts.extend(
                            neighbors
                                .iter()
                                .filter(|(_, sq)| !matches!(sq, Square::Land))
                                .map(|n| (n.0, dist + 1)),
                        );
                        // We also put these neighbor tiles into the list for the next stage,
                        // when BFSing the rest of the board
                        direct_pts.extend(neighbors.iter().map(|n| (n.0, dist + 1)));
                    } else {
                        // This tile is clear land — continue to flood fill everything
                        attackable_pts.extend(neighbors.iter().map(|n| (n.0, dist + 1)));
                    }
                }
                Ok(Square::Water) => continue,
                Ok(_) => {
                    let neighbors = self.neighbouring_squares(pt);
                    // Falling through from the above, these tiles are the edges of our attacking BFS.
                    // We put them aside to use as the starting list for our full-board DFS
                    direct_pts.extend(neighbors.iter().map(|n| (n.0, dist + 1)));
                }
                _ => continue,
            }
        }

        distances.copy_to_direct();

        while !direct_pts.is_empty() {
            let (pt, dist) = direct_pts.pop_front().unwrap();

            match distances.direct_distance_mut(&pt) {
                Some(Some(visited_dist)) => {
                    if *visited_dist > dist {
                        // We have now found a better path to this point, so we will reprocess it
                        *visited_dist = dist;
                    } else {
                        // We have previously found a better (or equal) path to this point, move to the next
                        continue;
                    }
                }
                _ => {
                    distances.set_direct(&pt, dist);
                }
            }

            match self.get(pt) {
                Ok(Square::Water) => continue,
                Ok(_) => {
                    let neighbors = self.neighbouring_squares(pt);
                    direct_pts.extend(neighbors.iter().map(|n| (n.0, dist + 1)));
                }
                _ => continue,
            }
        }

        distances
    }

    pub fn flood_fill_attacks(&self, attacker: usize) -> BoardDistances {
        let pos_is_attacker = |pos: &Coordinate| match self.get(*pos) {
            Ok(Square::Occupied { player, .. }) if player == attacker => true,
            _ => false,
        };

        let rows = self.height();
        let cols = self.width();

        // Always evaluate tiles furthest down the board first
        let outermost_attacker = if attacker == 0 {
            (0..rows)
                .rev()
                .flat_map(|y| (0..cols).zip(std::iter::repeat(y)))
                .map(|(x, y)| Coordinate { x, y })
                .find(pos_is_attacker)
        } else {
            (0..rows)
                .flat_map(|y| (0..cols).zip(std::iter::repeat(y)))
                .map(|(x, y)| Coordinate { x, y })
                .find(pos_is_attacker)
        };

        let Some(outermost_attacker) = outermost_attacker else {
            // Attacker has no tiles, cannot reach anywhere.
            // TODO: count from docks?
            return BoardDistances::new(self);
        };

        self.flood_fill(&outermost_attacker)
    }

    /// Find the shortest land path between any two points on a board.
    /// Does NOT take into account tiles defended by either player,
    /// so isn't strictly correct once gameplay has begun.
    /// Returned path is exlusive of the start and end points.
    pub fn shortest_path_between(
        &self,
        starting_pos: &Coordinate,
        ending_pos: &Coordinate,
    ) -> Option<Vec<Coordinate>> {
        let mut distances = BoardDistances::new(self);
        distances.set_direct(starting_pos, 0);

        let initial_neighbors = self.neighbouring_squares(*starting_pos);
        let mut bfs_queue: VecDeque<_> = initial_neighbors.iter().map(|n| (n.0, vec![])).collect();

        while !bfs_queue.is_empty() {
            let (pt, mut path) = bfs_queue.pop_front().unwrap();

            if pt == *ending_pos {
                return Some(path);
            }

            path.push(pt);

            match distances.direct_distance_mut(&pt) {
                Some(Some(visited_dist)) => {
                    if *visited_dist > path.len() {
                        // We have now found a better path to this point, so we will reprocess it
                        *visited_dist = path.len();
                    } else {
                        // We have previously found a better (or equal) path to this point, move to the next
                        continue;
                    }
                }
                _ => {
                    distances.set_direct(&pt, path.len());
                }
            }

            match self.get(pt) {
                Ok(Square::Land) => {
                    let neighbors = self.neighbouring_squares(pt);
                    bfs_queue.extend(neighbors.iter().map(|n| (n.0, path.clone())));
                }
                _ => continue,
            }
        }

        return None;
    }

    /// Finds the nearest non-land tile (assuming all play must happen on land).
    /// Allows certain points on the board to be ignored, to create false deadzones.
    pub fn distance_to_closest_obstruction(
        &self,
        pt: &Coordinate,
        excluding: &Vec<Coordinate>,
    ) -> usize {
        // Using BoardDistances here as a visited map — the distances themselves are unused.
        let mut distances = BoardDistances::new(self);

        let mut bfs_queue = VecDeque::from([(*pt, 0)]);
        let mut last_processed_distance = 0;

        while !bfs_queue.is_empty() {
            let (pt, dist) = bfs_queue.pop_front().unwrap();

            if excluding.contains(&pt) {
                continue;
            }

            // Move on if we have ever visited this point,
            // as this is a pure BFS.
            if distances.direct_distance(&pt).is_some() {
                continue;
            }
            distances.set_direct(&pt, 0); // distance unused.
            last_processed_distance = dist;

            match self.get(pt) {
                Ok(Square::Land) => {
                    let neighbors = self.neighbouring_squares(pt);
                    bfs_queue.extend(neighbors.iter().map(|n| (n.0, dist + 1)));
                }
                _ => return dist, // We have hit our closest obstruction (non-land), so we can bail out now
            }
        }

        // Unlikely, but catches if the entire board is clear land with no water
        return last_processed_distance;
    }

    pub fn get_shape(&self) -> Vec<u64> {
        let width = self.width();
        let num_buckets = Coordinate {
            x: self.width() - 1,
            y: self.height() - 1,
        }
        .to_1d(width)
            / 64
            + 1;

        let mut out = vec![0; num_buckets];

        for (y, row) in self.squares.iter().enumerate() {
            for (x, square) in row.iter().enumerate() {
                if matches!(square, Square::Occupied { .. }) {
                    let c = Coordinate { x, y }.to_1d(width);
                    let bucket = c / 64;
                    out[bucket] |= 1 << (c % 64);
                }
            }
        }

        out
    }

    pub fn get_words(&self, position: Coordinate) -> Vec<Vec<Coordinate>> {
        let mut words: Vec<Vec<Coordinate>> = Vec::new();
        let owner = match self.get(position) {
            Ok(Square::Occupied { player, .. }) => player,
            Ok(Square::Town { .. }) => return vec![vec![position]],
            _ => return words,
        };

        let axes = [
            [Direction::South, Direction::North],
            [Direction::East, Direction::West],
        ];

        // Build each of the two possible words from either side
        for axis in axes {
            let mut word = vec![position];
            for direction in axis {
                let fowards = direction == Direction::South || direction == Direction::East;
                let mut location = position.add(direction);

                while let Ok(Square::Occupied { player, .. }) = self.get(location) {
                    if player != owner {
                        break;
                    }
                    if fowards {
                        word.push(location);
                    } else {
                        word.insert(0, location);
                    }
                    location = location.add(direction);
                }
            }
            words.push(word);
        }

        // Reverse words based on the player's orientation
        let orientation = self.orientations[owner];
        if !orientation.read_top_to_bottom() {
            words[0].reverse();
        }
        if !orientation.read_left_to_right() {
            words[1].reverse();
        }

        // 1 letter words don't count except when there's only one tile, in which case it does count as a word
        if words.iter().all(|w| w.len() == 1) {
            words
        } else {
            words.into_iter().filter(|word| word.len() > 1).collect()
        }
    }

    pub fn collect_combanants(
        &self,
        player: usize,
        position: Coordinate,
    ) -> (Vec<Vec<Coordinate>>, Vec<Vec<Coordinate>>) {
        let attackers = self.get_words(position);
        // Any neighbouring square belonging to another player is attacked. The words containing those squares are the defenders.
        let defenders = self
            .neighbouring_squares(position)
            .iter()
            .filter(|(_, square)| match square {
                Square::Occupied {
                    player: adjacent_player,
                    ..
                } => player != *adjacent_player,
                Square::Town {
                    player: adjacent_player,
                    defeated,
                } => player != *adjacent_player && !defeated,
                _ => false,
            })
            .flat_map(|(position, _)| self.get_words(*position))
            .collect();
        (attackers, defenders)
    }

    pub fn word_strings(
        &self,
        coordinates: &Vec<Vec<Coordinate>>,
    ) -> Result<Vec<String>, GamePlayError> {
        let mut err = None; // TODO: is this a reasonable error handling method? We can't return an Err from the function from within the closure passed to map.
        use Square::*;
        let strings = coordinates
            .iter()
            .map(|word| {
                word.iter()
                    .map(|&square| match self.get(square) {
                        Ok(sq) => match sq {
                            Water | Land | Fog | Dock(_) => {
                                debug_assert!(false);
                                err = Some(GamePlayError::EmptySquareInWord);
                                '_'
                            }
                            Town { .. } => '#',
                            Occupied { tile, .. } => tile,
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

    // TODO: This needs to look at all tiles to work in no truncation mode
    pub fn playable_positions(&self, for_player: usize) -> HashSet<Coordinate> {
        let mut playable_squares = HashSet::new();
        for dock in &self.docks {
            let sq = self.get(*dock).unwrap();
            if !matches!(sq, Square::Dock(p) if p == for_player) {
                continue;
            }

            playable_squares.extend(
                self.depth_first_search(*dock)
                    .iter()
                    .flat_map(|sq| sq.neighbors_4())
                    .filter(|sq| matches!(self.get(*sq), Ok(Square::Land)))
                    .collect::<HashSet<_>>(),
            );
        }
        playable_squares
    }

    pub fn fog_of_war(&self, player_index: usize, visibility: &rules::Visibility) -> Self {
        let mut visible_coords: HashSet<Coordinate> = HashSet::new();

        let rows = self.height();
        let cols = self.width();
        let squares = (0..rows).flat_map(|y| (0..cols).zip(std::iter::repeat(y)));

        for (coord, square) in
            squares.map(|(x, y)| (Coordinate { x, y }, self.get(Coordinate { x, y })))
        {
            match square {
                Ok(Square::Occupied { player, .. })
                | Ok(Square::Dock(player))
                | Ok(Square::Town { player, .. })
                    if player == player_index =>
                {
                    visible_coords.insert(coord);

                    for (coord, square) in self.neighbouring_squares(coord) {
                        visible_coords.insert(coord);
                        match square {
                            Square::Occupied { player, .. } if player != player_index => {
                                visible_coords.extend(self.get_words(coord).iter().flatten());
                            }
                            _ => {}
                        }
                    }

                    // TODO: Enumerate squares a given manhattan distance away, as this double counts
                    for (coord, square) in self
                        .neighbouring_squares(coord)
                        .iter()
                        .flat_map(|(c, _)| self.neighbouring_squares(*c))
                        .collect::<Vec<_>>()
                    {
                        visible_coords.insert(coord);
                        match square {
                            Square::Occupied { player, .. } if player != player_index => {
                                visible_coords.extend(self.get_words(coord).iter().flatten());
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        let mut new_board = self.clone();

        let rows = self.height();
        let cols = self.width();
        let squares = (0..rows).flat_map(|y| (0..cols).zip(std::iter::repeat(y)));

        match visibility {
            rules::Visibility::Standard => {}
            rules::Visibility::TileFog => {
                for (x, y) in squares {
                    let c = Coordinate { x, y };
                    let is_tile = matches!(new_board.get(c), Ok(Square::Occupied { .. }));
                    if !visible_coords.contains(&c) && is_tile {
                        _ = new_board.set_square(c, Square::Land);
                    }
                }
            }
            rules::Visibility::LandFog => {
                for (x, y) in squares {
                    let c = Coordinate { x, y };
                    if !visible_coords.contains(&c) {
                        _ = new_board.set_square(c, Square::Fog);
                    }
                }
            }
        }

        new_board
    }

    /// Used for fog of war modes.
    /// Takes the coordinate given by a player, and maps it back
    /// to the full board that the player cannot see ( and thus does not have coordinates for)
    pub fn map_player_coord_to_game(
        &self,
        player_index: usize,
        player_coordinate: Coordinate,
        visibility: &rules::Visibility,
    ) -> Coordinate {
        let foggy_board = match visibility {
            rules::Visibility::Standard | rules::Visibility::TileFog => {
                // In these modes, the player knows the full coordinate space, so no remapping is required.
                return player_coordinate;
            }
            rules::Visibility::LandFog => self.fog_of_war(player_index, visibility),
        };

        let redundant_player = foggy_board.redundant_edges();
        let redundant_global = self.redundant_edges();

        Coordinate {
            x: player_coordinate.x + (redundant_player.left - redundant_global.left),
            y: player_coordinate.y + (redundant_player.top - redundant_global.top),
        }
    }

    /// Used for fog of war modes.
    /// Takes a concrete game coordinate, and maps it to the visible coordinate space of the player
    pub fn map_game_coord_to_player(
        &self,
        player_index: usize,
        game_coordinate: Coordinate,
        visibility: &rules::Visibility,
    ) -> Option<Coordinate> {
        let foggy_board = match visibility {
            rules::Visibility::Standard | rules::Visibility::TileFog => {
                // In these modes, the player knows the full coordinate space, so no remapping is required.
                return Some(game_coordinate);
            }
            rules::Visibility::LandFog => self.fog_of_war(player_index, visibility),
        };

        let redundant_player = foggy_board.redundant_edges();
        let redundant_global = self.redundant_edges();

        let Some(x) = game_coordinate
            .x
            .checked_sub(redundant_player.left - redundant_global.left)
        else {
            return None;
        };
        let Some(y) = game_coordinate
            .y
            .checked_sub(redundant_player.top - redundant_global.top)
        else {
            return None;
        };

        Some(Coordinate { x, y })
    }

    pub(crate) fn filter_to_player(
        &self,
        player_index: usize,
        visibility: &rules::Visibility,
        winner: &Option<usize>,
    ) -> Self {
        // All visibility is restored when the game ends
        if winner.is_some() {
            return self.clone();
        }

        match visibility {
            rules::Visibility::Standard => self.clone(),
            rules::Visibility::TileFog | rules::Visibility::LandFog => {
                let mut foggy = self.fog_of_war(player_index, visibility);
                // Remove extraneous water, so the client doesn't know the dimensions of the play area
                foggy.trim();

                foggy
            }
        }
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new(9, 9)
    }
}

impl Board {
    pub fn from_string<S: AsRef<str>>(s: S) -> Board {
        // Transform string into a board
        let mut squares: Vec<Vec<Square>> = vec![];
        for line in s.as_ref().split('\n') {
            if line.chars().all(|c| c.is_whitespace()) {
                continue;
            };
            squares.push(
                line.trim()
                    .split(' ')
                    .map(|tile| {
                        let mut chars = tile.chars();
                        match chars.next() {
                            Some('~') => Square::Water,
                            Some('_') => Square::Land,
                            Some('|') => Square::Dock(
                                chars
                                    .next()
                                    .expect("Square needs player")
                                    .to_digit(10)
                                    .unwrap() as usize,
                            ),
                            Some('#') => Square::Town {
                                player: chars
                                    .next()
                                    .expect("Square needs player")
                                    .to_digit(10)
                                    .unwrap() as usize,
                                defeated: false,
                            },
                            Some(tile) => Square::Occupied {
                                player: chars
                                    .next()
                                    .expect("Square needs player")
                                    .to_digit(10)
                                    .unwrap() as usize,
                                tile,
                                validity: SquareValidity::Unknown,
                            },
                            _ => panic!("Couldn't build board from string"),
                        }
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
            panic!("Tried to make a jagged board");
        }

        let mut board = Board {
            squares,
            towns: vec![],
            docks: vec![],
            orientations: vec![Direction::North, Direction::South],
        };
        board.cache_special_squares();

        board
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.squares
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|sq| sq.to_string())
                        .collect::<Vec<String>>()
                        .join(" ")
                })
                .enumerate()
                .map(|(_line_number, line)| line)
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord, Deserialize, Serialize)]
pub struct Coordinate {
    pub x: usize,
    pub y: usize,
}

impl Coordinate {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub fn add(self, direction: Direction) -> Coordinate {
        use Direction::*;

        Coordinate {
            x: match direction {
                West | NorthWest | SouthWest => usize::wrapping_sub(self.x, 1),
                East | NorthEast | SouthEast => self.x + 1,
                North | South => self.x,
            },
            y: match direction {
                North | NorthEast | NorthWest => usize::wrapping_sub(self.y, 1),
                South | SouthEast | SouthWest => self.y + 1,
                East | West => self.y,
            },
        }
    }

    pub fn to_1d(&self, width: usize) -> usize {
        return self.x + self.y * width;
    }

    pub fn from_1d(oned: usize, width: usize) -> Self {
        Self {
            x: oned % width,
            y: oned / width,
        }
    }

    /// Return coordinates of the horizontal and vertical neighbors, from north clockwise
    pub fn neighbors_4(&self) -> [Coordinate; 4] {
        use Direction::*;

        [
            self.add(North),
            self.add(East),
            self.add(South),
            self.add(West),
        ]
    }

    /// Return coordinates of the horizontal, vertical, and diagonal neighbors, from northwest clockwise
    pub fn neighbors_8(&self) -> [Coordinate; 8] {
        use Direction::*;

        [
            self.add(NorthWest),
            self.add(North),
            self.add(NorthEast),
            self.add(East),
            self.add(SouthEast),
            self.add(South),
            self.add(SouthWest),
            self.add(West),
        ]
    }

    pub fn distance_to(&self, other: &Coordinate) -> usize {
        self.x.abs_diff(other.x) + self.y.abs_diff(other.y)
    }
}

impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl std::cmp::PartialEq<(usize, usize)> for Coordinate {
    fn eq(&self, (x, y): &(usize, usize)) -> bool {
        return self.x == *x && self.y == *y;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SquareValidity {
    Unknown,
    Valid,
    Invalid,
    Partial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Square {
    Water,
    Land,
    Town {
        player: usize,
        defeated: bool,
    },
    Dock(usize),
    Occupied {
        player: usize,
        tile: char,
        validity: SquareValidity,
    },
    Fog,
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Square::Water => write!(f, "~~"),
            Square::Fog => write!(f, "░░"),
            Square::Land => write!(f, "__"),
            Square::Town {
                player: p,
                defeated: false,
            } => write!(f, "#{p}"),
            Square::Town {
                player: p,
                defeated: true,
            } => write!(f, "⊭{p}"),
            Square::Dock(p) => write!(f, "|{p}"),
            Square::Occupied {
                player: p,
                tile,
                validity: _,
            } => write!(f, "{tile}{p}"),
        }
    }
}

#[derive(Clone)]
pub struct BoardDistances {
    pub board_width: usize,
    pub attackable: Vec<Option<usize>>,
    pub direct: Vec<Option<usize>>,
}

impl BoardDistances {
    pub fn new(board: &Board) -> Self {
        let max_cord = Coordinate {
            x: board.width() - 1,
            y: board.height() - 1,
        };
        let len = max_cord.to_1d(board.width()) + 1;
        Self {
            board_width: board.width(),
            attackable: vec![None; len],
            direct: vec![None; len],
        }
    }

    pub fn copy_to_direct(&mut self) {
        self.direct = self.attackable.clone();
    }

    pub fn set_attackable(&mut self, coord: &Coordinate, distance: usize) {
        let pos = coord.to_1d(self.board_width);
        self.attackable[pos] = Some(distance);
    }

    pub fn set_direct(&mut self, coord: &Coordinate, distance: usize) {
        let pos = coord.to_1d(self.board_width);
        self.direct[pos] = Some(distance);
    }

    pub fn attackable_distance_mut(&mut self, coord: &Coordinate) -> Option<&mut Option<usize>> {
        let pos = coord.to_1d(self.board_width);
        self.attackable.get_mut(pos)
    }

    pub fn direct_distance_mut(&mut self, coord: &Coordinate) -> Option<&mut Option<usize>> {
        let pos = coord.to_1d(self.board_width);
        self.direct.get_mut(pos)
    }

    pub fn attackable_distance(&self, coord: &Coordinate) -> Option<usize> {
        let pos = coord.to_1d(self.board_width);
        self.attackable[pos]
    }

    pub fn direct_distance(&self, coord: &Coordinate) -> Option<usize> {
        let pos = coord.to_1d(self.board_width);
        self.direct[pos]
    }

    pub fn difference(&self, other: &BoardDistances) -> Self {
        assert_eq!(self.attackable.len(), other.attackable.len());

        let diff_attackable = self
            .attackable
            .iter()
            .zip(other.attackable.iter())
            .map(|dists| {
                let (Some(a), Some(b)) = dists else {
                    return None;
                };
                Some(a.abs_diff(*b))
            })
            .collect();

        let diff_direct = self
            .direct
            .iter()
            .zip(other.direct.iter())
            .map(|dists| {
                let (Some(a), Some(b)) = dists else {
                    return None;
                };
                Some(a.abs_diff(*b))
            })
            .collect();

        BoardDistances {
            board_width: self.board_width,
            attackable: diff_attackable,
            direct: diff_direct,
        }
    }

    pub fn iter_attackable(&self) -> impl Iterator<Item = (Coordinate, usize)> + '_ {
        self.attackable.iter().enumerate().flat_map(|(idx, dist)| {
            if let Some(dist) = dist {
                Some((Coordinate::from_1d(idx, self.board_width), *dist))
            } else {
                None
            }
        })
    }

    pub fn iter_direct(&self) -> impl Iterator<Item = (Coordinate, usize)> + '_ {
        self.direct.iter().enumerate().flat_map(|(idx, dist)| {
            if let Some(dist) = dist {
                Some((Coordinate::from_1d(idx, self.board_width), *dist))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{judge::Judge, rules::SwapPenalty};

    use super::*;

    pub fn short_dict() -> WordDict {
        Judge::new(vec![
            "BIG".into(),
            "FAT".into(),
            "JOLLY".into(),
            "AND".into(),
            "SILLY".into(),
            "FOLK".into(),
            "ARTS".into(),
        ])
        .builtin_dictionary
    }

    #[test]
    fn coord_flattening() {
        let coord = Coordinate { x: 4, y: 123 };
        let flat = coord.to_1d(51);
        assert_eq!(coord, Coordinate::from_1d(flat, 51));
    }

    fn default_swap_rules() -> SwapPenalty {
        SwapPenalty::Disallowed { allowed_swaps: 1 }
    }

    #[test]
    fn makes_default_boards() {
        assert_eq!(
            Board::new(3, 3).to_string(),
            "~~ ~~ |0 ~~ ~~\n\
             ~~ #0 __ #0 ~~\n\
             ~~ __ __ __ ~~\n\
             ~~ #1 __ #1 ~~\n\
             ~~ ~~ |1 ~~ ~~"
        );

        assert_eq!(
            Board::new(3, 2).to_string(),
            "~~ ~~ |0 ~~ ~~\n\
             ~~ #0 __ #0 ~~\n\
             ~~ #1 __ #1 ~~\n\
             ~~ ~~ |1 ~~ ~~"
        );

        // TODO: Balance uneven boards
        assert_eq!(
            Board::new(2, 2).to_string(),
            "~~ ~~ |0 ~~\n\
             ~~ #0 __ ~~\n\
             ~~ #1 __ ~~\n\
             ~~ ~~ |1 ~~"
        );

        assert_eq!(
            Board::new(5, 5).to_string(),
            "~~ ~~ ~~ |0 ~~ ~~ ~~\n\
             ~~ #0 #0 __ #0 #0 ~~\n\
             ~~ __ __ __ __ __ ~~\n\
             ~~ __ __ __ __ __ ~~\n\
             ~~ __ __ __ __ __ ~~\n\
             ~~ #1 #1 __ #1 #1 ~~\n\
             ~~ ~~ ~~ |1 ~~ ~~ ~~"
        );
    }

    #[test]
    fn trim_board() {
        let mut b = Board::from_string(
            "~~ ~~ |0 ~~ ~~\n\
             __ __ __ __ __\n\
             __ __ R0 __ __\n\
             __ W0 O0 R0 __\n\
             __ __ S0 __ __\n\
             __ __ __ __ __",
        );
        b.trim();
        assert_eq!(
            b.to_string(),
            "~~ ~~ |0 ~~ ~~\n\
             __ __ __ __ __\n\
             __ __ R0 __ __\n\
             __ W0 O0 R0 __\n\
             __ __ S0 __ __\n\
             __ __ __ __ __",
            "Don't trim docks or land"
        );

        let mut b = Board::from_string(
            "~~ ~~ ~~ ~~ ~~\n\
             ~~ __ R0 __ ~~\n\
             ~~ W0 O0 R0 ~~\n\
             ~~ __ S0 __ ~~\n\
             ~~ ~~ ~~ ~~ ~~",
        );
        b.trim();
        assert_eq!(
            b.to_string(),
            "~~ ~~ ~~ ~~ ~~\n\
             ~~ __ R0 __ ~~\n\
             ~~ W0 O0 R0 ~~\n\
             ~~ __ S0 __ ~~\n\
             ~~ ~~ ~~ ~~ ~~",
            "Leave an edge of water around the board"
        );

        let mut b = Board::from_string(
            "~~ ~~ ~~ ~~ ~~ ~~ ~~\n\
             ~~ ~~ ~~ |0 ~~ ~~ ~~\n\
             ~~ ~~ __ R0 __ ~~ ~~\n\
             ~~ ~~ W0 O0 R0 ~~ ~~\n\
             ~~ ~~ __ S0 __ |1 ~~\n\
             ~~ ~~ ~~ ~~ ~~ ~~ ~~\n\
             ~~ ~~ ~~ ~~ ~~ ~~ ~~",
        );
        b.trim();
        assert_eq!(
            b.to_string(),
            "~~ ~~ |0 ~~ ~~\n\
             ~~ __ R0 __ ~~\n\
             ~~ W0 O0 R0 ~~\n\
             ~~ __ S0 __ |1\n\
             ~~ ~~ ~~ ~~ ~~",
            "Trim excess water"
        );

        let mut b = Board::from_string(
            "__ __ __ ~~ __\n\
             __ __ R0 ~~ __\n\
             ~~ ~~ ~~ ~~ ~~\n\
             __ __ S0 ~~ __\n\
             ~~ ~~ ~~ ~~ ~~\n\
             ~~ ~~ ~~ ~~ ~~",
        );
        b.trim();
        assert_eq!(
            b.to_string(),
            "__ __ __ ~~ __\n\
             __ __ R0 ~~ __\n\
             ~~ ~~ ~~ ~~ ~~\n\
             __ __ S0 ~~ __\n\
             ~~ ~~ ~~ ~~ ~~",
            "Don't trim inner empty columns or rows"
        );

        let mut b = Board::from_string(
            "~~ ~~ ~~ ~~ ~~ ~~ ~~\n\
             |0 ~~ ~~ ~~ ~~ ~~ ~~\n\
             ~~ ~~ __ R0 __ ~~ ~~\n\
             ~~ ~~ W0 O0 R0 ~~ ~~\n\
             ~~ ~~ __ S0 __ |0 ~~\n\
             ~~ ~~ ~~ ~~ ~~ ~~ ~~\n\
             ~~ ~~ ~~ |1 ~~ ~~ ~~",
        );
        b.trim();
        assert_eq!(
            b.to_string(),
            "~~ ~~ ~~ ~~ ~~\n\
             ~~ __ R0 __ ~~\n\
             ~~ W0 O0 R0 ~~\n\
             ~~ __ S0 __ |0\n\
             ~~ ~~ ~~ ~~ ~~",
            "Do trim unconnected docks"
        );
    }

    #[test]
    fn width_height() {
        let b = Board::new(6, 1);
        assert_eq!(b.width(), 8);
        assert_eq!(b.height(), 3);
    }

    #[test]
    fn getset_errors_out_of_bounds() {
        let mut b = Board::from_string(
            "|0 __ __\n\
             __ ~~ __\n\
             __ __ __",
        );

        let position = Coordinate { x: 3, y: 1 };
        assert_eq!(
            b.get(position),
            Err(GamePlayError::OutSideBoardDimensions { position })
        );

        let position = Coordinate { x: 1, y: 3 };
        assert_eq!(
            b.set(position, 0, 'a', None),
            Err(GamePlayError::OutSideBoardDimensions { position })
        );
    }

    #[test]
    fn getset_errors_for_dead_squares() {
        let mut b = Board::from_string(
            "__ |0 __\n\
             __ ~~ __\n\
             __ |1 __",
        );

        let position = Coordinate { x: 1, y: 1 };
        assert_eq!(b.get(position), Ok(Square::Water));

        let position = Coordinate { x: 1, y: 1 };
        assert_eq!(
            b.set(position, 0, 'a', None),
            Err(GamePlayError::InvalidPosition { position })
        );
    }

    #[test]
    fn getset_handles_empty_squares() {
        let mut b = Board::from_string(
            "__ |0 __\n\
             __ |1 __",
        );

        assert_eq!(b.get(Coordinate { x: 0, y: 0 }), Ok(Square::Land));
        assert_eq!(b.get(Coordinate { x: 0, y: 1 }), Ok(Square::Land));
        assert_eq!(b.get(Coordinate { x: 2, y: 0 }), Ok(Square::Land));
        assert_eq!(b.get(Coordinate { x: 2, y: 1 }), Ok(Square::Land));

        assert_eq!(
            b.set(Coordinate { x: 0, y: 0 }, 0, 'a', Some(&short_dict())),
            Ok(BoardChangeDetail {
                square: Square::Occupied {
                    player: 0,
                    tile: 'a',
                    validity: SquareValidity::Invalid
                },
                coordinate: Coordinate { x: 0, y: 0 },
            })
        );
        assert_eq!(
            b.set(Coordinate { x: 0, y: 1 }, 0, 'a', Some(&short_dict())),
            Ok(BoardChangeDetail {
                square: Square::Occupied {
                    player: 0,
                    tile: 'a',
                    validity: SquareValidity::Invalid
                },
                coordinate: Coordinate { x: 0, y: 1 },
            })
        );
        assert_eq!(
            b.set(Coordinate { x: 2, y: 0 }, 0, 'a', Some(&short_dict())),
            Ok(BoardChangeDetail {
                square: Square::Occupied {
                    player: 0,
                    tile: 'a',
                    validity: SquareValidity::Invalid
                },
                coordinate: Coordinate { x: 2, y: 0 },
            })
        );
        assert_eq!(
            b.set(Coordinate { x: 2, y: 1 }, 0, 'a', Some(&short_dict())),
            Ok(BoardChangeDetail {
                square: Square::Occupied {
                    player: 0,
                    tile: 'a',
                    validity: SquareValidity::Invalid
                },
                coordinate: Coordinate { x: 2, y: 1 },
            })
        );
    }

    #[test]
    fn set_requires_valid_player() {
        let mut b = Board::from_string(
            "__ |0 __\n\
             __ |1 __",
        );

        assert_eq!(
            b.set(Coordinate { x: 0, y: 0 }, 0, 'a', Some(&short_dict())),
            Ok(BoardChangeDetail {
                square: Square::Occupied {
                    player: 0,
                    tile: 'a',
                    validity: SquareValidity::Invalid
                },
                coordinate: Coordinate { x: 0, y: 0 },
            })
        );
        assert_eq!(
            b.set(Coordinate { x: 0, y: 1 }, 1, 'a', Some(&short_dict())),
            Ok(BoardChangeDetail {
                square: Square::Occupied {
                    player: 1,
                    tile: 'a',
                    validity: SquareValidity::Invalid
                },
                coordinate: Coordinate { x: 0, y: 1 },
            })
        );
        assert_eq!(
            b.set(Coordinate { x: 2, y: 0 }, 2, 'a', None),
            Err(GamePlayError::NonExistentPlayer { index: 2 })
        );
        assert_eq!(
            b.set(Coordinate { x: 2, y: 0 }, 3, 'a', None),
            Err(GamePlayError::NonExistentPlayer { index: 3 })
        );
        assert_eq!(
            b.set(Coordinate { x: 2, y: 0 }, 100, 'a', None),
            Err(GamePlayError::NonExistentPlayer { index: 100 })
        );
    }

    #[test]
    fn set_changes_get() {
        let mut b = Board::new(3, 3); // Note, height is 3 from home rows
        assert_eq!(b.get(Coordinate { x: 2, y: 2 }), Ok(Square::Land));
        assert_eq!(
            b.set(Coordinate { x: 2, y: 2 }, 0, 'a', Some(&short_dict())),
            Ok(BoardChangeDetail {
                square: Square::Occupied {
                    player: 0,
                    tile: 'a',
                    validity: SquareValidity::Invalid
                },
                coordinate: Coordinate { x: 2, y: 2 },
            })
        );
        assert_eq!(
            b.get(Coordinate { x: 2, y: 2 }),
            Ok(Square::Occupied {
                player: 0,
                tile: 'a',
                validity: SquareValidity::Invalid
            })
        );
    }

    #[test]
    fn depth_first_search() {
        let mut b = Board::from_string(
            "~~ ~~ |0 ~~ ~~\n\
             ~~ __ __ __ ~~\n\
             ~~ __ __ __ ~~\n\
             ~~ __ __ __ ~~\n\
             ~~ ~~ |1 ~~ ~~",
        );

        // Create a connected tree
        let parts = [
            Coordinate { x: 2, y: 1 },
            Coordinate { x: 1, y: 1 },
            Coordinate { x: 1, y: 2 },
            Coordinate { x: 1, y: 3 },
        ];
        let parts_set = HashSet::from(parts);
        for part in parts {
            assert_eq!(
                b.set(part, 0, 'a', Some(&short_dict())),
                Ok(BoardChangeDetail {
                    square: Square::Occupied {
                        player: 0,
                        tile: 'a',
                        validity: SquareValidity::Invalid
                    },
                    coordinate: part,
                })
            );
        }

        // The tree should be returned no matter where in the tree we start DFS from
        for part in parts {
            assert!(b.depth_first_search(part).is_subset(&parts_set));
            assert!(b.depth_first_search(part).is_superset(&parts_set));
        }

        // Set a remaining unoccupied square on the board to be occupied by another player
        let other = Coordinate { x: 2, y: 2 };
        // When unoccupied it should give the empty set, when occupied, just itself
        assert!(b
            .depth_first_search(other)
            .iter()
            .collect::<Vec<_>>()
            .is_empty());
        assert_eq!(
            b.set(other, 1, 'a', Some(&short_dict())),
            Ok(BoardChangeDetail {
                square: Square::Occupied {
                    player: 1,
                    tile: 'a',
                    validity: SquareValidity::Invalid
                },
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
    fn simple_flood_fill_attacks() {
        let board = Board::from_string(
            r###"
            ~~ ~~ |0 ~~ ~~
            __ __ R0 __ __
            __ __ A0 __ X0
            __ __ __ __ __
            __ __ __ __ __
            __ __ __ __ __
            ~~ ~~ |1 ~~ ~~
            "###,
        );

        let dists = board.flood_fill_attacks(0);

        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 0, y: 1 }),
            Some(1)
        );
        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 3, y: 1 }),
            Some(0)
        );
        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 4, y: 5 }),
            Some(2)
        );
        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 3, y: 5 }),
            Some(3)
        );
        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 2, y: 5 }),
            Some(2)
        );
        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 0, y: 5 }),
            Some(4)
        );

        // Player 1's dock
        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 2, y: 6 }),
            Some(3)
        );
    }

    #[test]
    fn complex_flood_fill_attacks() {
        let board = Board::from_string(
            r###"
            ~~ ~~ |0 ~~ ~~ __ __ __ __ __
            __ __ R0 __ __ __ __ __ __ __
            __ __ A0 __ X0 __ __ Q1 __ __
            __ __ __ __ __ __ __ Q1 __ __
            __ __ __ __ __ __ __ Q1 __ __
            __ __ F1 __ __ __ __ Q1 __ __
            T1 __ A1 __ __ __ __ Q1 __ __
            A1 __ X1 __ G1 __ __ Q1 __ __
            ~~ ~~ |1 ~~ ~~ ~~ ~~ ~~ ~~ ~~
            "###,
        );

        let dists = board.flood_fill_attacks(0);

        // Probing the left, we can't get between T1 and A1
        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 1, y: 5 }),
            Some(3)
        );
        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 0, y: 5 }),
            Some(4)
        );
        assert_eq!(dists.direct_distance(&Coordinate { x: 0, y: 6 }), Some(5));
        assert_eq!(dists.attackable_distance(&Coordinate { x: 1, y: 6 }), None);

        // If we had a direct path there though, we can look up how far it is.
        assert_eq!(dists.direct_distance(&Coordinate { x: 1, y: 6 }), Some(4));

        // In the middle, the G1 blocks us from being adjacent to the A1
        assert_eq!(dists.attackable_distance(&Coordinate { x: 3, y: 6 }), None);

        // Far right, we have to go back and around the Q1 tower to reach the end
        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 9, y: 7 }),
            Some(13)
        );
        // Though if we could go straight there...
        assert_eq!(dists.direct_distance(&Coordinate { x: 9, y: 7 }), Some(9));
        // And to attack the bottom-most Q1, we would have to visit all the way from the right
        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 7, y: 7 }),
            Some(15)
        );
        // Though if we could go straight there...
        assert_eq!(dists.direct_distance(&Coordinate { x: 7, y: 7 }), Some(7));
        // The one above it we could attack from the left, though
        assert_eq!(
            dists.attackable_distance(&Coordinate { x: 7, y: 6 }),
            Some(6)
        );
        // Which is also the best we could do anyway
        assert_eq!(dists.direct_distance(&Coordinate { x: 7, y: 6 }), Some(6));
    }

    #[test]
    fn get_neighbours() {
        // (0,0) (1,0) (2,0) (3,0) (4,0)
        // (0,1) (1,1) (2,1) (3,1) (4,1)
        // (0,2) (1,2) (2,2) (3,2) (4,2)
        // (0,3) (1,3) (2,3) (3,3) (4,3)
        // (0,4) (1,4) (2,4) (3,4) (4,4)
        let b = Board::new(3, 3);

        assert_eq!(
            // TODO: should we allow you to find neighbours of an invalid square?
            b.neighbouring_squares(Coordinate { x: 0, y: 0 }),
            [
                (Coordinate { x: 1, y: 0 }, Square::Water),
                (Coordinate { x: 0, y: 1 }, Square::Water),
            ]
        );

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 1, y: 0 }),
            [
                (Coordinate { x: 2, y: 0 }, Square::Dock(0)),
                (
                    Coordinate { x: 1, y: 1 },
                    Square::Town {
                        player: 0,
                        defeated: false
                    }
                ),
                (Coordinate { x: 0, y: 0 }, Square::Water),
            ]
        );

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 2, y: 2 }),
            [
                (Coordinate { x: 2, y: 1 }, Square::Land),
                (Coordinate { x: 3, y: 2 }, Square::Land),
                (Coordinate { x: 2, y: 3 }, Square::Land),
                (Coordinate { x: 1, y: 2 }, Square::Land),
            ]
        );
    }

    #[test]
    fn swap() {
        let mut b = Board::from_string(
            "__ __ __ |0\n\
             __ __ __ __\n\
             __ __ __ |1",
        );
        let c0_1 = Coordinate { x: 0, y: 1 };
        let c1_1 = Coordinate { x: 1, y: 1 };
        let c2_1 = Coordinate { x: 2, y: 1 };
        assert_eq!(
            b.set(c0_1, 0, 'a', Some(&short_dict())),
            Ok(BoardChangeDetail {
                square: Square::Occupied {
                    player: 0,
                    tile: 'a',
                    validity: SquareValidity::Invalid
                },
                coordinate: c0_1,
            })
        );
        assert_eq!(
            b.set(c1_1, 0, 'b', Some(&short_dict())),
            Ok(BoardChangeDetail {
                square: Square::Occupied {
                    player: 0,
                    tile: 'b',
                    validity: SquareValidity::Invalid
                },
                coordinate: c1_1,
            })
        );
        assert_eq!(
            b.set(c2_1, 1, 'c', Some(&short_dict())),
            Ok(BoardChangeDetail {
                square: Square::Occupied {
                    player: 1,
                    tile: 'c',
                    validity: SquareValidity::Invalid
                },
                coordinate: c2_1,
            })
        );

        assert_eq!(
            b.get(c0_1),
            Ok(Square::Occupied {
                player: 0,
                tile: 'a',
                validity: SquareValidity::Invalid
            })
        );
        assert_eq!(
            b.get(c1_1),
            Ok(Square::Occupied {
                player: 0,
                tile: 'b',
                validity: SquareValidity::Invalid
            })
        );
        assert_eq!(
            b.swap(
                0,
                [c0_1, c1_1],
                &rules::Swapping::Contiguous(default_swap_rules()),
                Some(&short_dict())
            ),
            Ok(vec![
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied {
                            player: 0,
                            tile: 'b',
                            validity: SquareValidity::Invalid
                        },
                        coordinate: c0_1,
                    },
                    action: BoardChangeAction::Swapped
                }),
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied {
                            player: 0,
                            tile: 'a',
                            validity: SquareValidity::Invalid
                        },
                        coordinate: c1_1,
                    },
                    action: BoardChangeAction::Swapped
                })
            ])
        );
        assert_eq!(
            b.get(c0_1),
            Ok(Square::Occupied {
                player: 0,
                tile: 'b',
                validity: SquareValidity::Invalid
            })
        );
        assert_eq!(
            b.get(c1_1),
            Ok(Square::Occupied {
                player: 0,
                tile: 'a',
                validity: SquareValidity::Invalid
            })
        );
        assert_eq!(
            b.swap(
                0,
                [c0_1, c0_1],
                &rules::Swapping::Contiguous(default_swap_rules()),
                None
            ),
            Err(GamePlayError::SelfSwap)
        );
        assert_eq!(
            b.swap(
                0,
                [c0_1, c2_1],
                &rules::Swapping::Contiguous(default_swap_rules()),
                None
            ),
            Err(GamePlayError::UnownedSwap)
        );
        assert_eq!(
            b.swap(
                0,
                [c0_1, c2_1],
                &rules::Swapping::Universal(default_swap_rules()),
                None
            ),
            Err(GamePlayError::UnownedSwap)
        );
        assert_eq!(
            b.swap(
                1,
                [c0_1, c1_1],
                &rules::Swapping::Contiguous(default_swap_rules()),
                None
            ),
            Err(GamePlayError::UnownedSwap)
        );
    }

    #[test]
    fn disjoint_swapping() {
        let mut b = Board::from_string(
            "~~ ~~ |0 ~~ ~~\n\
             __ __ C0 __ __\n\
             __ __ R0 __ O0\n\
             __ __ __ __ __\n\
             __ __ S1 __ __\n\
             __ __ S1 __ __\n\
             ~~ ~~ |1 ~~ ~~",
        );

        let pos1 = Coordinate { x: 2, y: 2 };
        let pos2 = Coordinate { x: 4, y: 2 };

        assert_eq!(
            b.swap(0, [pos1, pos2], &rules::Swapping::None, None),
            Err(GamePlayError::NoSwapping)
        );

        assert_eq!(
            b.swap(
                0,
                [pos1, pos2],
                &rules::Swapping::Contiguous(default_swap_rules()),
                None
            ),
            Err(GamePlayError::DisjointSwap)
        );

        assert_eq!(
            b.swap(
                0,
                [pos1, pos2],
                &rules::Swapping::Universal(default_swap_rules()),
                Some(&short_dict())
            ),
            Ok(vec![
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied {
                            player: 0,
                            tile: 'O',
                            validity: SquareValidity::Invalid
                        },
                        coordinate: pos1,
                    },
                    action: BoardChangeAction::Swapped
                }),
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied {
                            player: 0,
                            tile: 'R',
                            validity: SquareValidity::Invalid
                        },
                        coordinate: pos2,
                    },
                    action: BoardChangeAction::Swapped
                })
            ])
        );
    }

    #[test]
    fn noop_swapping() {
        let mut b = Board::from_string(
            "~~ |0 ~~ ~~\n\
             __ A0 C0 __\n\
             __ A0 __ __\n\
             ~~ |1 ~~ ~~",
        );

        let a1 = Coordinate { x: 1, y: 1 };
        let a2 = Coordinate { x: 1, y: 2 };
        let c = Coordinate { x: 2, y: 1 };

        assert_eq!(
            b.swap(
                0,
                [a1, a2],
                &rules::Swapping::Contiguous(default_swap_rules()),
                None
            ),
            Err(GamePlayError::NoopSwap)
        );

        assert_eq!(
            b.swap(
                0,
                [a1, a1],
                &rules::Swapping::Contiguous(default_swap_rules()),
                None
            ),
            Err(GamePlayError::SelfSwap)
        );

        assert_eq!(
            b.swap(
                0,
                [a1, c],
                &rules::Swapping::Contiguous(default_swap_rules()),
                Some(&short_dict())
            ),
            Ok(vec![
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied {
                            player: 0,
                            tile: 'C',
                            validity: SquareValidity::Invalid
                        },
                        coordinate: a1,
                    },
                    action: BoardChangeAction::Swapped
                }),
                Change::Board(BoardChange {
                    detail: BoardChangeDetail {
                        square: Square::Occupied {
                            player: 0,
                            tile: 'A',
                            validity: SquareValidity::Invalid
                        },
                        coordinate: c,
                    },
                    action: BoardChangeAction::Swapped
                })
            ])
        );
    }

    #[test]
    fn get_words() {
        // Should return an empty list of words for all points on an empty board, and for positions off the board
        let empty: Vec<Vec<Coordinate>> = vec![];
        let b = Board::new(3, 3);
        for x in 0..12 {
            for y in 0..12 {
                let coord = Coordinate {
                    x: usize::wrapping_sub(x, 2),
                    y: usize::wrapping_sub(y, 2),
                };
                match b.get(coord) {
                    Ok(Square::Town { .. }) => {
                        assert_eq!(b.get_words(coord), vec![vec![coord]]);
                    }
                    _ => {
                        assert_eq!(b.get_words(coord), empty);
                    }
                }
            }
        }

        // Gets two words in the middle of a cross
        let b = Board::from_string(
            "__ __ C0 __ __\n\
             __ __ R0 __ __\n\
             S0 W0 O0 R0 D0\n\
             __ __ S0 __ __\n\
             __ __ S0 __ __",
        );
        let cross = ([4, 3, 2, 1, 0]).map(|y| Coordinate { x: 2, y }); // TODO: range
        let sword = ([4, 3, 2, 1, 0]).map(|x| Coordinate { x, y: 2 }); // TODO: range
        assert_eq!(b.get_words(Coordinate { x: 2, y: 2 }), vec![cross, sword]);

        let just_cross = ([0, 1, 3, 4]).map(|y| Coordinate { x: 2, y });
        for square in just_cross {
            assert_eq!(b.get_words(square), vec![cross]);
        }

        let just_sword = ([0, 1, 3, 4]).map(|x| Coordinate { x, y: 2 });
        for square in just_sword {
            assert_eq!(b.get_words(square), vec![sword]);
        }

        // Don't cross other players
        let b = Board::from_string(
            "__ __ C0 __ __\n\
             __ __ R0 __ __\n\
             __ __ O1 __ __\n\
             __ __ S0 __ __\n\
             __ __ S0 __ __",
        );
        assert_eq!(
            b.get(Coordinate { x: 2, y: 4 }),
            Ok(Square::Occupied {
                player: 0,
                tile: 'S',
                validity: SquareValidity::Unknown
            })
        );
        assert_eq!(
            b.get_words(Coordinate { x: 2, y: 4 }),
            vec![[Coordinate { x: 2, y: 4 }, Coordinate { x: 2, y: 3 }]]
        );
    }

    #[test]
    fn get_words_orientations() {
        let b = Board::from_string(
            "~~ ~~ ~~ |0 ~~ ~~ ~~\n\
             ~~ N0 U0 B0 #0 __ ~~\n\
             ~~ E0 __ __ __ G1 ~~\n\
             ~~ B0 __ __ __ A1 ~~\n\
             ~~ __ #1 Z1 E1 N1 ~~\n\
             ~~ ~~ ~~ |1 ~~ ~~ ~~",
        );

        {
            let mut words = b
                .word_strings(&b.get_words(Coordinate { x: 1, y: 1 }))
                .unwrap();
            words.sort();
            assert_eq!(words, vec!["BEN", "BUN"]);
        }
        {
            let mut words = b
                .word_strings(&b.get_words(Coordinate { x: 5, y: 4 }))
                .unwrap();
            words.sort();
            assert_eq!(words, vec!["GAN", "ZEN"]);
        }
    }

    #[test]
    fn apply_fog_of_war() {
        let board = Board::from_string(
            "~~ ~~ A0 ~~ ~~\n\
             A0 A0 A0 A0 A0\n\
             A0 __ __ A0 __\n\
             A0 __ __ __ __\n\
             A0 A0 __ B1 __\n\
             A0 __ B1 B1 __\n\
             ~~ ~~ B1 ~~ ~~",
        );

        let foggy = board.fog_of_war(1, &rules::Visibility::TileFog);
        assert_eq!(
            foggy.to_string(),
            "~~ ~~ __ ~~ ~~\n\
             A0 __ __ A0 __\n\
             A0 __ __ A0 __\n\
             A0 __ __ __ __\n\
             A0 A0 __ B1 __\n\
             A0 __ B1 B1 __\n\
             ~~ ~~ B1 ~~ ~~",
        );
    }

    #[test]
    fn apply_disjoint_fog_of_war() {
        let board = Board::from_string(
            "~~ ~~ A0 ~~ ~~\n\
             A0 A0 A0 __ A0\n\
             A0 __ __ A0 __\n\
             A0 __ __ __ __\n\
             __ B1 __ B1 __\n\
             __ B1 B1 B1 __\n\
             ~~ ~~ B1 ~~ ~~",
        );

        let foggy = board.fog_of_war(0, &rules::Visibility::TileFog);
        assert_eq!(
            foggy.to_string(),
            "~~ ~~ A0 ~~ ~~\n\
            A0 A0 A0 __ A0\n\
            A0 __ __ A0 __\n\
            A0 __ __ __ __\n\
            __ B1 __ B1 __\n\
            __ B1 __ B1 __\n\
            ~~ ~~ __ ~~ ~~",
        );
    }

    #[test]
    fn apply_land_fog_of_war() {
        let board = Board::from_string(
            "~~ ~~ A0 ~~ ~~ ~~ ~~ ~~ ~~ ~~\n\
             A0 A0 A0 __ A0 A0 __ __ __ __\n\
             A0 __ __ A0 __ A0 __ __ __ __\n\
             A0 __ __ __ __ __ __ __ __ __\n\
             __ B1 __ B1 __ __ __ __ __ __\n\
             __ B1 B1 B1 __ __ __ __ __ __\n\
             __ __ B1 __ __ __ __ __ __ __\n\
             __ __ B1 __ __ __ __ __ __ __\n\
             ~~ ~~ B1 ~~ ~~ ~~ ~~ ~~ ~~ ~~",
        );

        let mut foggy = board.fog_of_war(0, &rules::Visibility::LandFog);
        foggy.trim();
        assert_eq!(
            foggy.to_string(),
            "~~ ~~ A0 ~~ ~~ ~~ ~~ ░░ ░░\n\
             A0 A0 A0 __ A0 A0 __ __ ░░\n\
             A0 __ __ A0 __ A0 __ __ ░░\n\
             A0 __ __ __ __ __ __ ░░ ░░\n\
             __ B1 ░░ B1 ░░ __ ░░ ░░ ░░\n\
             __ B1 ░░ B1 ░░ ░░ ░░ ░░ ░░\n\
             ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░",
        );
    }

    #[test]
    fn remap_foggy_coordinates() {
        let board = Board::from_string(
            "__ __ __ __ __ __ __ __ __ __ __\n\
             __ __ __ __ __ __ ~~ __ __ __ __\n\
             __ __ __ __ ~~ ~~ ~~ ~~ ~~ ~~ ~~\n\
             __ __ __ __ ~~ ~~ A0 ~~ ~~ ~~ ~~\n\
             __ __ __ __ A0 ~~ A0 __ A0 A0 __\n\
             __ __ __ __ A0 __ __ A0 __ A0 __\n\
             __ __ __ __ A0 __ __ __ __ __ __\n\
             __ __ __ __ __ B1 __ B1 __ __ __\n\
             __ __ __ __ __ B1 B1 B1 __ __ __\n\
             ~~ __ __ __ __ __ B1 __ __ __ __\n\
             __ __ __ __ __ __ B1 __ __ __ __\n\
             __ __ __ __ ~~ ~~ B1 ~~ ~~ ~~ ~~",
        );
        {
            let mut foggy = board.fog_of_war(0, &rules::Visibility::LandFog);
            foggy.trim();
            assert_eq!(
                foggy.to_string(),
                "░░ ░░ ░░ ~~ ~~ ~~ ~~ ~~ ~~ ░░\n\
                 ░░ ░░ __ ~~ ~~ A0 ~~ ~~ ~~ ~~\n\
                 ░░ __ __ A0 ~~ A0 __ A0 A0 __\n\
                 ░░ __ __ A0 __ __ A0 __ A0 __\n\
                 ░░ __ __ A0 __ __ __ __ __ __\n\
                 ░░ ░░ __ __ B1 ░░ B1 ░░ __ ░░\n\
                 ░░ ░░ ░░ __ B1 ░░ B1 ░░ ░░ ░░\n\
                 ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░",
            );

            let source_coord = Coordinate { x: 4, y: 3 };
            let game_coord =
                board.map_player_coord_to_game(0, source_coord, &rules::Visibility::LandFog);
            assert_eq!(game_coord, Coordinate { x: 5, y: 5 });
            assert_eq!(
                board.map_game_coord_to_player(0, game_coord, &rules::Visibility::LandFog),
                Some(source_coord)
            );
        }
        {
            let mut foggy = board.fog_of_war(1, &rules::Visibility::LandFog);
            foggy.trim();
            assert_eq!(
                foggy.to_string(),
                "░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░ ░░\n\
                 ░░ ░░ A0 ░░ ░░ ░░ ░░ ░░ ░░\n\
                 ░░ ░░ A0 __ ░░ A0 ░░ ░░ ░░\n\
                 ░░ ░░ A0 __ __ __ __ ░░ ░░\n\
                 ░░ __ __ B1 __ B1 __ __ ░░\n\
                 ░░ __ __ B1 B1 B1 __ __ ░░\n\
                 ░░ ░░ __ __ B1 __ __ ░░ ░░\n\
                 ░░ ░░ __ __ B1 __ __ ░░ ░░\n\
                 ░░ ░░ ~~ ~~ B1 ~~ ~~ ░░ ░░",
            );

            let source_coord = Coordinate { x: 6, y: 4 };
            let game_coord =
                board.map_player_coord_to_game(1, source_coord, &rules::Visibility::LandFog);
            assert_eq!(game_coord, Coordinate { x: 8, y: 7 });
            assert_eq!(
                board.map_game_coord_to_player(1, game_coord, &rules::Visibility::LandFog),
                Some(source_coord)
            );
        }
    }
}
