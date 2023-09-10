use std::cmp::Ordering;
use std::fmt::Debug;

use crate::board::Board;

use super::WordQualityScores;

#[derive(Clone, Default, PartialEq)]
pub struct BoardScore {
    infinity: bool,
    neg_infinity: bool,
    turn_number: usize, // Lower number means later turn
    word_quality: WordQualityScores,
    self_frontline: f32,
    opponent_frontline: f32,
    self_progress: f32,
    opponent_progress: f32,
    self_defense: f32,
    self_win: bool,
    opponent_win: bool,
    pub board: Option<Board>,
}

impl Debug for BoardScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoardScore")
            .field("infinity", &self.infinity)
            .field("neg_infinity", &self.neg_infinity)
            .field("turn_number", &self.turn_number)
            .field("word_quality", &self.word_quality)
            .field("self_frontline", &self.self_frontline)
            .field("opponent_frontline", &self.opponent_frontline)
            .field("self_progress", &self.self_progress)
            .field("opponent_progress", &self.opponent_progress)
            .field("self_defense", &self.self_defense)
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

    pub fn turn_number(mut self, value: usize) -> Self {
        self.turn_number = value;
        self
    }

    pub fn word_quality(mut self, value: WordQualityScores) -> Self {
        self.word_quality = value;
        self
    }

    pub fn self_frontline(mut self, value: f32) -> Self {
        self.self_frontline = value;
        self
    }

    pub fn opponent_frontline(mut self, value: f32) -> Self {
        self.opponent_frontline = value;
        self
    }

    pub fn self_progress(mut self, value: f32) -> Self {
        self.self_progress = value;
        self
    }

    pub fn opponent_progress(mut self, value: f32) -> Self {
        self.opponent_progress = value;
        self
    }

    pub fn self_defense(mut self, value: f32) -> Self {
        self.self_defense = value;
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
        let opponent_scores = self.opponent_frontline * 2.0 + self.opponent_progress;
        let self_scores = self.self_frontline * 2.0
            + self.self_progress
            + self.self_defense
            + self.word_quality.word_length
            + self.word_quality.word_validity * 2.0
            + self.word_quality.word_extensibility;

        self_scores - opponent_scores
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
            .self_frontline(1.0);

        assert!(base > early_loss);
        assert!(base > late_loss);
        assert!(late_loss > early_loss);
        assert!(late_better_loss > late_loss);
    }
}
