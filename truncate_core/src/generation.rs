use std::{
    collections::{BinaryHeap, HashSet, VecDeque},
    hash::Hash,
    ops::Div,
};

use noise::{NoiseFn, Simplex};
use oorandom::Rand32;

use crate::{
    board::{Board, Coordinate, Square},
    game::Game,
};

#[derive(Debug, Clone, Copy)]
pub enum BoardType {
    Island,
    Continental,
}

#[derive(Debug, Clone)]
pub struct BoardParams {
    pub ideal_land_dimensions: [usize; 2],
    pub land_slop: usize,
    pub water_level: f64,
    pub dispersion: f64,
    pub town_density: f64,
    pub jitter: f64,
    pub town_jitter: f64,
    pub minimum_choke: usize,
    pub board_type: BoardType,
}

// Do not modify any numbered generations.
// Add a new generation number with new parameters.
// Updating an existing generation will break puzzle URLs.
const BOARD_GENERATIONS: [BoardParams; 1] = [BoardParams {
    ideal_land_dimensions: [30, 30],
    land_slop: 4,
    water_level: 0.004,
    dispersion: 3.0,
    town_density: 0.2,
    jitter: 0.637,
    town_jitter: 0.36,
    minimum_choke: 4,
    board_type: BoardType::Continental,
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
    pub resize_state: Option<PreviousBoardResize>,
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
            resize_state: None,
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
            resize_state: None,
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
        resize_state,
        max_attempts,
        params:
            BoardParams {
                ideal_land_dimensions,
                land_slop,
                water_level,
                dispersion,
                town_density,
                jitter,
                town_jitter,
                minimum_choke,
                board_type,
            },
    } = board_seed;

    let retry_with = |mut board_seed: BoardSeed, failed_board: Board| {
        board_seed.internal_reroll();
        if current_iteration > max_attempts {
            eprintln!("Could not resolve a playable board within {max_attempts} tries");
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
    let canvas_size = [ideal_land_dimensions[0] * 2, ideal_land_dimensions[1] * 2];
    // Expand the canvas when creating board squares to avoid setting anything in the outermost ring
    board.squares = vec![vec![crate::board::Square::Water; canvas_size[0] + 2]; canvas_size[1] + 2];

    for i in 1..=canvas_size[0] {
        for j in 1..=canvas_size[1] {
            let ni = i as f64 / (canvas_size[0] + 1) as f64; // normalized coordinates
            let nj = j as f64 / (canvas_size[1] + 1) as f64;
            let x = ni - 0.5; // centering the coordinates
            let y = nj - 0.5;

            let distance_to_center = (x * x + y * y).sqrt();
            let gradient = match board_type {
                BoardType::Island => {
                    // Simple radial gradient
                    distance_to_center
                }
                BoardType::Continental => {
                    // Radial gradient, extremely biased to only affect the edges
                    if distance_to_center < 0.5 {
                        0.0
                    } else {
                        distance_to_center.powf(2.0)
                    }
                }
            };

            // Get Simplex noise value
            let noise_value = (simplex.get([ni * dispersion, nj * dispersion, 0.0]) + 1.0) / 2.0;

            // Combine noise and gradient
            let value = noise_value - (gradient * jitter);

            let is_land = match board_type {
                BoardType::Island => value > water_level,
                BoardType::Continental => {
                    let water_amplitude = (water_level - 0.5) * 0.05;
                    noise_value > 0.5 + water_amplitude
                }
            };

            if is_land {
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

    let width_diff = board.width() as isize - (ideal_land_dimensions[0] + 2) as isize;
    let height_diff = board.height() as isize - (ideal_land_dimensions[1] + 2) as isize;

    // Raise or lower water slightly to try and hit the target island size
    if width_diff.is_negative() || height_diff.is_negative() {
        // Avoid oscillating around the target — once we have shrunk we won't enlarge again
        if board_seed.resize_state != Some(PreviousBoardResize::Shrunk) {
            board_seed.params.water_level -= 0.01;
            board_seed.resize_state = Some(PreviousBoardResize::Enlarged);
            return generate_board(board_seed);
        }
    } else if width_diff.is_positive() || height_diff.is_positive() {
        board_seed.params.water_level += 0.005;
        board_seed.resize_state = Some(PreviousBoardResize::Shrunk);
        return generate_board(board_seed);
    }

    /*
       TODO:
       To land on the correct width we should do something that only scales the noise dispersion in the Y axis,
       to have the effect of stretching the map vertically.
       This will allow us to avoid rerolls for invalid dimensions....
    */

    // If our steady state landed outside tolerances, start again.
    if width_diff.abs() > land_slop as isize || height_diff.abs() > land_slop as isize {
        return retry_with(board_seed, board);
    }

    match board_type {
        BoardType::Island => {
            if board.drop_island_docks(seed).is_err() {
                return retry_with(board_seed, board);
            }
        }
        BoardType::Continental => {
            if board.drop_continental_docks(seed).is_err() {
                return retry_with(board_seed, board);
            }
        }
    }

    let Some(mut shortest_attack_path) =
        board.shortest_path_between(&board.docks[0], &board.docks[1])
    else {
        return retry_with(board_seed, board);
    };

    let pathy: Vec<_> = shortest_attack_path
        .iter()
        .map(|pt| {
            let choke_distance = pt
                .neighbors_8()
                .iter()
                .map(|n| board.distance_to_closest_obstruction(&n, &shortest_attack_path))
                .max()
                .unwrap();
            let v = if choke_distance >= 10 {
                '+'
            } else {
                choke_distance.to_string().chars().next().unwrap()
            };
            return (pt, v);
        })
        .collect();

    for (i, pt) in shortest_attack_path.iter().enumerate() {
        if i < minimum_choke || i >= shortest_attack_path.len() - minimum_choke {
            continue;
        }

        let choke_distance = pt
            .neighbors_8()
            .iter()
            .map(|n| board.distance_to_closest_obstruction(&n, &shortest_attack_path))
            .max()
            .unwrap();
        if choke_distance < minimum_choke {
            let mid = (minimum_choke / 2) as isize;
            for x in -mid..(minimum_choke as isize - mid) {
                for y in -mid..(minimum_choke as isize - mid) {
                    let c = Coordinate {
                        x: pt.x + x as usize,
                        y: pt.y + y as usize,
                    };

                    if c.x == 0 || c.y == 0 || c.x == board.width() - 1 || c.y == board.height() - 1
                    {
                        // Don't touch the outer rim of the board.
                        continue;
                    }

                    match board.get(c) {
                        Ok(Square::Land | Square::Dock(_)) => {}
                        Err(_) => {}
                        Ok(_) => {
                            _ = board.set_square(
                                c,
                                Square::Town {
                                    player: 0,
                                    defeated: false,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    for (pt, v) in pathy {
        _ = board.set_square(*pt, Square::Occupied(0, v));
    }

    return Ok(BoardGenerationResult {
        board,
        iterations: current_iteration,
    });

    if board
        .generate_towns(seed, town_density, town_jitter, board_type)
        .is_err()
    {
        return retry_with(board_seed, board);
    };

    if board.ensure_paths().is_err() {
        return retry_with(board_seed, board);
    }

    println!(
        "Generated a board in {} step(s)",
        board_seed.current_iteration
    );

    Ok(BoardGenerationResult {
        board,
        iterations: current_iteration,
    })
}

trait BoardGenerator {
    fn trim_nubs(&mut self) -> Result<(), ()>;

    fn drop_island_docks(&mut self, seed: u32) -> Result<(), ()>;
    fn drop_continental_docks(&mut self, seed: u32) -> Result<(), ()>;

    fn generate_towns(
        &mut self,
        seed: u32,
        town_density: f64,
        town_jitter: f64,
        board_type: BoardType,
    ) -> Result<(), ()>;

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

    fn drop_continental_docks(&mut self, seed: u32) -> Result<(), ()> {
        let mut rng = Rand32::new(seed as u64);
        let mut coastal_water: Vec<Coordinate> = Vec::new();

        // Continental boards can have docks anywhere adjacent to water.
        for i in 0..self.width() {
            for j in 0..self.height() {
                let coord = Coordinate { x: i, y: j };

                if matches!(self.get(coord), Ok(Square::Water)) {
                    if coord
                        .neighbors_4()
                        .iter()
                        .any(|c| matches!(self.get(*c), Ok(Square::Land)))
                    {
                        coastal_water.push(coord);
                    }
                }
            }
        }

        // Continental boards really prefer inland docks, so we DFS open water and lower its chances.
        let mut open_water: HashSet<Coordinate> = HashSet::from([Coordinate { x: 0, y: 0 }]);
        let mut pts = VecDeque::from(vec![Coordinate { x: 0, y: 0 }]);
        while !pts.is_empty() {
            let pt = pts.pop_front().unwrap();
            for neighbor in pt
                .neighbors_4()
                .iter()
                .filter(|coord| !open_water.contains(&coord))
                .collect::<Vec<_>>()
            {
                match self.get(*neighbor) {
                    Ok(Square::Water) => {
                        pts.push_back(*neighbor);
                        open_water.insert(*neighbor);
                    }
                    _ => {}
                }
            }
        }

        assert_eq!(coastal_water.is_empty(), false);

        let mut get_water = || {
            let chance_of_open_water = 0.02;
            loop {
                let c = coastal_water[rng.rand_range(0..coastal_water.len() as u32) as usize];
                if open_water.contains(&c) {
                    if rng.rand_float() < chance_of_open_water {
                        return c;
                    }
                } else {
                    return c;
                }
            }
        };

        let dock_zero = get_water();

        self.set_square(dock_zero, Square::Dock(0))
            .expect("Board position should be settable");

        let distances = self.flood_fill(&dock_zero);
        let min_dist = self.width().max(self.height()) / 3;

        let mut dock_one = get_water();
        let mut dock_one_dist = distances.attackable_distance(&dock_one);

        while dock_one_dist.is_none() || dock_one_dist.unwrap() < min_dist {
            dock_one = get_water();
            dock_one_dist = distances.attackable_distance(&dock_one);
        }

        self.set_square(dock_one, Square::Dock(1))
            .expect("Board position should be settable");

        self.cache_special_squares();

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

    fn generate_towns(
        &mut self,
        seed: u32,
        town_density: f64,
        town_jitter: f64,
        board_type: BoardType,
    ) -> Result<(), ()> {
        let docks = &self.docks;
        let Some(Ok(Square::Dock(player_zero))) = docks.get(0).map(|d| self.get(*d)) else {
            return Err(());
        };
        let Some(Ok(Square::Dock(player_one))) = docks.get(1).map(|d| self.get(*d)) else {
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

            let gradient = match board_type {
                // Simple inverse radial gradient
                BoardType::Island => distance_to_center * 2.0,
                BoardType::Continental => 1.0,
            };

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
