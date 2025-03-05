use eframe::egui::{self, Align, Align2, Layout, Order};
use epaint::{vec2, Color32, TextureHandle};
use instant::Duration;
use truncate_core::{
    game::Game,
    moves::Move,
    reporting::{BoardChange, BoardChangeAction, BoardChangeDetail, Change},
};

use crate::{
    app_outer::Backchannel,
    lil_bits::BoardUI,
    utils::{
        depot::{
            AestheticDepot, AudioDepot, BoardDepot, GameplayDepot, InteractionDepot, RegionDepot,
            TimingDepot, TruncateDepot, UIStateDepot,
        },
        game_evals::get_main_dict,
        mapper::{MappedBoard, MappedTiles},
        text::TextHelper,
        timing::get_qs_tick,
        urls::back_to_menu,
        Theme,
    },
};

#[derive(Clone)]
enum PlaybackSpeed {
    Fast,
    Regular,
    Slow,
    Paused,
}

impl PlaybackSpeed {
    fn ticks(&self) -> u64 {
        match self {
            PlaybackSpeed::Fast => 1,
            PlaybackSpeed::Regular => 2,
            PlaybackSpeed::Slow => 3,
            PlaybackSpeed::Paused => u64::MAX,
        }
    }
}

#[derive(Clone)]
pub struct ReplayerState {
    as_player: usize,
    base_game: Game,
    game: Game,
    map_texture: TextureHandle,
    mapped_board: MappedBoard,
    mapped_overlay: MappedTiles,
    theme: Theme,
    move_sequence: Vec<Move>,
    next_move: usize,
    played_at_tick: Option<u64>,
    playback_speed: PlaybackSpeed,
    depot: TruncateDepot,
}

impl ReplayerState {
    pub fn new(
        ctx: &egui::Context,
        map_texture: TextureHandle,
        theme: Theme,
        mut game: Game,
        move_sequence: Vec<Move>,
        as_player: usize,
    ) -> Self {
        game.rules.battle_delay = 0;
        game.rules.timing = truncate_core::rules::Timing::None;
        game.rules.visibility = truncate_core::rules::Visibility::Standard;

        let player_colors: Vec<_> = game
            .players
            .iter()
            .map(|p| Color32::from_rgb(p.color.0, p.color.1, p.color.2))
            .collect();

        let aesthetics = AestheticDepot {
            theme: Theme::day(),
            qs_tick: 0,
            map_texture: map_texture.clone(),
            player_colors,
            destruction_tick: 0.05,
            destruction_duration: 0.6,
        };
        let mapped_board = MappedBoard::new(ctx, &aesthetics, &game.board, 2, as_player, true);

        let gameplay = GameplayDepot {
            room_code: "REPLAY".into(),
            player_number: as_player as u64,
            next_player_number: game.next_player.map(|p| p as u64),
            error_msg: None,
            winner: None,
            changes: vec![],
            last_battle_origin: None,
            npc: None,
            remaining_turns: None,
        };

        let depot = TruncateDepot {
            gameplay,
            aesthetics,
            interactions: InteractionDepot::default(),
            regions: RegionDepot::default(),
            ui_state: UIStateDepot::default(),
            board_info: BoardDepot::default(),
            timing: TimingDepot::default(),
            audio: AudioDepot::default(),
        };

        game.start();

        Self {
            depot,
            as_player,
            base_game: game.clone(),
            game,
            map_texture,
            mapped_board,
            mapped_overlay: MappedTiles::new(ctx, 1),
            theme,
            move_sequence,
            next_move: 0,
            played_at_tick: None,
            playback_speed: PlaybackSpeed::Regular,
        }
    }

