use std::collections::BTreeMap;

use chrono::Offset;
use epaint::TextureHandle;
use instant::Duration;
use serde::{Deserialize, Serialize};
use truncate_core::generation::{generate_board, get_game_verification, BoardSeed};

use crate::regions::{active_game::HeaderType, single_player::SinglePlayerState};

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

    let board = generate_board(board_seed.clone())
        .expect("Daily puzzle seeds should always generate a board")
        .board;
    let game_state = SinglePlayerState::new(
        map_texture.clone(),
        theme.clone(),
        board,
        Some(board_seed),
        human_starts,
        header,
    );

    if let Some(notes) = notes {
        let verification = get_game_verification(&game_state.game);
        if verification != notes.verification {
            header = HeaderType::Summary {
                title: format!("Truncate Town Day #{day}"),
                sentinel: '¤',
            };
        }
    }

    game_state
}
