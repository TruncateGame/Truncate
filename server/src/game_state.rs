use core::{game::Game, messages::GameMessage};
use std::{net::SocketAddr, time::Instant};

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub socket: Option<SocketAddr>,
}

pub struct GameState {
    pub game_id: String,
    pub started_at: Option<Instant>,
    pub players: Vec<Player>,
    pub game: Game,
}

impl GameState {
    pub fn new(game_id: String) -> Self {
        Self {
            game_id,
            started_at: None,
            players: vec![],
            game: Game::default(),
        }
    }

    pub fn add_player(&mut self, player: Player) -> Result<(), ()> {
        if self.started_at.is_some() {
            return Err(()); // TODO: Error types
        }
        // TODO: Check player #
        self.players.push(player);
        self.game.hands.add_player();
        Ok(())
    }

    pub fn start(&mut self) -> Vec<(&Player, GameMessage)> {
        // TODO: Check correct # of players
        self.started_at = Some(Instant::now());
        let mut messages = Vec::with_capacity(self.players.len());

        let mut hands = (0..self.players.len()).map(|player| {
            self.game
                .hands
                .get_hand(player)
                .expect("Player was not dealt a hand")
        });
        // TODO: Maintain an index of Player to the Game player index
        // For cases where players reconnect and game.hands[0] is players[1] etc
        for player in self.players.iter() {
            messages.push((
                player.clone(),
                GameMessage::StartedGame(
                    self.game_id.clone(),
                    self.game.board.clone(),
                    hands.next().cloned().unwrap(),
                ),
            ));
        }

        messages
    }
}
