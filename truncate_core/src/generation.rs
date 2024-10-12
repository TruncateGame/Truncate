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
pub enum ArtifactType {
    IslandV1,
    Coastal,
    Continental,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Symmetry {
    SmoothTwoFoldRotational,
    TwoFoldRotational,
    Asymmetric,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoardElements {
    pub artifacts: bool,
    pub towns: bool,
    pub obelisk: bool,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct BoardNoiseParams {
    pub dispersion: [f64; 2],
    pub island_influence: f64,
    pub symmetric: Symmetry,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct WaterLayer {
    pub params: BoardNoiseParams,
    pub density: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardParams {
    pub land_layer: BoardNoiseParams,
    pub water_layer: Option<WaterLayer>,
    pub land_dimensions: [usize; 2],
    pub canvas_dimensions: [usize; 2],
    pub maximum_town_density: f64,
    pub maximum_town_distance: f64,
    pub minimum_choke: usize,
    pub artifact_type: ArtifactType,
    pub ideal_artifact_extremity: f64,
    pub elements: BoardElements,
}

// Do not modify any numbered generations.
// Add a new generation number with new parameters.
// Updating an existing generation will break puzzle URLs.
const BOARD_GENERATIONS: [BoardParams; 2] = [
    BoardParams {
        land_layer: BoardNoiseParams {
            dispersion: [5.0, 5.0],
            island_influence: 0.0,
            symmetric: Symmetry::Asymmetric,
        },
        water_layer: None,
        land_dimensions: [10, 10],
        canvas_dimensions: [20, 20],
        maximum_town_density: 0.2,
        maximum_town_distance: 0.15,
        minimum_choke: 3,
        artifact_type: ArtifactType::IslandV1,
        ideal_artifact_extremity: 1.0,
        elements: BoardElements {
            artifacts: true,
            towns: true,
            obelisk: false,
        },
    },
    BoardParams {
        land_layer: BoardNoiseParams {
            dispersion: [5.0, 5.0],
            island_influence: 0.0,
            symmetric: Symmetry::Asymmetric,
        },
        water_layer: None,
        land_dimensions: [9, 10],
        canvas_dimensions: [18, 20],
        maximum_town_density: 0.2,
        maximum_town_distance: 0.15,
        minimum_choke: 3,
        artifact_type: ArtifactType::IslandV1,
        ideal_artifact_extremity: 1.0,
        elements: BoardElements {
            artifacts: true,
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
                land_layer,
                water_layer,
                land_dimensions: ideal_land_dimensions,
                canvas_dimensions,
                maximum_town_density,
                maximum_town_distance,
                minimum_choke,
                artifact_type,
                ideal_artifact_extremity,
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
    // Expand the canvas when creating board squares to avoid setting anything in the outermost ring
    board.squares = vec![
        vec![crate::board::Square::water(); canvas_dimensions[0] + 2];
        canvas_dimensions[1] + 2
    ];

    for i in 1..=canvas_dimensions[0] {
        for j in 1..=canvas_dimensions[1] {
            let ni = i as f64 / (canvas_dimensions[0] + 1) as f64; // normalized coordinates
            let nj = j as f64 / (canvas_dimensions[1] + 1) as f64;
            let x = ni - 0.5; // centering the coordinates
            let y = nj - 0.5;

            let distance_to_center = (x * x + y * y).sqrt();

            // Get Simplex noise value
            let noise_value = (simplex.get([
                ni * land_layer.dispersion[0],
                nj * land_layer.dispersion[1],
                0.0,
            ]) + 1.0)
                / 2.0;

            // Combine noise and gradient
            let value = noise_value - (distance_to_center * land_layer.island_influence);

            if value > water_level {
                board
                    .set_square(Coordinate { x: i, y: j }, crate::board::Square::land())
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
                    if matches!(sq, Square::Land { .. }) {
                        board.squares[row][i] = Square::land();
                    }
                }
                height_diff -= 1;
            }
        }
    }

    match land_layer.symmetric {
        Symmetry::Asymmetric => { /* nothing to do */ }
        Symmetry::TwoFoldRotational | Symmetry::SmoothTwoFoldRotational => {
            let input = board.squares.clone();
            let board_width = board.width();
            let board_height = board.height();

            match land_layer.symmetric {
                Symmetry::SmoothTwoFoldRotational => {
                    for (row_num, row) in input.into_iter().enumerate() {
                        for (col_num, square) in row.into_iter().enumerate().skip(row_num) {
                            let coord = Coordinate::new(col_num, row_num);
                            let recip = board.reciprocal_coordinate(coord);
                            let recip_square =
                                board.get(recip).expect("symmetric point should exist");

                            match (square, recip_square) {
                                (Square::Water { .. }, Square::Land { .. })
                                | (Square::Land { .. }, Square::Water { .. }) => {
                                    board.squares[coord.y][coord.x] = Square::land();
                                    board.squares[recip.y][recip.x] = Square::land();
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Symmetry::TwoFoldRotational => {
                    for (row_num, row) in input.into_iter().enumerate() {
                        for (col_num, square) in row.into_iter().enumerate().skip(row_num) {
                            let recip =
                                board.reciprocal_coordinate(Coordinate::new(col_num, row_num));

                            board.squares[recip.y][recip.x] = square;
                        }
                    }
                }
                Symmetry::Asymmetric => unreachable!(),
            }

            if board.trim_nubs().is_err() {
                return retry_with(board_seed, board);
            }

            // Remove extraneous water
            board.trim();

            if board.width() != board_width || board.height() != board_height {
                return retry_with(board_seed, board);
            }
        }
    }

    if let Some(water_layer) = water_layer {
        if board.generate_water_layer(seed, water_layer).is_err() {
            return retry_with(board_seed, board);
        }
    }

    if elements.artifacts {
        match artifact_type {
            ArtifactType::IslandV1 => {
                if board.drop_island_v1_artifacts(seed).is_err() {
                    return retry_with(board_seed, board);
                }
            }
            _ => {
                if board
                    .drop_artifacts(
                        seed,
                        ideal_artifact_extremity,
                        artifact_type,
                        land_layer.symmetric,
                    )
                    .is_err()
                {
                    return retry_with(board_seed, board);
                }
            }
        }
    }

    if board
        .expand_choke_points(
            minimum_choke,
            water_layer
                .map(|w| w.params.symmetric)
                .unwrap_or(land_layer.symmetric),
            false,
        )
        .is_err()
    {
        return retry_with(board_seed, board);
    }

    // Recalculate the shortest path, as expanding the choke points
    // may have created new paths altogether
    let Some(shortest_attack_path) =
        board.shortest_path_between(&board.artifacts[0], &board.artifacts[1])
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
                land_layer.symmetric,
            )
            .is_err()
        {
            return retry_with(board_seed, board);
        };
    }

    if elements.obelisk {
        if board
            .generate_obelisk(&shortest_attack_path, land_layer.symmetric)
            .is_err()
        {
            return retry_with(board_seed, board);
        }
    }

    Ok(BoardGenerationResult {
        board,
        iterations: current_iteration,
    })
}

trait BoardGenerator {
    fn generate_water_layer(&mut self, seed: u32, water_params: WaterLayer) -> Result<(), ()>;

    fn trim_nubs(&mut self) -> Result<(), ()>;

    fn expand_choke_points(
        &mut self,
        minimum_choke: usize,
        symmetric: Symmetry,
        debug: bool,
    ) -> Result<(), ()>;

    fn drop_island_v1_artifacts(&mut self, seed: u32) -> Result<(), ()>;

    fn drop_artifacts(
        &mut self,
        seed: u32,
        ideal_artifact_extremity: f64,
        artifact_type: ArtifactType,
        symmetric: Symmetry,
    ) -> Result<(), ()>;

    fn generate_towns(
        &mut self,
        seed: u32,
        main_road: &Vec<Coordinate>,
        maximum_town_density: f64,
        maximum_town_distance: f64,
        symmetric: Symmetry,
    ) -> Result<(), ()>;

    fn generate_obelisk(
        &mut self,
        main_road: &Vec<Coordinate>,
        symmetric: Symmetry,
    ) -> Result<(), ()>;
}

impl BoardGenerator for Board {
    fn generate_water_layer(&mut self, seed: u32, water_layer: WaterLayer) -> Result<(), ()> {
        let mut visited: HashSet<Coordinate> = HashSet::from([Coordinate { x: 0, y: 0 }]);
        let mut coastal_land: HashSet<Coordinate> = HashSet::new();

        let mut pts = VecDeque::from(vec![Coordinate { x: 0, y: 0 }]);
        while !pts.is_empty() {
            let pt = pts.pop_front().unwrap();
            for neighbor in pt
                .neighbors_8_iter()
                .filter(|coord| !visited.contains(&coord))
                .collect::<Vec<_>>()
            {
                match self.get(neighbor) {
                    Ok(Square::Water { .. }) => {
                        pts.push_back(neighbor);
                        visited.insert(neighbor);
                    }
                    Ok(Square::Land { .. }) => {
                        visited.insert(neighbor);
                        coastal_land.insert(neighbor);
                    }
                    _ => {}
                }
            }
        }

        let simplex = Simplex::new(seed);
        let canvas_x = self.width() - 2;
        let canvas_y = self.height() - 2;

        let water_params = water_layer.params;
        for i in 1..=canvas_x {
            let skip_horiz = match water_params.symmetric {
                Symmetry::SmoothTwoFoldRotational => 0,
                Symmetry::Asymmetric => 0,
                Symmetry::TwoFoldRotational => i,
            };
            for j in (1..=canvas_y).skip(skip_horiz) {
                let coord = Coordinate { x: i, y: j };

                if coastal_land.contains(&coord) {
                    continue;
                }

                let ni = i as f64 / (canvas_x + 1) as f64; // normalized coordinates
                let nj = j as f64 / (canvas_y + 1) as f64;
                let x = ni - 0.5; // centering the coordinates
                let y = nj - 0.5;

                let distance_to_center = (x * x + y * y).sqrt();

                // Get Simplex noise value
                let noise_value = (simplex.get([
                    ni * water_params.dispersion[0],
                    nj * water_params.dispersion[1],
                    0.0,
                ]) + 1.0)
                    / 2.0;

                // Combine noise and gradient
                let value = noise_value + (distance_to_center * water_params.island_influence);

                if value < water_layer.density {
                    self.set_square(coord, crate::board::Square::water())
                        .expect("Board position should be settable");

                    match water_params.symmetric {
                        Symmetry::SmoothTwoFoldRotational | Symmetry::TwoFoldRotational => {
                            let recip = self.reciprocal_coordinate(coord);
                            self.set_square(recip, crate::board::Square::water())
                                .expect("Board position should be settable");
                        }
                        Symmetry::Asymmetric => { /* no-op */ }
                    };
                }
            }
        }

        if self.trim_nubs().is_err() {
            return Err(());
        }

        // Remove extraneous water
        self.trim();

        Ok(())
    }

    fn trim_nubs(&mut self) -> Result<(), ()> {
        let sqs = || {
            self.squares.iter().enumerate().flat_map(|(y, row)| {
                row.iter().enumerate().flat_map(move |(x, sq)| {
                    if matches!(sq, Square::Land { .. }) {
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
                    .neighbors_4_iter()
                    .filter(|coord| !visited.contains(&coord))
                    .collect::<Vec<_>>()
                {
                    match self.get(neighbor) {
                        Ok(Square::Land { .. }) => {
                            pts.push_back(neighbor);
                            visited.insert(neighbor);
                            this_search.insert(neighbor);
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
                    self.set_square(coord, Square::water())
                        .expect("Board position should be settable");
                }
            }
        }

        Ok(())
    }

    fn expand_choke_points(
        &mut self,
        minimum_choke: usize,
        symmetric: Symmetry,
        debug: bool,
    ) -> Result<(), ()> {
        let Some(shortest_attack_path) =
            self.shortest_path_between(&self.artifacts[0], &self.artifacts[1])
        else {
            return Err(());
        };

        let measurements = shortest_attack_path
            .iter()
            .enumerate()
            .filter_map(|(i, pt)| {
                // Avoid processing the tiles closest to each players artifact
                if i < minimum_choke || i >= shortest_attack_path.len() - minimum_choke {
                    return None;
                }

                let choke_distance = pt
                    .neighbors_8_iter()
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
                            Ok(Square::Land { .. } | Square::Artifact { .. }) => {}
                            Err(_) => {}
                            Ok(_) => {
                                if debug {
                                    _ = self.set_square(
                                        c,
                                        Square::Town {
                                            player: 0,
                                            defeated: false,
                                            foggy: false,
                                        },
                                    );
                                } else {
                                    _ = self.set_square(c, Square::land());

                                    match symmetric {
                                        Symmetry::SmoothTwoFoldRotational
                                        | Symmetry::TwoFoldRotational => {
                                            let recip = self.reciprocal_coordinate(c);
                                            _ = self.set_square(recip, Square::land());
                                        }
                                        Symmetry::Asymmetric => { /* no-op */ }
                                    }
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
                        foggy: false,
                    },
                );
            }
        }

        Ok(())
    }

    // An old implementation of artifact placement that should be avoided when possible.
    // Retained so that past puzzles generate correctly.
    fn drop_island_v1_artifacts(&mut self, seed: u32) -> Result<(), ()> {
        let mut rng = Rand32::new(seed as u64);
        let mut visited: HashSet<Coordinate> = HashSet::from([Coordinate { x: 0, y: 0 }]);
        let mut coastal_water: HashSet<Coordinate> = HashSet::new();

        // Islands require artifacts on the coasts, so we DFS from the edges.
        let mut pts = VecDeque::from(vec![Coordinate { x: 0, y: 0 }]);
        while !pts.is_empty() {
            let pt = pts.pop_front().unwrap();
            for neighbor in pt
                .neighbors_4_iter()
                .filter(|coord| !visited.contains(&coord))
                .collect::<Vec<_>>()
            {
                match self.get(neighbor) {
                    Ok(Square::Water { .. }) => {
                        pts.push_back(neighbor);
                        visited.insert(neighbor);
                    }
                    Ok(Square::Land { .. }) => {
                        visited.insert(neighbor);
                        coastal_water.insert(pt);
                    }
                    _ => {}
                }
            }
        }

        let mut center_point = Coordinate {
            x: self.width() / 2,
            y: self.height() / 2,
        };
        while !matches!(self.get(center_point), Ok(Square::Land { .. })) {
            if self.get(center_point).is_err() {
                return Err(());
            }
            let neighbors = center_point.neighbors_8_iter().collect::<Vec<_>>();

            // If we have fewer than 8 valid neighboring coordinates,
            // we have hit some edge of the board and bail out
            if neighbors.len() != 8 {
                return Err(());
            }
            center_point = center_point
                .neighbors_8_iter()
                .find(|p| matches!(self.get(*p), Ok(Square::Land { .. })))
                .unwrap_or_else(|| neighbors[0])
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
            coord: artifact_zero,
            ..
        }) = pt
        else {
            return Err(());
        };

        self.set_square(artifact_zero, Square::artifact(0))
            .expect("Board position should be settable");

        let mut antipodes: BinaryHeap<_> = coastal_water
            .iter()
            .map(|l| DistanceToCoord {
                coord: l.clone(),
                distance: l.distance_to(&artifact_zero),
            })
            .collect();

        let mut pt = antipodes.pop();
        for _ in 0..rng.rand_range(0..distance_pool) {
            pt = antipodes.pop().or(pt);
        }
        let Some(DistanceToCoord {
            coord: artifact_one,
            ..
        }) = pt
        else {
            return Err(());
        };

        self.set_square(artifact_one, Square::artifact(1))
            .expect("Board position should be settable");

        self.cache_special_squares();

        Ok(())
    }

    fn drop_artifacts(
        &mut self,
        seed: u32,
        ideal_artifact_extremity: f64,
        artifact_type: ArtifactType,
        symmetric: Symmetry,
    ) -> Result<(), ()> {
        let mut rng = Rand32::new(seed as u64);
        let mut viable_water: HashSet<Coordinate> = HashSet::new();

        match artifact_type {
            ArtifactType::IslandV1 => {
                panic!("island_v1 artifacts must go through the island_v1 function")
            }
            ArtifactType::Coastal => {
                // Search from the corner of the map to find all contiguous outer coastal water
                let mut visited: HashSet<Coordinate> = HashSet::from([Coordinate { x: 0, y: 0 }]);
                let mut pts = VecDeque::from(vec![Coordinate { x: 0, y: 0 }]);
                while !pts.is_empty() {
                    let pt = pts.pop_front().unwrap();
                    for neighbor in pt
                        .neighbors_4_iter()
                        .filter(|coord| !visited.contains(&coord))
                        .collect::<Vec<_>>()
                    {
                        match self.get(neighbor) {
                            Ok(Square::Water { .. }) => {
                                pts.push_back(neighbor);
                                visited.insert(neighbor);
                            }
                            Ok(Square::Land { .. }) => {
                                visited.insert(neighbor);
                                viable_water.insert(pt);
                            }
                            _ => {}
                        }
                    }
                }
            }
            ArtifactType::Continental => {
                // All water bordering land is fair game
                let all_pts = (0..self.height())
                    .flat_map(|y| (0..self.width()).zip(std::iter::repeat(y)))
                    .map(|(x, y)| Coordinate::new(x, y));
                for pt in all_pts {
                    if matches!(self.get(pt), Ok(Square::Water { .. }))
                        && pt
                            .neighbors_4_iter()
                            .any(|p| matches!(self.get(p), Ok(Square::Land { .. })))
                    {
                        viable_water.insert(pt);
                    }
                }
            }
        }
        let mut viable_water: Vec<_> = viable_water.into_iter().collect();

        let ideal_center_point = Coordinate {
            x: self.width() / 2,
            y: self.height() / 2,
        };

        let all_pts = (0..self.height())
            .flat_map(|y| (0..self.width()).zip(std::iter::repeat(y)))
            .map(|(x, y)| Coordinate::new(x, y));

        let Some(center_point) = all_pts
            .filter(|p| matches!(self.get(*p), Ok(Square::Land { .. })))
            .min_by_key(|p| p.distance_to(&ideal_center_point))
        else {
            return Err(());
        };

        let distances = self.flood_fill(&center_point);

        viable_water.retain(|w| distances.direct_distance(&w).is_some());
        viable_water.sort_by_key(|w| {
            distances
                .direct_distance(&w)
                .expect("current land should be contiguous")
        });

        // How far away from the extremeties are we allowed to land?
        let slop = self.width().add(self.height()).div(4);

        let start =
            (ideal_artifact_extremity * (viable_water.len() - slop) as f64).floor() as usize;
        let offset = rng.rand_range(0..(slop as u32));

        let artifact_zero = *viable_water.get(start + offset as usize).unwrap();

        self.set_square(artifact_zero, Square::artifact(0))
            .expect("Board position should be settable");

        match symmetric {
            Symmetry::TwoFoldRotational | Symmetry::SmoothTwoFoldRotational => {
                let artifact_one = self.reciprocal_coordinate(artifact_zero);
                self.set_square(artifact_one, Square::artifact(1))
                    .expect("Board position should be settable");
            }
            Symmetry::Asymmetric => {
                // possible idea:
                // find all points that are the same distance from the center of the map
                // find the point that maximises distance between the artifacts within that set of points

                // let artifact_distances = self.flood_fill(&artifact_zero);

                unimplemented!("asymmetric non-island");
            }
        }

        self.cache_special_squares();

        Ok(())
    }

    fn generate_towns(
        &mut self,
        seed: u32,
        main_road: &Vec<Coordinate>,
        maximum_town_density: f64,
        maximum_town_distance: f64,
        symmetric: Symmetry,
    ) -> Result<(), ()> {
        let artifacts = &self.artifacts;
        let Some(Ok(Square::Artifact {
            player: player_zero,
            ..
        })) = artifacts.get(0).map(|d| self.get(*d))
        else {
            return Err(());
        };
        let Some(Ok(Square::Artifact {
            player: player_one, ..
        })) = artifacts.get(1).map(|d| self.get(*d))
        else {
            return Err(());
        };

        let mut town_seed = Rand32::new(seed as u64);

        let town_distance = ((main_road.len() as f64) * maximum_town_distance) as usize;

        let player_zero_dists = self.flood_fill(&artifacts[0]);
        let player_one_dists = self.flood_fill(&artifacts[1]);

        let mut candidates = |dists: BoardDistances| {
            let mut candies: Vec<_> = dists
                .iter_direct()
                .filter_map(|(coord, distance)| {
                    let is_land = matches!(self.get(coord), Ok(Square::Land { .. }));
                    let is_near_artifact = distance <= town_distance;
                    let is_on_critical_path = main_road.contains(&coord);

                    if is_land && is_near_artifact && !is_on_critical_path {
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

            _ = self.set_square(town_zero, Square::town(player_zero));

            match symmetric {
                Symmetry::TwoFoldRotational | Symmetry::SmoothTwoFoldRotational => {
                    let recip = self.reciprocal_coordinate(town_zero);
                    _ = self.set_square(recip, Square::town(player_one));
                }
                Symmetry::Asymmetric => {
                    _ = self.set_square(town_one, Square::town(player_one));
                }
            }
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

    fn generate_obelisk(
        &mut self,
        main_road: &Vec<Coordinate>,
        symmetric: Symmetry,
    ) -> Result<(), ()> {
        match symmetric {
            Symmetry::Asymmetric => {
                // For an asymmetric board, the center of the shortest path becomes the obelisk
                _ = self.set_square(main_road[main_road.len() / 2], Square::obelisk());
            }
            Symmetry::TwoFoldRotational | Symmetry::SmoothTwoFoldRotational => {
                // For a symmetric board, the dead center of the map can become the obelisk
                let board_mid = Coordinate::new(self.width() / 2, self.height() / 2);

                let square = self.get(board_mid).unwrap();
                // TODO: Gracefully add land for the obelisk, or add multiple, or something
                if !matches!(square, Square::Land { .. }) {
                    return Err(());
                }

                _ = self.set_square(board_mid, Square::obelisk());

                for pt in board_mid.neighbors_8_iter() {
                    _ = self.set_square(pt, Square::land());
                }
            }
        }

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
