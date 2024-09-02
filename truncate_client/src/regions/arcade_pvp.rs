use eframe::egui::{self, Layout, Sense};
use epaint::{emath::Align, hex_color, vec2, TextureHandle};
use instant::Duration;
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
            GameRules::generation(rules_generation),
        );
        game.add_player("P1".into());
        game.add_player("P2".into());

        game.players[0].color = GAME_COLOR_BLUE;
        game.players[1].color = GAME_COLOR_PURPLE;

        board.cache_special_squares();
        game.board = board.clone();

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
            Some(0),
            game.board.clone(),
            vec![game.players[0].hand.clone(), game.players[1].hand.clone()],
            map_texture.clone(),
            theme.clone(),
            GameLocation::Local,
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
                self.active_game.depot.interactions[player_index].error_msg = Some(msg);
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

        let (rect, _) = ui.allocate_exact_size(ui.available_size_before_wrap(), Sense::hover());
        let mut ui = ui.child_ui(rect, Layout::top_down(Align::LEFT));

        // Standard game helper
        let mut next_msg = self
            .active_game
            .render(&mut ui, current_time, Some(&self.game));

        if let Some(AssignedPlayerMessage {
            message: PlayerMessage::RequestDefinitions(words),
            ..
        }) = &next_msg
        {
            msgs_to_server.push(PlayerMessage::RequestDefinitions(words.clone()));
        }

        if self.winner.is_some() {
            return msgs_to_server;
        }

        let next_move = match next_msg {
            Some(AssignedPlayerMessage {
                message: PlayerMessage::Place(position, tile),
                player_id,
            }) => Some(Move::Place {
                player: player_id.unwrap_or_default() as _,
                tile,
                position,
            }),
            Some(AssignedPlayerMessage {
                message: PlayerMessage::Swap(from, to),
                player_id,
            }) => Some(Move::Swap {
                player: player_id.unwrap_or_default() as _,
                positions: [from, to],
            }),
            _ => None,
        };

        if let Some(next_move) = next_move {
            if let Ok(battle_words) = self.handle_move(next_move.clone(), backchannel, true) {
                // if !battle_words.is_empty() {
                //     msgs_to_server.push(PlayerMessage::RequestDefinitions(battle_words));
                // }
            }
        }

        msgs_to_server
    }
}
