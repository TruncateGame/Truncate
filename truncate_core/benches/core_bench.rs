use criterion::{criterion_group, criterion_main, Criterion};
use std::{
    collections::{HashMap, HashSet},
    fmt::format,
    hint::black_box,
};
use truncate_core::{
    bag::TileBag,
    board::{Board, Coordinate},
    game::Game,
    judge::{Judge, WordData, WordDict},
    npc::{scoring::NPCParams, Caches},
    player::{Hand, Player},
    rules,
};
use xxhash_rust::xxh3;

pub static TESTING_DICT: &str = include_str!("../../dict_builder/final_wordlist.txt");

/// Build an (expensive) word dictionary using the real game data.
fn dict() -> WordDict {
    let mut valid_words = HashMap::new();
    let lines = TESTING_DICT.lines();

    for line in lines {
        let mut chunks = line.split(' ');

        let mut word = chunks.next().unwrap().to_string();
        let objectionable = word.chars().next() == Some('*');
        if objectionable {
            word.remove(0);
        }

        valid_words.insert(
            word,
            WordData {
                extensions: chunks.next().unwrap().parse().unwrap(),
                rel_freq: chunks.next().unwrap().parse().unwrap(),
                objectionable,
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
        ..Game::new(3, 1, None)
    };
    game.players[next_player].hand = Hand(hand.chars().collect());
    game.start();

    game
}

pub fn npc_benches(c: &mut Criterion) {
    let game = test_game(
        // A board with some complexity,
        // and that has #1 towns that can be attacked
        r###"
        ~~ ~~ |0 ~~ ~~ ~~ ~~
        #0 #0 O0 #0 #0 #0 #0
        __ S0 O0 A0 A0 D0 __
        __ T0 __ __ __ I0 __
        __ __ __ __ __ A0 __
        __ __ __ __ E1 __ __
        __ __ T1 __ A1 __ __
        __ __ A1 P1 T1 __ __
        E1 A1 R1 __ __ __ __
        #1 #1 E1 #1 #1 #1 #1
        ~~ ~~ |1 ~~ ~~ ~~ ~~
        "###,
        "SPACERX",
    );
    let dict = dict();

    c.bench_function("total_board_eval", |b| {
        b.iter(|| game.static_eval(Some(&dict), 1, 1, &mut Caches::new(), &NPCParams::default()))
    });

    c.bench_function("quality_eval", |b| {
        b.iter(|| game.eval_word_quality(&dict, 1, &mut Caches::new()))
    });

    c.bench_function("defense_eval", |b| {
        b.iter(|| {
            let dists = game.board.flood_fill_attacks(0);
            game.eval_min_distance_to_towns(
                &dists,
                1,
                truncate_core::npc::DefenceEvalType::Attackable,
            )
        })
    });

    c.bench_function("move_finding", |b| {
        b.iter(|| {
            Game::best_move(
                &game,
                Some(&dict),
                Some(&dict),
                4,
                None,
                false,
                &NPCParams::default(),
            )
        })
    });

    c.bench_function("deeper_move_finding", |b| {
        b.iter(|| {
            Game::best_move(
                &game,
                Some(&dict),
                Some(&dict),
                6,
                None,
                false,
                &NPCParams::default(),
            )
        })
    });

    let open_game = test_game(
        r###"
        ~~ ~~ |0 ~~ ~~
        #0 #0 __ #0 #0
        __ __ __ __ __
        __ __ __ __ __
        __ __ __ __ __
        __ __ __ __ __
        #1 #1 __ #1 #1
        ~~ ~~ |1 ~~ ~~
        "###,
        "SPACERX",
    );

    c.bench_function("open_board_moves", |b| {
        b.iter(|| {
            Game::best_move(
                &open_game,
                Some(&dict),
                Some(&dict),
                10,
                None,
                false,
                &NPCParams::default(),
            )
        })
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
        b.iter(|| {
            Game::best_move(
                &small_hand_game,
                Some(&dict),
                Some(&dict),
                3,
                None,
                false,
                &NPCParams::default(),
            )
        })
    });
}

pub fn board_benches(c: &mut Criterion) {
    let board = Board::from_string(
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
    );

    c.bench_function("board_dfs", |b| {
        b.iter(|| board.depth_first_search(Coordinate { x: 2, y: 6 }))
    });

    c.bench_function("flood_fill_attacks", |b| {
        b.iter(|| board.flood_fill_attacks(0))
    });

    c.bench_function("get_shape", |b| b.iter(|| board.get_shape()));

    c.bench_function("get_word_coordinates", |b| {
        b.iter(|| board.get_words(Coordinate { x: 2, y: 5 }))
    });

    let coords = board.get_words(Coordinate { x: 2, y: 5 });
    c.bench_function("get_word_strings", |b| {
        b.iter(|| board.word_strings(&coords))
    });
}

pub fn judge_benches(c: &mut Criterion) {
    let dict = dict();
    let mut judge = Judge::new(vec![]);
    let alias = judge.set_alias("xvzaro".chars().collect());

    let aliased_judge_word = format!("P{alias}RTITI{alias}N");
    let win_condition = rules::WinCondition::Destination {
        town_defense: rules::TownDefense::BeatenWithDefenseStrength(0),
    };

    c.bench_function("judge_with_double_alias", |b| {
        b.iter(|| {
            judge.valid(
                &aliased_judge_word,
                &win_condition,
                Some(&dict),
                None,
                &mut None,
            )
        })
    });

    let wildcard_judge_word = format!("PAR*ITION");
    c.bench_function("judge_with_wildcard", |b| {
        b.iter(|| {
            judge.valid(
                &wildcard_judge_word,
                &win_condition,
                Some(&dict),
                None,
                &mut None,
            )
        })
    });
}

criterion_group!(benches, npc_benches, board_benches, judge_benches);
criterion_main!(benches);
