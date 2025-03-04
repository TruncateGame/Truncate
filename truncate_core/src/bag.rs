use oorandom::Rand32;
use std::fmt;

use crate::rules;

/*
INFO: Letter distributions in Truncate's dict

   | <= 3   | <= 5   | <= 7
a: | 15.30  | 12.93  | 11.64
b: | 5.76   | 3.91   | 3.53
c: | 3.41   | 4.76   | 5.11
d: | 6.29   | 5.54   | 6.23
e: | 12.31  | 14.36  | 16.95
f: | 3.09   | 2.84   | 2.42
g: | 5.50   | 3.76   | 4.40
h: | 5.19   | 4.11   | 3.58
i: | 8.75   | 8.25   | 9.94
j: | 1.15   | 0.64   | 0.45
k: | 2.36   | 3.31   | 2.19
l: | 4.61   | 7.86   | 7.94
m: | 5.61   | 4.45   | 4.11
n: | 5.71   | 6.54   | 7.91
o: | 13.52  | 10.27  | 8.77
p: | 6.55   | 4.97   | 4.51
q: | 0.16   | 0.23   | 0.27
r: | 5.76   | 8.89   | 10.35
s: | 6.08   | 13.34  | 12.65
t: | 7.18   | 7.72   | 8.18
u: | 6.29   | 5.50   | 5.43
v: | 1.41   | 1.57   | 1.42
w: | 4.09   | 2.61   | 1.90
x: | 1.78   | 0.67   | 0.51
y: | 4.98   | 4.18   | 2.98
z: | 1.15   | 0.80   | 0.65

*/

const TILE_GENERATIONS: [[usize; 26]; 2] = [
    [
        13, // a
        3,  // b
        3,  // c
        6,  // d
        18, // e
        3,  // f
        4,  // g
        3,  // h
        12, // i
        2,  // j
        2,  // k
        5,  // l
        3,  // m
        8,  // n
        11, // o
        3,  // p
        2,  // q
        9,  // r
        6,  // s
        9,  // t
        6,  // u
        3,  // v
        3,  // w
        2,  // x
        3,  // y
        2,  // z
    ],
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
];

#[derive(Debug, Clone)]
pub struct TileBag {
    bag: Vec<char>,
    rng: Rand32,
    letter_distribution: Option<[usize; 26]>,
}

impl TileBag {
    pub fn generation(gen: u32, seed: u64) -> Self {
        TileBag::custom(
            TILE_GENERATIONS
                .get(gen as usize)
                .expect("Tilebag generation should exist")
                .clone(),
            seed,
        )
    }

    pub fn latest(seed: u64) -> (u32, Self) {
        assert!(!TILE_GENERATIONS.is_empty());
        let generation = (TILE_GENERATIONS.len() - 1) as u32;
        (generation, TileBag::generation(generation as u32, seed))
    }

    pub fn custom(letter_distribution: [usize; 26], seed: u64) -> Self {
        let mut tile_bag = TileBag {
            bag: Vec::new(),
            rng: Rand32::new(seed),
            letter_distribution: Some(letter_distribution),
        };
        tile_bag.fill();
        tile_bag
    }

    pub fn explicit(tiles: Vec<char>, seed: u64) -> Self {
        TileBag {
            bag: tiles,
            rng: Rand32::new(seed),
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
        TileBag::custom(dist, 12345)
    }

    pub fn trivial_bag() -> TileBag {
        let mut dist = [0; 26];
        dist[0] = 1;
        TileBag::custom(dist, 12345)
    }
}
