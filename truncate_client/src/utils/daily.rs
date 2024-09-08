use std::collections::BTreeMap;

use chrono::Offset;
use eframe::egui;
use epaint::TextureHandle;
use serde::{Deserialize, Serialize};
use time::Duration;
use truncate_core::{
    generation::{generate_board, get_game_verification, BoardSeed},
    npc::scoring::NPCPersonality,
    rules::GameRules,
};

use crate::{
    app_outer::{Backchannel, EventDispatcher},
    regions::{active_game::HeaderType, single_player::SinglePlayerState},
};

use super::Theme;

const SEED_NOTES: &[u8] = include_bytes!("../../seed_notes.yml");
// January 29, 2023
pub const DAILY_PUZZLE_DAY_ZERO: usize = 19751;

/**
 * TODO: Store NotesFile and SeedNote type definitions in a common crate
 */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedNote {
    pub rerolls: usize,
    pub best_player: usize,
    pub board_generation: u32,
    pub rules_generation: u32,
    pub verification: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotesFile {
    pub notes: BTreeMap<u32, SeedNote>,
}

pub fn get_puzzle_day(current_time: Duration) -> u32 {
    let seconds_offset = chrono::Local::now().offset().fix().local_minus_utc();
    let local_seconds = current_time.whole_seconds() as i32 + seconds_offset;
    let seed = (local_seconds / (60 * 60 * 24)) as u32;
    let day = seed - DAILY_PUZZLE_DAY_ZERO as u32;

    day
}

pub type HumanStarts = bool;
pub fn get_raw_daily_puzzle(day: u32) -> (BoardSeed, Option<(HumanStarts, SeedNote)>) {
    let loaded_notes: NotesFile =
        serde_yaml::from_slice(SEED_NOTES).expect("Seed notes should match the spec");

    let notes = loaded_notes.notes.get(&day);

    if let Some(notes) = notes {
        let mut board_seed = BoardSeed::new_with_generation(notes.board_generation, day).day(day);

        for _ in 0..notes.rerolls {
            board_seed.external_reroll();
        }
        let info = Some((notes.best_player == 0, notes.clone()));

        (board_seed, info)
    } else {
        let board_seed = BoardSeed::new(day).day(day);

        (board_seed, None)
    }
}

pub fn get_playable_daily_puzzle(
    ctx: &egui::Context,
    day: u32,
    map_texture: &TextureHandle,
    theme: &Theme,
    _backchannel: &Backchannel,
    event_dispatcher: EventDispatcher,
) -> SinglePlayerState {
    let (board_seed, info) = get_raw_daily_puzzle(day);

    let mut header_sentinel = if info.is_some() { '#' } else { '?' };
    let human_starts = info.as_ref().map(|(h, _)| *h).unwrap_or(true);

    let board = generate_board(board_seed.clone())
        .expect("Common seeds should always generate a board")
        .board;

    let rules_generation = info
        .as_ref()
        .map(|(_, note)| note.rules_generation)
        .unwrap_or_else(|| GameRules::latest().0);

    let mut game_state = SinglePlayerState::new(
        "daily".to_string(),
        ctx,
        map_texture.clone(),
        theme.clone(),
        board,
        Some(board_seed.clone()),
        rules_generation,
        human_starts,
        HeaderType::None, // Replaced soon with HeaderType::Summary
        NPCPersonality::jet(),
        event_dispatcher,
    );

    if let Some((_, notes)) = info {
        let verification = get_game_verification(&game_state.game);
        if verification != notes.verification {
            header_sentinel = '!';
        }
    }

    game_state.header = HeaderType::Summary {
        title: format!("Truncate Town Day {header_sentinel}{day}"),
        attempt: Some(0),
    };

    game_state
}
