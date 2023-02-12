use core::{
    game::Game,
    messages::{GameMessage, PlayerMessage},
};
use std::{net::SocketAddr, time::Instant};

use tokio::sync::mpsc::UnboundedReceiver;
use uuid::Uuid;

use crate::PeerMap;

pub struct Player {
    pub name: String,
    pub socket: Option<SocketAddr>,
}

pub struct GameState {
    pub started_at: Instant,
    pub players: Vec<Player>,
    pub game: Game,
}

pub fn run_game(
    game_id: Uuid,
    mut rx: UnboundedReceiver<PlayerMessage>,
    peer_map: PeerMap,
    player: Player,
) {
    let game = GameState {
        started_at: Instant::now(),
        players: vec![player],
        game: Game::default(),
    };

    // TODO: Update the core game's RNG to be Send so that this fn can be
    // async in tokio's world, rather than this persistent thread with a block.
    // (maybe... this has some virtues too, in which case maybe change to poll_recv)
    while let Some(msg) = rx.blocking_recv() {
        use PlayerMessage::*;
        match msg {
            Place(_, _) => todo!(),
            StartGame => {
                for player in game.players.iter() {
                    let Some(socket) = player.socket.as_ref() else {continue};
                    let Some(peer) = peer_map.get(socket) else {continue};

                    peer.send(GameMessage::StartedGame(
                        game_id,
                        game.game.board.clone(),
                        game.game.hands.get_hand(0).clone(),
                    ))
                    .unwrap();
                }
            }
            NewGame => {}
        }
        println!("Game got {msg:#?}");
    }
}
