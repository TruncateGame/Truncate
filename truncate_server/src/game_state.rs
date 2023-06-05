use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};
use truncate_core::{
    board::{Board, Coordinate},
    game::Game,
    messages::{GameMessage, GamePlayerMessage, GameStateMessage, LobbyPlayerMessage},
    moves::Move,
    reporting::Change,
};

use crate::definitions::WordDB;

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub socket: Option<SocketAddr>,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerClaims {
    pub player_index: usize,
    pub room_code: String,
}

pub struct GameState {
    pub game_id: String,
    pub players: Vec<Player>,
    pub game: Game,
}

impl GameState {
    pub fn new(game_id: String) -> Self {
        let game = Game::new(9, 11);

        Self {
            game_id,
            players: vec![],
            game,
        }
    }

    pub fn add_player(&mut self, player: Player) -> Result<usize, ()> {
        if self.game.started_at.is_some() {
            return Err(()); // TODO: Error types
        }
        // TODO: Check player #
        self.game.add_player(player.name.clone());
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
        if let Some(player) = self.players.iter_mut().find(|p| p.socket == Some(socket)) {
            player.name = name;
            Ok(())
        } else {
            println!("Couldn't rename player. Nothing stored for player {socket}");
            Err(())
        }
    }

    pub fn player_list(&self) -> Vec<LobbyPlayerMessage> {
        self.game
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
        self.game.board = board;
    }

    pub fn game_msg(
        &self,
        player_index: usize,
        word_map: Option<&MutexGuard<'_, WordDB>>,
    ) -> GameStateMessage {
        let (board, mut changes) = self.game.filter_game_to_player(player_index);

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
                    if let Some(meanings) = definitions.get_word(&word.word.to_lowercase()) {
                        word.meanings = Some(meanings.clone());
                    }
                }

                for word in &mut battle
                    .defenders
                    .iter_mut()
                    .filter(|w| w.valid == Some(true))
                {
                    if let Some(meanings) = definitions.get_word(&word.word.to_lowercase()) {
                        word.meanings = Some(meanings.clone());
                    }
                }
            }
        }

        let hand = self
            .game
            .get_player(player_index)
            .expect("Player should have been dealt a hand")
            .hand
            .clone();

        GameStateMessage {
            room_code: self.game_id.clone(),
            players: self.game.players.iter().map(Into::into).collect(),
            player_number: player_index as u64,
            next_player_number: self.game.next() as u64,
            board,
            hand,
            changes,
        }
    }

    pub async fn start(&mut self) -> Vec<(&Player, GameMessage)> {
        // TODO: Check correct # of players

        // Trim off all edges and add one back for our land edges to show in the gui
        self.game.board.trim();

        self.game.start();
        let mut messages = Vec::with_capacity(self.players.len());

        // TODO: Maintain an index of Player to the Game player index
        // For cases where players reconnect and game.hands[0] is players[1] etc
        for (player_index, player) in self.players.iter().enumerate() {
            messages.push((
                player.clone(),
                GameMessage::StartedGame(self.game_msg(player_index, None)),
            ));
        }

        messages
    }

    pub async fn play(
        &mut self,
        player: SocketAddr,
        position: Coordinate,
        tile: char,
        words: Arc<Mutex<WordDB>>,
    ) -> Vec<(&Player, GameMessage)> {
        let mut messages = Vec::with_capacity(self.players.len());

        if let Some((player_index, _)) = self
            .players
            .iter()
            .enumerate()
            .find(|(_, p)| p.socket == Some(player))
        {
            let words_db = words.lock().await;
            match self.game.play_turn(
                Move::Place {
                    player: player_index,
                    tile,
                    position,
                },
                Some(&words_db.valid_words),
            ) {
                Ok(Some(winner)) => {
                    for (player_index, player) in self.players.iter().enumerate() {
                        messages.push((
                            player.clone(),
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
                            player.clone(),
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
    pub async fn swap(
        &mut self,
        player: SocketAddr,
        from: Coordinate,
        to: Coordinate,
        words: Arc<Mutex<WordDB>>,
    ) -> Vec<(&Player, GameMessage)> {
        let mut messages = Vec::with_capacity(self.players.len());

        if let Some((player_index, _)) = self
            .players
            .iter()
            .enumerate()
            .find(|(_, p)| p.socket == Some(player))
        {
            let words_db = words.lock().await;
            match self.game.play_turn(
                Move::Swap {
                    player: player_index,
                    positions: [from, to],
                },
                Some(&words_db.valid_words),
            ) {
                Ok(Some(_)) => {
                    unreachable!("Cannot win by swapping")
                }
                Ok(None) => {
                    for (player_index, player) in self.players.iter().enumerate() {
                        messages.push((
                            player.clone(),
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
