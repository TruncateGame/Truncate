use core::{game::Game, messages::PlayerMessage};
use std::{net::SocketAddr, time::Instant};

use tokio::sync::mpsc::UnboundedReceiver;

pub struct Player {
    pub name: String,
    pub socket: Option<SocketAddr>,
}

pub struct GameState {
    pub started_at: Instant,
    pub players: Vec<Player>,
    pub game: Game,
}

pub fn run_game(mut rx: UnboundedReceiver<PlayerMessage>, player: Player) {
    let game = GameState {
        started_at: Instant::now(),
        players: vec![player],
        game: Game::default(),
    };

    // TODO: Update the core game's RNG to be Send so that this fn can be
    // async in tokio's world, rather than this persistent thread with a block.
    // (maybe... this has some virtues too, in which case maybe change to poll_recv)
    while let Some(msg) = rx.blocking_recv() {
        match msg {
            PlayerMessage::Place(_, _) => todo!(),
            // TODO: List outer game messages here exhaustively so new messages throw compilation errors
            _ => {}
        }
        println!("Game got {msg:#?}");
    }
}
