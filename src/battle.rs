use std::collections::HashSet;
use std::fs::File;
use std::io::{self, prelude::*, BufReader};

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    AttackerWins(Vec<usize>), // A list of specific defenders who are defeated
    DefenderWins,             // If the defender wins, all attackers lose
    NoBattle,
}

pub struct Judge {
    dictionary: HashSet<String>,
}

impl Default for Judge {
    fn default() -> Self {
        let file = File::open("./dictionary.txt").expect("file missing"); // collins2018 list
        let reader = BufReader::new(file);
        let mut dictionary = HashSet::new();

        for line in reader.lines() {
            dictionary.insert(line.expect("bad encoding"));
        }
        Self { dictionary }
    }
}

impl Judge {
    pub fn new(words: Vec<&str>) -> Self {
        let mut dictionary = HashSet::new();
        for word in words {
            dictionary.insert(String::from(word));
        }
        Self { dictionary }
    }

    pub fn short_dict() -> Self {
        Self::new(vec!["BIG", "FAT", "JOLLY", "AND", "SILLY", "FOLK", "ARTS"]) // TODO: Collins 2018 list
    }

    // If there are no attackers or no defenders there is no battle
    // The defender wins if any attacking word is invalid, or all defending words are valid and stronger than the longest attacking words
    // Otherwise the attacker wins
    //
    // There is a defender's advantage, so an attacking word has to be at least 2 letters longer than a defending word to be stronger than it.
    pub fn Battle(&self, attackers: Vec<String>, defenders: Vec<String>) -> Outcome {
        // If there are no attackers or no defenders there is no battle
        if attackers.is_empty() || defenders.is_empty() {
            return Outcome::NoBattle;
        }

        // The defender wins if any attacking word is invalid
        let attackers_invalid = attackers
            .iter()
            .map(|word| !self.valid(word))
            .reduce(|prev, curr| prev || curr);
        if attackers_invalid.expect("already checked length") {
            return Outcome::DefenderWins;
        }

        // The defender wins if all their words are valid and long enough to defend against the longest attacker
        let longest_attacker = attackers
            .iter()
            .reduce(|longest, curr| {
                if curr.len() > longest.len() {
                    curr
                } else {
                    longest
                }
            })
            .expect("already checked length");

        let weak_defenders: Vec<usize> = defenders // Indices of the weak defenders
            .iter()
            .enumerate()
            .filter(|(_, word)| !self.valid(word) || word.len() + 1 < longest_attacker.len())
            .map(|(index, _)| index)
            .collect();
        if weak_defenders.is_empty() {
            return Outcome::DefenderWins;
        }

        // Otherwise the attacker wins
        Outcome::AttackerWins(weak_defenders)
    }

    fn valid(&self, word: &String) -> bool {
        self.dictionary.contains(word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn NoBattle_without_combatants() {
        let j = Judge::short_dict();
        assert_eq!(j.Battle(vec![word()], vec![]), Outcome::NoBattle);
        assert_eq!(j.Battle(vec![], vec![word()]), Outcome::NoBattle);
        assert_eq!(j.Battle(vec![], vec![]), Outcome::NoBattle);
    }

    #[test]
    fn attacker_invalid() {
        let j = Judge::short_dict();
        assert_eq!(j.Battle(vec![xyz()], vec![big()]), Outcome::DefenderWins);
        assert_eq!(j.Battle(vec![x__yz()], vec![big()]), Outcome::DefenderWins);
        assert_eq!(
            j.Battle(vec![xyz(), jolly()], vec![big()]),
            Outcome::DefenderWins
        );
        assert_eq!(
            j.Battle(vec![big(), xyz()], vec![big()]),
            Outcome::DefenderWins
        );
        assert_eq!(
            j.Battle(vec![xyz(), big()], vec![big()]),
            Outcome::DefenderWins
        );
    }

    #[test]
    fn defender_invalid() {
        let j = Judge::short_dict();
        assert_eq!(
            j.Battle(vec![big()], vec![xyz()]),
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.Battle(vec![big()], vec![x__yz()]),
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.Battle(vec![big()], vec![big(), xyz()]),
            Outcome::AttackerWins(vec![1])
        );
        assert_eq!(
            j.Battle(vec![big()], vec![xyz(), big()]),
            Outcome::AttackerWins(vec![0])
        );
    }

    #[test]
    fn attacker_weaker() {
        let j = Judge::short_dict();
        assert_eq!(j.Battle(vec![jolly()], vec![folk()]), Outcome::DefenderWins);
        assert_eq!(
            j.Battle(vec![jolly(), big()], vec![folk()]),
            Outcome::DefenderWins
        );
    }

    #[test]
    fn defender_weaker() {
        let j = Judge::short_dict();
        assert_eq!(
            j.Battle(vec![jolly()], vec![fat()]),
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.Battle(vec![jolly(), big()], vec![fat()]),
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.Battle(vec![jolly()], vec![fat(), big(), jolly(), folk(), x__yz()]),
            Outcome::AttackerWins(vec![0, 1, 4])
        );
    }

    #[test]
    fn collins2018() {
        let j = Judge::default();
        assert!(j.valid(&String::from("zyzzyva")));
        assert!(!j.valid(&String::from("zyzzyvava")));
    }

    // TODO: Refactor this silly thing! Just wanted immediate access to these strings
    fn jolly() -> String {
        String::from("JOLLY")
    }
    fn word() -> String {
        String::from("WORD")
    }
    fn xyz() -> String {
        String::from("XYZ")
    }
    fn big() -> String {
        String::from("BIG")
    }
    fn x__yz() -> String {
        String::from("XYZXYZXYZ")
    }
    fn folk() -> String {
        String::from("FOLK")
    }
    fn fat() -> String {
        String::from("FAT")
    }
}
