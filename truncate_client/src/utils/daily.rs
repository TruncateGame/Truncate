use std::collections::BTreeMap;

use chrono::Offset;
use epaint::TextureHandle;
use instant::Duration;
use serde::{Deserialize, Serialize};
use truncate_core::{
    board::Square,
    game::Game,
    generation::{generate_board, get_game_verification, BoardSeed},
    moves::Move,
    player,
    reporting::{BoardChange, BoardChangeAction, BoardChangeDetail},
};

use crate::{
    app_outer::Backchannel,
    regions::{
        active_game::{ActiveGame, HeaderType},
        single_player::SinglePlayerState,
    },
};

use super::{game_evals::get_main_dict, Theme};

const SEED_NOTES: &[u8] = include_bytes!("../../../truncate_dueller/seed_notes.yml");
// Nov 13, 2023
const DAILY_PUZZLE_DAY_ZERO: usize = 19673;

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
    let day = seed - DAILY_PUZZLE_DAY_ZERO as u32;
    let mut board_seed = BoardSeed::new(seed).day(day);
    let persisted_moves = get_persistent_game(&board_seed);
    let mut header = HeaderType::Summary {
        title: format!("Truncate Town Day #{day}"),
        sentinel: '*',
        attempt: Some(persisted_moves.attempts),
    };
    let mut human_starts = true;

    let notes = loaded_notes.notes.get(&seed);
    if let Some(notes) = notes {
        human_starts = notes.best_player == 0;
        header = HeaderType::Summary {
            title: format!("Truncate Town Day #{day}"),
            sentinel: '★',
            attempt: Some(persisted_moves.attempts),
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
                attempt: Some(persisted_moves.attempts),
            };
        }
    }

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
    pub won: bool,
    pub attempts: usize,
    pub moves: Vec<Move>,
}

pub fn persist_game_retry(seed: &BoardSeed) {
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

        current_game.attempts += 1;
        current_game.moves = vec![];

        local_storage
            .set_item(
                &key,
                &serde_json::to_string(&current_game).expect("Our game should be serializable"),
            )
            .unwrap();
    }
}

pub fn persist_game_win(seed: &BoardSeed) {
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

        current_game.won = true;

        local_storage
            .set_item(
                &key,
                &serde_json::to_string(&current_game).expect("Our game should be serializable"),
            )
            .unwrap();
    }
}

pub fn persist_game_move(seed: &BoardSeed, action: Move) {
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

        return current_game;
    }
    PersistentGame::default()
}

pub fn wipe_persistent_game(seed: &BoardSeed) {
    #[cfg(target_arch = "wasm32")]
    {
        let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();

        let key = format!("daily_{}", seed.seed);

        local_storage.remove_item(&key);
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DailyAttempt {
    pub moves: u32,
    pub battles: u32,
    pub largest_attack_destruction: u32,
    pub longest_word: Option<String>,
    pub won: bool,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DailyResult {
    pub attempts: Vec<DailyAttempt>,
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct DailyStats {
    pub days: BTreeMap<u32, DailyResult>,
}

pub fn persist_stats(seed: &BoardSeed, game: &Game, human_player: usize, attempt: usize) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(relative_day) = seed.day else {
            return;
        };
        let storage_key = "daily_puzzle_stats";

        let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
        let Ok(stored_stats) = local_storage.get_item(storage_key) else {
            eprintln!("Localstorage was inaccessible");
            return;
        };
        let mut stats: DailyStats = stored_stats
            .map(|stored| serde_json::from_str(&stored).unwrap_or_default())
            .unwrap_or_default();

        let today = stats.days.entry(relative_day).or_default();

        while today.attempts.get(attempt).is_none() {
            today.attempts.push(DailyAttempt::default());
        }

        let this_attempt = today.attempts.get_mut(attempt).unwrap();

        this_attempt.moves = game.player_turn_count[human_player];
        this_attempt.won = game.winner == Some(human_player);
        this_attempt.battles = game.battle_count;

        let recent_attack_destruction = game
            .recent_changes
            .iter()
            .filter(|change| {
                use truncate_core::reporting::Change::Board;
                match change {
                    Board(BoardChange {
                        detail:
                            BoardChangeDetail {
                                square: Square::Occupied(player, _),
                                ..
                            },
                        action: BoardChangeAction::Defeated | BoardChangeAction::Truncated,
                    }) if *player != human_player => true,
                    _ => false,
                }
            })
            .count();

        this_attempt.largest_attack_destruction = this_attempt
            .largest_attack_destruction
            .max(recent_attack_destruction as u32);

        let new_player_tile = game.recent_changes.iter().find_map(|change| {
            use truncate_core::reporting::Change::Board;
            match change {
                Board(BoardChange {
                    detail:
                        BoardChangeDetail {
                            square: Square::Occupied(player, _),
                            coordinate,
                        },
                    action: BoardChangeAction::Added,
                }) if *player == human_player => Some(coordinate),
                _ => None,
            }
        });
        if let Some(new_player_tile) = new_player_tile {
            let word_coords = game.board.get_words(*new_player_tile);
            let dict_lock = get_main_dict();
            let dict = dict_lock.as_ref().unwrap();
            if let Ok(words) = game.board.word_strings(&word_coords) {
                words
                    .into_iter()
                    .filter(|word| {
                        game.judge
                            .valid(
                                &word,
                                &game.rules.win_condition,
                                Some(dict),
                                None,
                                &mut None,
                            )
                            .is_some()
                    })
                    .for_each(|word| {
                        let prev_longest = this_attempt
                            .longest_word
                            .get_or_insert_with(|| word.clone());
                        if word.len() > prev_longest.len() {
                            *prev_longest = word;
                        }
                    });
            }
        }

        local_storage
            .set_item(
                storage_key,
                &serde_json::to_string(&stats).expect("Our stats should be serializable"),
            )
            .unwrap();
    }
}
