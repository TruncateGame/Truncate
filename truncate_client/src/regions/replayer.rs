use eframe::egui;
use epaint::{vec2, Color32, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    game::Game,
    moves::Move,
    reporting::{BoardChange, BoardChangeAction, BoardChangeDetail, Change},
};

use crate::{
    app_outer::Backchannel,
    utils::{
        depot::{AestheticDepot, GameplayDepot, TimingDepot},
        game_evals::get_main_dict,
        mapper::MappedBoard,
        text::TextHelper,
        timing::get_qs_tick,
        Theme,
    },
};

#[derive(Clone)]
enum PlaybackSpeed {
    Fast,
    Regular,
    Slow,
}

impl PlaybackSpeed {
    fn ticks(&self) -> u64 {
        match self {
            PlaybackSpeed::Fast => 1,
            PlaybackSpeed::Regular => 2,
            PlaybackSpeed::Slow => 3,
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
    theme: Theme,
    move_sequence: Vec<Move>,
    next_move: usize,
    played_at_tick: Option<u64>,
    playback_speed: PlaybackSpeed,
    aesthetics: AestheticDepot,
    timing: TimingDepot,
    gameplay: GameplayDepot,
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

        let player_colors: Vec<_> = game
            .players
            .iter()
            .map(|p| Color32::from_rgb(p.color.0, p.color.1, p.color.2))
            .collect();

        let aesthetics = AestheticDepot {
            theme: Theme::default(),
            qs_tick: 0,
            map_texture: map_texture.clone(),
            player_colors,
            destruction_tick: 0.05,
            destruction_duration: 0.6,
        };
        let mapped_board = MappedBoard::new(ctx, &aesthetics, &game.board, as_player);

        let gameplay = GameplayDepot {
            room_code: "REPLAY".into(),
            player_number: as_player as u64,
            next_player_number: game.next_player as u64,
            error_msg: None,
            winner: None,
            changes: vec![],
            last_battle_origin: None,
            npc: None,
        };

        game.start();

        Self {
            as_player,
            base_game: game.clone(),
            game,
            map_texture,
            mapped_board,
            theme,
            move_sequence,
            next_move: 0,
            played_at_tick: None,
            playback_speed: PlaybackSpeed::Regular,
            aesthetics,
            timing: TimingDepot::default(),
            gameplay,
        }
    }

    pub fn play_next_turn(&mut self, current_time: Duration, qs_tick: u64) {
        let Some(next_move) = self.move_sequence.get(self.next_move) else {
            return;
        };

        let dict_lock = get_main_dict();
        let dict = dict_lock.as_ref().unwrap();
        _ = self
            .game
            .play_turn(next_move.clone(), Some(dict), Some(dict), None);

        self.next_move += 1;

        self.timing.last_turn_change = current_time;

        self.gameplay.next_player_number = self.game.next_player as u64;
        self.gameplay.changes = self.game.recent_changes.clone();

        let battle_occurred = self
            .game
            .recent_changes
            .iter()
            .any(|change| matches!(change, Change::Battle(_)));

        self.gameplay.last_battle_origin =
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

        if battle_occurred {
            self.gameplay.last_battle_origin =
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
            self.gameplay.last_battle_origin = None;
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
        backchannel: &Backchannel,
    ) {
        let start = self
            .played_at_tick
            .get_or_insert_with(|| get_qs_tick(current_time));
        let now = get_qs_tick(current_time);
        let elapsed = now.saturating_sub(*start);

        self.timing.current_time = current_time.clone();

        if elapsed >= self.playback_speed.ticks() {
            self.play_next_turn(current_time, now);
        }

        ui.add_space(20.0);

        let text = TextHelper::heavy("RESTART REPLAY", 12.0, None, ui);
        if text
            .centered_button(theme.button_primary, theme.text, &self.map_texture, ui)
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

        self.mapped_board.remap_texture(
            ui.ctx(),
            &self.aesthetics,
            &self.timing,
            None,
            Some(&self.gameplay),
            &self.game.board,
        );

        let mut board_space = ui.available_rect_before_wrap().shrink(10.0);
        let height_from_width = self.game.board.height() as f32 / self.game.board.width() as f32;
        let target_height = board_space.width() * height_from_width;

        if target_height <= board_space.height() {
            let diff = (board_space.height() - target_height) / 2.0;
            board_space = board_space.shrink2(vec2(0.0, diff));
        } else {
            let width_from_height =
                self.game.board.width() as f32 / self.game.board.height() as f32;
            let target_width = board_space.height() * width_from_height;
            let diff = (board_space.width() - target_width) / 2.0;
            board_space = board_space.shrink2(vec2(diff, 0.0));
        }

        self.mapped_board.render_to_rect(board_space, ui);
    }
}
