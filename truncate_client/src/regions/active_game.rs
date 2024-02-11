use epaint::{emath::Align2, hex_color, vec2, Color32, FontId, Rect, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    board::{Board, Coordinate},
    generation::BoardSeed,
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage, RoomCode},
    player::Hand,
    reporting::{BoardChange, BoardChangeAction, BoardChangeDetail, Change, TimeChange},
};

use eframe::{
    egui::{self, CursorIcon, Layout, Order, ScrollArea, Sense},
    emath::Align,
};
use hashbrown::HashMap;

use crate::{
    lil_bits::{BattleUI, BoardUI, DictionaryUI, HandUI, TimerUI},
    utils::{
        depot::{
            AestheticDepot, AudioDepot, BoardDepot, GameplayDepot, InteractionDepot, RegionDepot,
            TimingDepot, TruncateDepot, UIStateDepot,
        },
        macros::tr_log,
        mapper::{MappedBoard, MappedTiles},
        tex::{render_tex_quad, render_tex_quads, tiles},
        text::TextHelper,
        timing::get_qs_tick,
        Lighten, Theme,
    },
};

#[derive(Clone, Default, Debug)]
pub enum HeaderType {
    #[default]
    Timers,
    Summary {
        title: String,
        sentinel: char,
        attempt: Option<usize>,
    },
    None,
}

#[derive(Clone)]
pub enum GameLocation {
    Local,
    Online,
}

#[derive(Clone)]
pub struct ActiveGame {
    pub depot: TruncateDepot,
    pub players: Vec<GamePlayerMessage>,
    pub board: Board,
    pub mapped_board: MappedBoard,
    pub mapped_hand: MappedTiles,
    pub mapped_overlay: MappedTiles,
    pub hand: Hand,
    pub board_changes: HashMap<Coordinate, BoardChange>,
    pub new_hand_tiles: Vec<usize>,
    pub time_changes: Vec<TimeChange>,
    pub turn_reports: Vec<Vec<Change>>,
    pub location: GameLocation,
    pub dictionary_ui: Option<DictionaryUI>,
}

impl ActiveGame {
    pub fn new(
        ctx: &egui::Context,
        room_code: RoomCode,
        game_seed: Option<BoardSeed>,
        players: Vec<GamePlayerMessage>,
        player_number: u64,
        next_player_number: u64,
        board: Board,
        hand: Hand,
        map_texture: TextureHandle,
        theme: Theme,
        location: GameLocation,
    ) -> Self {
        let player_colors = players
            .iter()
            .map(|p| Color32::from_rgb(p.color.0, p.color.1, p.color.2))
            .collect::<Vec<_>>();

        let mut depot = TruncateDepot {
            interactions: InteractionDepot::default(),
            regions: RegionDepot::default(),
            ui_state: UIStateDepot::default(),
            board_info: BoardDepot {
                board_seed: game_seed,
                ..BoardDepot::default()
            },
            timing: TimingDepot::default(),
            gameplay: GameplayDepot {
                room_code,
                player_number,
                next_player_number,
                error_msg: None,
                winner: None,
                changes: Vec::new(),
                last_battle_origin: None,
            },
            aesthetics: AestheticDepot {
                theme,
                qs_tick: 0,
                map_texture,
                player_colors,
                destruction_tick: 0.05,
                destruction_duration: 0.6,
            },
            audio: AudioDepot::default(),
        };

        #[cfg(target_arch = "wasm32")]
        {
            let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
            depot.audio.muted = local_storage
                .get_item("truncate_muted")
                .unwrap()
                .unwrap_or_default()
                .parse()
                .unwrap_or_default();
        }

        Self {
            mapped_board: MappedBoard::new(ctx, &depot.aesthetics, &board, player_number as usize),
            mapped_hand: MappedTiles::new(ctx, 7),
            mapped_overlay: MappedTiles::new(ctx, 1),
            depot,
            players,
            board,
            hand,
            board_changes: HashMap::new(),
            new_hand_tiles: vec![],
            time_changes: vec![],
            turn_reports: vec![],
            location,
            dictionary_ui: None,
        }
    }
}

