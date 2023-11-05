use std::cmp::Ordering;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::board::Board;

use super::WordQualityScores;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoardWeights {
    pub raced_defense: f32,
    pub raced_attack: f32,
    pub self_defense: f32,
    pub self_attack: f32,
    pub direct_defence: f32,
    pub direct_attack: f32,
    pub word_validity: f32,
    pub word_length: f32,
    pub word_extensibility: f32,
}

impl Default for BoardWeights {
    fn default() -> Self {
        Self {
            raced_defense: 6.0,
            raced_attack: 2.0,
            self_defense: 1.0,
            self_attack: 2.0,
            direct_defence: 1.0,
            direct_attack: 1.0,
            word_validity: 2.0,
            word_length: 1.0,
            word_extensibility: 1.0,
        }
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct BoardScore {
    infinity: bool,
    neg_infinity: bool,
    turn_number: usize, // Lower number means later turn
    word_quality: WordQualityScores,
    raced_defense: f32,
    raced_attack: f32,
    self_defense: f32,
    self_attack: f32,
    direct_defence: f32,
    direct_attack: f32,
    self_win: bool,
    opponent_win: bool,
    weights: BoardWeights,
    pub board: Option<Board>,
}

impl Debug for BoardScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoardScore")
            .field("infinity", &self.infinity)
            .field("neg_infinity", &self.neg_infinity)
            .field("turn_number", &self.turn_number)
            .field("word_quality", &self.word_quality)
            .field("raced_defense", &self.raced_defense)
            .field("raced_attack", &self.raced_attack)
            .field("self_defense", &self.self_defense)
            .field("self_attack", &self.self_attack)
            .field("direct_defence", &self.direct_defence)
            .field("direct_attack", &self.direct_attack)
            .field("self_win", &self.self_win)
            .field("opponent_win", &self.opponent_win)
            .finish()
    }
}

impl BoardScore {
    pub fn board(mut self, value: Board) -> Self {
        self.board = Some(value);
        self
    }

    pub fn weights(mut self, value: BoardWeights) -> Self {
        self.weights = value;
        self
    }

    pub fn turn_number(mut self, value: usize) -> Self {
        self.turn_number = value;
        self
    }

    pub fn word_quality(mut self, value: WordQualityScores) -> Self {
        self.word_quality = value;
        self
    }

    pub fn raced_defense(mut self, value: f32) -> Self {
        self.raced_defense = value;
        self
    }

    pub fn raced_attack(mut self, value: f32) -> Self {
        self.raced_attack = value;
        self
    }

    pub fn self_defense(mut self, value: f32) -> Self {
        self.self_defense = value;
        self
    }

    pub fn self_attack(mut self, value: f32) -> Self {
        self.self_attack = value;
        self
    }

    pub fn direct_defence(mut self, value: f32) -> Self {
        self.direct_defence = value;
        self
    }

    pub fn direct_attack(mut self, value: f32) -> Self {
        self.direct_attack = value;
        self
    }

    pub fn self_win(mut self, value: bool) -> Self {
        self.self_win = value;
        self
    }

    pub fn opponent_win(mut self, value: bool) -> Self {
        self.opponent_win = value;
        self
    }
}

impl BoardScore {
    pub fn inf() -> Self {
        Self {
            infinity: true,
            ..Self::default()
        }
    }

    pub fn neg_inf() -> Self {
        Self {
            neg_infinity: true,
            ..Self::default()
        }
    }
}

impl BoardScore {
    pub fn rank(&self) -> f32 {
        self.raced_defense * self.weights.raced_defense
            + self.raced_attack * self.weights.raced_attack
            + self.self_defense * self.weights.self_defense
            + self.self_attack * self.weights.self_attack
            + self.direct_defence * self.weights.direct_defence
            + self.direct_attack * self.weights.direct_attack
            + self.word_quality.word_validity * self.weights.word_validity
            + self.word_quality.word_length * self.weights.word_length
            + self.word_quality.word_extensibility * self.weights.word_extensibility
    }

    pub fn usize_rank(&self) -> usize {
        (self.rank() * 100000.0) as usize
    }
}

impl PartialOrd for BoardScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.infinity.partial_cmp(&other.infinity) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match other.neg_infinity.partial_cmp(&self.neg_infinity) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }

        match (self.self_win, other.self_win) {
            (true, false) => return Some(Ordering::Greater),
            (false, true) => return Some(Ordering::Less),
            // Rank early wins high
            (true, true) => return self.turn_number.partial_cmp(&other.turn_number),
            _ => {}
        }

        match (self.opponent_win, other.opponent_win) {
            (true, false) => return Some(Ordering::Less),
            (false, true) => return Some(Ordering::Greater),
            (true, true) => match other.turn_number.partial_cmp(&self.turn_number) {
                // Rank early losses low
                Some(Ordering::Greater) => return Some(Ordering::Greater),
                Some(Ordering::Less) => return Some(Ordering::Less),
                // Rank even losses on the rest of the board
                _ => {}
            },
            _ => {}
        }

        self.rank().partial_cmp(&other.rank())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infinities() {
        let max = BoardScore::inf();
        let base = BoardScore::default();
        let min = BoardScore::neg_inf();

        assert!(max > base);
        assert!(min < base);
        assert!(max > min)
    }

    #[test]
    fn validities() {
        let a = BoardScore::default().word_quality(WordQualityScores {
            word_length: 0.0,
            word_validity: 0.6,
            word_extensibility: 0.0,
        });
        let b = BoardScore::default().word_quality(WordQualityScores {
            word_length: 0.0,
            word_validity: 0.5,
            word_extensibility: 0.0,
        });

        assert!(a > b);
    }

    #[test]
    fn winning() {
        let base = BoardScore::default();
        let early_win = BoardScore::default().turn_number(1).self_win(true);
        let late_win = BoardScore::default().turn_number(0).self_win(true);

        assert!(early_win > base);
        assert!(late_win > base);
        assert!(early_win > late_win);
    }

    #[test]
    fn losing() {
        let base = BoardScore::default();
        let early_loss = BoardScore::default().turn_number(1).opponent_win(true);
        let late_loss = BoardScore::default().turn_number(0).opponent_win(true);
        let late_better_loss = BoardScore::default()
            .turn_number(0)
            .opponent_win(true)
            .self_attack(0.1);

        assert!(base > early_loss);
        assert!(base > late_loss);
        assert!(late_loss > early_loss);
        assert!(late_better_loss > late_loss);
    }
}
