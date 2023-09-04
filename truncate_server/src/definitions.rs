use std::collections::{HashMap, HashSet};

use rand::seq::SliceRandom;
use rusqlite::Connection;
use truncate_core::{
    judge::{WordData, WordDict},
    reporting::WordMeaning,
};

pub static WORDNIK: &str = include_str!("../../word_freqs/final_wordlist.txt");

pub struct WordDB {
    pub conn: Option<Connection>,
    pub valid_words: WordDict,
    pub room_codes: Vec<String>,
    pub allocated_room_codes: HashSet<String>,
}

impl WordDB {
    pub fn get_word(&self, word: &str) -> Option<Vec<WordMeaning>> {
        let Some(conn) = &self.conn else { return None };

        let mut stmt = conn
            .prepare("SELECT definitions FROM words WHERE word = ?")
            .unwrap();

        let def_str: Option<String> = stmt
            .query(&[word])
            .unwrap()
            .next()
            .unwrap()
            .map(|row| row.get_unwrap("definitions"));

        def_str
            .map(|def: String| serde_json::from_str(&def).ok())
            .flatten()
    }

    fn rand_code(&self) -> String {
        self.room_codes
            .choose(&mut rand::thread_rng())
            .cloned()
            .expect("No words in dataset")
    }

    // TODO: Reclaim codes after use
    pub fn get_free_code(&mut self) -> String {
        let mut word = self.rand_code();
        while self.allocated_room_codes.get(&word).is_some() {
            word = self.rand_code();
        }
        self.allocated_room_codes.insert(word.clone());
        word
    }
}

pub fn read_defs() -> WordDB {
    println!("Loading word definitions...");

    let defs_file = option_env!("TR_DEFS_FILE").unwrap_or_else(|| "/truncate/defs.db");

    let mut valid_words = HashMap::new();
    let lines = WORDNIK.lines();

    for line in lines {
        let mut chunks = line.split(' ');

        let mut word = chunks.next().unwrap().to_string();
        let objectionable = word.chars().next() == Some('*');
        if objectionable {
            word.remove(0);
        }

        valid_words.insert(
            word,
            WordData {
                extensions: chunks.next().unwrap().parse().unwrap(),
                rel_freq: chunks.next().unwrap().parse().unwrap(),
                objectionable,
            },
        );
    }

    let word_db_connection = Connection::open(defs_file).ok();
    if word_db_connection.is_some() {
        println!("Connected to the word definition database at {defs_file}");
    } else {
        println!("No word definitions available at {defs_file}. Set a TR_DEFS_FILE environment variable to point to a word db.");
    }

    let room_codes: Vec<_> = valid_words
        .iter()
        .filter(|(word, data)| word.len() < 6 && !data.objectionable)
        .map(|(word, _)| word)
        .cloned()
        .collect();

    println!("There are {} room codes available", room_codes.len());

    WordDB {
        conn: word_db_connection,
        room_codes,
        valid_words,
        allocated_room_codes: HashSet::new(),
    }
}
