use core::{
    game::Game,
    messages::{GameMessage, PlayerMessage},
};
use std::{net::SocketAddr, time::Instant};

use tokio::sync::mpsc::UnboundedReceiver;

use crate::PeerMap;

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub socket: Option<SocketAddr>,
}

pub struct GameState {
    pub started_at: Instant,
    pub players: Vec<Player>,
    pub game: Game,
}

#[derive(Debug)]
pub enum IncomingMessage {
    AddPlayer(Player),
    Instruction(PlayerMessage),
}

pub fn run_game(
    game_id: String,
    mut rx: UnboundedReceiver<IncomingMessage>,
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
        use IncomingMessage::*;
        use PlayerMessage::*;
        match &msg {
            AddPlayer(player) => {
                println!("Adding {player:?} to game {game_id}");
            }
            Instruction(Place(_, _)) => todo!(),
            Instruction(StartGame) => {
                let mut hands = (0..game.players.len()).map(|player| {
                    game.game
                        .hands
                        .get_hand(player)
                        .expect("Player was not dealt a hand")
                });
                // TODO: Maintain an index of Player to the Game player index
                // For cases where players reconnect and game.hands[0] is players[1] etc
                for player in game.players.iter() {
                    let Some(socket) = player.socket.as_ref() else {continue};
                    let Some(peer) = peer_map.get(socket) else {continue};

                    peer.send(GameMessage::StartedGame(
                        game_id.clone(),
                        game.game.board.clone(),
                        hands.next().cloned().unwrap(),
                    ))
                    .unwrap();
                }
            }
            // Outer-game messages should not have been passed through
            Instruction(NewGame) | Instruction(JoinGame(_)) => unreachable!(),
        }
        println!("Game got {msg:#?}");
    }
}
