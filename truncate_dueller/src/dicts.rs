use std::sync::{Mutex};

use truncate_core::judge::{WordData, WordDict};

pub static TRUNCATE_DICT: &str = include_str!("../../dict_builder/final_wordlist.txt");

pub struct Dicts {
    pub total: WordDict,
    pub restricted: WordDict,
}

impl Dicts {
    pub fn remember(&mut self, word: &String) {
        if let Some(word_data) = self.total.get(word).cloned() {
            self.restricted.insert(word.clone(), word_data.clone());
        }
    }
}

pub fn get_dicts() -> Dicts {
    let total_dict = TOTAL_DICT.lock().unwrap();
    let restricted_dict = RESTRICTED_DICT.lock().unwrap();

    Dicts {
        total: total_dict.as_ref().expect("dict has been created").clone(),
        restricted: restricted_dict
            .as_ref()
            .expect("dict has been created")
            .clone(),
    }
}

pub static TOTAL_DICT: Mutex<Option<WordDict>> = Mutex::new(None);
pub static RESTRICTED_DICT: Mutex<Option<WordDict>> = Mutex::new(None);

pub fn ensure_dicts() {
    let mut total_dict = TOTAL_DICT.lock().unwrap();
    let mut restricted_dict = RESTRICTED_DICT.lock().unwrap();

    if total_dict.is_none() {
        let mut valid_words = std::collections::HashMap::new();
        let mut restricted_words = std::collections::HashMap::new();
        let lines = TRUNCATE_DICT.lines();

        for line in lines {
            let mut chunks = line.split(' ');

            let mut word = chunks.next().unwrap().to_string();
            let extensions = chunks.next().unwrap().parse().unwrap();
            let rel_freq = chunks.next().unwrap().parse().unwrap();

            let objectionable = word.chars().next() == Some('*');
            if objectionable {
                word.remove(0);
            }

            valid_words.insert(
                word.clone(),
                WordData {
                    extensions,
                    rel_freq,
                    objectionable,
                },
            );

            // These are the words the NPC will think it recognizes,
            // and won't challenge if they're on the board.
            if rel_freq > 0.90 {
                restricted_words.insert(
                    word,
                    WordData {
                        extensions,
                        rel_freq,
                        objectionable,
                    },
                );
            }
        }

        _ = total_dict.insert(valid_words);
        _ = restricted_dict.insert(restricted_words);
    }
}
