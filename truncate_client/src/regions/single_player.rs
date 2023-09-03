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
    npc_known_dict: WordDict,
    next_response_at: Option<Duration>,
    winner: Option<usize>,
    map_texture: TextureHandle,
    theme: Theme,
    turns: usize,
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
            map_texture.clone(),
            theme.clone(),
        );

        let mut valid_words = HashMap::new();
        let mut npc_known_words = HashMap::new();
        let lines = WORDNIK.lines();

        for line in lines {
            let mut chunks = line.split(' ');

            let word = chunks.next().unwrap().to_string();
            let extensions = chunks.next().unwrap().parse().unwrap();
            let rel_freq = chunks.next().unwrap().parse().unwrap();

            valid_words.insert(
                word.clone(),
                WordData {
                    extensions,
                    rel_freq,
                },
            );

            if rel_freq > 0.95 {
                npc_known_words.insert(
                    word,
                    WordData {
                        extensions,
                        rel_freq,
                    },
                );
            }
        }

        Self {
            game,
            active_game,
            dict: valid_words,
            npc_known_dict: npc_known_words,
            next_response_at: None,
            winner: None,
            map_texture,
            theme,
            turns: 0,
        }
    }

    pub fn reset(&mut self) {
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
            self.map_texture.clone(),
            self.theme.clone(),
        );

        self.game = game;
        self.active_game = active_game;
        self.next_response_at = None;
        self.winner = None;
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

        if matches!(next_msg, Some((_, PlayerMessage::Rematch))) {
            self.reset();
            return;
        }

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
                    let search_depth = (7_usize.saturating_sub(self.turns / 2)).max(3);
                    println!("Looking forward {search_depth} turns");

                    // let start = time::Instant::now();
                    let mut arb = truncate_core::npc::Arborist::pruning();
                    next_msg = Some((
                        1,
                        Game::best_move(
                            &self.game,
                            Some(&self.npc_known_dict),
                            search_depth,
                            Some(&mut arb),
                        ),
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
            self.turns += 1;

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

                    // NPC learns words as a result of battles that reveal validity
                    for battle in changes.iter().filter_map(|change| match change {
                        truncate_core::reporting::Change::Battle(battle) => Some(battle),
                        _ => None,
                    }) {
                        for word in &battle.attackers {
                            if word.valid == Some(true) {
                                let dict_word = word.original_word.to_lowercase();
                                if let Some(word_data) = self.dict.get(&dict_word).cloned() {
                                    self.npc_known_dict.insert(dict_word, word_data);
                                }
                            }
                        }
                        for word in &battle.defenders {
                            if word.valid == Some(true) {
                                let dict_word = word.original_word.to_lowercase();
                                if let Some(word_data) = self.dict.get(&dict_word).cloned() {
                                    self.npc_known_dict.insert(dict_word, word_data);
                                }
                            }
                        }
                    }

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
