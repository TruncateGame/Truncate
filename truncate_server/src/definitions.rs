use dashmap::DashMap;
use serde::Deserialize;
use std::collections::HashMap;

const DEFS_A: &[u8] = include_bytes!("./word_definitions/a.json");
const DEFS_B: &[u8] = include_bytes!("./word_definitions/b.json");
const DEFS_C: &[u8] = include_bytes!("./word_definitions/c.json");
const DEFS_D: &[u8] = include_bytes!("./word_definitions/d.json");
const DEFS_E: &[u8] = include_bytes!("./word_definitions/e.json");
const DEFS_F: &[u8] = include_bytes!("./word_definitions/f.json");
const DEFS_G: &[u8] = include_bytes!("./word_definitions/g.json");
const DEFS_H: &[u8] = include_bytes!("./word_definitions/h.json");
const DEFS_I: &[u8] = include_bytes!("./word_definitions/i.json");
const DEFS_J: &[u8] = include_bytes!("./word_definitions/j.json");
const DEFS_K: &[u8] = include_bytes!("./word_definitions/k.json");
const DEFS_L: &[u8] = include_bytes!("./word_definitions/l.json");
const DEFS_M: &[u8] = include_bytes!("./word_definitions/m.json");
const DEFS_N: &[u8] = include_bytes!("./word_definitions/n.json");
const DEFS_O: &[u8] = include_bytes!("./word_definitions/o.json");
const DEFS_P: &[u8] = include_bytes!("./word_definitions/p.json");
const DEFS_Q: &[u8] = include_bytes!("./word_definitions/q.json");
const DEFS_R: &[u8] = include_bytes!("./word_definitions/r.json");
const DEFS_S: &[u8] = include_bytes!("./word_definitions/s.json");
const DEFS_T: &[u8] = include_bytes!("./word_definitions/t.json");
const DEFS_U: &[u8] = include_bytes!("./word_definitions/u.json");
const DEFS_V: &[u8] = include_bytes!("./word_definitions/v.json");
const DEFS_W: &[u8] = include_bytes!("./word_definitions/w.json");
const DEFS_X: &[u8] = include_bytes!("./word_definitions/x.json");
const DEFS_Y: &[u8] = include_bytes!("./word_definitions/y.json");
const DEFS_Z: &[u8] = include_bytes!("./word_definitions/z.json");

#[derive(Deserialize)]
pub struct Word {
    pub word: String,
    pub meanings: Option<Vec<WordDefinition>>,
}

#[derive(Deserialize, Clone)]
pub struct WordDefinition {
    pub def: String,
    pub example: Option<String>,
    pub speech_part: Option<String>,
}

pub fn definitions() -> DashMap<String, Word> {
    println!("Loading word definitions...");

    let mut words = DashMap::new();

    for defs in vec![
        DEFS_A, DEFS_B, DEFS_C, DEFS_D, DEFS_E, DEFS_F, DEFS_G, DEFS_H, DEFS_I, DEFS_J, DEFS_K,
        DEFS_L, DEFS_M, DEFS_N, DEFS_O, DEFS_P, DEFS_Q, DEFS_R, DEFS_S, DEFS_T, DEFS_U, DEFS_V,
        DEFS_W, DEFS_X, DEFS_Y, DEFS_Z,
    ] {
        match serde_json::from_slice::<HashMap<String, Word>>(defs) {
            Ok(word_chunk) => {
                words.extend(word_chunk.into_iter());
            }
            Err(e) => {
                eprintln!("Failed to decode word definitions: {e}");
            }
        }
    }

    words
}
