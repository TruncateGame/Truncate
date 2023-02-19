use core::{
    board::Coordinate,
    game::Game,
    messages::{GameMessage, GameStateMessage},
    moves::Move,
};
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
        for (number, player) in self.players.iter().enumerate() {
            messages.push((
                player.clone(),
                GameMessage::StartedGame(GameStateMessage {
                    room_code: self.game_id.clone(),
                    player_number: number as u64,
                    board: self.game.board.clone(),
                    hand: hands.next().cloned().unwrap(),
                }),
            ));
        }

        messages
    }

    pub fn play(
        &mut self,
        player: SocketAddr,
        position: Coordinate,
        tile: char,
    ) -> Vec<(&Player, GameMessage)> {
        let mut messages = Vec::with_capacity(self.players.len());

        if let Some((player_index, _)) = self
            .players
            .iter()
            .enumerate()
            .find(|(_, p)| p.socket == Some(player))
        {
            match self.game.play_move(Move::Place {
                player: player_index,
                tile,
                position,
            }) {
                Ok(Some(winner)) => {
                    for (number, player) in self.players.iter().enumerate() {
                        messages.push((
                            player.clone(),
                            GameMessage::GameEnd(
                                GameStateMessage {
                                    room_code: self.game_id.clone(),
                                    player_number: number as u64,
                                    board: self.game.board.clone(),
                                    hand: vec![],
                                },
                                winner as u64,
                            ),
                        ));
                    }
                    return messages;
                }
                Ok(None) => {}
                Err(msg) => {
                    return vec![(
                        &self.players[player_index],
                        GameMessage::GameError(
                            self.game_id.clone(),
                            player_index as u64,
                            msg.into(),
                        ),
                    )]
                }
            }
        } else {
            todo!("Handle missing player");
        }

        // TODO: Tidy
        let mut hands = (0..self.players.len()).map(|player| {
            self.game
                .hands
                .get_hand(player)
                .expect("Player was not dealt a hand")
        });
        for (number, player) in self.players.iter().enumerate() {
            messages.push((
                player.clone(),
                GameMessage::GameUpdate(GameStateMessage {
                    room_code: self.game_id.clone(),
                    player_number: number as u64,
                    board: self.game.board.clone(),
                    hand: hands.next().cloned().unwrap(),
                }),
            ));
        }

        messages
    }
}
