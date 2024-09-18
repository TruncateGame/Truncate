use eframe::egui::{self, Layout, Sense};
use epaint::{emath::Align, hex_color, vec2, TextureHandle};
use time::Duration;
use truncate_core::{
    board::Board,
    game::{Game, GAME_COLOR_BLUE, GAME_COLOR_PURPLE, GAME_COLOR_RED},
    generation::BoardSeed,
    messages::{
        AssignedPlayerMessage, DailyStats, GamePlayerMessage, GameStateMessage, PlayerMessage,
    },
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
        control_devices::Switchboard,
        game_evals::{client_best_move, forget, get_main_dict, remember},
        text::TextHelper,
        Theme,
    },
};

use super::active_game::{ActiveGame, GameLocation, HeaderType};

#[derive(Clone)]
pub struct ArcadeState {
    pub name: String,
    pub game: Game,
    rules_generation: u32,
    pub active_game: ActiveGame,
    winner: Option<usize>,
    map_texture: TextureHandle,
    theme: Theme,
    turns: usize,
}

impl ArcadeState {
    pub fn new(
        name: String,
        ctx: &egui::Context,
        map_texture: TextureHandle,
        theme: Theme,
        mut board: Board,
        seed: Option<BoardSeed>,
        rules_generation: u32,
    ) -> Self {
        let mut game = Game::new(
            9,
            9,
            seed.clone().map(|s| s.seed as u64),
            GameRules::arcade(),
        );
        game.add_player("P1".into());
        game.add_player("P2".into());

        match &game.rules.board_genesis {
            truncate_core::rules::BoardGenesis::Passthrough => {
                game.board = board.clone();
                game.board.cache_special_squares();
            }
            truncate_core::rules::BoardGenesis::SpecificBoard(_) => unimplemented!(),
            truncate_core::rules::BoardGenesis::Classic(_, _) => unimplemented!(),
            truncate_core::rules::BoardGenesis::Random(params) => {
                let rand_board = truncate_core::generation::generate_board(
                    truncate_core::generation::BoardSeed {
                        generation: 9999,
                        seed: (instant::SystemTime::now()
                            .duration_since(instant::SystemTime::UNIX_EPOCH)
                            .expect("Please don't play Truncate earlier than 1970")
                            .as_micros()
                            % 287520520) as u32,
                        day: None,
                        params: params.clone(),
                        current_iteration: 0,
                        width_resize_state: None,
                        height_resize_state: None,
                        water_level: 0.5,
                        max_attempts: 10000,
                    },
                );
                game.board = rand_board.expect("Board can be resolved").board;
                game.board.cache_special_squares();
            }
        }

        game.players[0].color = GAME_COLOR_BLUE;
        game.players[1].color = GAME_COLOR_PURPLE;

        game.start();

        let mut active_game = ActiveGame::new(
            ctx,
            "ARCADE_PVP".into(),
            seed,
            None,
            game.players
                .iter()
                .map(|p| GamePlayerMessage::new(p, &game))
                .collect(),
            vec![0, 1],
            None,
            game.board.clone(),
            vec![game.players[0].hand.clone(), game.players[1].hand.clone()],
            map_texture.clone(),
            theme.clone(),
            GameLocation::Arcade,
            None,
            None,
        );
        active_game.depot.ui_state.game_header = HeaderType::None;

        Self {
            name,
            game,
            rules_generation,
            active_game,
            winner: None,
            map_texture,
            theme,
            turns: 0,
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
    ) -> Result<(), ()> {
        self.turns += 1;
        let dict_lock = get_main_dict();
        let dict = dict_lock.as_ref().unwrap();

        let player = match next_move {
            Move::Place {
                player,
                tile,
                position,
            } => player,
            Move::Swap { player, positions } => player,
        };
        let player_index = self
            .active_game
            .depot
            .gameplay
            .player_numbers
            .iter()
            .position(|p| *p == player as u64)
            .unwrap_or(0);

        // When actually playing the turn, make sure we pass in the real dict
        // for both the attack and defense roles.
        match self.game.play_turn(next_move, Some(dict), Some(dict), None) {
            Ok(winner) => {
                self.winner = winner;
                self.active_game.depot.gameplay.winner = self.winner;

                let room_code = self.active_game.depot.gameplay.room_code.clone();
                let state_message = GameStateMessage {
                    room_code,
                    players: self
                        .game
                        .players
                        .iter()
                        .map(|p| GamePlayerMessage::new(p, &self.game))
                        .collect(),
                    player_number: player as u64,
                    next_player_number: self.game.next_player.map(|p| p as u64),
                    board: self.game.board.clone(),
                    hand: self.game.players[player].hand.clone(),
                    changes: self.game.recent_changes.clone(),
                    game_ends_at: None,
                    paused: false,
                    remaining_turns: None,
                };
                self.active_game.apply_new_state(state_message);

                return Ok(());
            }
            Err(msg) => {
                // Suppress errors in Arcade mode until we have a place on the screen for them
                // self.active_game.depot.interactions[player_index].error_msg = Some(msg);
                return Err(());
            }
        }
    }

    pub fn reset(
        &mut self,
        current_time: Duration,
        ctx: &egui::Context,
        backchannel: &Backchannel,
    ) {
        let next_seed = (current_time.whole_microseconds() % 243985691) as u32;

        let next_board_seed = match &self.game.rules.board_genesis {
            truncate_core::rules::BoardGenesis::Passthrough => None,
            truncate_core::rules::BoardGenesis::SpecificBoard(_) => unimplemented!(),
            truncate_core::rules::BoardGenesis::Classic(_, _) => unimplemented!(),
            truncate_core::rules::BoardGenesis::Random(params) => {
                Some(truncate_core::generation::BoardSeed {
                    generation: 9999,
                    seed: next_seed,
                    day: None,
                    params: params.clone(),
                    current_iteration: 0,
                    width_resize_state: None,
                    height_resize_state: None,
                    water_level: 0.5,
                    max_attempts: 10000,
                })
            }
        };

        self.reset_to(next_board_seed, ctx, backchannel);
    }

    pub fn reset_to(
        &mut self,
        seed: Option<BoardSeed>,
        ctx: &egui::Context,
        backchannel: &Backchannel,
    ) {
        let seed = seed.unwrap();

        let mut game = Game::new(9, 9, Some(seed.seed as u64), GameRules::arcade());

        game.add_player("P1".into());
        game.add_player("P2".into());

        let mut rand_board = truncate_core::generation::generate_board(seed.clone())
            .expect("Standard seeds should always generate a board")
            .board;
        rand_board.cache_special_squares();

        game.board = rand_board;
        game.start();

        let mut active_game = ActiveGame::new(
            ctx,
            "ARCADE_PVP".into(),
            Some(seed),
            None,
            game.players
                .iter()
                .map(|p| GamePlayerMessage::new(p, &game))
                .collect(),
            vec![0, 1],
            None,
            game.board.clone(),
            vec![game.players[0].hand.clone(), game.players[1].hand.clone()],
            self.map_texture.clone(),
            self.theme.clone(),
            GameLocation::Arcade,
            None,
            None,
        );
        active_game.depot.ui_state.game_header = HeaderType::None;

        self.game = game;
        self.active_game = active_game;
        self.turns = 0;
        self.winner = None;
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        current_time: Duration,
        backchannel: &Backchannel,
        switchboard: &mut Switchboard,
        logged_in_as: &Option<String>,
    ) -> Vec<PlayerMessage> {
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.reset(current_time, ui.ctx(), backchannel);
        }

        let mut msgs_to_server = vec![];

        let (rect, _) = ui.allocate_exact_size(ui.available_size_before_wrap(), Sense::hover());
        let mut ui = ui.child_ui(rect, Layout::top_down(Align::LEFT));

        // Standard game helper
        let mut msgs =
            self.active_game
                .render(&mut ui, current_time, switchboard, Some(&self.game));

        for msg in &msgs {
            if let AssignedPlayerMessage {
                message: PlayerMessage::RequestDefinitions(words),
                ..
            } = msg
            {
                msgs_to_server.push(PlayerMessage::RequestDefinitions(words.clone()));
            }
        }

        if self.winner.is_some() {
            return msgs_to_server;
        }

        for msg in msgs {
            let next_move = match msg {
                AssignedPlayerMessage {
                    message: PlayerMessage::Place(position, tile),
                    player_id,
                } => Some(Move::Place {
                    player: player_id.unwrap_or_default() as _,
                    tile,
                    position,
                }),
                AssignedPlayerMessage {
                    message: PlayerMessage::Swap(from, to),
                    player_id,
                } => Some(Move::Swap {
                    player: player_id.unwrap_or_default() as _,
                    positions: [from, to],
                }),
                _ => None,
            };

            if let Some(next_move) = next_move {
                if let Ok(battle_words) = self.handle_move(next_move.clone(), backchannel, true) {
                    // Skipping battle word fetching while arcade mode doesn't have the ability to show definitions
                    // if !battle_words.is_empty() {
                    //     msgs_to_server.push(PlayerMessage::RequestDefinitions(battle_words));
                    // }
                }
            }
        }

        msgs_to_server
    }
}
