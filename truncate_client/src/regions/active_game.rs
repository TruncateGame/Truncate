use epaint::{emath::Align2, hex_color, vec2, Color32, FontId, Rect, Stroke, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage, RoomCode},
    player::Hand,
    reporting::{BoardChange, Change, TimeChange},
};

use eframe::{
    egui::{self, Frame, Label, LayerId, Layout, Margin, Order, RichText, ScrollArea, Sense},
    emath::Align,
};
use hashbrown::HashMap;

use crate::{
    lil_bits::{BattleUI, BoardUI, HandUI, TimerUI},
    utils::{mapper::MappedBoard, text::TextHelper, Diaphanize, Lighten, Theme},
};

#[derive(Debug, Clone, PartialEq)]
pub struct HoveredRegion {
    pub rect: Rect,
    // If we're hovering the board, what coordinate is it?
    pub coord: Option<Coordinate>,
}

#[derive(Clone)]
pub struct GameCtx {
    pub theme: Theme,
    pub current_time: Duration,
    pub prev_to_next_turn: (Duration, Duration),
    pub qs_tick: u64,
    pub room_code: RoomCode,
    pub player_number: u64,
    pub next_player_number: u64,
    pub selected_tile_in_hand: Option<usize>,
    pub dragging_tile: bool,
    pub released_tile: Option<(usize, Coordinate)>,
    pub selected_square_on_board: Option<Coordinate>,
    pub hovered_tile_on_board: Option<HoveredRegion>,
    pub playing_tile: Option<char>,
    pub error_msg: Option<String>,
    pub map_texture: TextureHandle,
    pub player_colors: Vec<Color32>,
    pub board_moved: bool,
    pub board_zoom: f32,
    pub board_pan: Vec2,
    pub sidebar_visible: bool,
    pub timers_visible: bool,
    pub hand_total_rect: Option<Rect>,
    pub hand_companion_rect: Option<Rect>,
    pub highlight_tiles: Option<Vec<char>>,
    pub highlight_squares: Option<Vec<Coordinate>>,
    pub is_mobile: bool,
}

#[derive(Clone)]
pub struct ActiveGame {
    pub ctx: GameCtx,
    pub players: Vec<GamePlayerMessage>,
    pub board: Board,
    pub mapped_board: MappedBoard,
    pub hand: Hand,
    pub board_changes: HashMap<Coordinate, BoardChange>,
    pub new_hand_tiles: Vec<usize>,
    pub time_changes: Vec<TimeChange>,
    pub turn_reports: Vec<Vec<Change>>,
}

impl ActiveGame {
    pub fn new(
        room_code: RoomCode,
        players: Vec<GamePlayerMessage>,
        player_number: u64,
        next_player_number: u64,
        board: Board,
        hand: Hand,
        map_texture: TextureHandle,
        theme: Theme,
    ) -> Self {
        let player_colors = players
            .iter()
            .map(|p| Color32::from_rgb(p.color.0, p.color.1, p.color.2))
            .collect::<Vec<_>>();
        Self {
            ctx: GameCtx {
                theme,
                current_time: Duration::from_secs(0),
                prev_to_next_turn: (Duration::from_secs(0), Duration::from_secs(0)),
                qs_tick: 0,
                room_code,
                player_number,
                next_player_number,
                selected_tile_in_hand: None,
                released_tile: None,
                selected_square_on_board: None,
                hovered_tile_on_board: None,
                dragging_tile: false,
                playing_tile: None,
                error_msg: None,
                map_texture: map_texture.clone(),
                player_colors: player_colors.clone(),
                board_moved: false,
                board_zoom: 1.0,
                board_pan: vec2(0.0, 0.0),
                sidebar_visible: false,
                timers_visible: true,
                hand_companion_rect: None,
                hand_total_rect: None,
                highlight_tiles: None,
                highlight_squares: None,
                is_mobile: false,
            },
            mapped_board: MappedBoard::new(
                &board,
                map_texture.clone(),
                player_number == 0,
                &player_colors,
            ),
            players,
            board,
            hand,
            board_changes: HashMap::new(),
            new_hand_tiles: vec![],
            time_changes: vec![],
            turn_reports: vec![],
        }
    }
}

