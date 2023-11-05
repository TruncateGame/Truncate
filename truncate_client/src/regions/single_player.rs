use std::{collections::HashMap, fmt::format};

use eframe::egui::{self, DragValue, Frame, Grid, Layout, RichText, ScrollArea, Sense, Window};
use epaint::{emath::Align, hex_color, vec2, Color32, Stroke, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    board::{Board, Square},
    game::Game,
    judge::{WordData, WordDict},
    messages::{GameStateMessage, PlayerMessage},
    moves::Move,
    npc::scoring::{BoardScore, BoardWeights},
    reporting::{Change, HandChange, WordMeaning},
};

use crate::{
    app_outer::Backchannel,
    lil_bits::HandUI,
    utils::{
        game_evals::{best_move, get_main_dict, remember, WORDNIK},
        text::TextHelper,
        Lighten, Theme,
    },
};

use super::active_game::ActiveGame;

pub struct SinglePlayerState {
    game: Game,
    original_board: Board,
    pub active_game: ActiveGame,
    next_response_at: Option<Duration>,
    winner: Option<usize>,
    map_texture: TextureHandle,
    theme: Theme,
    turns: usize,
    debugging_npc: bool,
    weights: BoardWeights,
    last_target_score: Option<BoardScore>,
    last_target_game: Option<ActiveGame>,
    waiting_on_backchannel: Option<String>,
}

impl SinglePlayerState {
    pub fn new(map_texture: TextureHandle, theme: Theme, mut board: Board) -> Self {
        let mut game = Game::new(9, 9);
        game.add_player("You".into());
        game.add_player("Computer".into());

        board.cache_special_squares();
        game.board = board.clone();

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

        Self {
            game,
            active_game,
            original_board: board,
            next_response_at: None,
            winner: None,
            map_texture,
            theme,
            turns: 0,
            debugging_npc: false,
            weights: BoardWeights::default(),
            last_target_score: None,
            last_target_game: None,
            waiting_on_backchannel: None,
        }
    }

