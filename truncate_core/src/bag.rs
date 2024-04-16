use oorandom::Rand32;
use std::fmt;

use crate::rules;

#[derive(Debug, Clone)]
pub struct TileBag {
    bag: Vec<char>,
    rng: Rand32,
    letter_distribution: Option<[usize; 26]>,
}

impl TileBag {
    pub fn new(tile_distribution: &rules::TileDistribution, seed: Option<u64>) -> Self {
        match tile_distribution {
            rules::TileDistribution::Standard => Self::custom(
                [
                    14, // a
                    4,  // b
                    5,  // c
                    6,  // d
                    16, // e
                    3,  // f
                    4,  // g
                    4,  // h
                    10, // i
                    1,  // j
                    3,  // k
                    8,  // l
                    4,  // m
                    7,  // n
                    11, // o
                    5,  // p
                    1,  // q
                    9,  // r
                    10, // s
                    8,  // t
                    6,  // u
                    2,  // v
                    3,  // w
                    1,  // x
                    4,  // y
                    1,  // z
                ],
                seed,
            ),
        }
    }

    pub fn custom(letter_distribution: [usize; 26], seed: Option<u64>) -> Self {
        let mut tile_bag = TileBag {
            bag: Vec::new(),
            rng: Rand32::new(seed.unwrap_or_else(|| {
                instant::SystemTime::now()
                    .duration_since(instant::SystemTime::UNIX_EPOCH)
                    .expect("Please don't play Truncate earlier than 1970")
                    .as_secs()
            })),
            letter_distribution: Some(letter_distribution),
        };
        tile_bag.fill();
        tile_bag
    }

    pub fn explicit(tiles: Vec<char>, seed: Option<u64>) -> Self {
        TileBag {
            bag: tiles,
            rng: Rand32::new(seed.unwrap_or_else(|| {
                instant::SystemTime::now()
                    .duration_since(instant::SystemTime::UNIX_EPOCH)
                    .expect("Please don't play Truncate earlier than 1970")
                    .as_secs()
            })),
            letter_distribution: None,
        }
    }

    pub fn draw_tile(&mut self) -> char {
        if self.bag.is_empty() {
            self.fill();
        }
        let index = self.rng.rand_range(0..self.bag.len() as u32);
        self.bag.swap_remove(index as usize)
    }

    // TODO: this doesn't stop us from returning tiles that weren't originally in the bag
    pub fn return_tile(&mut self, c: char) {
        self.bag.push(c);
    }

    fn fill(&mut self) {
        if let Some(letter_distribution) = self.letter_distribution {
            self.bag.extend(
                letter_distribution
                    .iter()
                    .enumerate()
                    .flat_map(|(letter, count)| [((letter as u8) + 65) as char].repeat(*count)),
            );
        }
    }
}

impl Default for TileBag {
    fn default() -> Self {
        Self::new(&rules::TileDistribution::Standard, None)
    }
}

impl PartialEq for TileBag {
    fn eq(&self, rhs: &Self) -> bool {
        self.bag == rhs.bag && self.letter_distribution == rhs.letter_distribution
    }
}

impl fmt::Display for TileBag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Letters in the bag:\n{:?}", self.bag)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn refills() {
        let mut bag = a_b_bag();
        assert_eq!(bag.to_string(), "Letters in the bag:\n['A', 'B']");
        let drawn = (0..10).map(|_| bag.draw_tile());
        assert_eq!(drawn.filter(|&x| x == 'A').count(), 5);
    }

    // Util functions
    pub fn a_b_bag() -> TileBag {
        let mut dist = [0; 26];
        dist[0] = 1; // There is 1 A and
        dist[1] = 1; // 1 B in the bag
        TileBag::custom(dist, Some(12345))
    }

    pub fn trivial_bag() -> TileBag {
        let mut dist = [0; 26];
        dist[0] = 1;
        TileBag::custom(dist, Some(12345))
    }
}