impl ActiveGame {
    pub fn render_header_strip(
        &mut self,
        ui: &mut egui::Ui,
        game_ref: Option<&truncate_core::game::Game>,
    ) -> (Option<Rect>, Option<PlayerMessage>) {
        if matches!(self.depot.ui_state.game_header, HeaderType::None) {
            return (None, None);
        }

        let timer_area = ui.available_rect_before_wrap();
        let avail_width = ui.available_width();
        let mut msg = None;

        let area = egui::Area::new(egui::Id::new("timers_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_TOP, vec2(timer_area.left(), timer_area.top()));

        let resp = area.show(ui.ctx(), |ui| {
            // TODO: We can likely use Memory::area_rect now instead of tracking sizes ourselves
            if let Some(bg_rect) = self.depot.regions.headers_total_rect {
                ui.painter().clone().rect_filled(
                    bg_rect,
                    0.0,
                    self.depot.aesthetics.theme.water.gamma_multiply(0.9),
                );
            }

            ui.add_space(5.0);

            ui.allocate_ui_with_layout(
                vec2(avail_width, 10.0),
                Layout::right_to_left(Align::TOP),
                |ui| {
                    ui.expand_to_include_x(timer_area.left());
                    ui.expand_to_include_x(timer_area.right());

                    ui.spacing_mut().item_spacing = Vec2::splat(0.0);
                    let item_spacing = 10.0;
                    let button_size = 48.0;

                    if !self.depot.ui_state.is_mobile {
                        ui.add_space(item_spacing);
                        let (mut sidebar_button_rect, sidebar_button_resp) =
                            ui.allocate_exact_size(Vec2::splat(button_size), Sense::click());
                        if sidebar_button_resp.hovered() {
                            sidebar_button_rect = sidebar_button_rect.translate(vec2(0.0, -2.0));
                            ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                        }

                        if !self.depot.ui_state.sidebar_toggled {
                            if self.depot.ui_state.unread_sidebar {
                                render_tex_quads(
                                    &[tiles::quad::INFO_BUTTON, tiles::quad::BUTTON_NOTIFICATION],
                                    sidebar_button_rect,
                                    &self.depot.aesthetics.map_texture,
                                    ui,
                                );
                            } else {
                                render_tex_quad(
                                    tiles::quad::INFO_BUTTON,
                                    sidebar_button_rect,
                                    &self.depot.aesthetics.map_texture,
                                    ui,
                                );
                            }
                        } else {
                            render_tex_quad(
                                tiles::quad::TRI_EAST_BUTTON,
                                sidebar_button_rect,
                                &self.depot.aesthetics.map_texture,
                                ui,
                            );
                        }

                        if sidebar_button_resp.clicked() {
                            self.depot.ui_state.sidebar_toggled =
                                !self.depot.ui_state.sidebar_toggled;
                            self.depot.ui_state.unread_sidebar = false;
                        }

                        ui.add_space(item_spacing);
                    }

                    let remaining_width = ui.available_width();
                    let total_width = 700.0_f32.min(remaining_width);
                    let padding = (remaining_width - total_width) / 2.0;

                    ui.add_space(padding);

                    match &self.depot.ui_state.game_header {
                        HeaderType::Timers => {
                            ui.add_space(item_spacing);

                            let timer_width = (total_width - item_spacing * 3.0) / 2.0;

                            if let Some(player) = self
                                .players
                                .iter()
                                .find(|p| p.index == self.depot.gameplay.player_number as usize)
                            {
                                TimerUI::new(player, &self.depot, &self.time_changes)
                                    .friend(true)
                                    .active(
                                        player.index
                                            == self.depot.gameplay.next_player_number as usize,
                                    )
                                    .render(Some(timer_width), false, ui);
                            }

                            ui.add_space(item_spacing);

                            if let Some(opponent) = self
                                .players
                                .iter()
                                .find(|p| p.index != self.depot.gameplay.player_number as usize)
                            {
                                TimerUI::new(opponent, &self.depot, &self.time_changes)
                                    .friend(false)
                                    .active(
                                        opponent.index
                                            == self.depot.gameplay.next_player_number as usize,
                                    )
                                    .right_align()
                                    .render(Some(timer_width), false, ui);
                            }

                            ui.add_space(item_spacing);
                        }
                        HeaderType::Summary {
                            title,
                            sentinel,
                            attempt,
                        } => {
                            let summary_height = 50.0;
                            let summary_width = ui.available_width();
                            let (rect, _) = ui.allocate_exact_size(
                                vec2(summary_width, summary_height),
                                Sense::hover(),
                            );
                            let mut ui = ui.child_ui(rect, Layout::top_down(Align::LEFT));

                            let attempt_str = match attempt {
                                Some(attempt) => {
                                    format!(
                                        "{} attempt{}  {}  ",
                                        attempt + 1,
                                        if *attempt == 0 { "" } else { "s" },
                                        sentinel
                                    )
                                }
                                _ => "".to_string(),
                            };

                            let active_player = self.depot.gameplay.player_number;
                            let summary = if let Some(game) = game_ref {
                                format!(
                                    "{attempt_str}{} move{}",
                                    game.player_turn_count[active_player as usize],
                                    if game.player_turn_count[active_player as usize] == 1 {
                                        ""
                                    } else {
                                        "s"
                                    },
                                )
                            } else {
                                "".to_string()
                            };

                            let mut fz = 14.0;
                            let mut title_text = TextHelper::heavy(title, fz, None, &mut ui);
                            while title_text.mesh_size().x > summary_width {
                                fz -= 1.0;
                                title_text = TextHelper::heavy(title, fz, None, &mut ui)
                            }
                            let title_text_mesh_size = title_text.mesh_size();
                            let title_x_offset = (summary_width - title_text_mesh_size.x) / 2.0;

                            let mut fz = 10.0;
                            let mut summary_text = TextHelper::heavy(&summary, fz, None, &mut ui);
                            while summary_text.mesh_size().x > summary_width {
                                fz -= 1.0;
                                summary_text = TextHelper::heavy(&summary, fz, None, &mut ui);
                            }

                            let summary_text_mesh_size = summary_text.mesh_size();
                            let summary_x_offset = (summary_width - summary_text_mesh_size.x) / 2.0;

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
                                self.depot.aesthetics.theme.text,
                                &mut ui,
                            );
                            ui.add_space(spacing);

                            let (rect, _) = ui.allocate_exact_size(
                                vec2(ui.available_width(), summary_text_mesh_size.y),
                                Sense::hover(),
                            );
                            summary_text.paint_at(
                                rect.min + vec2(summary_x_offset, 0.0),
                                self.depot.aesthetics.theme.text,
                                &mut ui,
                            );

                            ui.add_space(y_offset);
                        }
                        HeaderType::None => unreachable!(),
                    }

                    ui.add_space(item_spacing);
                },
            );

            ui.add_space(10.0);
        });

        self.depot.regions.headers_total_rect = Some(resp.response.rect);

        (Some(resp.response.rect), msg)
    }

    pub fn render_control_strip(
        &mut self,
        ui: &mut egui::Ui,
    ) -> (Option<Rect>, Option<PlayerMessage>) {
        if self.depot.ui_state.hand_hidden {
            return (None, None);
        }

        let mut msg = None;
        let companion_space = 220.0;

        let control_anchor = if !matches!(self.depot.ui_state.game_header, HeaderType::None) {
            vec2(0.0, 0.0)
        } else {
            vec2(0.0, -companion_space)
        };

        if matches!(self.depot.ui_state.game_header, HeaderType::None) {
            let mut companion_pos = ui.available_rect_before_wrap();
            companion_pos.set_top(companion_pos.bottom() - companion_space);
            self.depot.regions.hand_companion_rect = Some(companion_pos);
        }

        let avail_width = ui.available_width();

        let error_area = egui::Area::new(egui::Id::new("error_layer"))
            .movable(false)
            .order(Order::Tooltip)
            .anchor(
                Align2::LEFT_BOTTOM,
                -vec2(
                    0.0,
                    self.depot
                        .regions
                        .hand_total_rect
                        .map(|r| r.height())
                        .unwrap_or_default(),
                ),
            );
        error_area.show(ui.ctx(), |ui| {
            if let Some(error) = &self.depot.gameplay.error_msg {
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
                    let (dialog_rect, _) = crate::utils::tex::paint_dialog_background(
                        false,
                        false,
                        false,
                        dialog_size,
                        hex_color!("#ffe6c9"),
                        &self.depot.aesthetics.map_texture,
                        ui,
                    );

                    let offset = (dialog_rect.size() - text_mesh_size) / 2.0 - vec2(0.0, 3.0);

                    let text_pos = dialog_rect.min + offset;
                    text.paint_at(text_pos, self.depot.aesthetics.theme.text, ui);
                });
            }

            if ui.input_mut(|i| i.pointer.any_click()) {
                self.depot.gameplay.error_msg = None;
            }
        });

