use std::collections::BTreeMap;

use chrono::Offset;
use eframe::egui;
use epaint::TextureHandle;
use instant::Duration;
use serde::{Deserialize, Serialize};
use truncate_core::{
    board::Square,
    game::Game,
    generation::{generate_board, get_game_verification, BoardSeed},
    moves::Move,
    reporting::{BoardChange, BoardChangeAction, BoardChangeDetail},
};

use crate::{
    app_outer::Backchannel,
    regions::{active_game::HeaderType, single_player::SinglePlayerState},
};

use super::{game_evals::get_main_dict, Theme};

const SEED_NOTES: &[u8] = include_bytes!("../../../truncate_dueller/seed_notes.yml");
// January 29, 2023
pub const DAILY_PUZZLE_DAY_ZERO: usize = 19751;

/**
 * TODO: Store NotesFile and SeedNote type definitions in a common crate
 */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedNote {
    pub rerolls: usize,
    pub best_player: usize,
    pub verification: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotesFile {
    pub notes: BTreeMap<u32, SeedNote>,
}

pub fn get_puzzle_day(current_time: Duration) -> u32 {
    let seconds_offset = chrono::Local::now().offset().fix().local_minus_utc();
    let local_seconds = current_time.as_secs() as i32 + seconds_offset;
    let seed = (local_seconds / (60 * 60 * 24)) as u32;
    let day = (seed - DAILY_PUZZLE_DAY_ZERO as u32);

    day
}

pub type HumanStarts = bool;
pub fn get_raw_daily_puzzle(day: u32) -> (BoardSeed, Option<(HumanStarts, SeedNote)>) {
    let loaded_notes: NotesFile =
        serde_yaml::from_slice(SEED_NOTES).expect("Seed notes should match the spec");

    let mut board_seed = BoardSeed::new(day).day(day);

    let mut info = None;

    let notes = loaded_notes.notes.get(&day);
    if let Some(notes) = notes {
        for _ in 0..notes.rerolls {
            board_seed.external_reroll();
        }
        info = Some((notes.best_player == 0, notes.clone()));
    }

    (board_seed, info)
}

pub fn get_playable_daily_puzzle(
    ctx: &egui::Context,
    day: u32,
    map_texture: &TextureHandle,
    theme: &Theme,
    backchannel: &Backchannel,
) -> SinglePlayerState {
    let (board_seed, info) = get_raw_daily_puzzle(day);

    let header_title = format!("Truncate Town Day #{day}");
    let mut header_sentinel = if info.is_some() { '★' } else { '*' };
    let human_starts = info.as_ref().map(|(h, _)| *h).unwrap_or(true);

    let board = generate_board(board_seed.clone())
        .expect("Common seeds should always generate a board")
        .board;
    let mut game_state = SinglePlayerState::new(
        ctx,
        map_texture.clone(),
        theme.clone(),
        board,
        Some(board_seed.clone()),
        human_starts,
        HeaderType::None, // Replaced soon with HeaderType::Summary
    );

    if let Some((_, notes)) = info {
        let verification = get_game_verification(&game_state.game);
        if verification != notes.verification {
            header_sentinel = '¤';
        }
    }

    game_state.header = HeaderType::Summary {
        title: header_title,
        sentinel: header_sentinel,
        attempt: Some(0),
    };

    game_state
}
