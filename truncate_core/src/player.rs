use std::fmt;
use time::Duration;

use serde::{Deserialize, Serialize};

use super::bag::TileBag;
use crate::{
    error::GamePlayError,
    reporting::{Change, HandChange},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hand(pub Vec<char>);

impl fmt::Display for Hand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0.iter().map(|c| c.to_string()).collect::<String>()
        )
    }
}

impl Hand {
    pub fn iter(&self) -> std::slice::Iter<'_, char> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, index: usize) -> Option<&char> {
        self.0.get(index)
    }

    pub fn find(&self, tile: char) -> Option<usize> {
        self.0.iter().position(|t| *t == tile)
    }

    pub fn replace(&mut self, index: usize, tile: char) {
        self.0[index] = tile;
    }

    pub fn replace_tile(&mut self, from: char, to: char) {
        if let Some(index) = self.find(from) {
            self.replace(index, to);
        }
    }

    pub fn add(&mut self, tile: char) {
        self.0.push(tile);
    }

    pub fn remove(&mut self, index: usize) {
        self.0.swap_remove(index);
    }

    pub fn rearrange(&mut self, from: usize, to: usize) {
        let c = self.0.remove(from);
        self.0.insert(to, c);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Player {
    pub name: String,
    pub index: usize,
    pub hand: Hand,
    pub hand_capacity: usize,
    pub allotted_time: Option<Duration>,
    pub time_remaining: Option<Duration>,
    pub turn_starts_no_later_than: Option<u64>,
    pub turn_starts_no_sooner_than: Option<u64>,
    pub paused_turn_delta: Option<i64>,
    pub swap_count: usize,
    pub penalties_incurred: usize,
    pub color: (u8, u8, u8),
}

impl Player {
    pub fn new(
        name: String,
        index: usize,
        hand_capacity: usize,
        bag: &mut TileBag,
        time_allowance: Option<Duration>,
        color: (u8, u8, u8),
    ) -> Self {
        Self {
            name,
            index,
            hand: Hand((0..hand_capacity).map(|_| bag.draw_tile()).collect()),
            hand_capacity,
            allotted_time: time_allowance,
            time_remaining: time_allowance,
            turn_starts_no_later_than: None,
            turn_starts_no_sooner_than: None,
            paused_turn_delta: None,
            swap_count: 0,
            penalties_incurred: 0,
            color,
        }
    }

    pub fn use_tile(&mut self, tile: char, bag: &mut TileBag) -> Result<Change, GamePlayError> {
        match self.hand.iter().position(|t| t == &tile) {
            None => Err(GamePlayError::PlayerDoesNotHaveTile {
                player: self.index,
                tile,
            }),
            Some(index) => {
                if self.hand.len() > self.hand_capacity {
                    // They have too many tiles, so we don't give them a new one
                    self.hand.remove(index);
                    Ok(Change::Hand(HandChange {
                        player: self.index,
                        removed: vec![tile],
                        added: vec![],
                    }))
                } else {
                    self.hand.replace(index, bag.draw_tile());
                    Ok(Change::Hand(HandChange {
                        player: self.index,
                        removed: vec![tile],
                        added: vec![*self.hand.get(index).unwrap()],
                    }))
                }
            }
        }
    }

    pub fn add_special_tile(&mut self, tile: char) -> Change {
        self.hand.add(tile);
        Change::Hand(HandChange {
            player: self.index,
            removed: vec![],
            added: vec![tile],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default() {
        let mut bag = TileBag::latest(None).1;
        let player = Player::new(
            "Liam Gallagher".into(),
            0,
            7,
            &mut bag,
            Some(Duration::new(60, 0)),
            (255, 0, 0),
        );
        assert_eq!(player.hand.len(), 7);
        // Note, this relies on randomness, so could fail even though it generally passes
        // TODO(liam): Disabling this check for as the randomness has changed and may change again
        // TODO(liam): Moved this from hand.rs and haven't updated it
        // TODO: Change all game logic to run off a seeded RNG to make this deterministic,
        //       (also makes game seeds replayable, if we ever want that)
        // for player in h.hands {
        //     for tile in player {
        //         let num = tile as u8;
        //         assert!((65..=90_u8).contains(&num)); // A-Z
        //     }
        // }
    }

    // TODO(liam): Redo / re-enable tests
    // #[test]
    // fn get_works() -> Result<(), GamePlayError> {
    //     let mut h = Hands::new(2, 12, TileUtils::a_b_bag());
    //     // Make sure that we get an equal amount of As and Bs if we draw an even number
    //     let mut drawn_tiles: Vec<char> = Vec::new();
    //     for _ in 0..10 {
    //         h.use_tile(0, h.hands[0][0])?;
    //         drawn_tiles.push(h.hands[0][0]);
    //         h.use_tile(0, h.hands[0][0])?;
    //         drawn_tiles.push(h.hands[0][0]);
    //         assert_eq!(
    //             drawn_tiles.iter().filter(|&&t| t == 'A').count(),
    //             drawn_tiles.iter().filter(|&&t| t == 'B').count(),
    //         )
    //     }
    //     Ok(())
    // }

    // #[test]
    // fn get_errors() {
    //     let mut h = Hands::new(2, 7, TileUtils::trivial_bag());
    //     assert_eq!(
    //         h.use_tile(2, 'A'),
    //         Err(GamePlayError::NonExistentPlayer { index: 2 })
    //     );
    //     assert_eq!(
    //         h.use_tile(0, 'B'),
    //         Err(GamePlayError::PlayerDoesNotHaveTile {
    //             player: 0,
    //             tile: 'B'
    //         })
    //     );
    // }
}
