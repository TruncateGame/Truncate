use std::sync::{Mutex, MutexGuard};

use truncate_core::{
    game::Game,
    judge::{WordData, WordDict},
    messages::PlayerMessage,
    npc::scoring::BoardWeights,
};

pub static WORDNIK: &str = include_str!("../../../word_freqs/final_wordlist.txt");

static TOTAL_DICT: Mutex<Option<WordDict>> = Mutex::new(None);
static NPC_KNOWN_DICT: Mutex<Option<WordDict>> = Mutex::new(None);
static PLAYER_KNOWN_DICT: Mutex<Option<WordDict>> = Mutex::new(None);

fn ensure_dicts() {
    let mut total_dict = TOTAL_DICT.lock().unwrap();
    let mut npc_known_dict = NPC_KNOWN_DICT.lock().unwrap();
    let mut player_known_dict = PLAYER_KNOWN_DICT.lock().unwrap();

    if total_dict.is_none() {
        let mut valid_words = std::collections::HashMap::new();
        let mut npc_known_words = std::collections::HashMap::new();
        let mut player_known_words = std::collections::HashMap::new();
        let lines = WORDNIK.lines();

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
            if rel_freq > 0.95 && !objectionable {
                npc_known_words.insert(
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
                player_known_words.insert(
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
        _ = npc_known_dict.insert(npc_known_words);
        _ = player_known_dict.insert(player_known_words);
    }
}

pub fn get_main_dict() -> MutexGuard<'static, Option<WordDict>> {
    ensure_dicts();

    TOTAL_DICT.lock().unwrap()
}

pub fn best_move(game: &Game, weights: &BoardWeights) -> PlayerMessage {
    ensure_dicts();

    let npc_known_dict = NPC_KNOWN_DICT.lock().unwrap();
    let player_known_dict = PLAYER_KNOWN_DICT.lock().unwrap();

    let mut arb = truncate_core::npc::Arborist::pruning();
    arb.capped(15000);
    let search_depth = 12;

    let (best_move, score) = truncate_core::game::Game::best_move(
        game,
        npc_known_dict.as_ref(),
        player_known_dict.as_ref(),
        search_depth,
        Some(&mut arb),
        true,
        weights,
    );

    best_move
}

/// Adds the given word to the static dictionaries for the NPC
pub fn remember(word: &String) {
    ensure_dicts();

    let total_dict = TOTAL_DICT.lock().unwrap();
    let mut npc_known_dict = NPC_KNOWN_DICT.lock().unwrap();
    let mut player_known_dict = PLAYER_KNOWN_DICT.lock().unwrap();

    if let Some(word_data) = total_dict.as_ref().unwrap().get(word).cloned() {
        player_known_dict
            .as_mut()
            .unwrap()
            .insert(word.clone(), word_data.clone());

        // We don't want the NPC to learn bad words from the player
        if !word_data.objectionable {
            npc_known_dict
                .as_mut()
                .unwrap()
                .insert(word.clone(), word_data.clone());
        }
    }
}
