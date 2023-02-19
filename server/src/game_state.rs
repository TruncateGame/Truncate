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
        self.game.add_player(player.name.clone());
        self.players.push(player);
        Ok(())
    }

    pub fn start(&mut self) -> Vec<(&Player, GameMessage)> {
        // TODO: Check correct # of players
        self.started_at = Some(Instant::now());
        let mut messages = Vec::with_capacity(self.players.len());

        let mut hands = (0..self.players.len()).map(|player| {
            self.game
                .get_player(player)
                .expect("Player was not dealt a hand")
                .hand
                .clone()
        });

        // TODO: Maintain an index of Player to the Game player index
        // For cases where players reconnect and game.hands[0] is players[1] etc
        for (number, player) in self.players.iter().enumerate() {
            messages.push((
                player.clone(),
                GameMessage::StartedGame(GameStateMessage {
                    room_code: self.game_id.clone(),
                    player_number: number as u64,
                    next_player_number: self.game.next() as u64,
                    board: self.game.board.clone(),
                    hand: hands.next().unwrap(),
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
                                    next_player_number: self.game.next() as u64,
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
                .get_player(player)
                .expect("Player was not dealt a hand")
                .hand
                .clone()
        });
        for (number, player) in self.players.iter().enumerate() {
            messages.push((
                player.clone(),
                GameMessage::GameUpdate(GameStateMessage {
                    room_code: self.game_id.clone(),
                    player_number: number as u64,
                    next_player_number: self.game.next() as u64,
                    board: self.game.board.clone(),
                    hand: hands.next().unwrap(),
                }),
            ));
        }

        messages
    }

    // TODO: Combine method with play and pass in a `Move` type
    // (need to solve the player lookup first)
    pub fn swap(
        &mut self,
        player: SocketAddr,
        from: Coordinate,
        to: Coordinate,
    ) -> Vec<(&Player, GameMessage)> {
        let mut messages = Vec::with_capacity(self.players.len());

        if let Some((player_index, _)) = self
            .players
            .iter()
            .enumerate()
            .find(|(_, p)| p.socket == Some(player))
        {
            match self.game.play_move(Move::Swap {
                player: player_index,
                positions: [from, to],
            }) {
                Ok(Some(_)) => {
                    unreachable!("Cannot win by swapping")
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
                .get_player(player)
                .expect("Player was not dealt a hand")
                .hand
                .clone()
        });
        for (number, player) in self.players.iter().enumerate() {
            messages.push((
                player.clone(),
                GameMessage::GameUpdate(GameStateMessage {
                    room_code: self.game_id.clone(),
                    player_number: number as u64,
                    next_player_number: self.game.next() as u64,
                    board: self.game.board.clone(),
                    hand: hands.next().unwrap(),
                }),
            ));
        }

        messages
    }
}