        let area = egui::Area::new(egui::Id::new("controls_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_BOTTOM, control_anchor);

        let resp = area.show(ui.ctx(), |ui| {
            // TODO: We can likely use Memory::area_rect now instead of tracking sizes ourselves
            if let Some(bg_rect) = self.depot.regions.hand_total_rect {
                ui.painter().clone().rect_filled(
                    bg_rect,
                    0.0,
                    self.depot.aesthetics.theme.water.gamma_multiply(0.9),
                );
            }

            ui.allocate_ui_with_layout(
                vec2(avail_width, 10.0),
                Layout::top_down(Align::LEFT),
                |ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(0.0);

                    ui.add_space(10.0);

                    if self.depot.gameplay.winner.is_some() {
                        if matches!(self.location, GameLocation::Online) {
                            let text = TextHelper::heavy("REMATCH", 12.0, None, ui);
                            if text
                                .centered_button(
                                    self.depot.aesthetics.theme.button_primary,
                                    self.depot.aesthetics.theme.text,
                                    &self.depot.aesthetics.map_texture,
                                    ui,
                                )
                                .clicked()
                            {
                                msg = Some(PlayerMessage::Rematch);
                            }
                        }
                    }

                    let button_size = 50.0;
                    let item_spacing = 10.0;

                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), 50.0),
                        Layout::right_to_left(Align::TOP),
                        |ui| {
                            ui.add_space(item_spacing);

                            let (mut button_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(button_size), Sense::click());

                            let shrink = button_size - self.depot.ui_state.hand_height_last_frame;
                            button_rect.set_left(button_rect.left() + shrink);
                            button_rect.set_bottom(button_rect.bottom() - shrink);

                            let button_resp = ui.interact(
                                button_rect,
                                ui.id().with("action button"),
                                Sense::click(),
                            );

                            if button_resp.hovered() {
                                button_rect = button_rect.translate(vec2(0.0, -2.0));
                                ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                            }

                            render_tex_quad(
                                if self.depot.ui_state.actions_menu_open {
                                    tiles::quad::TRI_SOUTH_BUTTON
                                } else {
                                    tiles::quad::TRI_NORTH_BUTTON
                                },
                                button_rect,
                                &self.depot.aesthetics.map_texture,
                                ui,
                            );

                            if button_resp.clicked() {
                                self.depot.ui_state.actions_menu_open =
                                    !self.depot.ui_state.actions_menu_open;
                            }

                            ui.add_space(item_spacing);

                            let (hand_alloc, _) = ui.allocate_at_least(
                                vec2(ui.available_width() - item_spacing, 50.0),
                                Sense::hover(),
                            );
                            let mut hand_ui =
                                ui.child_ui(hand_alloc, Layout::top_down(Align::LEFT));
                            let active = self.depot.gameplay.player_number
                                == self.depot.gameplay.next_player_number;
                            HandUI::new(&mut self.hand).active(active).render(
                                &mut hand_ui,
                                &mut self.depot,
                                &mut self.mapped_hand,
                            );
                        },
                    );

                    ui.add_space(10.0);
                },
            );
        });

        self.depot.regions.hand_total_rect = Some(resp.response.rect);

        (Some(resp.response.rect), msg)
    }

    pub fn render_actions_menu(
        &mut self,
        ui: &mut egui::Ui,
        was_open_last_frame: bool,
    ) -> Option<PlayerMessage> {
        let mut msg = None;

        let actions_area = ui.available_rect_before_wrap();
        let area = egui::Area::new(egui::Id::new("actions_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(
                Align2::LEFT_TOP,
                vec2(actions_area.left(), actions_area.top()),
            );

        let actions_alloc = ui.max_rect();
        let inner_actions_area = actions_alloc.shrink2(vec2(10.0, 5.0));
        let menu_spacing = 10.0;

        area.show(ui.ctx(), |ui| {
            ui.painter().clone().rect_filled(
                actions_alloc,
                0.0,
                self.depot.aesthetics.theme.water.gamma_multiply(0.3),
            );

            // Avoid closing the menu _immediately_ on the frame it was opened.
            if was_open_last_frame {
                let interaction = ui.interact(
                    actions_alloc,
                    ui.id().with("actions background"),
                    Sense::click(),
                );
                // Close the menu if they do something like click back on the board or the hand.
                // clicked_elsewhere() checks against the coordinates of the input region,
                // so the comparison below only leaves clicks within the bounds of our area,
                // but not on the area background itself (i.e. on something within)
                if interaction.clicked()
                    || interaction.clicked_elsewhere()
                    || ui.memory(|m| m.is_anything_being_dragged())
                {
                    self.depot.ui_state.actions_menu_open = false;
                }
            }

            ui.allocate_ui_at_rect(inner_actions_area, |ui| {
                ui.expand_to_include_rect(inner_actions_area);
                ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
                    if self.depot.ui_state.is_mobile {
                        let text = TextHelper::heavy("VIEW BATTLES", 14.0, None, ui);
                        if text
                            .button(
                                self.depot.aesthetics.theme.button_secondary,
                                self.depot.aesthetics.theme.text,
                                &self.depot.aesthetics.map_texture,
                                ui,
                            )
                            .clicked()
                        {
                            self.depot.ui_state.sidebar_toggled = true;
                            self.depot.ui_state.actions_menu_open = false;
                        }
                        ui.add_space(menu_spacing);
                    }

                    let text = TextHelper::heavy("OPEN DICTIONARY", 14.0, None, ui);
                    if text
                        .button(
                            self.depot.aesthetics.theme.button_secondary,
                            self.depot.aesthetics.theme.text,
                            &self.depot.aesthetics.map_texture,
                            ui,
                        )
                        .clicked()
                    {
                        self.dictionary_ui = Some(DictionaryUI::new());
                        self.depot.ui_state.actions_menu_open = false;
                    }
                    ui.add_space(menu_spacing);

                    // TODO: Resigning is largely implented for multiplayer games as well, but we need to:
                    // - Resolve why the update isn't being sent from the server
                    // - Show the confirmation modal inside active_game (we only show it in single player)
                    //   otherwise this button is an immediate resign.
                    if matches!(self.location, GameLocation::Local) {
                        let text = TextHelper::heavy("RESIGN", 14.0, None, ui);
                        if text
                            .button(
                                self.depot.aesthetics.theme.button_primary,
                                self.depot.aesthetics.theme.text,
                                &self.depot.aesthetics.map_texture,
                                ui,
                            )
                            .clicked()
                        {
                            msg = Some(PlayerMessage::Resign);
                            self.depot.ui_state.actions_menu_open = false;
                        }
                        ui.add_space(menu_spacing);
                    }

                    let text = if self.depot.audio.muted {
                        TextHelper::heavy("UNMUTE SOUNDS", 14.0, None, ui)
                    } else {
                        TextHelper::heavy("MUTE SOUNDS", 14.0, None, ui)
                    };

                    if text
                        .button(
                            self.depot.aesthetics.theme.button_secondary,
                            self.depot.aesthetics.theme.text,
                            &self.depot.aesthetics.map_texture,
                            ui,
                        )
                        .clicked()
                    {
                        self.depot.audio.muted = !self.depot.audio.muted;

                        #[cfg(target_arch = "wasm32")]
                        {
                            let local_storage =
                                web_sys::window().unwrap().local_storage().unwrap().unwrap();
                            local_storage
                                .set_item("truncate_muted", &self.depot.audio.muted.to_string())
                                .unwrap();
                        }
                    }
                    ui.add_space(menu_spacing);
                });
            });
        });

        msg
    }

    pub fn render_dictionary(&mut self, ui: &mut egui::Ui) -> Option<PlayerMessage> {
        let mut msg = None;
        let mut close_dict = false;

        if let Some(dict_ui) = self.dictionary_ui.as_mut() {
            let area = egui::Area::new(egui::Id::new("dict_layer"))
                .movable(false)
                .order(Order::Foreground)
                .anchor(Align2::RIGHT_TOP, vec2(0.0, 0.0));

            let dict_alloc = ui.max_rect();
            let inner_dict_area = dict_alloc.shrink2(vec2(10.0, 5.0));
            let button_size = 48.0;

            area.show(ui.ctx(), |ui| {
                ui.painter().clone().rect_filled(
                    dict_alloc,
                    0.0,
                    self.depot.aesthetics.theme.water.gamma_multiply(0.9),
                );

                ui.allocate_ui_at_rect(inner_dict_area, |ui| {
                    ui.expand_to_include_rect(inner_dict_area);

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
                                tiles::quad::CLOSE_BUTTON,
                                button_rect,
                                &self.depot.aesthetics.map_texture,
                                ui,
                            );

                            if button_resp.clicked() {
                                close_dict = true;
                            }
                        },
                    );

                    ui.add_space(10.0);

                    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                        msg = dict_ui.render(ui, &mut self.depot);
                    });
                });
            });
        }

        if close_dict {
            self.dictionary_ui = None;
        }
        msg
    }

    pub fn render_sidebar(&mut self, ui: &mut egui::Ui) -> Option<PlayerMessage> {
        if self.depot.ui_state.sidebar_hidden || !self.depot.ui_state.sidebar_toggled {
            return None;
        }

        let area = egui::Area::new(egui::Id::new("sidebar_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::RIGHT_TOP, vec2(0.0, 0.0));

        let sidebar_alloc = ui.max_rect();
        let inner_sidebar_area = sidebar_alloc.shrink2(vec2(10.0, 5.0));
        let button_size = 48.0;

        area.show(ui.ctx(), |ui| {
            ui.painter().clone().rect_filled(
                sidebar_alloc,
                0.0,
                self.depot.aesthetics.theme.water.gamma_multiply(0.9),
            );

            ui.allocate_ui_at_rect(inner_sidebar_area, |ui| {
                ui.expand_to_include_rect(inner_sidebar_area);
                if self.depot.ui_state.is_mobile {
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
                                tiles::quad::CLOSE_BUTTON,
                                button_rect,
                                &self.depot.aesthetics.map_texture,
                                ui,
                            );

                            if button_resp.clicked() {
                                self.depot.ui_state.sidebar_toggled = false;
                            }
                        },
                    );

                    ui.add_space(10.0);
                }

                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                        ScrollArea::new([false, true]).show(ui, |ui| {
                            // Small hack to fill the scroll area
                            ui.allocate_at_least(vec2(ui.available_width(), 1.0), Sense::hover());

                            let room = ui.painter().layout_no_wrap(
                                "Battles".into(),
                                FontId::new(
                                    self.depot.aesthetics.theme.letter_size / 2.0,
                                    egui::FontFamily::Name("Truncate-Heavy".into()),
                                ),
                                self.depot.aesthetics.theme.text,
                            );
                            let (r, _) = ui.allocate_at_least(room.size(), Sense::hover());
                            ui.painter().galley(r.min, room);
                            ui.add_space(15.0);

                            for turn in self.turn_reports.iter().rev() {
                                for battle in turn.iter().filter_map(|change| match change {
                                    Change::Battle(battle) => Some(battle),
                                    _ => None,
                                }) {
                                    BattleUI::new(battle, true).render(ui, &mut self.depot);

                                    ui.add_space(8.0);
                                }
                            }
                        });
                    })
                });
            });
        });

        None
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        current_time: Duration,
        game_ref: Option<&truncate_core::game::Game>,
    ) -> Option<PlayerMessage> {
        self.depot.timing.current_time = current_time;
        let cur_tick = get_qs_tick(current_time);
        if cur_tick > self.depot.aesthetics.qs_tick {
            self.depot.aesthetics.qs_tick = cur_tick;
        }

        if !self.depot.ui_state.is_touch {
            // If we ever receive any touch event,
            // irrevocably put Truncate into touch mode.
            if ui.input(|i| {
                i.events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Touch { .. }))
            }) {
                self.depot.ui_state.is_touch = true;
            }
        }

        let mut game_space = ui.available_rect_before_wrap();
        let mut sidebar_space = game_space.clone();
        let actions_menu_open_last_frame = self.depot.ui_state.actions_menu_open;

        if !self.depot.ui_state.sidebar_hidden
            && ui.available_size().x >= self.depot.aesthetics.theme.mobile_breakpoint
        {
            self.depot.ui_state.is_mobile = false;
        } else {
            if self.depot.ui_state.is_mobile == false {
                // Close the sidebar overlay when transitioning to the mobile breakpoint
                self.depot.ui_state.sidebar_toggled = false;
            }
            self.depot.ui_state.is_mobile = true;
        }

        if !self.depot.ui_state.is_mobile && self.depot.ui_state.sidebar_toggled {
            game_space.set_right(game_space.right() - 300.0);
            sidebar_space.set_left(sidebar_space.right() - 300.0);
        }

        let mut control_strip_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));
        let (control_strip_rect, control_player_message) =
            self.render_control_strip(&mut control_strip_ui);

        let mut timer_strip_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));
        let (timer_strip_rect, timer_player_message) =
            self.render_header_strip(&mut timer_strip_ui, game_ref);

        let mut sidebar_space_ui = ui.child_ui(sidebar_space, Layout::top_down(Align::LEFT));
        let sidebar_player_message = self.render_sidebar(&mut sidebar_space_ui);

        if let Some(timer_strip_rect) = timer_strip_rect {
            game_space.set_top(timer_strip_rect.bottom());
        }
        if let Some(control_strip_rect) = control_strip_rect {
            game_space.set_bottom(control_strip_rect.top());
        }
        let mut game_space_ui = ui.child_ui(game_space, Layout::top_down(Align::LEFT));

        let actions_player_message = if self.depot.ui_state.actions_menu_open {
            self.render_actions_menu(&mut game_space_ui, actions_menu_open_last_frame)
        } else {
            None
        };

        let dict_player_message = self.render_dictionary(ui);

        let player_message = BoardUI::new(&self.board)
            .interactive(!self.depot.interactions.view_only)
            .render(
                &self.hand,
                &self.board_changes,
                &mut game_space_ui,
                &mut self.mapped_board,
                &mut self.mapped_overlay,
                &mut self.depot,
            )
            .or(actions_player_message)
            .or(control_player_message)
            .or(timer_player_message)
            .or(dict_player_message)
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
        if !self.depot.audio.muted {
            // Play the turn sound if the player has changed
            if self.depot.gameplay.next_player_number != next_player_number {
                use eframe::wasm_bindgen::JsCast;

                let window = web_sys::window().expect("window should exist in browser");
                let document = window.document().expect("documnt should exist in window");
                if let Some(element) = document.query_selector("#tr_move").unwrap() {
                    if let Ok(audio) = element.dyn_into::<web_sys::HtmlAudioElement>() {
                        // TODO: Rework audio, as this sound often gets filtered out from headphones
                        _ = audio.play().expect("Audio should be playable");
                    }
                }
            }
        }

        self.depot.gameplay.next_player_number = next_player_number;
        if let Some(GamePlayerMessage {
            turn_starts_no_later_than: Some(time),
            ..
        }) = self.players.get(next_player_number as usize)
        {
            self.depot.timing.last_turn_change = self.depot.timing.current_time;
        }

        self.depot.gameplay.changes = changes.clone();

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

        let battle_occurred = changes
            .iter()
            .any(|change| matches!(change, Change::Battle(_)));

        if battle_occurred {
            self.depot.ui_state.unread_sidebar = true;

            self.depot.gameplay.last_battle_origin =
                changes.iter().find_map(|change| match change {
                    Change::Board(BoardChange {
                        detail: BoardChangeDetail { coordinate, .. },
                        action: BoardChangeAction::Added,
                    }) => Some(*coordinate),
                    _ => None,
                });
        } else {
            self.depot.gameplay.last_battle_origin = None;
        }

        self.turn_reports.push(changes);

        // TODO: Verify that our modified hand matches the actual hand in GameStateMessage

        self.depot.interactions.playing_tile = None;
        self.depot.gameplay.error_msg = None;
    }
}