    pub fn play_next_turn(&mut self, current_time: Duration, qs_tick: u64) {
        let Some(next_move) = self.move_sequence.get(self.next_move) else {
            return;
        };

        match next_move {
            Move::Place { player, tile, .. } => {
                if !self.game.players[*player].has_tile(*tile) {
                    // Escape hatch — if we have the wrong tile seed in the replay
                    // we don't want to error out, so we carry on blindly.
                    self.game.players[*player].hand.add(*tile);
                }
            }
            _ => {}
        }

        let dict_lock = get_main_dict();
        let dict = dict_lock.as_ref().unwrap();
        _ = self
            .game
            .play_turn(next_move.clone(), Some(dict), Some(dict), None);

        self.next_move += 1;

        self.depot.timing.last_turn_change = current_time;

        self.depot.gameplay.next_player_number = self.game.next_player.map(|p| p as u64);
        self.depot.gameplay.changes = self.game.recent_changes.clone();

        let battle_occurred = self
            .game
            .recent_changes
            .iter()
            .any(|change| matches!(change, Change::Battle(_)));

        if battle_occurred {
            self.depot.gameplay.last_battle_origin =
                self.game
                    .recent_changes
                    .iter()
                    .find_map(|change| match change {
                        Change::Board(BoardChange {
                            detail: BoardChangeDetail { coordinate, .. },
                            action: BoardChangeAction::Added,
                        }) => Some(*coordinate),
                        _ => None,
                    });
        } else {
            self.depot.gameplay.last_battle_origin = None;
        }

        // Add a delay after a battle to let animations play out
        if battle_occurred {
            self.played_at_tick = Some(qs_tick + 4);
        } else {
            self.played_at_tick = Some(qs_tick);
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        current_time: Duration,
        _backchannel: &Backchannel,
    ) {
        let start = self
            .played_at_tick
            .get_or_insert_with(|| get_qs_tick(current_time));
        let now = get_qs_tick(current_time);
        let elapsed = now.saturating_sub(*start);

        self.depot.timing.current_time = current_time.clone();

        if elapsed >= self.playback_speed.ticks() {
            self.play_next_turn(current_time, now);
        }

        let mut game_space = ui.available_rect_before_wrap();
        let replay_control_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));

        let area_id = egui::Id::new("replay_controls_layer");
        let area = egui::Area::new(area_id)
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_TOP, vec2(0.0, 0.0));
        let last_size = ui.memory(|m| m.area_rect(area_id));

        let resp = area.show(replay_control_ui.ctx(), |ui| {
            if let Some(last_size) = last_size {
                ui.painter().clone().rect_filled(
                    last_size,
                    0.0,
                    self.depot.aesthetics.theme.water.gamma_multiply(0.9),
                );
            }
            ui.allocate_ui_with_layout(
                vec2(game_space.width(), 10.0),
                Layout::top_down(Align::LEFT),
                |ui| {
                    ui.add_space(20.0);

                    let text = TextHelper::heavy("BACK TO MENU", 12.0, None, ui);
                    if text
                        .button(theme.button_primary, theme.text, &self.map_texture, ui)
                        .clicked()
                    {
                        back_to_menu();
                    }

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        let text = TextHelper::heavy("RESTART REPLAY", 12.0, None, ui);
                        if text
                            .button(theme.button_primary, theme.text, &self.map_texture, ui)
                            .clicked()
                        {
                            *self = Self::new(
                                ui.ctx(),
                                self.map_texture.clone(),
                                theme.clone(),
                                self.base_game.clone(),
                                self.move_sequence.clone(),
                                self.as_player,
                            );
                        }

                        let text = TextHelper::heavy("PAUSE", 12.0, None, ui);
                        if text
                            .button(theme.button_primary, theme.text, &self.map_texture, ui)
                            .clicked()
                        {
                            self.playback_speed = PlaybackSpeed::Paused
                        }

                        let text = TextHelper::heavy("TICK", 12.0, None, ui);
                        if text
                            .button(theme.button_primary, theme.text, &self.map_texture, ui)
                            .clicked()
                        {
                            self.play_next_turn(current_time, now);
                        }

                        let text = TextHelper::heavy("SLOW", 12.0, None, ui);
                        if text
                            .button(theme.button_primary, theme.text, &self.map_texture, ui)
                            .clicked()
                        {
                            self.playback_speed = PlaybackSpeed::Slow
                        }

                        let text = TextHelper::heavy("PLAY", 12.0, None, ui);
                        if text
                            .button(theme.button_primary, theme.text, &self.map_texture, ui)
                            .clicked()
                        {
                            self.playback_speed = PlaybackSpeed::Regular
                        }

                        let text = TextHelper::heavy("FAST", 12.0, None, ui);
                        if text
                            .button(theme.button_primary, theme.text, &self.map_texture, ui)
                            .clicked()
                        {
                            self.playback_speed = PlaybackSpeed::Fast
                        }
                    });

                    ui.add_space(10.0);
                },
            );
        });
        let replay_control_rect = resp.response.rect;
        game_space.set_top(replay_control_rect.bottom());

        self.mapped_board.remap_texture(
            ui.ctx(),
            &self.depot.aesthetics,
            &self.depot.timing,
            Some(&self.depot.interactions),
            Some(&self.depot.gameplay),
            &self.game.board,
        );

        let mut board_space_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));

        // TODO: Add a semi-interactive board
        // (panning + zooming okay, interacting with tiles not okay)
        BoardUI::new(&self.game.board).interactive(true).render(
            &self.game.players[0].hand,
            &mut board_space_ui,
            &mut self.mapped_board,
            &mut self.mapped_overlay,
            &mut self.depot,
        );
    }
}
