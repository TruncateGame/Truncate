use std::collections::{HashMap, HashSet, VecDeque};

use noise::{NoiseFn, Simplex};
use oorandom::Rand32;

use crate::board::{self, Board, Coordinate, Square};

#[derive(Clone)]
pub struct BoardParams {
    pub seed: u32,
    pub bounding_width: usize,
    pub bounding_height: usize,
    pub maximum_land_width: Option<usize>,
    pub maximum_land_height: Option<usize>,
    pub water_level: f64,
    pub town_density: f64,
    pub jitter: f64,
    pub town_jitter: f64,
    pub current_iteration: usize,
}

impl Default for BoardParams {
    fn default() -> Self {
        Self {
            seed: 1234,
            bounding_width: 16,
            bounding_height: 18,
            maximum_land_width: Some(10),
            maximum_land_height: Some(14),
            water_level: 0.5,
            town_density: 0.5,
            jitter: 0.5,
            town_jitter: 0.5,
            current_iteration: 0,
        }
    }
}

impl BoardParams {
    pub fn seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    fn regen(&mut self) {
        let mut rng = Rand32::new(self.seed as u64);
        let r = rng.rand_u32();
        self.seed = r;
        self.current_iteration += 1;
    }
}

pub fn generate_board(mut params: BoardParams) -> Board {
    let BoardParams {
        seed,
        bounding_width: width,
        bounding_height: height,
        maximum_land_width,
        maximum_land_height,
        water_level,
        town_density,
        jitter,
        town_jitter,
        current_iteration,
    } = params.clone();

    if current_iteration > 10000 {
        panic!("Wow that's deep");
    }

    let simplex = Simplex::new(seed);

    let mut board = Board::new(3, 3);
    board.squares = vec![vec![crate::board::Square::Water; width + 2]; height + 2];

    for i in 1..=width {
        for j in 1..=height {
            let ni = i as f64 / (width + 1) as f64; // normalized coordinates
            let nj = j as f64 / (height + 1) as f64;
            let x = ni - 0.5; // centering the coordinates
            let y = nj - 0.5;
            let distance_to_center = (x * x + y * y).sqrt();

            // Simple radial gradient
            let gradient = 1.0 - distance_to_center * 2.0;

            // Get Simplex noise value
            let noise_value = (simplex.get([80.0 * ni, 80.0 * nj, 0.0]) + 1.0) / 2.0;

            // Combine noise and gradient
            let value = (noise_value * jitter) + (noise_value * gradient);

            if value > water_level {
                board
                    .set_square(Coordinate { x: i, y: j }, crate::board::Square::Land)
                    .expect("Board position should be settable");
            }
        }
    }

    if board.trim_nubs().is_err() {
        params.water_level *= 0.5;
        return generate_board(params);
    }

    // Remove extraneous water
    board.trim();
    if let Some(maximum_land_width) = maximum_land_width {
        if board.width() > maximum_land_width + 2 {
            params.bounding_width -= 1;
            return generate_board(params);
        }
    }
    if let Some(maximum_land_height) = maximum_land_height {
        if board.height() > maximum_land_height + 2 {
            params.bounding_height -= 1;
            return generate_board(params);
        }
    }

    board.drop_docks(seed);

    if board
        .generate_towns(seed, town_density, town_jitter)
        .is_err()
    {
        params.regen();
        return generate_board(params);
    };

    if board.ensure_paths().is_err() {
        params.regen();
        return generate_board(params);
    }

    println!("Generated a board in {} step(s)", params.current_iteration);

    board
}

trait BoardGenerator {
    fn trim_nubs(&mut self) -> Result<(), ()>;

    fn drop_docks(&mut self, seed: u32);

    fn generate_towns(&mut self, seed: u32, town_density: f64, town_jitter: f64) -> Result<(), ()>;

    fn ensure_paths(&mut self) -> Result<(), ()>;
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

    fn drop_docks(&mut self, seed: u32) {
        let mut visited: HashSet<Coordinate> = HashSet::from([Coordinate { x: 0, y: 0 }]);
        let mut land_adjacent: Vec<Coordinate> = Vec::new();
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
                        land_adjacent.push(pt);
                    }
                    _ => {}
                }
            }
        }

        assert_eq!(land_adjacent.is_empty(), false);

        let mut rng = Rand32::new(seed as u64);
        let chosen_zero = rng.rand_range(0..land_adjacent.len() as u32);
        let dock_zero = land_adjacent[chosen_zero as usize];
        self.set_square(dock_zero, Square::Dock(0))
            .expect("Board position should be settable");

        let furthest_distance = land_adjacent
            .iter()
            .map(|l| l.distance_to(&dock_zero))
            .max()
            .expect("Other water exists");

        let min_distance = (furthest_distance / 3) * 2;
        let far_positions = land_adjacent
            .into_iter()
            .filter(|pt| pt.distance_to(&dock_zero) >= min_distance)
            .collect::<Vec<_>>();

        let chosen_one = rng.rand_range(0..far_positions.len() as u32);
        let dock_one = far_positions[chosen_one as usize];
        self.set_square(dock_one, Square::Dock(1))
            .expect("Board position should be settable");

        self.cache_special_squares();
    }

    fn generate_towns(&mut self, seed: u32, town_density: f64, town_jitter: f64) -> Result<(), ()> {
        let docks = &self.docks;
        let Ok(Square::Dock(player_zero)) = self.get(docks[0]) else {
            return Err(());
        };
        let Ok(Square::Dock(player_one)) = self.get(docks[1]) else {
            return Err(());
        };

        let max_ratio = 0.4;
        let candidates = self
            .squares
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter().enumerate().flat_map(move |(x, sq)| {
                    if matches!(sq, Square::Land) {
                        Some(Coordinate { x, y })
                    } else {
                        None
                    }
                })
            })
            .filter_map(|coord| {
                let distance_zero = coord.distance_to(&docks[0]);
                let distance_one = coord.distance_to(&docks[1]);

                if (distance_zero as f32 / distance_one as f32) < max_ratio {
                    Some((player_zero, coord))
                } else if (distance_one as f32 / distance_zero as f32) < max_ratio {
                    Some((player_one, coord))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut town_seed = Rand32::new(seed as u64);
        let simplex = Simplex::new(town_seed.rand_u32());

        for (player, coord) in candidates {
            let Coordinate { x, y } = coord;
            let rel_i = x as f64 / (self.width() - 1) as f64; // normalized coordinates
            let rel_j = y as f64 / (self.height() - 1) as f64;
            let centered_x = rel_i - 0.5; // centering the coordinates
            let centered_y = rel_j - 0.5;
            let distance_to_center = (centered_x * centered_x + centered_y * centered_y).sqrt();

            // Simple inverse radial gradient
            let gradient = distance_to_center * 2.0;

            // Get Simplex noise value
            let noise_value = (simplex.get([80.0 * rel_i, 80.0 * rel_j, 0.0]) + 1.0) / 2.0;

            // Combine noise and gradient
            let value = (noise_value * town_jitter) + (noise_value * gradient);

            if value > (1.0 - town_density) {
                self.set_square(
                    coord,
                    crate::board::Square::Town {
                        player,
                        defeated: false,
                    },
                )
                .expect("Board position should be settable");
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

    fn ensure_paths(&mut self) -> Result<(), ()> {
        let dock_zero_dists = self.flood_fill(&self.docks[0]);
        let dock_one_dists = self.flood_fill(&self.docks[1]);

        if dock_zero_dists
            .attackable_distance(&self.docks[1])
            .is_none()
        {
            return Err(());
        }

        Ok(())
    }
}
