mod bag;
mod board;
mod game;
mod hand;
mod judge;
mod moves;

use board::Coordinate;
use game::Game;
use moves::Move;
use std::io;

// TODO: this whole file is absolute garbage, clean it up

fn main() {
    // let (mut game, players) = setup();
    let (mut game, players) = test_setup();
    loop {
        // pre_turn(&game, &players);
        match turn(&mut game) {
            Some(winner) => {
                clear();
                render_board(&game);
                println!();
                println!("Congratulations! {} wins!", players[winner]);
                return;
            }
            None => {
                clear();
            }
        }
    }
}

fn turn(game: &mut Game) -> Option<usize> {
    render_board(&game);
    println!();
    println!();
    render_hand(&game);
    place(game)
}

fn render_board(game: &Game) {
    let flipped = game.next() == 0;
    let mut orientations = game.board.get_orientations();
    let flipped_orientations = &orientations
        .iter()
        .map(|o| o.opposite())
        .collect::<Vec<board::Direction>>();
    if flipped {
        orientations = flipped_orientations;
    }

    let mut board = game.board.render_squares(
        |sq| sq.to_oriented_string(orientations),
        |line_number, line| {
            let mut line = line;
            let f: String = line.chars().rev().collect();
            let flipped_line = &f;
            if flipped {
                line = flipped_line;
            }
            let mut s = char::from_u32((line_number + 65) as u32)
                .unwrap()
                .to_string();
            s.push_str("  ");
            s.push_str(&line.clone());
            s
        },
    );
    // TODO: this is an absurd way of reversing something when we have rev()
    let split = board.split('\n').collect::<Vec<&str>>();
    let mut rev: Vec<String> = Vec::new();
    for x in split {
        rev.insert(0, String::from(x));
    }
    let rev = rev.join("\n");
    if flipped {
        board = rev
    }
    println!("{}", board);
    println!();
    let mut numbers = (0..game.board.width())
        .map(|n| {
            let mut s = (n + 1).to_string();
            if s.len() == 1 {
                if flipped {
                    s.insert(0, ' ');
                } else {
                    s.push(' ');
                }
            }
            s
        })
        .collect::<Vec<String>>()
        .join("");
    if flipped {
        numbers = numbers.chars().rev().collect();
    }
    println!("   {}", numbers);
}

fn render_hand(game: &Game) {
    println!(
        "{}",
        game.hands
            .get_hand(game.next())
            .clone()
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<String>>()
            .join(" ")
    );
}

fn place(game: &mut Game) -> Option<usize> {
    loop {
        let mut tile: Option<char> = None;
        while tile.is_none() {
            let input = user_input("Which tile would you like to place?");
            if input.len() == 1 {
                tile = Some(input.chars().next().unwrap());
            } else {
                println!("Sorry, I couldn't read that letter");
            }
        }

        let mut position: Option<Coordinate> = None;
        while position.is_none() {
            let input = user_input("Where would you like to place your tile? I.e., A1");
            if input.len() == 2 {
                let mut chars = input.chars();
                let y = chars.next().unwrap() as isize - 65;
                let x = chars.next().unwrap() as isize - 49; // 48 is the character 0, and our board is 1 indexed on scren
                position = Some(Coordinate { x, y });
            } else {
                println!("Sorry, I couldn't read that coordinate");
            }
        }

        match game.play_move(Move::Place {
            player: game.next(),
            position: position.unwrap(),
            tile: tile.unwrap(),
        }) {
            Err(e) => {
                println!("{}", e)
            }
            Ok(winner) => return winner,
        }
    }
}

fn pre_turn(game: &Game, players: &Vec<String>) {
    render_board(game);
    println!();
    println!();

    println!(
        "It's {}'s turn",
        players
            .get(game.next())
            .expect("next should only ever be 0 or 1 and there should be 2 players") // TODO: generalise to multiple players
    );
    println!(
        "Look away from the screen, {}",
        players
            .get((game.next() + 1) % 2)
            .expect("next should only ever be 0 or 1 and there should be 2 players")
    );
    user_input("Press any key to see your hand");
    clear();
}

fn test_setup() -> (Game, Vec<String>) {
    (
        Game::new(3, 3),
        vec![String::from("Blake"), String::from("Liam")],
    )
}

fn setup() -> (Game, Vec<String>) {
    clear();
    // Get player names
    let player_zero = user_input("Player 1:");
    let player_one = user_input("Player 2:");
    let players = vec![player_zero, player_one];

    // Build board
    let width = user_input_usize("Board width:");
    let height = user_input_usize("Board height");

    let game = Game::new(width, height);

    println!();
    user_input("Setup complete! Press any key to start game");
    clear();
    (game, players)
}

// Utilities
fn user_input(prompt: &str) -> String {
    println!("{}", prompt); // TODO: don't print line necessarily
    let mut capture_string = String::new();
    io::stdin()
        .read_line(&mut capture_string)
        .expect("Failed to read line");
    println!();
    capture_string = capture_string.trim().to_string();
    capture_string
}

fn user_input_usize(prompt: &str) -> usize {
    let mut result: Option<usize> = None;
    while result.is_none() {
        let input_str = user_input(prompt);
        let parsed: usize = match input_str.parse() {
            Ok(num) => num,
            Err(_) => {
                println!("Couldn't read number, please try again");
                continue;
            }
        };
        result = Some(parsed);
    }
    result.expect("Should unwrap because the above loop can only end when result is Ok")
}

fn clear() {
    clearscreen::clear().expect("failed to clear screen"); // TODO: use sub terminals like `git log` etc, rather than actually clearing the user's screen
}
