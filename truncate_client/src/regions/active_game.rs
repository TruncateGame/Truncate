use epaint::{emath::Align2, hex_color, vec2, Color32, Rect, Stroke, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage, RoomCode},
    player::Hand,
    reporting::{BoardChange, Change, TimeChange},
};

use eframe::{
    egui::{self, Frame, Label, Layout, Margin, Order, RichText, ScrollArea, Sense},
    emath::Align,
};
use hashbrown::HashMap;

use crate::{
    lil_bits::{BattleUI, BoardUI, HandUI, TimerUI},
    utils::{mapper::MappedBoard, Theme},
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
    pub room_code: RoomCode,
    pub player_number: u64,
    pub next_player_number: u64,
    pub selected_tile_in_hand: Option<usize>,
    pub released_tile: Option<(usize, Coordinate)>,
    pub selected_square_on_board: Option<Coordinate>,
    pub hovered_tile_on_board: Option<HoveredRegion>,
    pub playing_tile: Option<char>,
    pub error_msg: Option<String>,
    pub map_texture: TextureHandle,
    pub player_colors: Vec<Color32>,
    pub board_zoom: f32,
    pub board_pan: Vec2,
    pub sidebar_visible: bool,
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
                room_code,
                player_number,
                next_player_number,
                selected_tile_in_hand: None,
                released_tile: None,
                selected_square_on_board: None,
                hovered_tile_on_board: None,
                playing_tile: None,
                error_msg: None,
                map_texture: map_texture.clone(),
                player_colors: player_colors.clone(),
                board_zoom: 1.0,
                board_pan: vec2(0.0, 0.0),
                sidebar_visible: true,
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

    pub fn render_control_strip(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        winner: Option<usize>,
    ) {
        let area = egui::Area::new(egui::Id::new("controls_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_BOTTOM, vec2(0.0, 0.0));

        let avail_width = ui.available_width();

        let resp = area.show(ui.ctx(), |ui| {
            ui.allocate_ui_with_layout(
                vec2(avail_width, 10.0),
                Layout::top_down(Align::LEFT),
                |ui| {
                    let (hand_alloc, _) =
                        ui.allocate_at_least(vec2(ui.available_width(), 50.0), Sense::hover());
                    let mut hand_ui = ui.child_ui(hand_alloc, Layout::top_down(Align::LEFT));
                    HandUI::new(&mut self.hand)
                        .active(self.ctx.player_number == self.ctx.next_player_number)
                        .render(&mut self.ctx, &mut hand_ui);

                    ui.add_space(10.0);

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

                    ui.add_space(10.0);
                },
            );
        });
        // TODO: Paint this to an appropriate layer, or do sizing within the area above and paint it there.
        // ui.painter().rect_filled(resp.response.rect, 0.0, hex_color!("#ff000088"));
    }

    pub fn render_mobile_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        winner: Option<usize>,
    ) {
        /* TODO: Mobile sidebar that slides over the board */
    }

    pub fn render_sidebar(&mut self, ui: &mut egui::Ui, theme: &Theme, winner: Option<usize>) {
        if !self.ctx.sidebar_visible {
            return;
        }

        let area = egui::Area::new(egui::Id::new("sidebar_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::RIGHT_TOP, vec2(0.0, 0.0));

        let mut outer_sidebar_area = ui.max_rect().shrink2(vec2(0.0, 8.0));
        outer_sidebar_area.set_right(outer_sidebar_area.right() - 8.0);
        let inner_sidebar_area = outer_sidebar_area.shrink(8.0);

        let resp = area.show(ui.ctx(), |ui| {
            ui.painter()
                .rect_filled(outer_sidebar_area, 4.0, hex_color!("#111111aa"));

            ui.allocate_ui_at_rect(inner_sidebar_area, |ui| {
                ScrollArea::new([false, true]).show(ui, |ui| {
                    ui.label(RichText::new("Game history").color(Color32::WHITE));
                    ui.separator();

                    if let Some(error) = &self.ctx.error_msg {
                        ui.label(error);
                        ui.separator();
                    }

                    for turn in self.turn_reports.iter() {
                        for placement in turn.iter().filter_map(|change| match change {
                            Change::Board(placement) => Some(placement),
                            _ => None,
                        }) {
                            let Square::Occupied(player, tile) = placement.detail.square else {
                                                continue;
                                            };
                            let Some(player) = self.players.get(player) else {
                                                continue;
                                            };

                            match placement.action {
                                truncate_core::reporting::BoardChangeAction::Added => {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} placed the tile {}",
                                            player.name, tile
                                        ))
                                        .color(Color32::WHITE),
                                    );
                                }
                                truncate_core::reporting::BoardChangeAction::Swapped => {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} swapped the tile {}",
                                            player.name, tile
                                        ))
                                        .color(Color32::WHITE),
                                    );
                                }
                                _ => {}
                            }
                        }

                        for battle in turn.iter().filter_map(|change| match change {
                            Change::Battle(battle) => Some(battle),
                            _ => None,
                        }) {
                            BattleUI::new(battle).render(&mut self.ctx, ui);
                        }
                        ui.separator();
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
    ) -> Option<PlayerMessage> {
        // We have to go through the instant crate as
        // most std time functions are not implemented
        // in Rust's wasm targets.
        // instant::SystemTime::now() conditionally uses
        // a js function on wasm targets, and otherwise aliases
        // to the std SystemTime type.
        self.ctx.current_time = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("Please don't play Truncate earlier than 1970");

        let mut game_space = ui.available_rect_before_wrap();
        let mut sidebar_space = game_space.clone();
        sidebar_space.set_left(sidebar_space.right() - 300.0);

        if ui.available_size().x >= self.ctx.theme.mobile_breakpoint {
            game_space.set_right(game_space.right() - 300.0);
        }

        let mut game_space_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));
        self.render_control_strip(&mut game_space_ui, theme, winner);

        let mut sidebar_space_ui = ui.child_ui(sidebar_space, Layout::top_down(Align::LEFT));
        self.render_sidebar(&mut sidebar_space_ui, theme, winner);

        let player_message = BoardUI::new(&self.board).render(
            &self.hand,
            &self.board_changes,
            winner.clone(),
            &mut self.ctx,
            &mut game_space_ui,
            &self.mapped_board,
        );

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
