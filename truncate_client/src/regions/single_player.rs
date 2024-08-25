use eframe::egui::{self, Layout, Sense};
use epaint::{emath::Align, hex_color, vec2, TextureHandle};
use instant::Duration;
use truncate_core::{
    board::Board,
    game::{Game, GAME_COLOR_BLUE, GAME_COLOR_RED},
    generation::BoardSeed,
    messages::{DailyStats, GamePlayerMessage, GameStateMessage, PlayerMessage},
    moves::Move,
    npc::scoring::NPCPersonality,
    reporting::WordMeaning,
    rules::GameRules,
};

use crate::{
    app_outer::{Backchannel, EventDispatcher},
    lil_bits::{
        result_modal::{ResultModalAction, ResultModalDaily, ResultModalVariant},
        ResultModalUI,
    },
    utils::{
        game_evals::{client_best_move, forget, get_main_dict, remember},
        text::TextHelper,
        Theme,
    },
};

use super::active_game::{ActiveGame, GameLocation, HeaderType};

#[derive(Clone)]
pub struct SinglePlayerState {
    pub name: String,
    pub game: Game,
    rules_generation: u32,
    human_starts: bool,
    pub active_game: ActiveGame,
    next_response_at: Option<Duration>,
    winner: Option<usize>,
    map_texture: TextureHandle,
    theme: Theme,
    turns: usize,
    debugging_npc: bool,
    npc: NPCPersonality,
    waiting_on_backchannel: Option<String>,
    pub header: HeaderType,
    pub daily_stats: Option<DailyStats>,
    pub best_game: Option<Game>,
    splash: Option<ResultModalUI>,
    hide_splash: bool,
    pub move_sequence: Vec<Move>,
    event_dispatcher: EventDispatcher,
}

impl SinglePlayerState {
    pub fn new(
        name: String,
        ctx: &egui::Context,
        map_texture: TextureHandle,
        theme: Theme,
        mut board: Board,
        seed: Option<BoardSeed>,
        rules_generation: u32,
        human_starts: bool,
        header: HeaderType,
        npc: NPCPersonality,
        mut event_dispatcher: EventDispatcher,
    ) -> Self {
        event_dispatcher.event(format!("single_player_{name}"));

        let mut game = Game::new(
            9,
            9,
            seed.clone().map(|s| s.seed as u64),
            GameRules::generation(rules_generation),
        );
        if human_starts {
            game.add_player("You".into());
            game.add_player("Computer".into());

            game.players[0].color = GAME_COLOR_BLUE;
            game.players[1].color = GAME_COLOR_RED;
        } else {
            game.add_player("Computer".into());
            game.add_player("You".into());

            game.players[0].color = GAME_COLOR_RED;
            game.players[1].color = GAME_COLOR_BLUE;
        }

        board.cache_special_squares();
        game.board = board.clone();

        game.start();

        let (filtered_board, _) = game.filter_game_to_player(if human_starts { 0 } else { 1 });

        let mut active_game = ActiveGame::new(
            ctx,
            "SINGLE_PLAYER".into(),
            seed,
            Some(npc.clone()),
            game.players
                .iter()
                .map(|p| GamePlayerMessage::new(p, &game))
                .collect(),
            if human_starts { 0 } else { 1 },
            Some(0),
            filtered_board.clone(),
            game.players[if human_starts { 0 } else { 1 }].hand.clone(),
            map_texture.clone(),
            theme.clone(),
            GameLocation::Local,
            None,
            None,
        );
        active_game.depot.ui_state.game_header = header.clone();

        Self {
            name,
            game,
            rules_generation,
            human_starts,
            active_game,
            next_response_at: None,
            winner: None,
            map_texture,
            theme,
            turns: 0,
            debugging_npc: false,
            npc,
            waiting_on_backchannel: None,
            header,
            daily_stats: None,
            best_game: None,
            splash: None,
            hide_splash: false,
            move_sequence: vec![],
            event_dispatcher,
        }
    }

    fn sub_event(&mut self, event: String) {
        self.event_dispatcher
            .event(format!("single_player_{}_{}", self.name, event));
    }

    pub fn reset(
        &mut self,
        current_time: Duration,
        ctx: &egui::Context,
        backchannel: &Backchannel,
    ) {
        if let Some(seed) = &self.active_game.depot.board_info.board_seed {
            if seed.day.is_some() {
                match &mut self.header {
                    HeaderType::Summary {
                        attempt: Some(attempt),
                        ..
                    } => {
                        *attempt += 1;
                    }
                    _ => {}
                };
                return self.reset_to(seed.clone(), self.human_starts, ctx, backchannel);
            }
        }

        let next_seed = (current_time.as_micros() % 243985691) as u32;
        let next_board_seed = BoardSeed::new(next_seed);

        self.reset_to(next_board_seed, !self.human_starts, ctx, backchannel);
    }

