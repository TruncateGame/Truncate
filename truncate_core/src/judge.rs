use serde::{Deserialize, Serialize};

use crate::{
    error::GamePlayError,
    reporting::{BattleReport, BattleWord},
    rules,
};

use super::board::{Board, Square};
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display},
};

#[derive(Debug, Clone)]
pub struct WordData {
    pub extensions: u32,
    pub rel_freq: f32,
}
pub type WordDict = HashMap<String, WordData>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    AttackerWins(Vec<usize>), // A list of specific defenders who are defeated
    DefenderWins,             // If the defender wins, all attackers lose
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Outcome::AttackerWins(losers) => {
                write!(f, "Attacker wins against {:#?}", losers)
            }
            Outcome::DefenderWins => write!(f, "Defender wins"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Judge {
    builtin_dictionary: WordDict,
    aliases: HashMap<char, Vec<char>>,
}

impl Default for Judge {
    fn default() -> Self {
        Self {
            builtin_dictionary: HashMap::new(),
            aliases: HashMap::new(),
        }
    }
}

impl Judge {
    pub fn new(words: Vec<String>) -> Self {
        let mut dictionary = HashMap::new();
        for word in words {
            dictionary.insert(
                word.to_lowercase(),
                WordData {
                    extensions: 0,
                    rel_freq: 0.0,
                },
            );
        }
        Self {
            builtin_dictionary: dictionary,
            aliases: HashMap::new(),
        }
    }

    pub fn set_alias(&mut self, alias_target: Vec<char>) -> char {
        for p in ['①', '②', '③', '④', '⑤', '⑥', '⑦', '⑧', '⑨'] {
            if self.aliases.contains_key(&p) {
                continue;
            }
            self.aliases.insert(p, alias_target);
            return p;
        }
        panic!("Too many aliases!");
    }

    pub fn remove_aliases(&mut self) {
        self.aliases.clear();
    }

    // A player wins if they touch an opponent's town
    // TODO: accept a config that chooses between different win conditions, like occupying enough quadrants
    // TODO: error (or possibly return a tie) if there are multiple winners - this assume turn based play
    // TODO: put this somewhere better, it conceptually works as a judge associated function, but it only uses values from the board
    pub fn winner(board: &Board) -> Option<usize> {
        for town_coord in board.towns() {
            if let Ok(Square::Town {
                player,
                defeated: true,
            }) = board.get(*town_coord)
            {
                return Some((player + 1) % 2);
            }
        }

        None
    }

    // If there are no attackers or no defenders there is no battle
    // The defender wins if any attacking word is invalid, or all defending words are valid and stronger than the longest attacking words
    // Otherwise the attacker wins
    //
    // There is a defender's advantage, so an attacking word has to be at least 2 letters longer than a defending word to be stronger than it.
    pub fn battle<S: AsRef<str> + Clone + Display>(
        &self,
        attackers: Vec<S>,
        defenders: Vec<S>,
        battle_rules: &rules::BattleRules,
        win_rules: &rules::WinCondition,
        external_dictionary: Option<&WordDict>,
    ) -> Option<BattleReport> {
        // If there are no attackers or no defenders there is no battle
        if attackers.is_empty() || defenders.is_empty() {
            return None;
        }

        let mut battle_report = BattleReport {
            attackers: attackers
                .iter()
                .map(|w| {
                    let valid = self.valid(w, win_rules, external_dictionary, None);
                    BattleWord {
                        original_word: w.to_string(),
                        valid: Some(valid.is_some()),
                        meanings: None,
                        resolved_word: valid.unwrap_or_else(|| w.to_string()),
                    }
                })
                .collect(),
            defenders: defenders
                .iter()
                .map(|w| BattleWord {
                    original_word: w.to_string(),
                    resolved_word: w.to_string(),
                    meanings: None,
                    valid: None,
                })
                .collect(),
            outcome: Outcome::DefenderWins,
        };

        // The defender wins if any attacking word is invalid
        if battle_report
            .attackers
            .iter()
            .any(|word| word.valid == Some(false))
        {
            battle_report.outcome = Outcome::DefenderWins;
            return Some(battle_report);
        }

        for defense in &mut battle_report.defenders {
            let valid = self.valid(
                &*defense.resolved_word,
                win_rules,
                external_dictionary,
                None,
            );
            if let Some(valid) = valid {
                defense.resolved_word = valid;
                defense.valid = Some(true);
            } else {
                defense.valid = Some(false);
            }
        }

        // The defender wins if all their words are valid and long enough to defend against the longest attacker
        let longest_attacker = attackers
            .iter()
            .reduce(|longest, curr| {
                if curr.as_ref().len() > longest.as_ref().len() {
                    curr
                } else {
                    longest
                }
            })
            .expect("already checked length");

        let attacker_wins_outright = attackers.iter().any(|word| word.as_ref().contains('¤'));
        if attacker_wins_outright {
            battle_report.outcome = Outcome::AttackerWins(vec![]);
            return Some(battle_report);
        }

        let non_town_words: Vec<_> = battle_report
            .defenders
            .iter()
            .enumerate()
            .filter(|(_, word)| !word.original_word.contains('#'))
            .collect();

        let town_words: Vec<_> = battle_report
            .defenders
            .iter()
            .enumerate()
            .filter(|(_, word)| word.original_word.contains('#'))
            .collect();

        let weak_word_defenders: Vec<_> = non_town_words
            .iter()
            .filter(|(_, word)| {
                word.valid != Some(true)
                    || word.resolved_word.len() as isize + battle_rules.length_delta as isize
                        <= longest_attacker.as_ref().len() as isize
            })
            .map(|(index, _)| *index)
            .collect();

        let weak_town_defenders: Vec<_> = town_words
            .iter()
            .filter(|(_, word)| {
                word.valid != Some(true)
                    || word.resolved_word.len() as isize + battle_rules.length_delta as isize
                        <= longest_attacker.as_ref().len() as isize
            })
            .map(|(index, _)| *index)
            .collect();

        // Normal battles without towns, easy cases.
        if town_words.is_empty() {
            if weak_word_defenders.is_empty() {
                battle_report.outcome = Outcome::DefenderWins;
            } else {
                battle_report.outcome = Outcome::AttackerWins(weak_word_defenders);
            }

            return Some(battle_report);
        }

        // Towns were involved in this battle, resolve using the town battle rules
        let has_beatable_towns = !weak_town_defenders.is_empty();
        let has_words = !non_town_words.is_empty();
        let has_beatable_words = !weak_word_defenders.is_empty();

        let mut all_weak_defenders = weak_word_defenders.clone();
        all_weak_defenders.extend(weak_town_defenders);

        battle_report.outcome = match (has_beatable_towns, has_words, has_beatable_words) {
            // Towns can be beat, and there are also some weak real words
            (true, true, true) => Outcome::AttackerWins(all_weak_defenders),
            // Towns can be beat, but all real words can defend
            (true, true, false) => Outcome::DefenderWins,
            // Towns can be beat, and no words are involved in the battle
            (true, false, _) => Outcome::AttackerWins(all_weak_defenders),
            // Towns cannot be beat directly, but there are weak words that lose the defense anyway
            (false, true, true) => Outcome::AttackerWins(all_weak_defenders),
            // Towns cannot be beat, and there were no beatable words either
            (false, _, false) => Outcome::DefenderWins,
            // Catch the unreachable case of no words with beatable words
            (_, false, true) => unreachable!(),
        };

        Some(battle_report)
    }

    /// Returns the string that was matched if word was a wildcard
    pub fn valid<S: AsRef<str>>(
        &self,
        word: S,
        win_rules: &rules::WinCondition,
        external_dictionary: Option<&WordDict>,
        used_aliases: Option<HashMap<char, Vec<usize>>>,
    ) -> Option<String> {
        if word.as_ref().contains('¤') {
            return Some(word.as_ref().to_string().to_uppercase());
        }

        if word.as_ref().contains('#') {
            return match win_rules {
                rules::WinCondition::Destination { town_defense } => match town_defense {
                    rules::TownDefense::BeatenByContact => None,
                    rules::TownDefense::BeatenByValidity => None,
                    rules::TownDefense::BeatenWithDefenseStrength(town_strength) => {
                        Some(vec!['#'; *town_strength].into_iter().collect())
                    }
                },
                rules::WinCondition::Elimination => {
                    debug_assert!(false);
                    None
                }
            };
        }

        // Handle the first matching alias we find (others will be handled in the next recursion)
        for (alias, resolved) in &self.aliases {
            if word.as_ref().contains(*alias) {
                return resolved.iter().enumerate().find_map(|(i, c)| {
                    let mut used = used_aliases.clone().unwrap_or_default();
                    if used.get(c).map(|tiles| tiles.contains(&i)) == Some(true) {
                        return None;
                    }

                    if let Some(used_alias) = used.get_mut(c) {
                        used_alias.push(i);
                    } else {
                        used.insert(*c, vec![i]);
                    }

                    self.valid(
                        word.as_ref().replacen(*alias, &c.to_string(), 1),
                        win_rules,
                        external_dictionary,
                        Some(used),
                    )
                });
            }
        }

        if word.as_ref().contains('*') {
            // Try all letters in the first wildcard spot
            // TODO: find a fun way to optimize this to not be 26^wildcard_count (regex?)
            return (97..=122_u8).find_map(|c| {
                self.valid(
                    word.as_ref().replacen('*', &(c as char).to_string(), 1),
                    win_rules,
                    external_dictionary,
                    used_aliases.clone(),
                )
            });
        }

        if external_dictionary
            .unwrap_or(&self.builtin_dictionary)
            .contains_key(&word.as_ref().to_lowercase())
        {
            Some(word.as_ref().to_string().to_uppercase())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::board::{tests as BoardUtils, Coordinate, Direction};

    use super::*;

    fn test_battle_rules() -> rules::BattleRules {
        rules::BattleRules { length_delta: 2 }
    }

    fn test_win_rules() -> rules::WinCondition {
        rules::WinCondition::Destination {
            town_defense: rules::TownDefense::BeatenWithDefenseStrength(0),
        }
    }

    #[test]
    fn no_battle_without_combatants() {
        let j = short_dict();
        assert_eq!(
            j.battle(
                vec!["WORD"],
                vec![],
                &test_battle_rules(),
                &test_win_rules(),
                None
            ),
            None
        );
        assert_eq!(
            j.battle(
                vec![],
                vec!["WORD"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            ),
            None
        );
        // need to specify a generic here since the vecs are empty, only needed in test
        assert_eq!(
            j.battle::<&'static str>(
                vec![],
                vec![],
                &test_battle_rules(),
                &test_win_rules(),
                None
            ),
            None
        );
    }

    #[test]
    fn attacker_invalid() {
        let j = short_dict();
        assert_eq!(
            j.battle(
                vec!["XYZ"],
                vec!["BIG"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(
                vec!["XYZXYZXYZ"],
                vec!["BIG"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(
                vec!["XYZ", "JOLLY"],
                vec!["BIG"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(
                vec!["BIG", "XYZ"],
                vec!["BIG"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(
                vec!["XYZ", "BIG"],
                vec!["BIG"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
    }

    #[test]
    fn defender_invalid() {
        let j = short_dict();
        assert_eq!(
            j.battle(
                vec!["BIG"],
                vec!["XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(
                vec!["BIG"],
                vec!["XYZXYZXYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(
                vec!["BIG"],
                vec!["BIG", "XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![1])
        );
        assert_eq!(
            j.battle(
                vec!["BIG"],
                vec!["XYZ", "BIG"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
    }

    #[test]
    fn attacker_weaker() {
        let j = short_dict();
        assert_eq!(
            j.battle(
                vec!["JOLLY"],
                vec!["FOLK"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(
                vec!["JOLLY", "BIG"],
                vec!["FOLK"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
    }

    #[test]
    fn defender_weaker() {
        let j = short_dict();
        assert_eq!(
            j.battle(
                vec!["JOLLY"],
                vec!["FAT"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(
                vec!["JOLLY", "BIG"],
                vec!["FAT"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(
                vec!["JOLLY"],
                vec!["FAT", "BIG", "JOLLY", "FOLK", "XYZXYZXYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0, 1, 4])
        );
    }

    #[test]
    fn wildcards() {
        let j = short_dict();
        assert_eq!(
            j.battle(
                vec!["B*G"],
                vec!["XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(
                vec!["R*G"],
                vec!["XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(
                vec!["ARTS"],
                vec!["JALL*"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(
                vec!["BAG"],
                vec!["JOLL*"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
    }

    #[test]
    fn aliases() {
        let mut j = short_dict();

        let a_or_b = j.set_alias(vec!['a', 'b']);
        let b_or_c = j.set_alias(vec!['b', 'c']);

        assert_eq!(
            j.battle(
                vec![format!("B{a_or_b}G").as_str()],
                vec!["XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(
                vec![format!("B{b_or_c}G").as_str()],
                vec!["XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(
                vec![format!("{b_or_c}{a_or_b}G").as_str()],
                vec!["XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
    }

    #[test]
    fn multi_aliases() {
        let mut j = short_dict();

        let o_or_l = j.set_alias(vec!['o', 'l']);
        let two_ls = j.set_alias(vec!['l', 'l']);

        // We can't double-dip on a tile with an alias
        assert_eq!(
            j.battle(
                vec![format!("JO{o_or_l}{o_or_l}Y").as_str()],
                vec!["XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
        // But we can use multiple of the same
        assert_eq!(
            j.battle(
                vec![format!("JO{two_ls}{two_ls}Y").as_str()],
                vec!["XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
        // Or multiple that are different
        assert_eq!(
            j.battle(
                vec![format!("J{o_or_l}L{o_or_l}Y").as_str()],
                vec!["XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
    }

    #[test]
    fn battle_report() {
        let j = short_dict();
        assert_eq!(
            j.battle(
                vec!["B*G"],
                vec!["XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            ),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    original_word: "B*G".into(),
                    resolved_word: "BAG".into(),
                    meanings: None,
                    valid: Some(true)
                }],
                defenders: vec![BattleWord {
                    original_word: "XYZ".into(),
                    resolved_word: "XYZ".into(),
                    meanings: None,
                    valid: Some(false)
                }],
                outcome: Outcome::AttackerWins(vec![0])
            })
        );
        assert_eq!(
            j.battle(
                vec!["R*G"],
                vec!["XYZ"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            ),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    original_word: "R*G".into(),
                    resolved_word: "R*G".into(),
                    meanings: None,
                    valid: Some(false)
                }],
                defenders: vec![BattleWord {
                    original_word: "XYZ".into(),
                    resolved_word: "XYZ".into(),
                    meanings: None,
                    valid: None
                }],
                outcome: Outcome::DefenderWins
            })
        );

        assert_eq!(
            j.battle(
                vec!["ARTS"],
                vec!["JALL*"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            ),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    original_word: "ARTS".into(),
                    resolved_word: "ARTS".into(),
                    meanings: None,
                    valid: Some(true)
                }],
                defenders: vec![BattleWord {
                    original_word: "JALL*".into(),
                    resolved_word: "JALL*".into(),
                    meanings: None,
                    valid: Some(false)
                }],
                outcome: Outcome::AttackerWins(vec![0])
            })
        );
        assert_eq!(
            j.battle(
                vec!["BAG"],
                vec!["JOLL*"],
                &test_battle_rules(),
                &test_win_rules(),
                None
            ),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    original_word: "BAG".into(),
                    resolved_word: "BAG".into(),
                    meanings: None,
                    valid: Some(true)
                }],
                defenders: vec![BattleWord {
                    original_word: "JOLL*".into(),
                    resolved_word: "JOLLY".into(),
                    meanings: None,
                    valid: Some(true)
                }],
                outcome: Outcome::DefenderWins
            })
        );
    }

    // #[test]
    // fn main_dict() {
    //     let j = Judge::default();
    //     assert_eq!(j.valid("R*G", None), Some("RAG".into()));
    //     assert_eq!(j.valid("zyzzyva", None), Some("ZYZZYVA".into()));
    //     assert_eq!(j.valid("zyzzyvava", None), None);
    //     // Casing indepdendent
    //     assert_eq!(j.valid("ZYZZYVA", None), Some("ZYZZYVA".into()));
    // }

    // #[test]
    // fn win_condition() {
    //     let mut b = BoardUtils::from_string(
    //         [
    //             "    X    ",
    //             "X X X _ _",
    //             "X _ _ _ _",
    //             "X _ _ _ _",
    //             "_ _ _ _ _",
    //             "    _    ",
    //         ]
    //         .join("\n"),
    //         vec![Coordinate { x: 0, y: 0 }],
    //         vec![Direction::North],
    //     )
    //     .unwrap();

    //     assert_eq!(Judge::winner(&b), None);
    //     b.set(Coordinate { x: 0, y: 4 }, 0, 'X').unwrap();
    //     assert_eq!(Judge::winner(&b), Some(0));
    // }

    // Utils
    pub fn short_dict() -> Judge {
        Judge::new(vec![
            "BIG".into(),
            "BAG".into(),
            "FAT".into(),
            "JOLLY".into(),
            "AND".into(),
            "SILLY".into(),
            "FOLK".into(),
            "ARTS".into(),
        ]) // TODO: Collins 2018 list
    }
}
