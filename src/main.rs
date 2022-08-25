use rand::Rng;
use std::fmt;

fn main() {
    let mut bag = TileBag::new_default();
    for i in 0..1000 {
        let tile = bag.draw_tile();
        println!("{}", tile)
    }
}

struct TileBag {
    bag: Vec<char>,
    rng: rand::rngs::ThreadRng,
}

impl TileBag {
    fn new(letter_distribution: [u8; 26]) -> Self {
        let mut bag: Vec<char> = vec![];
        let range: std::ops::Range<u8> = std::ops::Range { start: 0, end: 26 };
        for letter in range {
            for i in 0..letter_distribution[letter as usize] {
                bag.push((letter + 65) as char);
            }
        }

        TileBag {
            bag: bag,
            rng: rand::thread_rng(),
        }
    }

    fn new_default() -> Self {
        Self::new([
            // banagrams letter distribution
            13, 3, 3, 6, 18, 3, 4, 3, 12, 2, 2, 5, 3, 8, 11, 3, 2, 9, 6, 9, 6, 3, 3, 2, 3, 2,
        ])
    }

    // TODO: get O(1) performance on draw - maybe a hashmap?
    // TODO: handle empty bag - maybe fill the bag again with the appropriate letter distribution?
    fn draw_tile(&mut self) -> char {
        let index = self.rng.gen_range(0..self.bag.len());
        self.bag.remove(index)
    }

    // TODO: this doesn't stop us from returning tiles that weren't originally in the bag
    fn return_tile(&mut self, c: char) {
        self.bag.push(c);
    }
}

impl fmt::Display for TileBag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Letters in the bag:\n{:?}", self.bag)
    }
}
