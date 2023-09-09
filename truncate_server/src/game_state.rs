use parking_lot::{Mutex, MutexGuard};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use truncate_core::{
    board::{Board, Coordinate},
    game::Game,
    messages::{GameMessage, GameStateMessage, LobbyPlayerMessage},
    moves::Move,
    reporting::Change,
};

use crate::definitions::WordDB;

#[derive(Debug, Clone)]
pub struct Player {
    pub socket: Option<SocketAddr>,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerClaims {
    pub player_index: usize,
    pub room_code: String,
}

pub struct GameManager {
    pub game_id: String,
    pub players: Vec<Player>,
    pub core_game: Game,
}

impl GameManager {
    pub fn new(game_id: String) -> Self {
        let game = Game::new(9, 11);

        Self {
            game_id,
            players: vec![],
            core_game: game,
        }
    }

    pub fn get_player_index(&self, player_addr: SocketAddr) -> Option<usize> {
        if let Some((player_index, _)) = self
            .players
            .iter()
            .enumerate()
            .find(|(_, p)| p.socket == Some(player_addr))
        {
            Some(player_index)
        } else {
            None
        }
    }

    pub fn add_player(&mut self, player: Player, name: String) -> Result<usize, ()> {
        if self.core_game.started_at.is_some() {
            return Err(()); // TODO: Error types
        }
        // TODO: Check player #
        self.core_game.add_player(name);
        self.players.push(player);
        Ok(self.players.len() - 1)
    }

    pub fn reconnect_player(&mut self, socket: SocketAddr, index: usize) -> Result<(), ()> {
        match self.players.get_mut(index) {
            Some(existing_player) => {
                existing_player.socket = Some(socket);
                Ok(())
            }
            None => {
                println!("Couldn't reconnect player. Nothing stored for player {index}");
                Err(())
            }
        }
    }

    pub fn rename_player(&mut self, socket: SocketAddr, name: String) -> Result<(), ()> {
        if let Some(player_index) = self.get_player_index(socket) {
            self.core_game.players[player_index].name = name;
            Ok(())
        } else {
            println!("Couldn't rename player. Nothing stored for player {socket}");
            Err(())
        }
    }

    pub fn player_list(&self) -> Vec<LobbyPlayerMessage> {
        self.core_game
            .players
            .iter()
            .map(|p| LobbyPlayerMessage {
                name: p.name.clone(),
                index: p.index,
                color: p.color,
            })
            .collect()
    }

    pub fn edit_board(&mut self, board: Board) {
        self.core_game.board = board;
    }

    pub fn game_msg(
        &self,
        player_index: usize,
        word_map: Option<&MutexGuard<'_, WordDB>>,
    ) -> GameStateMessage {
        let (board, mut changes) = self.core_game.filter_game_to_player(player_index);

        if let Some(definitions) = word_map {
            for battle in changes.iter_mut().filter_map(|change| match change {
                Change::Battle(battle) => Some(battle),
                _ => None,
            }) {
                for word in &mut battle
                    .attackers
                    .iter_mut()
                    .filter(|w| w.valid == Some(true))
                {
                    if let Some(meanings) = definitions.get_word(&word.resolved_word.to_lowercase())
                    {
                        word.meanings = Some(meanings.clone());
                    }
                }

                for word in &mut battle
                    .defenders
                    .iter_mut()
                    .filter(|w| w.valid == Some(true))
                {
                    if let Some(meanings) = definitions.get_word(&word.resolved_word.to_lowercase())
                    {
                        word.meanings = Some(meanings.clone());
                    }
                }
            }
        }

        let hand = self
            .core_game
            .get_player(player_index)
            .expect("Player should have been dealt a hand")
            .hand
            .clone();

        GameStateMessage {
            room_code: self.game_id.clone(),
            players: self.core_game.players.iter().map(Into::into).collect(),
            player_number: player_index as u64,
            next_player_number: self.core_game.next() as u64,
            board,
            hand,
            changes,
        }
    }

    pub fn start(&mut self) -> Vec<(&Player, GameMessage)> {
        // TODO: Check correct # of players

        // Trim off all edges and add one back for our land edges to show in the gui
        self.core_game.board.trim();

        self.core_game.start();
        let mut messages = Vec::with_capacity(self.players.len());

        // TODO: Maintain an index of Player to the Game player index
        // For cases where players reconnect and game.hands[0] is players[1] etc
        for (player_index, player) in self.players.iter().enumerate() {
            messages.push((
                player,
                GameMessage::StartedGame(self.game_msg(player_index, None)),
            ));
        }

        messages
    }

    pub fn play(
        &mut self,
        player: SocketAddr,
        position: Coordinate,
        tile: char,
        words: Arc<Mutex<WordDB>>,
    ) -> Vec<(&Player, GameMessage)> {
        let mut messages = Vec::with_capacity(self.players.len());

        if let Some(player_index) = self.get_player_index(player) {
            let words_db = words.lock();
            match self.core_game.play_turn(
                Move::Place {
                    player: player_index,
                    tile,
                    position,
                },
                Some(&words_db.valid_words),
                Some(&words_db.valid_words),
            ) {
                Ok(Some(winner)) => {
                    for (player_index, player) in self.players.iter().enumerate() {
                        messages.push((
                            player,
                            GameMessage::GameEnd(
                                self.game_msg(player_index, Some(&words_db)),
                                winner as u64,
                            ),
                        ));
                    }
                    return messages;
                }
                Ok(None) => {
                    for (player_index, player) in self.players.iter().enumerate() {
                        messages.push((
                            player,
                            GameMessage::GameUpdate(self.game_msg(player_index, Some(&words_db))),
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
        words: Arc<Mutex<WordDB>>,
    ) -> Vec<(&Player, GameMessage)> {
        let mut messages = Vec::with_capacity(self.players.len());

        if let Some(player_index) = self.get_player_index(player) {
            let words_db = words.lock();
            match self.core_game.play_turn(
                Move::Swap {
                    player: player_index,
                    positions: [from, to],
                },
                Some(&words_db.valid_words),
                Some(&words_db.valid_words),
            ) {
                Ok(Some(_)) => {
                    unreachable!("Cannot win by swapping")
                }
                Ok(None) => {
                    for (player_index, player) in self.players.iter().enumerate() {
                        messages.push((
                            player,
                            GameMessage::GameUpdate(self.game_msg(player_index, None)),
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
