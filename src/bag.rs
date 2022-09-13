use rand::Rng;
use std::fmt;

pub struct TileBag {
    bag: Vec<char>,
    rng: rand::rngs::ThreadRng,
    letter_distribution: [usize; 26],
}

impl TileBag {
    pub fn new(letter_distribution: [usize; 26]) -> Self {
        let mut tile_bag = TileBag {
            bag: Vec::new(),
            rng: rand::thread_rng(),
            letter_distribution,
        };
        tile_bag.fill();
        tile_bag
    }

    pub fn draw_tile(&mut self) -> char {
        if self.bag.is_empty() {
            self.fill();
        }
        let index = self.rng.gen_range(0..self.bag.len());
        self.bag.swap_remove(index)
    }

    // TODO: this doesn't stop us from returning tiles that weren't originally in the bag
    pub fn return_tile(&mut self, c: char) {
        self.bag.push(c);
    }

    fn fill(&mut self) {
        self.bag.extend(
            self.letter_distribution
                .iter()
                .enumerate()
                .flat_map(|(letter, count)| [((letter as u8) + 65) as char].repeat(*count)),
        );
    }
}

impl Default for TileBag {
    fn default() -> Self {
        Self::new([
            // banagrams letter distribution
            13, 3, 3, 6, 18, 3, 4, 3, 12, 2, 2, 5, 3, 8, 11, 3, 2, 9, 6, 9, 6, 3, 3, 2, 3, 2,
        ])
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
        TileBag::new(dist)
    }

    pub fn trivial_bag() -> TileBag {
        let mut dist = [0; 26];
        dist[0] = 1;
        TileBag::new(dist)
    }
}
