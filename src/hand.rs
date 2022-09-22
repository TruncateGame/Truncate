use crate::error::GamePlayError;

use super::bag::TileBag;

pub struct Hands {
    hands: Vec<Vec<char>>,
    bag: TileBag,
}

impl Hands {
    pub fn new(player_count: usize, capacity: usize, mut bag: TileBag) -> Self {
        let mut hands = Vec::new();
        for player in 0..player_count {
            hands.push(Vec::with_capacity(capacity));
            for _ in 0..capacity {
                hands[player].push(bag.draw_tile());
            }
        }
        Self { hands, bag }
    }

    pub fn use_tile(&mut self, player: usize, tile: char) -> Result<(), GamePlayError> {
        if let Some(hand) = self.hands.get_mut(player) {
            match hand.iter().position(|t| t == &tile) {
                None => Err(GamePlayError::PlayerDoesNotHaveTile { player, tile }),
                Some(index) => {
                    hand[index] = self.bag.draw_tile();
                    Ok(())
                }
            }
        } else {
            Err(GamePlayError::NonExistentPlayer { index: player })
        }
    }

    pub fn get_hand(&self, player: usize) -> &Vec<char> {
        &self.hands[player]
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
    use crate::game::Game;

    use super::super::bag::tests as TileUtils;
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

    #[test]
    fn new() {
        let h = Hands::new(2, 7, TileUtils::trivial_bag());
        assert_eq!(h.hands, vec!(vec!('A'; 7); 2));

        let h = Hands::new(10, 15, TileUtils::trivial_bag());
        assert_eq!(h.hands, vec!(vec!('A'; 15); 10));
    }

    #[test]
    fn get_works() -> Result<(), GamePlayError> {
        let mut h = Hands::new(2, 12, TileUtils::a_b_bag());
        // Make sure that we get an equal amount of As and Bs if we draw an even number
        let mut drawn_tiles: Vec<char> = Vec::new();
        for _ in 0..10 {
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
        let mut h = Hands::new(2, 7, TileUtils::trivial_bag());
        assert_eq!(
            h.use_tile(2, 'A'),
            Err(GamePlayError::NonExistentPlayer { index: 2 })
        );
        assert_eq!(
            h.use_tile(0, 'B'),
            Err(GamePlayError::PlayerDoesNotHaveTile {
                player: 0,
                tile: 'B'
            })
        );
    }
}
