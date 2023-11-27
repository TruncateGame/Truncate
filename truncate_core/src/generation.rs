use std::{
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
    hash::Hash,
};

use noise::{NoiseFn, Simplex};
use oorandom::Rand32;

use crate::{
    board::{self, Board, Coordinate, Square},
    game::Game,
};

#[derive(Debug, Clone)]
pub struct BoardParams {
    pub bounding_width: usize,
    pub bounding_height: usize,
    pub maximum_land_width: Option<usize>,
    pub maximum_land_height: Option<usize>,
    pub water_level: f64,
    pub town_density: f64,
    pub jitter: f64,
    pub town_jitter: f64,
}

// Do not modify any numbered generations.
// Add a new generation number with new parameters.
// Updating an existing generation will break puzzle URLs.
const BOARD_GENERATIONS: [BoardParams; 1] = [BoardParams {
    bounding_width: 16,
    bounding_height: 18,
    maximum_land_width: Some(10),
    maximum_land_height: Some(14),
    water_level: 0.5,
    town_density: 0.37,
    jitter: 0.5,
    town_jitter: 0.5,
}];

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

#[derive(Debug, Clone)]
pub struct BoardSeed {
    pub generation: u32,
    pub seed: u32,
    pub day: Option<u32>,
    pub params: BoardParams,
    pub current_iteration: usize,
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

pub fn generate_board(mut board_seed: BoardSeed) -> Board {
    let BoardSeed {
        generation: _,
        seed,
        day: _,
        current_iteration,
        params:
            BoardParams {
                bounding_width,
                bounding_height,
                maximum_land_width,
                maximum_land_height,
                water_level,
                town_density,
                jitter,
                town_jitter,
            },
    } = board_seed;

    if current_iteration > 10000 {
        panic!("Wow that's deep");
    }

    let simplex = Simplex::new(seed);

    let mut board = Board::new(3, 3);
    board.squares =
        vec![vec![crate::board::Square::Water; bounding_width + 2]; bounding_height + 2];

    for i in 1..=bounding_width {
        for j in 1..=bounding_height {
            let ni = i as f64 / (bounding_width + 1) as f64; // normalized coordinates
            let nj = j as f64 / (bounding_height + 1) as f64;
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
        board_seed.params.water_level *= 0.5;
        return generate_board(board_seed);
    }

    // Remove extraneous water
    board.trim();
    if let Some(maximum_land_width) = maximum_land_width {
        if board.width() > maximum_land_width + 2 {
            board_seed.params.water_level *= 1.05;
            return generate_board(board_seed);
        }
    }
    if let Some(maximum_land_height) = maximum_land_height {
        if board.height() > maximum_land_height + 2 {
            board_seed.params.water_level *= 1.05;
            return generate_board(board_seed);
        }
    }

    if board.drop_docks(seed).is_err() {
        board_seed.internal_reroll();
        return generate_board(board_seed);
    }

    if board
        .generate_towns(seed, town_density, town_jitter)
        .is_err()
    {
        board_seed.internal_reroll();
        return generate_board(board_seed);
    };

    if board.ensure_paths().is_err() {
        board_seed.internal_reroll();
        return generate_board(board_seed);
    }

    println!(
        "Generated a board in {} step(s)",
        board_seed.current_iteration
    );

    board
}

trait BoardGenerator {
    fn trim_nubs(&mut self) -> Result<(), ()>;

    fn drop_docks(&mut self, seed: u32) -> Result<(), ()>;

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

    fn drop_docks(&mut self, seed: u32) -> Result<(), ()> {
        let mut rng = Rand32::new(seed as u64);
        let mut visited: HashSet<Coordinate> = HashSet::from([Coordinate { x: 0, y: 0 }]);
        let mut coastal_land: HashSet<Coordinate> = HashSet::new();
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
                        coastal_land.insert(pt);
                    }
                    _ => {}
                }
            }
        }

        assert_eq!(coastal_land.is_empty(), false);

        let center_point = Coordinate {
            x: self.width() / 2,
            y: self.height() / 2,
        };
        if self.get(center_point) != Ok(Square::Land) {
            return Err(());
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
            .filter(|DistanceToCoord { coord, .. }| coastal_land.contains(coord))
            .collect::<BinaryHeap<_>>();

        let mut pt = distances.pop();
        for _ in 0..rng.rand_range(0..6) {
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

        let mut antipodes: BinaryHeap<_> = coastal_land
            .iter()
            .map(|l| DistanceToCoord {
                coord: l.clone(),
                distance: l.distance_to(&dock_zero),
            })
            .collect();

        let mut pt = antipodes.pop();
        for _ in 0..rng.rand_range(0..6) {
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

    fn generate_towns(&mut self, seed: u32, town_density: f64, town_jitter: f64) -> Result<(), ()> {
        let docks = &self.docks;
        let Ok(Square::Dock(player_zero)) = self.get(docks[0]) else {
            return Err(());
        };
        let Ok(Square::Dock(player_one)) = self.get(docks[1]) else {
            return Err(());
        };

        let max_defense_imbalance_ratio = 0.4;
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

                if (distance_zero as f32 / distance_one as f32) < max_defense_imbalance_ratio {
                    Some((player_zero, coord))
                } else if (distance_one as f32 / distance_zero as f32) < max_defense_imbalance_ratio
                {
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

        'recheck_towns: loop {
            self.cache_special_squares();

            let dock_zero_dists = self.flood_fill(&self.docks[0]);
            let dock_one_dists = self.flood_fill(&self.docks[1]);

            let positions = self.towns.iter().flat_map(|town| {
                let Ok(Square::Town { player, .. }) = self.get(*town) else {
                    panic!("Town position is invalid");
                };
                town.neighbors_4()
                    .into_iter()
                    .filter_map(|coord| {
                        if matches!(self.get(coord), Ok(Square::Land)) {
                            Some((town, coord, player))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            });

            for (town, attackable_land, defending_player) in positions {
                let (attacker_distances, defender_distances) = match defending_player {
                    0 => (&dock_one_dists, &dock_zero_dists),
                    1 => (&dock_zero_dists, &dock_one_dists),
                    _ => unimplemented!(),
                };

                let Some(attack_distance) =
                    attacker_distances.attackable_distance(&attackable_land)
                else {
                    continue;
                };
                let Some(defense_distance) =
                    defender_distances.attackable_distance(&attackable_land)
                else {
                    self.set_square(*town, Square::Land).unwrap();
                    continue 'recheck_towns;
                };

                let ratio = defense_distance as f32 / attack_distance as f32;
                if ratio > max_defense_imbalance_ratio {
                    self.set_square(*town, Square::Land).unwrap();
                    continue 'recheck_towns;
                }
            }

            break 'recheck_towns;
        }

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

        if dock_zero_dists
            .attackable_distance(&self.docks[1])
            .is_none()
        {
            return Err(());
        }

        Ok(())
    }
}

#[derive(Hash)]
struct BoardVerification {
    board: String,
    hands: Vec<String>,
}

pub fn get_game_verification(game: &Game) -> u64 {
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    let verification = BoardVerification {
        board: game.board.to_string(),
        hands: game
            .players
            .iter()
            .map(|p| p.hand.0.iter().collect::<String>())
            .collect(),
    };
    verification.hash(&mut hasher);
    hasher.digest()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reroll_test() {
        let mut seed = BoardSeed::new(12345);
        let bare_seed_1 = seed.seed;
        let board_one = generate_board(seed.clone());
        seed.external_reroll();
        let bare_seed_2 = seed.seed;
        let board_two = generate_board(seed);

        insta::assert_snapshot!(format!(
            "Board 1 from {bare_seed_1}:\n{board_one}\n\nrerolled to {bare_seed_2}:\n{board_two}"
        ));
    }
}