impl ActiveGame {
    pub fn render_control_strip(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        winner: Option<usize>,
    ) -> (Rect, Option<PlayerMessage>) {
        let mut msg = None;
        let companion_space = 220.0;

        let control_anchor = if self.ctx.timers_visible {
            vec2(0.0, 0.0)
        } else {
            vec2(0.0, -companion_space)
        };

        if !self.ctx.timers_visible {
            let mut companion_pos = ui.available_rect_before_wrap();
            companion_pos.set_top(companion_pos.bottom() - companion_space);
            self.ctx.hand_companion_rect = Some(companion_pos);
        }

        let area = egui::Area::new(egui::Id::new("controls_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_BOTTOM, control_anchor);

        let avail_width = ui.available_width();

        let resp = area.show(ui.ctx(), |ui| {
            if let Some(bg_rect) = self.ctx.hand_total_rect {
                ui.painter().clone().rect_filled(
                    bg_rect,
                    0.0,
                    self.ctx.theme.water.gamma_multiply(0.75),
                );
            }

            ui.allocate_ui_with_layout(
                vec2(avail_width, 10.0),
                Layout::top_down(Align::LEFT),
                |ui| {
                    ui.add_space(10.0);

                    if winner.is_some() {
                        let text = TextHelper::heavy("REMATCH", 12.0, None, ui);
                        if text
                            .centered_button(
                                theme.selection.lighten().lighten(),
                                theme.text,
                                &self.ctx.map_texture,
                                ui,
                            )
                            .clicked()
                        {
                            msg = Some(PlayerMessage::Rematch);
                        }
                    }

                    let (hand_alloc, _) =
                        ui.allocate_at_least(vec2(ui.available_width(), 50.0), Sense::hover());
                    let mut hand_ui = ui.child_ui(hand_alloc, Layout::top_down(Align::LEFT));
                    HandUI::new(&mut self.hand)
                        .active(self.ctx.player_number == self.ctx.next_player_number)
                        .render(&mut self.ctx, &mut hand_ui);

                    ui.add_space(10.0);

                    if self.ctx.timers_visible {
                        if let Some(player) = self
                            .players
                            .iter()
                            .find(|p| p.index == self.ctx.player_number as usize)
                        {
                            TimerUI::new(player, self.ctx.current_time, &self.time_changes)
                                .friend(true)
                                .active(player.index == self.ctx.next_player_number as usize)
                                .winner(winner.clone())
                                .render(ui, theme, &mut self.ctx);
                            ui.add_space(10.0);
                        }

                        if let Some(opponent) = self
                            .players
                            .iter()
                            .find(|p| p.index != self.ctx.player_number as usize)
                        {
                            TimerUI::new(opponent, self.ctx.current_time, &self.time_changes)
                                .friend(false)
                                .active(opponent.index == self.ctx.next_player_number as usize)
                                .winner(winner.clone())
                                .render(ui, theme, &mut self.ctx);
                            ui.add_space(10.0);
                        }

                        ui.add_space(5.0);
                    }

                    if self.ctx.is_mobile {
                        let text = TextHelper::heavy("VIEW INFO", 12.0, None, ui);
                        if text
                            .centered_button(
                                Color32::WHITE.diaphanize(),
                                theme.text,
                                &self.ctx.map_texture,
                                ui,
                            )
                            .clicked()
                        {
                            self.ctx.sidebar_visible = true;
                        }
                    }
                    ui.add_space(10.0);
                },
            );
        });

        self.ctx.hand_total_rect = Some(resp.response.rect);

        (resp.response.rect, msg)
    }

    pub fn render_sidebar(&mut self, ui: &mut egui::Ui, theme: &Theme, winner: Option<usize>) {
        if self.ctx.is_mobile && !self.ctx.sidebar_visible {
            return;
        }

        let area = egui::Area::new(egui::Id::new("sidebar_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::RIGHT_TOP, vec2(0.0, 0.0));

        let sidebar_alloc = ui.max_rect();
        let mut outer_sidebar_area = sidebar_alloc.shrink2(vec2(0.0, 8.0));
        outer_sidebar_area.set_right(outer_sidebar_area.right() - 8.0);
        let inner_sidebar_area = outer_sidebar_area.shrink(8.0);

        let resp = area.show(ui.ctx(), |ui| {
            ui.painter().clone().rect_filled(
                sidebar_alloc,
                0.0,
                self.ctx.theme.water.gamma_multiply(0.9),
            );

            ui.allocate_ui_at_rect(inner_sidebar_area, |ui| {
                ui.expand_to_include_rect(inner_sidebar_area);
                if self.ctx.is_mobile {
                    let text = TextHelper::heavy("CLOSE INFO", 12.0, None, ui);
                    if text
                        .centered_button(
                            Color32::WHITE.diaphanize(),
                            theme.text,
                            &self.ctx.map_texture,
                            ui,
                        )
                        .clicked()
                    {
                        self.ctx.sidebar_visible = false;
                    }

                    ui.add_space(10.0);
                }

                ScrollArea::new([false, true]).show(ui, |ui| {
                    let room = ui.painter().layout_no_wrap(
                        "Game Info".into(),
                        FontId::new(
                            self.ctx.theme.letter_size / 2.0,
                            egui::FontFamily::Name("Truncate-Heavy".into()),
                        ),
                        self.ctx.theme.text,
                    );
                    let (r, _) = ui.allocate_at_least(room.size(), Sense::hover());
                    ui.painter().galley(r.min, room);
                    ui.add_space(15.0);

                    if let Some(error) = &self.ctx.error_msg {
                        ui.label(error);
                        ui.separator();
                    }

                    let mut rendered_battles = 0;
                    let label_font =
                        FontId::new(8.0, egui::FontFamily::Name("Truncate-Heavy".into()));

                    for turn in self.turn_reports.iter().rev() {
                        for battle in turn.iter().filter_map(|change| match change {
                            Change::Battle(battle) => Some(battle),
                            _ => None,
                        }) {
                            if let Some(label) = if rendered_battles == 0 {
                                Some("Latest Battle")
                            } else if rendered_battles == 1 {
                                Some("Previous Battles")
                            } else {
                                None
                            } {
                                let label = ui.painter().layout_no_wrap(
                                    label.into(),
                                    label_font.clone(),
                                    self.ctx.theme.text,
                                );
                                let (r, _) = ui.allocate_at_least(label.size(), Sense::hover());
                                ui.painter().galley(r.min, label);
                            }

                            BattleUI::new(battle).render(&mut self.ctx, ui);
                            rendered_battles += 1;
                            ui.add_space(15.0);
                        }
                    }
                });
            });
        });
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        winner: Option<usize>,
        current_time: Duration,
    ) -> Option<PlayerMessage> {
        self.ctx.current_time = current_time;
        let cur_tick = current_time.as_secs() * 4 + current_time.subsec_millis() as u64 / 250;
        if cur_tick > self.ctx.qs_tick {
            self.ctx.qs_tick = cur_tick;
            self.mapped_board
                .remap(&self.board, &self.ctx.player_colors, self.ctx.qs_tick);
        }

        let mut game_space = ui.available_rect_before_wrap();
        let mut sidebar_space = game_space.clone();

        if ui.available_size().x >= self.ctx.theme.mobile_breakpoint {
            self.ctx.is_mobile = false;
            game_space.set_right(game_space.right() - 300.0);
            sidebar_space.set_left(sidebar_space.right() - 300.0);
        } else {
            self.ctx.is_mobile = true;
        }

        let mut control_strip_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));
        let (control_strip_rect, control_player_message) =
            self.render_control_strip(&mut control_strip_ui, theme, winner);

        let mut sidebar_space_ui = ui.child_ui(sidebar_space, Layout::top_down(Align::LEFT));
        self.render_sidebar(&mut sidebar_space_ui, theme, winner);

        game_space.set_bottom(control_strip_rect.top());
        let mut game_space_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));

        let player_message = BoardUI::new(&self.board)
            .render(
                &self.hand,
                &self.board_changes,
                winner.clone(),
                &mut self.ctx,
                &mut game_space_ui,
                &self.mapped_board,
            )
            .or(control_player_message);

        player_message
    }

