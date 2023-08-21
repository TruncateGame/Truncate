use std::collections::HashMap;

use eframe::egui;
use epaint::TextureHandle;
use instant::Duration;
use truncate_core::{
    game::Game,
    judge::{WordData, WordDict},
    messages::{GameStateMessage, PlayerMessage},
    moves::Move,
    reporting::WordMeaning,
};

use crate::utils::Theme;

use super::active_game::ActiveGame;

pub static WORDNIK: &str = include_str!("../../../word_freqs/final_wordlist.txt");

pub struct SinglePlayerState {
    game: Game,
    pub active_game: ActiveGame,
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

    /// If the server sent through some new word definitions,
    /// dig deep and update all past battles to reference the definitions
    pub fn hydrate_meanings(&mut self, definitions: Vec<(String, Option<Vec<WordMeaning>>)>) {
        self.active_game
            .turn_reports
            .iter_mut()
            .flat_map(|t| t.iter_mut())
            .filter_map(|change| {
                if let truncate_core::reporting::Change::Battle(battle) = change {
                    Some(battle)
                } else {
                    None
                }
            })
            .flat_map(|b| b.attackers.iter_mut().chain(b.defenders.iter_mut()))
            .for_each(|battle_word| {
                if battle_word.meanings.is_none() {
                    for (word, meanings) in &definitions {
                        if battle_word.resolved_word.to_lowercase() == word.to_lowercase() {
                            battle_word.meanings = meanings.clone();
                        }
                    }
                }
            });
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        current_time: Duration,
    ) -> Option<PlayerMessage> {
        let mut msg_to_server = None;

        // Standard game helper
        let mut next_msg = self
            .active_game
            .render(ui, theme, self.winner, current_time)
            .map(|msg| (0, msg));

        if self.winner.is_some() {
            return msg_to_server;
        }

        if let Some(next_response_at) = self.next_response_at {
            if next_response_at > self.active_game.ctx.current_time {
                return msg_to_server;
            }
        }
        self.next_response_at = None;

        if self.game.next_player != 0 {
            if let Some(turn_starts_at) = self
                .game
                .get_player(self.game.next_player)
                .unwrap()
                .turn_starts_at
            {
                if turn_starts_at <= current_time.as_secs() {
                    // let start = time::Instant::now();
                    let mut arb = truncate_core::npc::Arborist::pruning();
                    next_msg = Some((
                        1,
                        Game::best_move(&self.game, Some(&self.dict), 3, Some(&mut arb)),
                    ));
                    // println!(
                    //     "Looked at {} leaves in {}ms",
                    //     arb.assessed(),
                    //     start.elapsed().whole_milliseconds()
                    // );
                }
            }
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

                    let battle_words: Vec<_> = changes
                        .iter()
                        .filter_map(|change| {
                            if let truncate_core::reporting::Change::Battle(battle) = change {
                                Some(battle)
                            } else {
                                None
                            }
                        })
                        .flat_map(|b| b.attackers.iter().chain(b.defenders.iter()))
                        .map(|b| b.resolved_word.clone())
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

                    let delay = if battle_words.is_empty() { 200 } else { 1200 };

                    if !battle_words.is_empty() {
                        msg_to_server = Some(PlayerMessage::RequestDefinitions(battle_words));
                    }

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

        msg_to_server
    }
}
