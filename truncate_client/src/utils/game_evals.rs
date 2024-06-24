use std::sync::{Mutex, MutexGuard};

use truncate_core::{
    game::Game,
    judge::{WordData, WordDict},
    messages::PlayerMessage,
    npc::scoring::{NPCParams, NPCVocab},
};

pub static TRUNCATE_DICT: &str = include_str!("../../final_wordlist.txt");

static TOTAL_DICT: Mutex<Option<WordDict>> = Mutex::new(None);
static SMALL_VOCAB_DICT_SAFE: Mutex<Option<WordDict>> = Mutex::new(None);
static MEDIUM_VOCAB_DICT_SAFE: Mutex<Option<WordDict>> = Mutex::new(None);
static LARGE_VOCAB_DICT_UNSAFE: Mutex<Option<WordDict>> = Mutex::new(None);

fn ensure_dicts() {
    let mut total_dict = TOTAL_DICT.lock().unwrap();
    let mut small_vocab_dict = SMALL_VOCAB_DICT_SAFE.lock().unwrap();
    let mut medium_vocab_dict = MEDIUM_VOCAB_DICT_SAFE.lock().unwrap();
    let mut large_vocab_dict = LARGE_VOCAB_DICT_UNSAFE.lock().unwrap();

    if total_dict.is_none() {
        let mut valid_words = std::collections::HashMap::new();
        let mut small_vocab_words = std::collections::HashMap::new();
        let mut medium_vocab_words = std::collections::HashMap::new();
        let mut large_vocab_words = std::collections::HashMap::new();
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

            // These are the words the NPC has recall of,
            // and will play during their turn.
            if rel_freq > 0.985 && !objectionable {
                small_vocab_words.insert(
                    word.clone(),
                    WordData {
                        extensions,
                        rel_freq,
                        objectionable,
                    },
                );
            }

            // These are the words the NPC has recall of,
            // and will play during their turn.
            if rel_freq > 0.95 && !objectionable {
                medium_vocab_words.insert(
                    word.clone(),
                    WordData {
                        extensions,
                        rel_freq,
                        objectionable,
                    },
                );
            }

            // These are the words the NPC will think it recognizes,
            // and won't challenge if they're on the board.
            if rel_freq > 0.90 {
                large_vocab_words.insert(
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
        _ = small_vocab_dict.insert(small_vocab_words);
        _ = medium_vocab_dict.insert(medium_vocab_words);
        _ = large_vocab_dict.insert(large_vocab_words);
    }
}

pub fn get_main_dict() -> MutexGuard<'static, Option<WordDict>> {
    ensure_dicts();

    TOTAL_DICT.lock().unwrap()
}

pub fn client_best_move(game: &Game, npc_params: &NPCParams) -> PlayerMessage {
    ensure_dicts();

    let npc_known_dict = match npc_params.vocab {
        NPCVocab::Medium => MEDIUM_VOCAB_DICT_SAFE.lock().unwrap(),
        NPCVocab::Small => SMALL_VOCAB_DICT_SAFE.lock().unwrap(),
    };
    let player_known_dict = LARGE_VOCAB_DICT_UNSAFE.lock().unwrap();

    let _start = instant::SystemTime::now()
        .duration_since(instant::SystemTime::UNIX_EPOCH)
        .expect("Please don't play Truncate before 1970")
        .as_millis();

    let mut arb = truncate_core::npc::Arborist::pruning();
    arb.capped(npc_params.evaluation_cap);

    let (best_move, _score) = truncate_core::game::Game::best_move(
        game,
        npc_known_dict.as_ref(),
        player_known_dict.as_ref(),
        npc_params.max_depth,
        Some(&mut arb),
        false,
        npc_params,
    );

    let _end = instant::SystemTime::now()
        .duration_since(instant::SystemTime::UNIX_EPOCH)
        .expect("Please don't play Truncate before 1970")
        .as_millis();

    best_move
}

/// Adds the given word to the static dictionaries for the NPC
pub fn remember(word: &String) {
    ensure_dicts();

    let total_dict = TOTAL_DICT.lock().unwrap();
    let mut small_dict = SMALL_VOCAB_DICT_SAFE.lock().unwrap();
    let mut medium_dict = MEDIUM_VOCAB_DICT_SAFE.lock().unwrap();
    let mut large_dict = LARGE_VOCAB_DICT_UNSAFE.lock().unwrap();

    if let Some(word_data) = total_dict.as_ref().unwrap().get(word).cloned() {
        large_dict
            .as_mut()
            .unwrap()
            .insert(word.clone(), word_data.clone());

        // We don't want the NPC to learn bad words from the player
        if !word_data.objectionable {
            medium_dict
                .as_mut()
                .unwrap()
                .insert(word.clone(), word_data.clone());
            small_dict
                .as_mut()
                .unwrap()
                .insert(word.clone(), word_data.clone());
        }
    }
}
