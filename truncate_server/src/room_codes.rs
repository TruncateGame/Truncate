use rand::seq::SliceRandom;

use super::GameMap;

#[derive(Clone)]
pub struct RoomCodes {
    available_codes: Vec<&'static str>,
    active_games: GameMap,
}

impl RoomCodes {
    pub fn new(game_map: GameMap) -> Self {
        // TODO: Tidy the available room codes,
        // store as separate dict list rather than calculating from the judge DICT
        Self {
            available_codes: truncate_core::judge::WORDNIK
                .lines()
                .filter(|l| l.len() < 6)
                .collect(),
            active_games: game_map,
        }
    }

    fn rand_code(&self) -> &'static str {
        self.available_codes
            .choose(&mut rand::thread_rng())
            .cloned()
            .expect("No words in dataset")
    }

    // TODO: Track codes given out internally through a Mutex rather than accessing the GameMap directly
    pub fn get_free_code(&self) -> String {
        let mut word = self.rand_code();
        while self.active_games.get(word).is_some() {
            word = self.rand_code();
        }
        word.to_owned()
    }
}