    pub fn apply_new_state(&mut self, state_message: GameStateMessage) {
        let GameStateMessage {
            room_code: _,
            players,
            player_number: _,
            next_player_number,
            board,
            hand: _,
            changes,
        } = state_message;

        // assert_eq!(self.room_code, room_code);
        // assert_eq!(self.player_number, player_number);
        self.players = players;
        self.board = board;

        #[cfg(target_arch = "wasm32")]
        // Play the turn sound if the player has changed
        if self.ctx.next_player_number != next_player_number {
            use eframe::wasm_bindgen::JsCast;

            let window = web_sys::window().expect("window should exist in browser");
            let document = window.document().expect("documnt should exist in window");
            if let Some(element) = document.query_selector("#tr_move").unwrap() {
                if let Ok(audio) = element.dyn_into::<web_sys::HtmlAudioElement>() {
                    audio.play().expect("Audio should be playable");
                }
            }
        }

        self.ctx.next_player_number = next_player_number;
        if let Some(GamePlayerMessage {
            turn_starts_at: Some(time),
            ..
        }) = self.players.get(next_player_number as usize)
        {
            self.ctx.prev_to_next_turn = (self.ctx.current_time, Duration::from_secs(*time));
        }

        self.board_changes.clear();
        for board_change in changes.iter().filter_map(|c| match c {
            Change::Board(change) => Some(change),
            _ => None,
        }) {
            self.board_changes
                .insert(board_change.detail.coordinate, board_change.clone());
        }

        for hand_change in changes.iter().filter_map(|c| match c {
            Change::Hand(change) => Some(change),
            _ => None,
        }) {
            for removed in &hand_change.removed {
                self.hand.remove(
                    self.hand
                        .iter()
                        .position(|t| t == removed)
                        .expect("Player doesn't have tile being removed"),
                );
            }
            let reduced_length = self.hand.len();
            self.hand.0.extend(&hand_change.added);
            self.new_hand_tiles = (reduced_length..self.hand.len()).collect();
        }

        self.time_changes = changes
            .iter()
            .filter_map(|change| match change {
                Change::Time(time_change) => Some(time_change.clone()),
                _ => None,
            })
            .collect();

        self.turn_reports.push(changes);

        // TODO: Verify that our modified hand matches the actual hand in GameStateMessage

        self.ctx.playing_tile = None;
        self.ctx.error_msg = None;
    }
}
