use std::collections::HashMap;

use eframe::egui;
use epaint::TextureHandle;
use instant::Duration;
use truncate_core::{
    game::Game,
    judge::{WordData, WordDict},
    messages::{GameStateMessage, PlayerMessage},
    moves::Move,
};

use crate::utils::Theme;

use super::active_game::ActiveGame;

pub static WORDNIK: &str = include_str!("../../../word_freqs/final_wordlist.txt");

pub struct SinglePlayerState {
    game: Game,
    active_game: ActiveGame,
    dict: WordDict,
    next_response_at: Option<Duration>,
    winner: Option<usize>,
}

impl SinglePlayerState {
    pub fn new(map_texture: TextureHandle, theme: Theme) -> Self {
        let mut game = Game::new(9, 9);
        game.add_player("You".into());
        game.add_player("Computer".into());
        game.start();

        let active_game = ActiveGame::new(
            "SINGLE_PLAYER".into(),
            game.players.iter().map(Into::into).collect(),
            0,
            0,
            game.board.clone(),
            game.players[0].hand.clone(),
            map_texture,
            theme,
        );

        let mut valid_words = HashMap::new();
        let lines = WORDNIK.lines();

        for line in lines {
            let mut chunks = line.split(' ');
            valid_words.insert(
                chunks.next().unwrap().to_string(),
                WordData {
                    extensions: chunks.next().unwrap().parse().unwrap(),
                    rel_freq: chunks.next().unwrap().parse().unwrap(),
                },
            );
        }

        Self {
            game,
            active_game,
            dict: valid_words,
            next_response_at: None,
            winner: None,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        // Standard game helper
        let mut next_msg = self
            .active_game
            .render(ui, theme, self.winner)
            .map(|msg| (0, msg));

        if self.winner.is_some() {
            return;
        }

        if let Some(next_response_at) = self.next_response_at {
            if next_response_at > self.active_game.ctx.current_time {
                return;
            }
        }
        self.next_response_at = None;

        if self.game.next_player != 0 {
            next_msg = Some((1, Game::brute_force(&self.game, Some(&self.dict))));
        }

        // A version of the above that includes a turn delay.
        // TODO: Store time that the turn changes and use that instead, since we only have whole seconds here.
        // TODO: Maybe only delay when attacks happened?
        // if self.game.next_player != 0 {
        //     let current_time = self.active_game.ctx.current_time.as_secs();
        //     let turn_starts = self.game.players[self.game.next_player]
        //         .turn_starts_at
        //         .unwrap_or_default();

        //     if current_time - turn_starts > 0 {
        //         next_msg = Some((1, Game::brute_force(&self.game, Some(&self.dict))));
        //     }
        // }

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
                Ok(winner) => {
                    self.winner = winner;

                    let changes: Vec<_> = self
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

                    let has_battles = changes.iter().any(|change| {
                        matches!(change, truncate_core::reporting::Change::Battle(_))
                    });

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

                    let delay = if has_battles { 1200 } else { 200 };

                    self.next_response_at = Some(
                        self.active_game
                            .ctx
                            .current_time
                            .saturating_add(Duration::from_millis(delay)),
                    );
                    ui.ctx()
                        .request_repaint_after(Duration::from_millis(delay / 2));
                }
                Err(msg) => eprintln!("Failed: {msg}"),
            }
        }
    }
}
