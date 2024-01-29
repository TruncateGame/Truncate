use std::hash::Hash;

use chrono::{offset, Offset};
use dicts::{get_dicts, Dicts};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use storage::{load_file, write_file, SeedNote};
use truncate_core::{
    game::Game,
    generation::{generate_board, get_game_verification, BoardSeed},
    messages::PlayerMessage,
    moves::Move,
    npc::scoring::BoardWeights,
};

use crate::dicts::{ensure_dicts, RESTRICTED_DICT, TOTAL_DICT};

mod dicts;
mod storage;

fn best_move(game: &Game, weights: &BoardWeights, dicts: &Dicts) -> PlayerMessage {
    ensure_dicts();

    let mut arb = truncate_core::npc::Arborist::pruning();
    arb.capped(15000);
    let search_depth = 12;

    let (best_move, score) = truncate_core::game::Game::best_move(
        game,
        Some(&dicts.restricted),
        Some(&dicts.restricted),
        search_depth,
        Some(&mut arb),
        false,
        weights,
    );

    best_move
}

fn evaluate_single_seed(seed: BoardSeed, log: bool) -> Option<SeedNote> {
    let maximum_turns = 200;

    let mut game = get_game_for_seed(seed.clone());

    let verification = get_game_verification(&game);
    let weights = BoardWeights::default();
    let mut dicts = get_dicts();

    while game.turn_count < maximum_turns {
        let best_move_for_next_player = best_move(&game, &weights, &dicts);
        let next_player = game.next_player;

        let next_move = match best_move_for_next_player {
            PlayerMessage::Place(position, tile) => Move::Place {
                player: next_player,
                tile,
                position,
            },
            PlayerMessage::Swap(from, to) => Move::Swap {
                player: next_player,
                positions: [from, to],
            },
            _ => unreachable!(),
        };

        let pre_board = game.board.to_string();
        let pre_tiles = game.players[next_player].hand.clone();

        match game.play_turn(
            next_move.clone(),
            Some(&dicts.total),
            Some(&dicts.total),
            None,
        ) {
            Ok(Some(winner)) => {
                if log {
                    println!("\nWINNING BOARD:\n{}", game.board);
                }
                return Some(SeedNote {
                    rerolls: 0,
                    best_player: winner,
                    verification,
                });
            }
            Ok(None) => {
                let post_board = game.board.to_string();

                let zipped_board = pre_board
                    .lines()
                    .zip(post_board.lines())
                    .map(|(a, b)| format!("{a}   >   {b}"))
                    .collect::<Vec<_>>()
                    .join("\n");

                if log {
                    println!("\nPlayer {next_player} had tiles {pre_tiles:?}\nPicked {next_move:?}:\n{zipped_board}");
                }

                // NPC learns words as a result of battles that reveal validity
                for battle in game
                    .recent_changes
                    .iter()
                    .filter_map(|change| match change {
                        truncate_core::reporting::Change::Battle(battle) => Some(battle),
                        _ => None,
                    })
                {
                    for word in battle.attackers.iter().chain(battle.defenders.iter()) {
                        if word.valid == Some(true) {
                            let dict_word = word.original_word.to_lowercase();

                            dicts.remember(&dict_word);
                        }
                    }
                }
            }
            Err(e) => {
                panic!("Errored on seed {seed:?}:\n{e}");
            }
        }
    }

    None
}

fn get_game_for_seed(seed: BoardSeed) -> Game {
    let mut board = generate_board(seed.clone())
        .expect("Generation should be possible from this seed")
        .board;
    board.cache_special_squares();

    let mut game = Game::new(9, 9, Some(seed.seed as u64));
    game.add_player("P1".into());
    game.add_player("P2".into());

    game.board = board.clone();
    game.rules.battle_delay = 0;
    game.start();

    game
}

fn evaluate_seed(mut seed: BoardSeed, log: bool) -> (u32, SeedNote) {
    let mut rerolls = 0;
    let mut seed_result = None;
    let core_seed = seed.seed;

    while seed_result.is_none() {
        seed_result = evaluate_single_seed(seed.clone(), log);
        if seed_result.is_none() {
            rerolls += 1;
            seed.external_reroll();
        }
    }

    println!("Evaluated notes for {core_seed} with {rerolls} reroll(s)");

    let mut seed_notes = seed_result.unwrap();
    seed_notes.rerolls = rerolls;
    (core_seed, seed_notes)
}

fn verify_note(seed: &u32, note: &SeedNote) {
    let mut board_seed = BoardSeed::new(*seed);
    for _ in 0..(note.rerolls) {
        board_seed.external_reroll();
    }

    let game = get_game_for_seed(board_seed);
    let verification = get_game_verification(&game);

    if verification != note.verification {
        panic!("Failed verification for {seed}");
    } else {
        println!("Seed {seed} was verified ({} rerolls)", note.rerolls);
    }
}

fn main() {
    let quantity = 50;

    let mut current_notes = load_file();
    ensure_dicts();

    current_notes.notes.iter().for_each(|(seed, note)| {
        verify_note(seed, note);
    });

    let args = std::env::args().collect::<Vec<_>>();

    if let Some(seed) = args.get(1) {
        let seed = BoardSeed::new(seed.parse().expect("Seed should be a number"));
        let result = evaluate_seed(seed, true);
        println!("{result:#?}");
        return;
    };

    let mut starting_day = 0;
    while current_notes.notes.contains_key(&starting_day) {
        starting_day += 1;
    }

    let results: Vec<_> = (0..quantity)
        .into_par_iter()
        .map(|offset| {
            let seed = BoardSeed::new(starting_day + offset);
            evaluate_seed(seed, false)
        })
        .collect();

    for (seed, notes) in results {
        current_notes.notes.insert(seed, notes);
    }

    write_file(current_notes);
}
