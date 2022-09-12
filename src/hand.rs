use std::ops::Index;

use super::bag::TileBag;

pub struct Hands {
    hands: Vec<Vec<char>>,
    bag: TileBag,
}

impl Hands {
    pub fn new(player_count: usize, capacity: usize, mut bag: TileBag) -> Self {
        let mut hands = Vec::new();
        for player in 0..player_count {
            hands.push(Vec::new());
            for _ in 0..capacity {
                hands[player].push(bag.draw_tile());
            }
        }
        Self { hands, bag }
    }

    pub fn use_tile(&mut self, player: usize, tile: char) -> Result<(), &str> {
        if self.hands.len() <= player {
            return Err("Invalid player"); // TODO: say which player is wrong
        }

        let index = self.hands[player].iter().position(|t| t == &tile);
        match index {
            None => Err("Player doesn't have that tile"), // TODO: say which player and tile
            Some(index) => {
                self.hands[player][index] = self.bag.draw_tile();
                Ok(())
            }
        }
    }
}

impl Default for Hands {
    fn default() -> Self {
        let bag = TileBag::default();
        Hands::new(2, 7, bag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default() {
        let h = Hands::default();
        assert_eq!(h.hands.len(), 2);
        assert_eq!(h.hands[0].len(), 7);
        assert_eq!(h.hands[1].len(), 7);
        // Note, this relies on randomness, so could fail even though it generally passes
        for player in h.hands {
            for tile in player {
                let num = tile as u8;
                assert!((65..=90_u8).contains(&num)); // A-Z
            }
        }
    }

    // Makes trivial tile bag that always gives 'A'
    fn trivial_bag() -> TileBag {
        let mut dist = [0; 26];
        dist[0] = 1;
        TileBag::new(dist)
    }

    #[test]
    fn new() {
        let h = Hands::new(2, 7, trivial_bag());
        assert_eq!(h.hands, vec!(vec!('A'; 7); 2));

        let h = Hands::new(10, 15, trivial_bag());
        assert_eq!(h.hands, vec!(vec!('A'; 15); 10));
    }

    #[test]
    fn get_works() -> Result<(), String> {
        let mut dist = [0; 26];
        dist[0] = 1;
        dist[1] = 1;
        let a_b_bag = TileBag::new(dist);

        let mut h = Hands::new(2, 12, a_b_bag);
        // Make sure that we get an equal amount of As and Bs if we draw an even number
        let mut drawn_tiles: Vec<char> = Vec::new();
        for i in 0..10 {
            h.use_tile(0, h.hands[0][0])?;
            drawn_tiles.push(h.hands[0][0]);
            h.use_tile(0, h.hands[0][0])?;
            drawn_tiles.push(h.hands[0][0]);
            assert_eq!(
                drawn_tiles.iter().filter(|&&t| t == 'A').count(), // TODO: why does this end up being a double reference?
                drawn_tiles.iter().filter(|&&t| t == 'B').count(),
            )
        }
        Ok(())
    }

    #[test]
    fn get_errors() {
        let mut h = Hands::new(2, 7, trivial_bag());
        assert_eq!(h.use_tile(2, 'A'), Err("Invalid player"));
        assert_eq!(h.use_tile(0, 'B'), Err("Player doesn't have that tile"));
    }
}
