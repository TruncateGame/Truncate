use std::collections::BTreeMap;

use chrono::Offset;
use epaint::TextureHandle;
use instant::Duration;
use serde::{Deserialize, Serialize};
use truncate_core::{
    generation::{generate_board, get_game_verification, BoardSeed},
    moves::Move,
};

use crate::{
    app_outer::Backchannel,
    regions::{active_game::HeaderType, single_player::SinglePlayerState},
};

use super::Theme;

const SEED_NOTES: &[u8] = include_bytes!("../../../truncate_dueller/seed_notes.yml");

/**
 * TODO: Store NotesFile and SeedNote type definitions in a common crate
 */

#[derive(Debug, Serialize, Deserialize)]
pub struct SeedNote {
    pub rerolls: usize,
    pub best_player: usize,
    pub verification: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotesFile {
    pub notes: BTreeMap<u32, SeedNote>,
}

pub fn get_daily_puzzle(
    current_time: Duration,
    map_texture: &TextureHandle,
    theme: &Theme,
    backchannel: &Backchannel,
) -> SinglePlayerState {
    let loaded_notes: NotesFile =
        serde_yaml::from_slice(SEED_NOTES).expect("Seed notes should match the spec");

    let seconds_offset = chrono::Local::now().offset().fix().local_minus_utc();
    let local_seconds = current_time.as_secs() as i32 + seconds_offset;
    let seed = (local_seconds / (60 * 60 * 24)) as u32;
    let day = seed - 19673; // Nov 13, 2023
    let mut board_seed = BoardSeed::new(seed).day(day);
    let mut header = HeaderType::Summary {
        title: format!("Truncate Town Day #{day}"),
        sentinel: '*',
    };
    let mut human_starts = true;

    let notes = loaded_notes.notes.get(&seed);
    if let Some(notes) = notes {
        human_starts = notes.best_player == 0;
        header = HeaderType::Summary {
            title: format!("Truncate Town Day #{day}"),
            sentinel: '★',
        };
        for _ in 0..notes.rerolls {
            board_seed.external_reroll();
        }
    }

    let board = generate_board(board_seed.clone());
    let mut game_state = SinglePlayerState::new(
        map_texture.clone(),
        theme.clone(),
        board,
        Some(board_seed.clone()),
        human_starts,
        header.clone(),
    );

    if let Some(notes) = notes {
        let verification = get_game_verification(&game_state.game);
        if verification != notes.verification {
            game_state.active_game.ctx.header_visible = HeaderType::Summary {
                title: format!("Truncate Town Day #{day}"),
                sentinel: '¤',
            };
        }
    }

    let persisted_moves = get_persistent_game(&board_seed);

    let delay = game_state.game.rules.battle_delay;
    game_state.game.rules.battle_delay = 0;
    for next_move in persisted_moves.moves.into_iter() {
        if game_state.handle_move(next_move, backchannel).is_err() {
            wipe_persistent_game(&board_seed);
            return get_daily_puzzle(current_time, map_texture, theme, backchannel);
        }
    }
    game_state.game.rules.battle_delay = delay;

    game_state
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PersistentGame {
    pub moves: Vec<Move>,
}

pub fn persist_game(seed: &BoardSeed, action: Move) {
    #[cfg(target_arch = "wasm32")]
    {
        let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();

        let key = format!("daily_{}", seed.seed);

        let Ok(record) = local_storage.get_item(&key) else {
            eprintln!("Localstorage was inaccessible");
            return;
        };

        let mut current_game: PersistentGame = record
            .map(|stored| serde_json::from_str(&stored).unwrap_or_default())
            .unwrap_or_default();

        current_game.moves.push(action);

        local_storage
            .set_item(
                &key,
                &serde_json::to_string(&current_game).expect("Our game should be serializable"),
            )
            .unwrap();
    }
}

pub fn get_persistent_game(seed: &BoardSeed) -> PersistentGame {
    #[cfg(target_arch = "wasm32")]
    {
        let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();

        let key = format!("daily_{}", seed.seed);

        let Ok(record) = local_storage.get_item(&key) else {
            eprintln!("Localstorage was inaccessible");
            return PersistentGame::default();
        };

        let current_game: PersistentGame = record
            .map(|stored| serde_json::from_str(&stored).unwrap_or_default())
            .unwrap_or_default();

        current_game
    }
}

pub fn wipe_persistent_game(seed: &BoardSeed) {
    #[cfg(target_arch = "wasm32")]
    {
        let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();

        let key = format!("daily_{}", seed.seed);

        local_storage.remove_item(&key);
    }
}

pub fn record_result() {}
