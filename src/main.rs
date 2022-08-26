use rand::Rng;
use std::fmt;

fn main() {
    let mut bag = TileBag::default();
    for i in 0..1000 {
        let tile = bag.draw_tile();
        println!("{}", tile)
    }
}

struct TileBag {
    bag: Vec<char>,
    rng: rand::rngs::ThreadRng,
    letter_distribution: [usize; 26],
}

impl TileBag {
    fn new(letter_distribution: [usize; 26]) -> Self {
        let mut tile_bag = TileBag {
            bag: Vec::new(),
            rng: rand::thread_rng(),
            letter_distribution,
        };
        tile_bag.fill();
        tile_bag
    }

    fn draw_tile(&mut self) -> char {
        if (self.bag.len() == 0) {
            self.fill();
        }
        let index = self.rng.gen_range(0..self.bag.len());
        self.bag.swap_remove(index)
    }

    // TODO: this doesn't stop us from returning tiles that weren't originally in the bag
    fn return_tile(&mut self, c: char) {
        self.bag.push(c);
    }

    fn fill(&mut self) {
        self.bag.append(
            self.letter_distribution
                .iter()
                .enumerate()
                .flat_map(|(letter, count)| [((letter as u8) + 65) as char].repeat(*count))
                .collect(),
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
