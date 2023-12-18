use epaint::{emath::Align2, hex_color, vec2, Color32, FontId, Rect, Stroke, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    board::{Board, Coordinate, Square},
    generation::BoardSeed,
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage, RoomCode},
    player::Hand,
    reporting::{BoardChange, Change, TimeChange},
};

use eframe::{
    egui::{
        self, CursorIcon, Frame, Label, LayerId, Layout, Margin, Order, RichText, ScrollArea, Sense,
    },
    emath::Align,
};
use hashbrown::HashMap;

use crate::{
    app_outer::Backchannel,
    lil_bits::{BattleUI, BoardUI, HandUI, TimerUI},
    utils::{
        mapper::MappedBoard,
        tex::{render_tex_quad, render_tex_quads, Tex},
        text::TextHelper,
        Diaphanize, Lighten, Theme,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct HoveredRegion {
    pub rect: Rect,
    // If we're hovering the board, what coordinate is it?
    pub coord: Option<Coordinate>,
}

#[derive(Clone)]
pub enum HeaderType {
    Timers,
    Summary {
        title: String,
        sentinel: char,
        attempt: Option<usize>,
    },
    None,
}

#[derive(Clone)]
pub struct GameCtx {
    pub board_seed: Option<BoardSeed>,
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
    pub sidebar_toggled: bool,
    pub sidebar_visible: bool,
    pub header_visible: HeaderType,
    pub headers_total_rect: Option<Rect>,
    pub hand_visible: bool,
    pub hand_total_rect: Option<Rect>,
    pub hand_companion_rect: Option<Rect>,
    pub highlight_tiles: Option<Vec<char>>,
    pub highlight_squares: Option<Vec<Coordinate>>,
    pub is_mobile: bool,
    pub is_touch: bool,
    pub unread_sidebar: bool,
    pub interactive: bool,
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
    pub share_copied: bool,
}

impl ActiveGame {
    pub fn new(
        room_code: RoomCode,
        game_seed: Option<BoardSeed>,
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
                board_seed: game_seed,
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
                sidebar_toggled: false,
                sidebar_visible: true,
                header_visible: HeaderType::Timers,
                hand_visible: true,
                hand_companion_rect: None,
                headers_total_rect: None,
                hand_total_rect: None,
                highlight_tiles: None,
                highlight_squares: None,
                is_mobile: false,
                is_touch: false,
                unread_sidebar: false,
                interactive: true,
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
            share_copied: false,
        }
    }
}

impl ActiveGame {
    pub fn render_header_strip(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        winner: Option<usize>,
        game_ref: Option<&truncate_core::game::Game>,
    ) -> (Option<Rect>, Option<PlayerMessage>) {
        if matches!(self.ctx.header_visible, HeaderType::None) {
            return (None, None);
        }

        let mut msg = None;

        let timer_area = ui.available_rect_before_wrap();
        let avail_width = ui.available_width();

        let area = egui::Area::new(egui::Id::new("timers_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_TOP, vec2(timer_area.left(), timer_area.top()));

        let mut resp = area.show(ui.ctx(), |ui| {
            // TODO: We can likely use Memory::area_rect now instead of tracking sizes ourselves
            if let Some(bg_rect) = self.ctx.headers_total_rect {
                ui.painter().clone().rect_filled(
                    bg_rect,
                    0.0,
                    self.ctx.theme.water.gamma_multiply(0.75),
                );
            }

            ui.add_space(5.0);

            ui.allocate_ui_with_layout(
                vec2(avail_width, 10.0),
                Layout::left_to_right(Align::TOP),
                |ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(0.0);
                    let button_size = 48.0;
                    let mut total_width = 700.0;

                    if self.ctx.is_mobile {
                        if total_width + button_size + 10.0 > ui.available_width() {
                            total_width = ui.available_width() - button_size - 10.0;
                        }
                    } else {
                        if total_width + 10.0 > ui.available_width() {
                            total_width = ui.available_width() - 10.0;
                        }
                    }

                    let item_spacing = 10.0;
                    let outer_x_padding = if !self.ctx.is_mobile {
                        (ui.available_width() - total_width) / 2.0
                    } else {
                        0.0
                    };
                    ui.add_space(outer_x_padding);

                    match &self.ctx.header_visible {
                        HeaderType::Timers => {
                            ui.add_space(item_spacing);

                            let timer_width = (total_width - item_spacing * 3.0) / 2.0;

                            if let Some(player) = self
                                .players
                                .iter()
                                .find(|p| p.index == self.ctx.player_number as usize)
                            {
                                TimerUI::new(player, self.ctx.current_time, &self.time_changes)
                                    .friend(true)
                                    .active(player.index == self.ctx.next_player_number as usize)
                                    .winner(winner.clone())
                                    .render(Some(timer_width), false, ui, theme, &mut self.ctx);
                            }

                            ui.add_space(item_spacing);

                            if let Some(opponent) = self
                                .players
                                .iter()
                                .find(|p| p.index != self.ctx.player_number as usize)
                            {
                                TimerUI::new(opponent, self.ctx.current_time, &self.time_changes)
                                    .friend(false)
                                    .active(opponent.index == self.ctx.next_player_number as usize)
                                    .winner(winner.clone())
                                    .right_align()
                                    .render(Some(timer_width), false, ui, theme, &mut self.ctx);
                            }

                            ui.add_space(item_spacing);
                        }
                        HeaderType::Summary {
                            title,
                            sentinel,
                            attempt,
                        } => {
                            let summary_height = 50.0;
                            let (rect, _) = ui.allocate_exact_size(
                                vec2(total_width, summary_height),
                                Sense::hover(),
                            );
                            let mut ui = ui.child_ui(rect, Layout::top_down(Align::LEFT));

                            let attempt_str = match attempt {
                                Some(attempt) if *attempt > 0 => {
                                    format!("{} attempts  {}  ", attempt + 1, sentinel)
                                }
                                _ => "".to_string(),
                            };

                            let summary = if let Some(game) = game_ref {
                                format!(
                                    "{}{} turn{}  {}  {} battle{}",
                                    attempt_str,
                                    game.player_turn_count[0],
                                    if game.player_turn_count[0] == 1 {
                                        ""
                                    } else {
                                        "s"
                                    },
                                    sentinel,
                                    game.battle_count,
                                    if game.battle_count == 1 { "" } else { "s" },
                                )
                            } else {
                                "".to_string()
                            };

                            let title_text = TextHelper::heavy(title, 14.0, None, &mut ui);
                            let title_text_mesh_size = title_text.mesh_size();
                            let title_x_offset = (total_width - title_text_mesh_size.x) / 2.0;

                            let summary_text = TextHelper::heavy(&summary, 10.0, None, &mut ui);
                            let summary_text_mesh_size = summary_text.mesh_size();
                            let summary_x_offset = (total_width - summary_text_mesh_size.x) / 2.0;

                            let spacing = 5.0;
                            let y_offset = (summary_height
                                - summary_text_mesh_size.y
                                - title_text_mesh_size.y)
                                / 2.0;
                            ui.add_space(y_offset);

                            let (rect, _) = ui.allocate_exact_size(
                                vec2(ui.available_width(), title_text_mesh_size.y),
                                Sense::hover(),
                            );
                            title_text.paint_at(
                                rect.min + vec2(title_x_offset, 0.0),
                                self.ctx.theme.text,
                                &mut ui,
                            );
                            ui.add_space(spacing);

                            let (rect, _) = ui.allocate_exact_size(
                                vec2(ui.available_width(), summary_text_mesh_size.y),
                                Sense::hover(),
                            );
                            summary_text.paint_at(
                                rect.min + vec2(summary_x_offset, 0.0),
                                self.ctx.theme.text,
                                &mut ui,
                            );

                            ui.add_space(y_offset);
                        }
                        HeaderType::None => unreachable!(),
                    }

                    if !self.ctx.is_mobile {
                        ui.add_space(outer_x_padding);
                    } else {
                        let (mut button_rect, button_resp) =
                            ui.allocate_exact_size(Vec2::splat(button_size), Sense::click());
                        if button_resp.hovered() {
                            button_rect = button_rect.translate(vec2(0.0, -2.0));
                            ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                        }

                        if self.ctx.unread_sidebar {
                            render_tex_quads(
                                &[Tex::BUTTON_INFO, Tex::BUTTON_NOTIF],
                                button_rect,
                                &self.ctx.map_texture,
                                ui,
                            );
                        } else {
                            render_tex_quad(
                                Tex::BUTTON_INFO,
                                button_rect,
                                &self.ctx.map_texture,
                                ui,
                            );
                        }

                        if button_resp.clicked() {
                            self.ctx.sidebar_toggled = !self.ctx.sidebar_toggled;
                            self.ctx.unread_sidebar = false;
                        }

                        ui.add_space(item_spacing);
                    }
                },
            );

            ui.add_space(10.0);
        });

        self.ctx.headers_total_rect = Some(resp.response.rect);

        (Some(resp.response.rect), msg)
    }

    pub fn render_control_strip(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        winner: Option<usize>,
        backchannel: Option<&Backchannel>,
        game_ref: Option<&truncate_core::game::Game>,
    ) -> (Option<Rect>, Option<PlayerMessage>) {
        if !self.ctx.hand_visible {
            return (None, None);
        }

        let mut msg = None;
        let companion_space = 220.0;

        let control_anchor = if !matches!(self.ctx.header_visible, HeaderType::None) {
            vec2(0.0, 0.0)
        } else {
            vec2(0.0, -companion_space)
        };

        if matches!(self.ctx.header_visible, HeaderType::None) {
            let mut companion_pos = ui.available_rect_before_wrap();
            companion_pos.set_top(companion_pos.bottom() - companion_space);
            self.ctx.hand_companion_rect = Some(companion_pos);
        }

        let avail_width = ui.available_width();

        let error_area = egui::Area::new(egui::Id::new("error_layer"))
            .movable(false)
            .order(Order::Tooltip)
            .anchor(
                Align2::LEFT_BOTTOM,
                -vec2(
                    0.0,
                    self.ctx
                        .hand_total_rect
                        .map(|r| r.height())
                        .unwrap_or_default(),
                ),
            );
        let mut resp = error_area.show(ui.ctx(), |ui| {
            if let Some(error) = &self.ctx.error_msg {
                let error_fz = if avail_width < 550.0 { 24.0 } else { 32.0 };
                let max_width = f32::min(600.0, avail_width - 100.0);
                let text = TextHelper::light(error, error_fz, Some(max_width), ui);
                let text_mesh_size = text.mesh_size();
                let dialog_size = text_mesh_size + vec2(100.0, 20.0);
                let x_offset = (avail_width - dialog_size.x) / 2.0;

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(0.0);
                    ui.add_space(x_offset);
                    let (dialog_rect, dialog_resp) = crate::utils::tex::paint_dialog_background(
                        false,
                        false,
                        false,
                        dialog_size,
                        hex_color!("#ffe6c9"),
                        &self.ctx.map_texture,
                        ui,
                    );

                    let offset = (dialog_rect.size() - text_mesh_size) / 2.0 - vec2(0.0, 3.0);

                    let text_pos = dialog_rect.min + offset;
                    text.paint_at(text_pos, self.ctx.theme.text, ui);
                });
            }

            if ui.input_mut(|i| i.pointer.any_click()) {
                self.ctx.error_msg = None;
            }
        });

        let area = egui::Area::new(egui::Id::new("controls_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_BOTTOM, control_anchor);

        let resp = area.show(ui.ctx(), |ui| {
            // TODO: We can likely use Memory::area_rect now instead of tracking sizes ourselves
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
                    ui.spacing_mut().item_spacing = Vec2::splat(0.0);

                    ui.add_space(10.0);

                    if let Some(winner) = winner {
                        if let Some(BoardSeed { day: Some(_), .. }) = &self.ctx.board_seed {
                            if winner as u64 != self.ctx.player_number {
                                let text = TextHelper::heavy("TRY AGAIN", 12.0, None, ui);
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
                        } else {
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

                        let msg = if self.share_copied {
                            "COPIED TEXT!"
                        } else {
                            "SHARE"
                        };
                        let text = TextHelper::heavy(msg, 12.0, None, ui);
                        let share_button = text.centered_button(
                            theme.selection.lighten().lighten(),
                            theme.text,
                            &self.ctx.map_texture,
                            ui,
                        );
                        if share_button.clicked()
                            || share_button.drag_started()
                            || share_button.is_pointer_button_down_on()
                        {
                            #[allow(unused_mut)]
                            let mut url_prefix = "".to_string();

                            #[cfg(target_arch = "wasm32")]
                            {
                                let host = web_sys::window()
                                    .unwrap()
                                    .location()
                                    .host()
                                    .unwrap_or_else(|_| "truncate.town".into());
                                url_prefix = format!("https://{host}/#");
                            }

                            let attempt = match self.ctx.header_visible {
                                HeaderType::Summary { attempt, .. } => attempt,
                                _ => None,
                            };

                            let text = self.board.emojify(
                                self.ctx.player_number as usize,
                                Some(winner),
                                game_ref,
                                self.ctx.board_seed.clone(),
                                attempt,
                                url_prefix,
                            );

                            if let Some(backchannel) = backchannel {
                                if backchannel.is_open() {
                                    backchannel
                                        .send_msg(crate::app_outer::BackchannelMsg::Copy { text });
                                } else {
                                    ui.ctx().output_mut(|o| o.copied_text = text.clone());
                                }
                            } else {
                                ui.ctx().output_mut(|o| o.copied_text = text.clone());
                            }

                            self.share_copied = true;
                        }
                    }

                    let (hand_alloc, _) =
                        ui.allocate_at_least(vec2(ui.available_width(), 50.0), Sense::hover());
                    let mut hand_ui = ui.child_ui(hand_alloc, Layout::top_down(Align::LEFT));
                    HandUI::new(&mut self.hand)
                        .active(self.ctx.player_number == self.ctx.next_player_number)
                        .render(&mut self.ctx, &mut hand_ui);

                    ui.add_space(10.0);
                },
            );
        });

        self.ctx.hand_total_rect = Some(resp.response.rect);

        (Some(resp.response.rect), msg)
    }

    pub fn render_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        winner: Option<usize>,
    ) -> Option<PlayerMessage> {
        let mut msg = None;

        if !self.ctx.sidebar_visible || (self.ctx.is_mobile && !self.ctx.sidebar_toggled) {
            return msg;
        }

        let area = egui::Area::new(egui::Id::new("sidebar_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::RIGHT_TOP, vec2(0.0, 0.0));

        let sidebar_alloc = ui.max_rect();
        let inner_sidebar_area = sidebar_alloc.shrink2(vec2(10.0, 5.0));
        let button_size = 48.0;

        let resp = area.show(ui.ctx(), |ui| {
            ui.painter().clone().rect_filled(
                sidebar_alloc,
                0.0,
                self.ctx.theme.water.gamma_multiply(0.9),
            );

            ui.allocate_ui_at_rect(inner_sidebar_area, |ui| {
                ui.expand_to_include_rect(inner_sidebar_area);
                if self.ctx.is_mobile {
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), button_size),
                        Layout::right_to_left(Align::TOP),
                        |ui| {
                            let (mut button_rect, button_resp) =
                                ui.allocate_exact_size(Vec2::splat(button_size), Sense::click());
                            if button_resp.hovered() {
                                button_rect = button_rect.translate(vec2(0.0, -2.0));
                                ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                            }
                            render_tex_quad(
                                Tex::BUTTON_CLOSE,
                                button_rect,
                                &self.ctx.map_texture,
                                ui,
                            );

                            if button_resp.clicked() {
                                self.ctx.sidebar_toggled = false;
                            }
                        },
                    );

                    ui.add_space(10.0);
                }

                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    // let text = TextHelper::heavy("RESIGN", 12.0, None, ui);
                    // if text
                    //     .full_button(
                    //         Color32::RED.diaphanize(),
                    //         theme.text,
                    //         &self.ctx.map_texture,
                    //         ui,
                    //     )
                    //     .clicked()
                    // {
                    //     // TODO
                    // }

                    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                        ScrollArea::new([false, true]).show(ui, |ui| {
                            // Small hack to fill the scroll area
                            ui.allocate_at_least(vec2(ui.available_width(), 1.0), Sense::hover());

                            let room = ui.painter().layout_no_wrap(
                                "Battles".into(),
                                FontId::new(
                                    self.ctx.theme.letter_size / 2.0,
                                    egui::FontFamily::Name("Truncate-Heavy".into()),
                                ),
                                self.ctx.theme.text,
                            );
                            let (r, _) = ui.allocate_at_least(room.size(), Sense::hover());
                            ui.painter().galley(r.min, room);
                            ui.add_space(15.0);

                            let mut rendered_battles = 0;
                            let label_font =
                                FontId::new(8.0, egui::FontFamily::Name("Truncate-Heavy".into()));

                            for turn in self.turn_reports.iter().rev() {
                                for battle in turn.iter().filter_map(|change| match change {
                                    Change::Battle(battle) => Some(battle),
                                    _ => None,
                                }) {
                                    let is_latest_battle = rendered_battles == 0;

                                    if let Some(label) = if is_latest_battle {
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
                                        let (r, _) =
                                            ui.allocate_at_least(label.size(), Sense::hover());
                                        ui.painter().galley(r.min, label);
                                    }

                                    BattleUI::new(battle, is_latest_battle)
                                        .render(&mut self.ctx, ui);
                                    rendered_battles += 1;
                                    ui.add_space(8.0);
                                }
                            }
                        });
                    })
                });
            });
        });

        msg
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        winner: Option<usize>,
        current_time: Duration,
        backchannel: Option<&Backchannel>,
        game_ref: Option<&truncate_core::game::Game>,
    ) -> Option<PlayerMessage> {
        self.ctx.current_time = current_time;
        let cur_tick = current_time.as_secs() * 4 + current_time.subsec_millis() as u64 / 250;
        if cur_tick > self.ctx.qs_tick {
            self.ctx.qs_tick = cur_tick;
            self.mapped_board
                .remap(&self.board, &self.ctx.player_colors, self.ctx.qs_tick);

            self.mapped_board.remap_texture(
                ui.ctx(),
                &self.board,
                &self.ctx.player_colors,
                self.ctx.qs_tick,
                false,
            );
        }

        if !self.ctx.is_touch {
            // If we ever receive any touch event,
            // irrevocably put Truncate into touch mode.
            if ui.input(|i| {
                i.events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Touch { .. }))
            }) {
                self.ctx.is_touch = true;
            }
        }

        let mut game_space = ui.available_rect_before_wrap();
        let mut sidebar_space = game_space.clone();

        if self.ctx.sidebar_visible && ui.available_size().x >= self.ctx.theme.mobile_breakpoint {
            self.ctx.is_mobile = false;
            game_space.set_right(game_space.right() - 300.0);
            sidebar_space.set_left(sidebar_space.right() - 300.0);
        } else {
            self.ctx.is_mobile = true;
        }

        let mut control_strip_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));
        let (control_strip_rect, control_player_message) =
            self.render_control_strip(&mut control_strip_ui, theme, winner, backchannel, game_ref);

        let mut timer_strip_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));
        let (timer_strip_rect, timer_player_message) =
            self.render_header_strip(&mut timer_strip_ui, theme, winner, game_ref);

        let mut sidebar_space_ui = ui.child_ui(sidebar_space, Layout::top_down(Align::LEFT));
        let sidebar_player_message = self.render_sidebar(&mut sidebar_space_ui, theme, winner);

        if let Some(timer_strip_rect) = timer_strip_rect {
            game_space.set_top(timer_strip_rect.bottom());
        }
        if let Some(control_strip_rect) = control_strip_rect {
            game_space.set_bottom(control_strip_rect.top());
        }
        let mut game_space_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));

        let player_message = BoardUI::new(&self.board)
            .interactive(self.ctx.interactive)
            .render(
                &self.hand,
                &self.board_changes,
                winner.clone(),
                &mut self.ctx,
                &mut game_space_ui,
                &self.mapped_board,
            )
            .or(control_player_message)
            .or(timer_player_message)
            .or(sidebar_player_message);

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

        if changes
            .iter()
            .any(|change| matches!(change, Change::Battle(_)))
        {
            self.ctx.unread_sidebar = true;
        }

        self.turn_reports.push(changes);

        // TODO: Verify that our modified hand matches the actual hand in GameStateMessage

        self.ctx.playing_tile = None;
        self.ctx.error_msg = None;
    }
}