    pub fn reset_to(
        &mut self,
        seed: BoardSeed,
        human_starts: bool,
        ctx: &egui::Context,
        backchannel: &Backchannel,
    ) {
        let mut game = Game::new(
            9,
            9,
            Some(seed.seed as u64),
            GameRules::generation(self.rules_generation),
        );
        self.human_starts = human_starts;
        if self.human_starts {
            game.add_player("You".into());
            game.add_player("Computer".into());

            game.players[0].color = GAME_COLOR_BLUE;
            game.players[1].color = GAME_COLOR_RED;
        } else {
            game.add_player("Computer".into());
            game.add_player("You".into());

            game.players[0].color = GAME_COLOR_RED;
            game.players[1].color = GAME_COLOR_BLUE;
        }

        let mut rand_board = truncate_core::generation::generate_board(seed.clone())
            .expect("Standard seeds should always generate a board")
            .board;
        rand_board.cache_special_squares();

        game.board = rand_board;
        game.start();

        let mut active_game = ActiveGame::new(
            ctx,
            "SINGLE_PLAYER".into(),
            Some(seed),
            Some(self.npc.clone()),
            game.players
                .iter()
                .map(|p| GamePlayerMessage::new(p, &game))
                .collect(),
            if self.human_starts { 0 } else { 1 },
            Some(0),
            game.board.clone(),
            game.players[if self.human_starts { 0 } else { 1 }]
                .hand
                .clone(),
            self.map_texture.clone(),
            self.theme.clone(),
            GameLocation::Local,
            None,
            None,
        );
        active_game.depot.ui_state.game_header = self.header.clone();

        self.sub_event("replay".to_string());

        self.game = game;
        self.active_game = active_game;
        self.turns = 0;
        self.next_response_at = None;
        self.winner = None;
        self.move_sequence = vec![];
        self.event_dispatcher = self.event_dispatcher.clone();

        if backchannel.is_open() {
            backchannel.send_msg(crate::app_outer::BackchannelMsg::Forget);
        } else {
            forget();
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

    pub fn handle_move(
        &mut self,
        next_move: Move,
        backchannel: &Backchannel,
        track_events: bool,
    ) -> Result<Vec<String>, ()> {
        let human_player = if self.human_starts { 0 } else { 1 };

        self.turns += 1;
        let dict_lock = get_main_dict();
        let dict = dict_lock.as_ref().unwrap();

        // When actually playing the turn, make sure we pass in the real dict
        // for both the attack and defense roles.
        match self.game.play_turn(next_move, Some(dict), Some(dict), None) {
            Ok(winner) => {
                self.winner = winner;

                if track_events {
                    if let Some(winner) = winner {
                        if winner == human_player {
                            self.sub_event("won".to_string())
                        } else {
                            self.sub_event("lost".to_string())
                        }
                    }
                }

                let changes: Vec<_> = self
                    .game
                    .recent_changes
                    .clone()
                    .into_iter()
                    .filter(|change| match change {
                        truncate_core::reporting::Change::Board(_) => true,
                        truncate_core::reporting::Change::Hand(hand_change) => {
                            hand_change.player == human_player
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

                // Need to release our dict mutex, so that
                // the remember() function below can lock it itself.
                drop(dict_lock);

                // NPC learns words as a result of battles that reveal validity
                for battle in changes.iter().filter_map(|change| match change {
                    truncate_core::reporting::Change::Battle(battle) => Some(battle),
                    _ => None,
                }) {
                    for word in battle.attackers.iter().chain(battle.defenders.iter()) {
                        if word.valid == Some(true) {
                            let dict_word = word.original_word.to_lowercase();

                            if backchannel.is_open() {
                                backchannel.send_msg(crate::app_outer::BackchannelMsg::Remember {
                                    word: dict_word,
                                });
                            } else {
                                remember(&dict_word);
                            }
                        }
                    }
                }

                let room_code = self.active_game.depot.gameplay.room_code.clone();
                let state_message = GameStateMessage {
                    room_code,
                    players: self
                        .game
                        .players
                        .iter()
                        .map(|p| GamePlayerMessage::new(p, &self.game))
                        .collect(),
                    player_number: human_player as u64,
                    next_player_number: self.game.next_player.map(|p| p as u64),
                    board: self.game.board.clone(),
                    hand: self.game.players[human_player].hand.clone(),
                    changes,
                    game_ends_at: None,
                    paused: false,
                    remaining_turns: None,
                };
                self.active_game.apply_new_state(state_message);

                return Ok(battle_words);
            }
            Err(msg) => {
                self.active_game.depot.gameplay.error_msg = Some(msg);
                return Err(());
            }
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        current_time: Duration,
        backchannel: &Backchannel,
        logged_in_as: &Option<String>,
    ) -> Vec<PlayerMessage> {
        let mut msgs_to_server = vec![];
        let human_player = if self.human_starts { 0 } else { 1 };
        let npc_player = if self.human_starts { 1 } else { 0 };

        if self.debugging_npc {
            ui.painter().rect_filled(
                ui.available_rect_before_wrap(),
                0.0,
                hex_color!("#00000055"),
            );
        }

        if matches!(option_env!("TR_ENV"), Some("outpost_disabled")) {
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
                    theme.button_primary,
                    theme.text,
                    &self.map_texture,
                    &mut banner_ui,
                )
                .clicked()
            {
                self.debugging_npc = !self.debugging_npc;
            }
        }

        let (rect, _) = ui.allocate_exact_size(ui.available_size_before_wrap(), Sense::hover());
        let mut ui = ui.child_ui(rect, Layout::top_down(Align::LEFT));

        // Standard game helper
        let mut next_msg = self
            .active_game
            .render(&mut ui, current_time, Some(&self.game))
            .map(|msg| (human_player, msg));

        if matches!(next_msg, Some((_, PlayerMessage::Rematch))) {
            self.reset(current_time, ui.ctx(), backchannel);
            return msgs_to_server;
        } else if matches!(next_msg, Some((_, PlayerMessage::Resign))) {
            if self.hide_splash {
                self.hide_splash = false;
            } else {
                match self.active_game.location {
                    GameLocation::Tutorial | GameLocation::Local => {
                        self.splash = Some(ResultModalUI::new_resigning(
                            &mut ui,
                            "Start again?".to_string(),
                        ))
                    }
                    GameLocation::Online => {
                        self.splash = Some(ResultModalUI::new_resigning(
                            &mut ui,
                            "Resign this game?".to_string(),
                        ))
                    }
                }
            }
        } else if let Some((_, PlayerMessage::RequestDefinitions(words))) = &next_msg {
            msgs_to_server.push(PlayerMessage::RequestDefinitions(words.clone()));
        }

        if let Some(splash) = &mut self.splash {
            if self.hide_splash == false {
                let splash_msg = splash.render(
                    &mut ui,
                    theme,
                    &self.map_texture,
                    &self.active_game.depot,
                    Some(backchannel),
                );

                match splash_msg {
                    Some(ResultModalAction::NewPuzzle) => {
                        self.splash = None;
                        self.reset(current_time, ui.ctx(), backchannel);
                    }
                    Some(ResultModalAction::TryAgain) => {
                        self.splash = None;

                        if let Some(seed) = &self.active_game.depot.board_info.board_seed {
                            self.reset_to(seed.clone(), self.human_starts, ui.ctx(), backchannel);
                        } else {
                            self.reset(current_time, ui.ctx(), backchannel);
                        }
                    }
                    Some(ResultModalAction::Dismiss) => {
                        self.hide_splash = true;

                        // Trigger showing the "view summary" button below the game board
                        self.active_game.depot.gameplay.winner = self.winner;
                    }
                    Some(ResultModalAction::Resign) => {
                        self.sub_event("resign".to_string());
                        self.splash = None;
                        self.game.resign_player(human_player);
                        self.winner = Some(npc_player);
                    }
                    Some(ResultModalAction::SharedText) => {
                        self.sub_event("shared_text".to_string());
                    }
                    Some(ResultModalAction::SharedReplay) => {
                        self.sub_event("shared_replay".to_string());
                    }
                    None => {}
                }
            }
        }

        if self.winner.is_some() {
            let is_daily_puzzle = self
                .active_game
                .depot
                .board_info
                .board_seed
                .as_ref()
                .map(|s| s.day)
                .flatten();

            if let Some(puzzle_day) = is_daily_puzzle {
                if let Some(token) = logged_in_as {
                    if self.splash.is_none() {
                        msgs_to_server.push(PlayerMessage::RequestStats(token.clone()));
                    }
                }

                if self.winner == Some(human_player) {
                    if self.best_game.is_none()
                        || self
                            .best_game
                            .as_ref()
                            .is_some_and(|best| best.turn_count > self.game.turn_count)
                    {
                        self.best_game = Some(self.game.clone());
                    }
                }

                // Refresh our stats UI if we receive updated stats from the server
                if let Some(mut stats) = self.daily_stats.take() {
                    stats.hydrate_missing_days();

                    let matches = match &self.splash {
                        Some(ResultModalUI {
                            contents:
                                ResultModalVariant::Daily(ResultModalDaily {
                                    stats: existing_stats,
                                    ..
                                }),
                            ..
                        }) => *existing_stats == stats,
                        _ => false,
                    };

                    if !matches {
                        self.splash = Some(ResultModalUI::new_daily(
                            &mut ui,
                            &self.game,
                            self.turns as u32,
                            &mut self.active_game.depot,
                            stats,
                            self.best_game.as_ref(),
                            puzzle_day,
                        ));
                    }
                }

                if self.splash.is_none() {
                    // TODO: Add a special splash screen for somehow having no token / not being logged in
                    self.splash = Some(ResultModalUI::new_loading(&mut ui));

                    if let Some(token) = logged_in_as {
                        msgs_to_server.push(PlayerMessage::RequestStats(token.clone()));
                    }
                }
            } else {
                if self.splash.is_none() {
                    self.splash = Some(ResultModalUI::new_unique(
                        &mut ui,
                        &self.game,
                        &mut self.active_game.depot,
                        matches!(
                            self.winner,
                            Some(p) if  p == human_player
                        ),
                    ));
                }
            }
            return msgs_to_server;
        }

        if let Some(next_response_at) = self.next_response_at {
            if self.game.next_player.unwrap() == npc_player
                && next_response_at > self.active_game.depot.timing.current_time
            {
                return msgs_to_server;
            }
        }
        self.next_response_at = None;

        if self.game.next_player.unwrap() == npc_player {
            if let Some(turn_starts_no_later_than) = self
                .game
                .get_player(self.game.next_player.unwrap())
                .unwrap()
                .turn_starts_no_later_than
            {
                if backchannel.is_open() {
                    if let Some(pending_msg) = &self.waiting_on_backchannel {
                        // Do nothing if a message is pending but our turn hasn't yet started,
                        // we'll fetch the turn once we're allowed to play.
                        // It is allowed to play here, but waiting lets battle animations play out.
                        if turn_starts_no_later_than <= current_time.as_secs() {
                            let msg_response =
                                backchannel.send_msg(crate::app_outer::BackchannelMsg::QueryFor {
                                    id: pending_msg.clone(),
                                });
                            if let Some(msg_response) = msg_response {
                                let player_msg: PlayerMessage = serde_json::from_str(&msg_response)
                                    .expect("Backchannel should be sending valid JSON");
                                next_msg = Some((npc_player, player_msg));
                                self.waiting_on_backchannel = None;
                            }
                        }
                    } else {
                        let (filtered_board, _) = self.game.filter_game_to_player(npc_player);
                        let pending_msg =
                            backchannel.send_msg(crate::app_outer::BackchannelMsg::EvalGame {
                                board: filtered_board,
                                rules: self.game.rules.clone(),
                                players: self.game.players.clone(),
                                next_player: npc_player,
                                npc_params: self.npc.params,
                            });
                        self.waiting_on_backchannel = pending_msg;
                    }
                } else {
                    // If we have no backchannel available to evaluate moves through,
                    // just evaluate the move on this thread and live with blocking.
                    let (filtered_board, _) = self.game.filter_game_to_player(npc_player);
                    let mut evaluation_game = self.game.clone();
                    evaluation_game.board = filtered_board;

                    if turn_starts_no_later_than <= current_time.as_secs() {
                        let best = client_best_move(&evaluation_game, &self.npc.params);
                        next_msg = Some((npc_player, best));
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
            if let Ok(battle_words) = self.handle_move(next_move.clone(), backchannel, true) {
                self.move_sequence.push(next_move.clone());

                if let Some(seed) = &self.active_game.depot.board_info.board_seed {
                    if seed.day.is_some() {
                        if let Some(token) = logged_in_as {
                            msgs_to_server.push(PlayerMessage::PersistPuzzleMoves {
                                player_token: token.clone(),
                                day: seed.day.unwrap(),
                                human_player: human_player as u32,
                                moves: self.move_sequence.clone(),
                                won: self.winner == Some(human_player),
                            });

                            // Ensure we never pull up an old splash screen without this move
                            self.daily_stats = None;
                        }
                    }
                }
                let delay = if battle_words.is_empty() { 650 } else { 2000 };

                if !battle_words.is_empty() {
                    msgs_to_server.push(PlayerMessage::RequestDefinitions(battle_words));
                }

                self.next_response_at = Some(
                    self.active_game
                        .depot
                        .timing
                        .current_time
                        .saturating_add(Duration::from_millis(delay)),
                );
                ui.ctx()
                    .request_repaint_after(Duration::from_millis(delay / 2));
            }
        }

        msgs_to_server
    }
}
