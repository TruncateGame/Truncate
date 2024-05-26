use std::{
    collections::{BinaryHeap, HashSet, VecDeque},
    ops::{Add, Div, Mul},
};

use noise::{NoiseFn, Simplex};
use oorandom::Rand32;
use serde::{Deserialize, Serialize};

use crate::{
    board::{Board, BoardDistances, Coordinate, Square, SquareValidity},
    game::Game,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BoardType {
    Island,
    Continental,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoardElements {
    pub docks: bool,
    pub towns: bool,
    pub obelisk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardParams {
    pub land_dimensions: [usize; 2],
    pub dispersion: [f64; 2],
    pub island_influence: f64,
    pub maximum_town_density: f64,
    pub maximum_town_distance: f64,
    pub minimum_choke: usize,
    pub board_type: BoardType,
    pub ideal_dock_radius: f64,
    pub ideal_dock_separation: f64,
    pub elements: BoardElements,
}

// Do not modify any numbered generations.
// Add a new generation number with new parameters.
// Updating an existing generation will break puzzle URLs.
const BOARD_GENERATIONS: [BoardParams; 2] = [
    BoardParams {
        land_dimensions: [10, 10],
        dispersion: [5.0, 5.0],
        maximum_town_density: 0.2,
        maximum_town_distance: 0.15,
        island_influence: 0.0,
        minimum_choke: 3,
        board_type: BoardType::Island,
        ideal_dock_radius: 1.0,
        ideal_dock_separation: 0.7,
        elements: BoardElements {
            docks: true,
            towns: true,
            obelisk: false,
        },
    },
    BoardParams {
        land_dimensions: [9, 10],
        dispersion: [5.0, 5.0],
        maximum_town_density: 0.2,
        maximum_town_distance: 0.15,
        island_influence: 0.0,
        minimum_choke: 3,
        board_type: BoardType::Island,
        ideal_dock_radius: 1.0,
        ideal_dock_separation: 0.7,
        elements: BoardElements {
            docks: true,
            towns: true,
            obelisk: false,
        },
    },
];

impl BoardParams {
    pub fn generation(gen: u32) -> Self {
        BOARD_GENERATIONS
            .get(gen as usize)
            .expect("Board generation should exist")
            .clone()
    }

    pub fn latest() -> (u32, Self) {
        assert!(!BOARD_GENERATIONS.is_empty());
        let generation = (BOARD_GENERATIONS.len() - 1) as u32;
        (generation, BoardParams::generation(generation as u32))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PreviousBoardResize {
    Enlarged,
    Shrunk,
}

#[derive(Debug, Clone)]
pub struct BoardSeed {
    pub generation: u32,
    pub seed: u32,
    pub day: Option<u32>,
    pub params: BoardParams,
    pub current_iteration: usize,
    pub width_resize_state: Option<PreviousBoardResize>,
    pub height_resize_state: Option<PreviousBoardResize>,
    pub water_level: f64,
    pub max_attempts: usize,
}

impl BoardSeed {
    pub fn new(seed: u32) -> Self {
        let (generation, params) = BoardParams::latest();
        Self {
            generation,
            seed,
            day: None,
            params,
            current_iteration: 0,
            width_resize_state: None,
            height_resize_state: None,
            water_level: 0.5,
            max_attempts: 10000, // Default to trying for a very long time (try not to panic for a user)
        }
    }

    pub fn new_with_generation(generation: u32, seed: u32) -> Self {
        let params = BoardParams::generation(generation);
        Self {
            generation,
            seed,
            day: None,
            params,
            current_iteration: 0,
            width_resize_state: None,
            height_resize_state: None,
            water_level: 0.5,
            max_attempts: 10000, // Default to trying for a very long time (try not to panic for a user)
        }
    }

    pub fn day(mut self, day: u32) -> Self {
        self.day = Some(day);
        self
    }

    pub fn seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    fn internal_reroll(&mut self) {
        let mut rng = Rand32::new(self.seed as u64);
        let r = rng.rand_u32();
        self.seed = r;
        self.current_iteration += 1;
    }

    pub fn external_reroll(&mut self) {
        let mut rng = Rand32::new(self.seed as u64);
        // If externally rerolling, advance this RNG state and pick a later number.
        // otherwise, the external reroll might do nothing if the previous seed
        // was internally rerolled.
        // e.g. seed_1 generated a board which failed generation checks,
        // so seed_1 internally rerolls and succeeds on attempt #2.
        // This state fails gameplay checks, so we need to externally reroll.
        // If this used the same reroll function on seed_1, we would end up with an identical board.
        _ = rng.rand_u32();
        let r = rng.rand_u32();
        self.seed = r;
    }
}

#[derive(Debug)]
pub struct BoardGenerationResult {
    pub board: Board,
    pub iterations: usize,
}

pub fn generate_board(
    mut board_seed: BoardSeed,
) -> Result<BoardGenerationResult, BoardGenerationResult> {
    let BoardSeed {
        generation: _,
        seed,
        day: _,
        current_iteration,
        width_resize_state,
        height_resize_state: _,
        water_level,
        max_attempts,
        params:
            BoardParams {
                land_dimensions: ideal_land_dimensions,
                dispersion,
                maximum_town_density,
                maximum_town_distance,
                island_influence: jitter,
                minimum_choke,
                board_type,
                ideal_dock_radius,
                ideal_dock_separation,
                elements,
            },
    } = board_seed;

    let retry_with = |mut board_seed: BoardSeed, failed_board: Board| {
        board_seed.internal_reroll();
        if current_iteration > max_attempts {
            return Err(BoardGenerationResult {
                board: failed_board,
                iterations: max_attempts,
            });
        } else {
            return generate_board(board_seed);
        }
    };

    let simplex = Simplex::new(seed);

    let mut board = Board::new(3, 3);
    let canvas_multiplier = match board_type {
        BoardType::Island => 2.0,
        BoardType::Continental => 1.0,
    };
    let canvas_size = [
        (ideal_land_dimensions[0] as f64 * canvas_multiplier) as usize,
        (ideal_land_dimensions[1] as f64 * canvas_multiplier) as usize,
    ];
    // Expand the canvas when creating board squares to avoid setting anything in the outermost ring
    board.squares = vec![vec![crate::board::Square::Water; canvas_size[0] + 2]; canvas_size[1] + 2];

    for i in 1..=canvas_size[0] {
        for j in 1..=canvas_size[1] {
            let ni = i as f64 / (canvas_size[0] + 1) as f64; // normalized coordinates
            let nj = j as f64 / (canvas_size[1] + 1) as f64;
            let x = ni - 0.5; // centering the coordinates
            let y = nj - 0.5;

            let distance_to_center = (x * x + y * y).sqrt();

            // Get Simplex noise value
            let noise_value =
                (simplex.get([ni * dispersion[0], nj * dispersion[1], 0.0]) + 1.0) / 2.0;

            // Combine noise and gradient
            let value = noise_value - (distance_to_center * jitter);

            if value > water_level {
                board
                    .set_square(Coordinate { x: i, y: j }, crate::board::Square::Land)
                    .expect("Board position should be settable");
            }
        }
    }

    if board.trim_nubs().is_err() {
        return retry_with(board_seed, board);
    }

    // Remove extraneous water
    board.trim();

    let mut width_diff = board.width() as isize - (ideal_land_dimensions[0] + 2) as isize;

    // Raise or lower water slightly to try and hit the target island width
    if width_diff.is_negative() {
        // Avoid oscillating around the target — once we have shrunk we won't enlarge via water again
        if width_resize_state != Some(PreviousBoardResize::Shrunk) {
            board_seed.water_level -= 0.01;
            board_seed.width_resize_state = Some(PreviousBoardResize::Enlarged);
            return generate_board(board_seed);
        } else {
            let mut rng = Rand32::new(seed as u64);
            while width_diff < 0 {
                // Pick a random column to duplicate
                let col = rng.rand_range(1..(board.squares[0].len() as u32 - 1)) as usize;

                for row in board.squares.iter_mut() {
                    row.insert(col, row[col]);
                }

                width_diff += 1;
            }
        }
    } else if width_diff.is_positive() {
        board_seed.water_level += 0.005;
        board_seed.width_resize_state = Some(PreviousBoardResize::Shrunk);
        return generate_board(board_seed);
    }

    let mut height_diff = board.height() as isize - (ideal_land_dimensions[1] + 2) as isize;

    if height_diff != 0 {
        let mut rng = Rand32::new(seed as u64);
        while height_diff != 0 && board.squares.len() > 2 {
            // Pick a random row to duplicate or delete
            let row = rng.rand_range(1..(board.squares.len() as u32 - 2)) as usize;

            if height_diff.is_negative() {
                board.squares.insert(row, board.squares[row].clone());
                height_diff += 1;
            } else {
                let removed = board.squares.remove(row);
                // Overlay this row on a neighbor to avoid cutting the island
                for (i, sq) in removed.into_iter().enumerate() {
                    if sq == Square::Land {
                        board.squares[row][i] = Square::Land;
                    }
                }
                height_diff -= 1;
            }
        }
    }

    if elements.docks {
        match board_type {
            BoardType::Island => {
                if board.drop_island_docks(seed).is_err() {
                    return retry_with(board_seed, board);
                }
            }
            BoardType::Continental => {
                if board
                    .drop_continental_docks(seed, ideal_dock_radius, ideal_dock_separation)
                    .is_err()
                {
                    return retry_with(board_seed, board);
                }
            }
        }
    }

    if board.expand_choke_points(minimum_choke, false).is_err() {
        return retry_with(board_seed, board);
    }

    // Recalculate the shortest path, as expanding the choke points
    // may have created new paths altogether
    let Some(shortest_attack_path) = board.shortest_path_between(&board.docks[0], &board.docks[1])
    else {
        return retry_with(board_seed, board);
    };

    if elements.towns {
        if board
            .generate_towns(
                seed,
                &shortest_attack_path,
                maximum_town_density,
                maximum_town_distance,
            )
            .is_err()
        {
            return retry_with(board_seed, board);
        };
    }

    if elements.obelisk {
        if board.generate_obelisk(&shortest_attack_path).is_err() {
            return retry_with(board_seed, board);
        }
    }

    Ok(BoardGenerationResult {
        board,
        iterations: current_iteration,
    })
}

trait BoardGenerator {
    fn trim_nubs(&mut self) -> Result<(), ()>;

    fn expand_choke_points(&mut self, minimum_choke: usize, debug: bool) -> Result<(), ()>;

    fn drop_island_docks(&mut self, seed: u32) -> Result<(), ()>;

    fn drop_continental_docks(
        &mut self,
        seed: u32,
        ideal_dock_radius: f64,
        ideal_dock_separation: f64,
    ) -> Result<(), ()>;

    fn generate_towns(
        &mut self,
        seed: u32,
        main_road: &Vec<Coordinate>,
        maximum_town_density: f64,
        maximum_town_distance: f64,
    ) -> Result<(), ()>;

    fn generate_obelisk(&mut self, main_road: &Vec<Coordinate>) -> Result<(), ()>;
}

impl BoardGenerator for Board {
    fn trim_nubs(&mut self) -> Result<(), ()> {
        let sqs = || {
            self.squares.iter().enumerate().flat_map(|(y, row)| {
                row.iter().enumerate().flat_map(move |(x, sq)| {
                    if matches!(sq, Square::Land) {
                        Some(Coordinate { x, y })
                    } else {
                        None
                    }
                })
            })
        };

        let mut visited: HashSet<Coordinate> = HashSet::new();
        let mut searches: Vec<HashSet<Coordinate>> = vec![];

        while let Some(coord) = sqs().find(|coord| !visited.contains(&coord)) {
            let mut this_search: HashSet<Coordinate> = HashSet::new();
            visited.insert(coord);
            this_search.insert(coord);
            let mut pts = VecDeque::from(vec![coord]);

            while !pts.is_empty() {
                let pt = pts.pop_front().unwrap();
                for neighbor in pt
                    .neighbors_4()
                    .iter()
                    .filter(|coord| !visited.contains(&coord))
                    .collect::<Vec<_>>()
                {
                    match self.get(*neighbor) {
                        Ok(Square::Land) => {
                            pts.push_back(*neighbor);
                            visited.insert(*neighbor);
                            this_search.insert(*neighbor);
                        }
                        _ => {}
                    }
                }
            }

            searches.push(this_search);
        }

        let Some(largest) = searches.iter().max_by_key(|s| s.len()) else {
            return Err(());
        };

        for i in 0..self.width() {
            for j in 0..self.height() {
                let coord = Coordinate { x: i, y: j };

                if !largest.contains(&coord) {
                    self.set_square(coord, Square::Water)
                        .expect("Board position should be settable");
                }
            }
        }

        Ok(())
    }

    fn expand_choke_points(&mut self, minimum_choke: usize, debug: bool) -> Result<(), ()> {
        let Some(shortest_attack_path) = self.shortest_path_between(&self.docks[0], &self.docks[1])
        else {
            return Err(());
        };

        let measurements = shortest_attack_path
            .iter()
            .enumerate()
            .filter_map(|(i, pt)| {
                // Avoid processing the tiles closest to each players dock
                if i < minimum_choke || i >= shortest_attack_path.len() - minimum_choke {
                    return None;
                }

                let choke_distance = pt
                    .neighbors_8()
                    .iter()
                    .map(|n| self.distance_to_closest_obstruction(&n, &shortest_attack_path))
                    .max()
                    .unwrap();

                Some((*pt, choke_distance))
            })
            .collect::<Vec<_>>();

        for (pt, choke_distance) in &measurements {
            let buffer = minimum_choke / 2 + 1; // How far our points must be from the edges of the world

            let buffered_pt = Coordinate {
                x: if pt.x.checked_sub(buffer).is_none() {
                    buffer
                } else if (pt.x + buffer) >= self.width() {
                    self.width() - buffer - 1
                } else {
                    pt.x
                },
                y: if pt.y.checked_sub(buffer).is_none() {
                    buffer
                } else if (pt.y + buffer) >= self.height() {
                    self.height() - buffer - 1
                } else {
                    pt.y
                },
            };

            let mid = (minimum_choke / 2) as isize;
            if *choke_distance < minimum_choke {
                for x in (-mid)..(minimum_choke as isize - mid) {
                    for y in (-mid)..(minimum_choke as isize - mid) {
                        let c = Coordinate {
                            x: buffered_pt.x.saturating_add_signed(x),
                            y: buffered_pt.y.saturating_add_signed(y),
                        };

                        if c.x == 0
                            || c.y == 0
                            || c.x >= self.width() - 1
                            || c.y >= self.height() - 1
                        {
                            // Don't touch the outer rim of the board.
                            continue;
                        }

                        match self.get(c) {
                            Ok(Square::Land | Square::Dock(_)) => {}
                            Err(_) => {}
                            Ok(_) => {
                                if debug {
                                    _ = self.set_square(
                                        c,
                                        Square::Town {
                                            player: 0,
                                            defeated: false,
                                        },
                                    );
                                } else {
                                    _ = self.set_square(c, Square::Land);
                                }
                            }
                        }
                    }
                }
            }
        }

        if debug {
            for (pt, dist) in measurements {
                let tile = if dist < 10 {
                    dist.to_string().chars().next().unwrap()
                } else {
                    '+'
                };
                _ = self.set_square(
                    pt,
                    Square::Occupied {
                        player: 0,
                        tile,
                        validity: SquareValidity::Unknown,
                    },
                );
            }
        }

        Ok(())
    }

    fn drop_island_docks(&mut self, seed: u32) -> Result<(), ()> {
        let mut rng = Rand32::new(seed as u64);
        let mut visited: HashSet<Coordinate> = HashSet::from([Coordinate { x: 0, y: 0 }]);
        let mut coastal_water: HashSet<Coordinate> = HashSet::new();

        // Islands require docks on the coasts, so we DFS from the edges.
        let mut pts = VecDeque::from(vec![Coordinate { x: 0, y: 0 }]);
        while !pts.is_empty() {
            let pt = pts.pop_front().unwrap();
            for neighbor in pt
                .neighbors_4()
                .iter()
                .filter(|coord| !visited.contains(&coord))
                .collect::<Vec<_>>()
            {
                match self.get(*neighbor) {
                    Ok(Square::Water) => {
                        pts.push_back(*neighbor);
                        visited.insert(*neighbor);
                    }
                    Ok(Square::Land) => {
                        visited.insert(*neighbor);
                        coastal_water.insert(pt);
                    }
                    _ => {}
                }
            }
        }

        assert_eq!(coastal_water.is_empty(), false);

        let mut center_point = Coordinate {
            x: self.width() / 2,
            y: self.height() / 2,
        };
        while self.get(center_point) != Ok(Square::Land) {
            if self.get(center_point).is_err() {
                return Err(());
            }
            let neighbors = center_point.neighbors_8();
            center_point = neighbors
                .iter()
                .find(|p| self.get(**p) == Ok(Square::Land))
                .unwrap_or_else(|| &neighbors[0])
                .clone();
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        struct DistanceToCoord {
            coord: Coordinate,
            distance: usize,
        }

        impl Ord for DistanceToCoord {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                match self.distance.cmp(&other.distance) {
                    std::cmp::Ordering::Equal => self.coord.to_1d(100).cmp(&other.coord.to_1d(100)),
                    o => o,
                }
            }
        }
        impl PartialOrd for DistanceToCoord {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        let distances = self.flood_fill(&center_point);
        let mut distances = distances
            .direct
            .iter()
            .enumerate()
            .filter_map(|(idx, d)| {
                d.map(|d| DistanceToCoord {
                    coord: Coordinate::from_1d(idx, self.width()),
                    distance: d,
                })
            })
            .filter(|DistanceToCoord { coord, .. }| coastal_water.contains(coord))
            .collect::<BinaryHeap<_>>();

        // How far away from the extremeties are we allowed to land?
        let distance_pool = self.width().max(self.height()).div(2) as u32;

        let mut pt = distances.pop();
        for _ in 0..rng.rand_range(0..distance_pool) {
            pt = distances.pop().or(pt);
        }
        let Some(DistanceToCoord {
            coord: dock_zero, ..
        }) = pt
        else {
            return Err(());
        };

        self.set_square(dock_zero, Square::Dock(0))
            .expect("Board position should be settable");

        let mut antipodes: BinaryHeap<_> = coastal_water
            .iter()
            .map(|l| DistanceToCoord {
                coord: l.clone(),
                distance: l.distance_to(&dock_zero),
            })
            .collect();

        let mut pt = antipodes.pop();
        for _ in 0..rng.rand_range(0..distance_pool) {
            pt = antipodes.pop().or(pt);
        }
        let Some(DistanceToCoord {
            coord: dock_one, ..
        }) = pt
        else {
            return Err(());
        };

        self.set_square(dock_one, Square::Dock(1))
            .expect("Board position should be settable");

        self.cache_special_squares();

        Ok(())
    }

    fn drop_continental_docks(
        &mut self,
        seed: u32,
        ideal_dock_radius: f64,
        ideal_dock_separation: f64,
    ) -> Result<(), ()> {
        let mut rng = Rand32::new(seed as u64);

        let mut center_point = Coordinate {
            x: self.width() / 2,
            y: self.height() / 2,
        };
        while self.get(center_point) != Ok(Square::Land) {
            if self.get(center_point).is_err() {
                return Err(());
            }
            let neighbors = center_point.neighbors_8();
            center_point = neighbors
                .iter()
                .find(|p| self.get(**p) == Ok(Square::Land))
                .unwrap_or_else(|| &neighbors[0])
                .clone();
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        struct DistanceToCoord {
            coord: Coordinate,
            distance: usize,
        }

        impl Ord for DistanceToCoord {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                match self.distance.cmp(&other.distance) {
                    std::cmp::Ordering::Equal => self.coord.to_1d(100).cmp(&other.coord.to_1d(100)),
                    o => o,
                }
            }
        }
        impl PartialOrd for DistanceToCoord {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        let distances = self.flood_fill(&center_point);
        let mut candidates = distances
            .direct
            .iter()
            .enumerate()
            .filter_map(|(idx, d)| {
                d.map(|d| DistanceToCoord {
                    coord: Coordinate::from_1d(idx, self.width()),
                    distance: d,
                })
            })
            .filter_map(|DistanceToCoord { coord, distance }| {
                if matches!(self.get(coord), Ok(Square::Land)) {
                    coord
                        .neighbors_4()
                        .iter()
                        .find(|p| matches!(self.get(**p), Ok(Square::Water)))
                        .cloned()
                        .map(|c| (distance, c))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if candidates.is_empty() {
            return Err(());
        }

        let radius_target = (self.width().max(self.height()) as f64)
            .div(2.0)
            .mul(ideal_dock_radius)
            .round() as usize;

        let zero_candidates = candidates
            .iter()
            .filter(|(d, _)| d.abs_diff(radius_target) < 5)
            .cloned()
            .collect::<Vec<_>>();
        let (_, dock_zero) = if zero_candidates.is_empty() {
            *candidates.get(0).unwrap()
        } else {
            *zero_candidates
                .get(rng.rand_range(0..zero_candidates.len() as u32) as usize)
                .unwrap()
        };

        candidates.iter_mut().for_each(|(dist, coord)| {
            *dist = coord.distance_to(&dock_zero);
        });

        let separation_target = (self.width().max(self.height()) as f64)
            .div(2.0)
            .mul(ideal_dock_separation)
            .round() as usize;

        let one_candidates = candidates
            .iter()
            .filter(|(d, _)| d.abs_diff(separation_target) < 5)
            .cloned()
            .collect::<Vec<_>>();
        let (_, dock_one) = if one_candidates.is_empty() {
            *candidates.get(0).unwrap()
        } else {
            *one_candidates
                .get(rng.rand_range(0..one_candidates.len() as u32) as usize)
                .unwrap()
        };

        self.set_square(dock_zero, Square::Dock(0))
            .expect("Board position should be settable");

        self.set_square(dock_one, Square::Dock(1))
            .expect("Board position should be settable");

        self.cache_special_squares();

        Ok(())
    }

    fn generate_towns(
        &mut self,
        seed: u32,
        main_road: &Vec<Coordinate>,
        maximum_town_density: f64,
        maximum_town_distance: f64,
    ) -> Result<(), ()> {
        let docks = &self.docks;
        let Some(Ok(Square::Dock(player_zero))) = docks.get(0).map(|d| self.get(*d)) else {
            return Err(());
        };
        let Some(Ok(Square::Dock(player_one))) = docks.get(1).map(|d| self.get(*d)) else {
            return Err(());
        };

        let mut town_seed = Rand32::new(seed as u64);

        let town_distance = ((main_road.len() as f64) * maximum_town_distance) as usize;

        let player_zero_dists = self.flood_fill(&docks[0]);
        let player_one_dists = self.flood_fill(&docks[1]);

        let mut candidates = |dists: BoardDistances| {
            let mut candies: Vec<_> = dists
                .iter_direct()
                .filter_map(|(coord, distance)| {
                    let is_land = matches!(self.get(coord), Ok(Square::Land));
                    let is_near_dock = distance <= town_distance;
                    let is_on_critical_path = main_road.contains(&coord);

                    if is_land && is_near_dock && !is_on_critical_path {
                        Some(coord)
                    } else {
                        None
                    }
                })
                .collect();
            candies.sort_by_cached_key(|_| town_seed.rand_u32());
            candies
        };

        let player_zero_candidates = candidates(player_zero_dists);
        let player_one_candidates = candidates(player_one_dists);

        let mut town_pairs = player_zero_candidates
            .into_iter()
            .zip(player_one_candidates.into_iter());
        if town_pairs.len() == 0 {
            return Err(());
        }
        let maximum_town_goal = ((town_pairs.len() as f64 * maximum_town_density) as u32).max(1);
        let town_goal = town_seed.rand_range(0..maximum_town_goal) + 1;

        for _ in 0..town_goal {
            let Some((town_zero, town_one)) = town_pairs.next() else {
                break;
            };

            _ = self.set_square(
                town_zero,
                Square::Town {
                    player: player_zero,
                    defeated: false,
                },
            );
            _ = self.set_square(
                town_one,
                Square::Town {
                    player: player_one,
                    defeated: false,
                },
            );
        }

        self.cache_special_squares();
        let check_player_has_town = |p: usize| {
            self.towns.iter().any(
                |coord| matches!(self.get(*coord), Ok(Square::Town { player, .. }) if player == p),
            )
        };

        if check_player_has_town(0) && check_player_has_town(1) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn generate_obelisk(&mut self, main_road: &Vec<Coordinate>) -> Result<(), ()> {
        _ = self.set_square(main_road[main_road.len() / 2], Square::Obelisk);

        Ok(())
    }
}

pub fn get_game_verification(game: &Game) -> String {
    let mut digest = chksum_hash_sha2::sha2_256::default();

    digest.update(game.board.to_string());
    for player in &game.players {
        digest.update(player.hand.0.iter().collect::<String>());
    }

    digest.digest().to_hex_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reroll_test() {
        let mut seed = BoardSeed::new(12345);
        let bare_seed_1 = seed.seed;
        let board_one = generate_board(seed.clone())
            .expect("Board can be resolved")
            .board;
        seed.external_reroll();
        let bare_seed_2 = seed.seed;
        let board_two = generate_board(seed).expect("Board can be resolved").board;

        insta::assert_snapshot!(format!(
            "Board 1 from {bare_seed_1}:\n{board_one}\n\nrerolled to {bare_seed_2}:\n{board_two}"
        ));
    }
}