    pub fn reset(&mut self) {
        let mut game = Game::new(9, 9);
        game.add_player("You".into());
        game.add_player("Computer".into());
        game.board = self.original_board.clone();
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
        self.turns = 0;
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
        backchannel: &Backchannel,
    ) -> Option<PlayerMessage> {
        let mut msg_to_server = None;

        if self.debugging_npc {
            ui.painter().rect_filled(
                ui.available_rect_before_wrap(),
                0.0,
                hex_color!("#00000055"),
            );
        }

        if matches!(option_env!("TR_ENV"), Some("outpost")) {
            let (top_banner, _) =
                ui.allocate_at_least(vec2(ui.available_width(), 40.0), Sense::hover());
            let mut banner_ui = ui.child_ui(top_banner, Layout::left_to_right(Align::Center));

            let text = if self.debugging_npc {
                TextHelper::heavy("CLOSE NPC DEBUGGER", 12.0, None, ui)
            } else {
                TextHelper::heavy("NPC DEBUGGER", 12.0, None, ui)
            };
            if text
                .centered_button(
                    theme.selection.lighten().lighten(),
                    theme.text,
                    &self.map_texture,
                    &mut banner_ui,
                )
                .clicked()
            {
                self.debugging_npc = !self.debugging_npc;
            }
        }

        if self.debugging_npc {
            Frame::none().inner_margin(8.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    TextHelper::heavy("NPC's current hand", 14.0, None, ui).paint(
                        Color32::WHITE,
                        ui,
                        true,
                    );
                    TextHelper::heavy("(new tiles highlighted)", 8.0, None, ui).paint(
                        Color32::WHITE,
                        ui,
                        true,
                    );

                    let added_tiles = self
                        .game
                        .recent_changes
                        .iter()
                        .filter_map(|change| match change {
                            Change::Hand(HandChange { player, added, .. }) if *player == 1 => {
                                Some(added)
                            }
                            _ => None,
                        })
                        .next();

                    self.active_game.ctx.highlight_tiles = added_tiles.cloned();
                    HandUI::new(&mut self.game.players[1].hand)
                        .interactive(false)
                        .render(&mut self.active_game.ctx, ui);
                    self.active_game.ctx.highlight_tiles = None;

                    ui.add_space(28.0);

                    TextHelper::heavy("Play with weights", 14.0, None, ui).paint(
                        Color32::WHITE,
                        ui,
                        true,
                    );
                    ui.add_space(24.0);

                    let BoardWeights {
                        raced_defense,
                        raced_attack,
                        self_defense,
                        self_attack,
                        direct_defence,
                        direct_attack,
                        word_validity,
                        word_length,
                        word_extensibility,
                    } = &mut self.weights;

                    fn dragger(v: &mut f32) -> DragValue {
                        DragValue::new(v).clamp_range(0.0..=100.0).speed(0.1)
                    }

                    let prev_text_styles = ui.style().text_styles.clone();
                    use egui::{FontFamily, FontId, TextStyle::*};
                    ui.style_mut().text_styles = [
                        (Heading, FontId::new(32.0, FontFamily::Proportional)),
                        (Body, FontId::new(24.0, FontFamily::Proportional)),
                        (Monospace, FontId::new(24.0, FontFamily::Monospace)),
                        (Button, FontId::new(24.0, FontFamily::Proportional)),
                        (Small, FontId::new(16.0, FontFamily::Proportional)),
                    ]
                    .into();

                    ui.horizontal(|ui| {
                        let sp = ui.available_width();
                        let pad = (sp - 308.0) * 0.5;
                        ui.add_space(pad);
                        let r = egui::Grid::new("weightings")
                            .spacing(Vec2::splat(8.0))
                            .min_col_width(150.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new("Raced defense").color(Color32::WHITE));
                                ui.add(dragger(raced_defense));
                                ui.end_row();

                                ui.label(RichText::new("Raced attack").color(Color32::WHITE));
                                ui.add(dragger(raced_attack));
                                ui.end_row();

                                ui.label(RichText::new("Self defense").color(Color32::WHITE));
                                ui.add(dragger(self_defense));
                                ui.end_row();

                                ui.label(RichText::new("Self attack").color(Color32::WHITE));
                                ui.add(dragger(self_attack));
                                ui.end_row();

                                ui.label(RichText::new("Direct defense").color(Color32::WHITE));
                                ui.add(dragger(direct_defence));
                                ui.end_row();

                                ui.label(RichText::new("Direct attack").color(Color32::WHITE));
                                ui.add(dragger(direct_attack));
                                ui.end_row();

                                ui.label(RichText::new("Word validity").color(Color32::WHITE));
                                ui.add(dragger(word_validity));
                                ui.end_row();

                                ui.label(RichText::new("Word length").color(Color32::WHITE));
                                ui.add(dragger(word_length));
                                ui.end_row();

                                ui.label(RichText::new("Word extensibility").color(Color32::WHITE));
                                ui.add(dragger(word_extensibility));
                                ui.end_row();
                            });
                        ui.painter().rect_stroke(
                            r.response.rect.expand(12.0),
                            0.0,
                            Stroke::new(2.0, self.theme.text),
                        );
                    });

                    ui.style_mut().text_styles = prev_text_styles;
                });
            });
            return None;
        }

        let (mut rect, _) = ui.allocate_exact_size(ui.available_size_before_wrap(), Sense::hover());
        let mut ui = ui.child_ui(rect, Layout::top_down(Align::LEFT));

        // Standard game helper
        let mut next_msg = self
            .active_game
            .render(&mut ui, theme, self.winner, current_time)
            .map(|msg| (0, msg));

        if matches!(next_msg, Some((_, PlayerMessage::Rematch))) {
            self.reset();
            return msg_to_server;
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
                if backchannel.is_open() {
                    if let Some(pending_msg) = &self.waiting_on_backchannel {
                        // Do nothing if a message is pending but our turn hasn't yet started,
                        // we'll fetch the turn once we're allowed to play.
                        if turn_starts_at <= current_time.as_secs() {
                            let msg_response =
                                backchannel.send_msg(crate::app_outer::BackchannelMsg::QueryFor {
                                    id: pending_msg.clone(),
                                });
                            if let Some(msg_response) = msg_response {
                                let player_msg: PlayerMessage = serde_json::from_str(&msg_response)
                                    .expect("Backchannel should be sending valid JSON");
                                next_msg = Some((1, player_msg));
                                self.waiting_on_backchannel = None;
                            }
                        }
                    } else {
                        let pending_msg =
                            backchannel.send_msg(crate::app_outer::BackchannelMsg::EvalGame {
                                board: self.game.board.clone(),
                                rules: self.game.rules.clone(),
                                players: self.game.players.clone(),
                                next_player: self.game.next_player,
                                weights: self.weights,
                            });
                        self.waiting_on_backchannel = pending_msg;
                    }
                } else {
                    // If we have no backchannel available to evaluate moves through,
                    // just evaluate the move on this thread and live with blocking.
                    if turn_starts_at <= current_time.as_secs() {
                        let best = best_move(&self.game, &self.weights);
                        next_msg = Some((1, best));
                    }
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
            let dict_lock = get_main_dict();
            let dict = dict_lock.as_ref().unwrap();

            // When actually playing the turn, make sure we pass in the real dict
            // for both the attack and defense roles.
            match self.game.play_turn(next_move, Some(dict), Some(dict)) {
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
                        for word in battle.attackers.iter().chain(battle.defenders.iter()) {
                            if word.valid == Some(true) {
                                let dict_word = word.original_word.to_lowercase();

                                if backchannel.is_open() {
                                    backchannel.send_msg(
                                        crate::app_outer::BackchannelMsg::Remember {
                                            word: dict_word,
                                        },
                                    );
                                } else {
                                    remember(&dict_word);
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
                Err(msg) => {
                    self.active_game.ctx.error_msg = Some(msg);
                }
            }
        }

        msg_to_server
    }
}
