use core::{
    board::Coordinate,
    game::Game,
    messages::{GameMessage, GameStateMessage},
    moves::Move,
    reporting::{Change, HandChange},
};
use std::{net::SocketAddr, time::Instant};

fn filter_changes_for_player(changes: &Vec<Change>, player: usize) -> Vec<Change> {
    changes
        .iter()
        .filter(|change| match change {
            Change::Hand(HandChange {
                player: changed_player,
                removed: _,
                added: _,
            }) => *changed_player == player,
            Change::Board(_) => true,
        })
        .cloned()
        .collect::<Vec<_>>()
}

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
                    changes: vec![],
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
                Ok((changes, Some(winner))) => {
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
                                    changes: filter_changes_for_player(&changes, number),
                                },
                                winner as u64,
                            ),
                        ));
                    }
                    return messages;
                }
                Ok((changes, None)) => {
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
                                changes: filter_changes_for_player(&changes, number),
                            }),
                        ));
                    }
                    return messages;
                }
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
                Ok((_, Some(_))) => {
                    unreachable!("Cannot win by swapping")
                }
                Ok((changes, None)) => {
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
                                changes: filter_changes_for_player(&changes, number),
                            }),
                        ));
                    }

                    messages
                }
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
    }
}
