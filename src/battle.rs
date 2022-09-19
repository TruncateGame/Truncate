use std::collections::HashSet;

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    AttackerWins(Vec<usize>), // A list of specific defenders who are defeated
    DefenderWins,             // If the defender wins, all attackers lose
    NoBattle,
}

pub struct Judge {
    dictionary: HashSet<String>,
}

impl Judge {
    pub fn new(words: Vec<&str>) -> Self {
        let mut dictionary = HashSet::new();
        for word in words {
            dictionary.insert(String::from(word));
        }
        Self { dictionary }
    }

    pub fn default() -> Self {
        Self::new(vec!["BIG", "FAT", "JOLLY", "AND", "SILLY", "FOLK"]) // TODO: Collins 2018 list
    }

    // If there are no attackers or no defenders there is no battle
    // The defender wins if any attacking word is invalid, or all defending words are valid and stronger than the longest attacking words
    // Otherwise the attacker wins
    //
    // There is a defender's advantage, so an attacking word has to be at least 2 letters longer than a defending word to be stronger than it.
    pub fn Battle(&self, attackers: Vec<&str>, defenders: Vec<&str>) -> Outcome {
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
            .filter(|(_, &word)| !self.valid(word) || word.len() + 1 < longest_attacker.len())
            .map(|(index, _)| index)
            .collect();
        if weak_defenders.is_empty() {
            return Outcome::DefenderWins;
        }

        // Otherwise the attacker wins
        Outcome::AttackerWins(weak_defenders)
    }

    fn valid(&self, word: &str) -> bool {
        self.dictionary.contains(word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn NoBattle_without_combatants() {
        let j = Judge::default();
        assert_eq!(j.Battle(vec!["WORD"], vec![]), Outcome::NoBattle);
        assert_eq!(j.Battle(vec![], vec!["WORD"]), Outcome::NoBattle);
        assert_eq!(j.Battle(vec![], vec![]), Outcome::NoBattle);
    }

    #[test]
    fn attacker_invalid() {
        let j = Judge::default();
        assert_eq!(j.Battle(vec!["XYZ"], vec!["BIG"]), Outcome::DefenderWins);
        assert_eq!(
            j.Battle(vec!["XYZXYZXYZ"], vec!["BIG"]),
            Outcome::DefenderWins
        );
        assert_eq!(
            j.Battle(vec!["XYZ", "JOLLY"], vec!["BIG"]),
            Outcome::DefenderWins
        );
        assert_eq!(
            j.Battle(vec!["BIG", "XYZ"], vec!["BIG"]),
            Outcome::DefenderWins
        );
        assert_eq!(
            j.Battle(vec!["XYZ", "BIG"], vec!["BIG"]),
            Outcome::DefenderWins
        );
    }

    #[test]
    fn defender_invalid() {
        let j = Judge::default();
        assert_eq!(
            j.Battle(vec!["BIG"], vec!["XYZ"]),
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.Battle(vec!["BIG"], vec!["XYZXYZXYZ"]),
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.Battle(vec!["BIG"], vec!["BIG", "XYZ"]),
            Outcome::AttackerWins(vec![1])
        );
        assert_eq!(
            j.Battle(vec!["BIG"], vec!["XYZ", "BIG"]),
            Outcome::AttackerWins(vec![0])
        );
    }

    #[test]
    fn attacker_weaker() {
        let j = Judge::default();
        assert_eq!(j.Battle(vec!["JOLLY"], vec!["FOLK"]), Outcome::DefenderWins);
        assert_eq!(
            j.Battle(vec!["JOLLY", "BIG"], vec!["FOLK"]),
            Outcome::DefenderWins
        );
    }

    #[test]
    fn defender_weaker() {
        let j = Judge::default();
        assert_eq!(
            j.Battle(vec!["JOLLY"], vec!["FAT"]),
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.Battle(vec!["JOLLY", "BIG"], vec!["FAT"]),
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.Battle(
                vec!["JOLLY"],
                vec!["FAT", "BIG", "JOLLY", "FOLK", "XYZXYZXYZ"]
            ),
            Outcome::AttackerWins(vec![0, 1, 4])
        );
    }
}
