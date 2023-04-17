use dashmap::DashMap;
use flate2::read::GzDecoder;
use serde_jsonlines::JsonLinesReader;
use std::{fs::File, io::BufReader};

use crate::WordMap;

pub fn read_defs(words: WordMap) {
    println!("Loading word definitions...");

    let defs_file = option_env!("TR_DEFS_FILE").unwrap_or_else(|| "/truncate/defs.json.gz");

    let defs_file = match File::open(defs_file) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("❌ Failed to load word defs: {e}");
            eprintln!("❌ Run with TR_DEFS_FILE env var pointing to defs");
            return;
        }
    };

    let d = GzDecoder::new(defs_file);
    let j = JsonLinesReader::new(BufReader::new(d));

    let mut errored = false;
    for res in j.read_all().into_iter() {
        match res {
            Ok((word, data)) => {
                words.insert(word, data);
            }
            Err(e) => {
                if !errored {
                    eprintln!("❌ Failed to parse word data: {e}");
                    errored = true;
                }
            }
        }
    }

    if words.is_empty() {
        eprintln!("❌ Error: No words loaded, likely bad dict.");
    } else {
        println!("Loaded definitions for {} words", words.len());
    }
}
