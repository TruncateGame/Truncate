use criterion::{criterion_group, criterion_main, Criterion};
use std::{collections::HashMap, hint::black_box};
use truncate_core::{
    bag::TileBag,
    board::Board,
    game::Game,
    judge::{WordData, WordDict},
    player::{Hand, Player},
};

pub static TESTING_DICT: &str = include_str!("../../word_freqs/final_wordlist.txt");

/// Build an (expensive) word dictionary using the real game data.
fn dict() -> WordDict {
    let mut valid_words = HashMap::new();
    let lines = TESTING_DICT.lines();

    for line in lines {
        let mut chunks = line.split(' ');
        valid_words.insert(
            chunks.next().unwrap().to_string(),
            WordData {
                extensions: chunks.next().unwrap().parse().unwrap(),
                rel_freq: chunks.next().unwrap().parse().unwrap(),
            },
        );
    }

    valid_words
}

fn test_game(board: &str, hand: &str) -> Game {
    let b = Board::from_string(board);
    let next_player = 1;
    let mut bag = TileBag::default();
    let players = vec![
        Player::new("A".into(), 0, 7, &mut bag, None, (0, 0, 0)),
        Player::new("B".into(), 1, 7, &mut bag, None, (0, 0, 0)),
    ];

    let mut game = Game {
        board: b.clone(),
        bag,
        players,
        next_player,
        ..Game::new(3, 1)
    };
    game.players[next_player].hand = Hand(hand.chars().collect());
    game.start();

    game
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let game = test_game(
        r###"
        ~~ ~~ |0 ~~ ~~
        __ S0 O0 __ __
        __ T0 __ __ __
        __ R0 __ __ Q1
        __ __ T1 __ X1
        __ __ A1 P1 T1
        E1 A1 R1 __ __
        ~~ ~~ |1 ~~ ~~
        "###,
        "SPACE",
    );
    let dict = dict();

    c.bench_function("static_board_eval", |b| {
        b.iter(|| game.static_eval(Some(&dict), 1, 1))
    });

    c.bench_function("move_finding", |b| {
        b.iter(|| Game::best_move(&game, Some(&dict), 3, None))
    });

    let small_hand_game = test_game(
        r###"
        ~~ ~~ |0 ~~ ~~
        __ S0 O0 __ __
        __ T0 __ __ __
        __ R0 __ __ Q1
        __ __ T1 __ X1
        __ __ A1 P1 T1
        E1 A1 R1 __ __
        ~~ ~~ |1 ~~ ~~
        "###,
        "****",
    );

    c.bench_function("monotile_move_finder", |b| {
        b.iter(|| Game::best_move(&small_hand_game, Some(&dict), 3, None))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
