use std::collections::{HashMap, HashSet};

use eframe::egui;
use epaint::{vec2, TextureHandle};
use serde::Deserialize;
use truncate_core::{
    bag::TileBag,
    board::{Board, Coordinate},
    game::Game,
    judge::Judge,
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage},
    moves::Move,
    player::{Hand, Player},
    rules::{GameRules, TileDistribution},
};

use crate::utils::Theme;

use super::active_game::ActiveGame;

pub static WORDNIK: &str = include_str!("../../../truncate_server/wordnik_wordlist.txt");

pub struct SinglePlayerState {
    game: Game,
    active_game: ActiveGame,
    dict: HashSet<String>,
}

impl SinglePlayerState {
    pub fn new(map_texture: TextureHandle, theme: Theme) -> Self {
        let mut game = Game::new(9, 9);
        game.add_player("You".into());
        game.add_player("Computer".into());
        game.start();

        let active_game = ActiveGame::new(
            "TUTORIAL_01".into(),
            game.players.iter().map(Into::into).collect(),
            0,
            0,
            game.board.clone(),
            game.players[0].hand.clone(),
            map_texture,
            theme,
        );

        let mut valid_words = HashSet::new();
        let mut lines = WORDNIK.lines();
        lines.next(); // Skip copyright

        for line in lines {
            valid_words.insert(line.to_string());
        }

        Self {
            game,
            active_game,
            dict: valid_words,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        let mut next_msg = None;

        // Standard game helper
        next_msg = self.active_game.render(ui, theme, None).map(|msg| (0, msg));

        if self.game.next_player != 0 {
            println!(
                "Before turn, computer's hand is: {:?}",
                self.game.players[1].hand.0
            );
            next_msg = Some((1, Game::brute_force(&self.game, Some(&self.dict))));
        }

        let next_move = match next_msg {
            Some((player, PlayerMessage::Place(position, tile))) => Some(Move::Place {
                player,
                tile,
                position,
            }),
            Some((player, PlayerMessage::Swap(from, to))) => Some(Move::Swap {
                player,
                positions: [from, to],
            }),
            _ => None,
        };

        if let Some(next_move) = next_move {
            match self.game.play_turn(next_move, Some(&self.dict)) {
                Ok(None) => {
                    let changes = self
                        .game
                        .recent_changes
                        .clone()
                        .into_iter()
                        .filter(|change| match change {
                            truncate_core::reporting::Change::Board(_) => true,
                            truncate_core::reporting::Change::Hand(hand_change) => {
                                hand_change.player == 0
                            }
                            truncate_core::reporting::Change::Battle(_) => true,
                            truncate_core::reporting::Change::Time(_) => true,
                        })
                        .collect();
                    let ctx = &self.active_game.ctx;
                    let state_message = GameStateMessage {
                        room_code: ctx.room_code.clone(),
                        players: self.game.players.iter().map(Into::into).collect(),
                        player_number: 0,
                        next_player_number: self.game.next_player as u64,
                        board: self.game.board.clone(),
                        hand: self.game.players[0].hand.clone(),
                        changes,
                    };
                    self.active_game.apply_new_state(state_message);
                }
                Ok(Some(winner)) => panic!("Hooray!"),
                Err(msg) => eprintln!("Failed: {msg}"),
            }
        }
    }
}
