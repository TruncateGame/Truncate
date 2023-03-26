use std::net::SocketAddr;
use truncate_core::{
    board::{Board, Coordinate},
    game::Game,
    messages::{GameMessage, GamePlayerMessage, GameStateMessage},
    moves::Move,
    player::Hand,
    reporting::Change,
};

use crate::definitions::Definitions;

async fn hydrate_change_definitions(definitions: &Definitions, changes: &mut Vec<Change>) {
    for battle in changes.iter_mut().filter_map(|change| match change {
        Change::Battle(battle) => Some(battle),
        _ => None,
    }) {
        println!("Evaluating battle {battle:#?}");
        for word in &mut battle
            .attackers
            .iter_mut()
            .filter(|w| w.valid == Some(true))
        {
            println!("Hydrating word {word:#?}");
            word.definition = definitions.get_word(&word.word).await;
        }
        for word in &mut battle
            .defenders
            .iter_mut()
            .filter(|w| w.valid == Some(true))
        {
            println!("Hydrating word {word:#?}");
            word.definition = definitions.get_word(&word.word).await;
        }
    }
}

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub socket: Option<SocketAddr>,
}

pub struct GameState {
    pub game_id: String,
    pub players: Vec<Player>,
    pub game: Game,
}

impl GameState {
    pub fn new(game_id: String) -> Self {
        let game = Game::new(9, 9);

        Self {
            game_id,
            players: vec![],
            game,
        }
    }

    pub fn add_player(&mut self, player: Player) -> Result<(), ()> {
        if self.game.started_at.is_some() {
            return Err(()); // TODO: Error types
        }
        // TODO: Check player #
        self.game.add_player(player.name.clone());
        self.players.push(player);
        Ok(())
    }

    pub fn edit_board(&mut self, board: Board) {
        self.game.board = board;
    }

    pub fn start(&mut self) -> Vec<(&Player, GameMessage)> {
        // TODO: Check correct # of players
        self.game.board.trim();
        self.game.start();
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
        for (player_index, player) in self.players.iter().enumerate() {
            let (board, changes) = self.game.filter_game_to_player(player_index);
            messages.push((
                player.clone(),
                GameMessage::StartedGame(GameStateMessage {
                    room_code: self.game_id.clone(),
                    players: self
                        .game
                        .players
                        .iter()
                        .map(|p| GamePlayerMessage {
                            name: p.name.clone(),
                            index: p.index,
                            allotted_time: p.allotted_time,
                            time_remaining: p.time_remaining,
                            turn_starts_at: p.turn_starts_at,
                        })
                        .collect(),
                    player_number: player_index as u64,
                    next_player_number: self.game.next() as u64,
                    board,
                    hand: hands.next().unwrap(),
                    changes,
                }),
            ));
        }

        messages
    }

    pub async fn play(
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
            match self.game.play_turn(Move::Place {
                player: player_index,
                tile,
                position,
            }) {
                Ok(Some(winner)) => {
                    // TODO: Provide a way for the player to request a definition for a specific word,
                    // rather than requesting them all every time.
                    // hydrate_change_definitions(&Definitions::new(), &mut changes).await;
                    for (player_index, player) in self.players.iter().enumerate() {
                        let (board, changes) = self.game.filter_game_to_player(player_index);

                        messages.push((
                            player.clone(),
                            GameMessage::GameEnd(
                                GameStateMessage {
                                    room_code: self.game_id.clone(),
                                    players: self
                                        .game
                                        .players
                                        .iter()
                                        .map(|p| GamePlayerMessage {
                                            name: p.name.clone(),
                                            index: p.index,
                                            allotted_time: p.allotted_time,
                                            time_remaining: p.time_remaining,
                                            turn_starts_at: p.turn_starts_at,
                                        })
                                        .collect(),
                                    player_number: player_index as u64,
                                    next_player_number: self.game.next() as u64,
                                    board,
                                    hand: Hand(vec![]),
                                    changes,
                                },
                                winner as u64,
                            ),
                        ));
                    }
                    return messages;
                }
                Ok(None) => {
                    // TODO: Provide a way for the player to request a definition for a specific word,
                    // rather than requesting them all every time.
                    // hydrate_change_definitions(&Definitions::new(), &mut changes).await;
                    // TODO: Tidy
                    let mut hands = (0..self.players.len()).map(|player| {
                        self.game
                            .get_player(player)
                            .expect("Player was not dealt a hand")
                            .hand
                            .clone()
                    });
                    for (player_index, player) in self.players.iter().enumerate() {
                        let (board, changes) = self.game.filter_game_to_player(player_index);

                        messages.push((
                            player.clone(),
                            GameMessage::GameUpdate(GameStateMessage {
                                room_code: self.game_id.clone(),
                                players: self
                                    .game
                                    .players
                                    .iter()
                                    .map(|p| GamePlayerMessage {
                                        name: p.name.clone(),
                                        index: p.index,
                                        allotted_time: p.allotted_time,
                                        time_remaining: p.time_remaining,
                                        turn_starts_at: p.turn_starts_at,
                                    })
                                    .collect(),
                                player_number: player_index as u64,
                                next_player_number: self.game.next() as u64,
                                board,
                                hand: hands.next().unwrap(),
                                changes,
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
            match self.game.play_turn(Move::Swap {
                player: player_index,
                positions: [from, to],
            }) {
                Ok(Some(_)) => {
                    unreachable!("Cannot win by swapping")
                }
                Ok(None) => {
                    // TODO: Tidy
                    let mut hands = (0..self.players.len()).map(|player| {
                        self.game
                            .get_player(player)
                            .expect("Player was not dealt a hand")
                            .hand
                            .clone()
                    });
                    for (player_index, player) in self.players.iter().enumerate() {
                        let (board, changes) = self.game.filter_game_to_player(player_index);

                        messages.push((
                            player.clone(),
                            GameMessage::GameUpdate(GameStateMessage {
                                room_code: self.game_id.clone(),
                                players: self
                                    .game
                                    .players
                                    .iter()
                                    .map(|p| GamePlayerMessage {
                                        name: p.name.clone(),
                                        index: p.index,
                                        allotted_time: p.allotted_time,
                                        time_remaining: p.time_remaining,
                                        turn_starts_at: p.turn_starts_at,
                                    })
                                    .collect(),
                                player_number: player_index as u64,
                                next_player_number: self.game.next() as u64,
                                board,
                                hand: hands.next().unwrap(),
                                changes,
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
