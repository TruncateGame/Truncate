use epaint::{
    emath::{Align, Align2},
    hex_color, vec2, Color32, Stroke, TextureHandle, Vec2,
};

use instant::Duration;
use truncate_core::{
    board::Board,
    generation::BoardSeed,
    messages::{LobbyPlayerMessage, PlayerMessage, RoomCode},
};

use eframe::egui::{self, Layout, Order, RichText, ScrollArea};

use crate::{
    lil_bits::EditorUI,
    utils::{
        depot::{AestheticDepot, TimingDepot},
        mapper::MappedBoard,
        text::TextHelper,
        Diaphanize, Lighten, Theme,
    },
};

#[derive(Clone)]
pub enum BoardEditingMode {
    None,
    Land,
    Town(usize),
    Dock(usize),
}

#[derive(Clone)]
pub struct Lobby {
    pub board: Board,
    pub board_seed: Option<BoardSeed>,
    pub room_code: RoomCode,
    pub players: Vec<LobbyPlayerMessage>,
    pub player_index: u64,
    pub mapped_board: MappedBoard,
    pub editing_mode: BoardEditingMode,
    pub copied_code: bool,
    pub aesthetics: AestheticDepot,
    pub timing: TimingDepot,
}

impl Lobby {
    pub fn new(
        ctx: &egui::Context,
        room_code: RoomCode,
        players: Vec<LobbyPlayerMessage>,
        player_index: u64,
        board: Board,
        map_texture: TextureHandle,
    ) -> Self {
        let player_colors: Vec<_> = players
            .iter()
            .map(|p| Color32::from_rgb(p.color.0, p.color.1, p.color.2))
            .collect();

        let aesthetics = AestheticDepot {
            theme: Theme::night(),
            qs_tick: 0,
            map_texture,
            player_colors,
            destruction_tick: 0.0,
            destruction_duration: 0.0,
        };

        Self {
            room_code,
            board_seed: None,
            mapped_board: MappedBoard::new(ctx, &aesthetics, &board, 1, true),
            players,
            player_index,
            board,
            editing_mode: BoardEditingMode::None,
            copied_code: false,
            aesthetics,
            timing: TimingDepot::default(),
        }
    }

    pub fn update_board(&mut self, board: Board, ui: &mut egui::Ui) {
        self.mapped_board.remap_texture(
            &ui.ctx(),
            &self.aesthetics,
            &self.timing,
            None,
            None,
            &board,
        );
        self.board = board;
    }

    pub fn render_lobby(&mut self, ui: &mut egui::Ui, theme: &Theme) -> Option<PlayerMessage> {
        let mut msg = None;

        let area = egui::Area::new(egui::Id::new("lobby_sidebar_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::RIGHT_TOP, vec2(0.0, 0.0));

        let sidebar_padding = 8.0;

        let outer_sidebar_area = ui.max_rect();
        let inner_sidebar_area = outer_sidebar_area.shrink(sidebar_padding);

        area.show(ui.ctx(), |ui| {
            ui.painter()
                .rect_filled(outer_sidebar_area, 0.0, hex_color!("#111111aa"));

            ui.allocate_ui_at_rect(inner_sidebar_area, |ui| {
                ui.style_mut().spacing.item_spacing = Vec2::splat(6.0);
                ScrollArea::new([false, true]).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Room Code:").color(Color32::WHITE));
                        let text = TextHelper::heavy(&self.room_code, 14.0, None, ui);
                        text.paint(Color32::WHITE, ui, false);
                    });

                    if self.players.len() == 1 {
                        if self.copied_code {
                            let text = TextHelper::heavy("Copied link", 10.0, None, ui);
                            text.paint(Color32::WHITE, ui, false);
                        }

                        let text = TextHelper::heavy("COPY GAME LINK", 14.0, None, ui);
                        if text
                            .full_button(
                                theme.button_primary,
                                theme.text,
                                &self.aesthetics.map_texture,
                                ui,
                            )
                            .clicked()
                        {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let host = web_sys::window()
                                    .unwrap()
                                    .location()
                                    .host()
                                    .unwrap_or_else(|_| "truncate.town".into());
                                ui.output_mut(|o| {
                                    o.copied_text = format!("https://{host}/#{}", &self.room_code);
                                });
                                self.copied_code = true;
                            }
                        }
                    }

                    let start_button_color = if self.players.len() > 1 {
                        theme.button_primary
                    } else {
                        theme.text.lighten().lighten()
                    };

                    let text = TextHelper::heavy("START GAME", 14.0, None, ui);
                    if text
                        .full_button(
                            start_button_color,
                            theme.text,
                            &self.aesthetics.map_texture,
                            ui,
                        )
                        .clicked()
                    {
                        msg = Some(PlayerMessage::StartGame);
                    }

                    ui.add_space(12.0);

                    // ui.text_edit_singleline(&mut self.players.get_mut(0).unwrap().name);

                    // ui.add_space(12.0);

                    ui.label(RichText::new("Playing as:").color(Color32::WHITE));
                    if let Some(player) = self.players.get_mut(self.player_index as usize) {
                        let input = ui.add(
                            egui::TextEdit::singleline(&mut player.name)
                                .frame(false)
                                .margin(egui::vec2(0.0, 0.0))
                                .min_size(vec2(0.0, theme.letter_size * 0.75))
                                .text_color(Color32::WHITE)
                                .vertical_align(Align::BOTTOM)
                                .font(egui::FontId::new(
                                    theme.letter_size / 2.0,
                                    egui::FontFamily::Name("Truncate-Heavy".into()),
                                )),
                        );

                        if input.changed() {
                            msg = Some(PlayerMessage::EditName(player.name.clone()));

                            #[cfg(target_arch = "wasm32")]
                            {
                                let local_storage =
                                    web_sys::window().unwrap().local_storage().unwrap().unwrap();
                                local_storage
                                    .set_item("truncate_name_history", &player.name)
                                    .unwrap();
                            }
                        }

                        ui.painter().rect_stroke(
                            input.rect.expand2(vec2(4.0, 2.0)),
                            2.0,
                            Stroke::new(1.0, Color32::WHITE),
                        );
                    }

                    ui.label(RichText::new("Other Players in Lobby:").color(Color32::WHITE));
                    for player in &self.players {
                        if player.index == self.player_index as usize {
                            continue;
                        }
                        ui.label(RichText::new(&player.name).color(Color32::WHITE).font(
                            egui::FontId::new(
                                theme.letter_size / 2.0,
                                egui::FontFamily::Name("Truncate-Heavy".into()),
                            ),
                        ));
                    }

                    ui.add_space(32.0);

                    let text = TextHelper::heavy("EDIT BOARD", 10.0, None, ui);
                    if text
                        .button(
                            Color32::WHITE.diaphanize(),
                            theme.text,
                            &self.aesthetics.map_texture,
                            ui,
                        )
                        .clicked()
                    {
                        self.editing_mode = BoardEditingMode::Land;
                    }
                });
            });
        });

        msg
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme) -> Option<PlayerMessage> {
        let mut msg = None;

        let render_space = ui.available_rect_before_wrap();

        if matches!(self.editing_mode, BoardEditingMode::None) {
            let mut lobby_ui = ui.child_ui(render_space, Layout::top_down(Align::LEFT));
            if let Some(board_update) = self.render_lobby(&mut lobby_ui, theme) {
                msg = Some(board_update);
            }
        } else {
            let mut lobby_ui = ui.child_ui(render_space, Layout::bottom_up(Align::RIGHT));
            if let Some(board_update) = EditorUI::new(
                &mut self.board,
                &mut self.mapped_board,
                &mut self.editing_mode,
                &self.aesthetics.player_colors,
            )
            .render(true, &mut lobby_ui, theme, &self.aesthetics.map_texture)
            {
                msg = Some(board_update);
                self.mapped_board.remap_texture(
                    &ui.ctx(),
                    &self.aesthetics,
                    &self.timing,
                    None,
                    None,
                    &self.board,
                );
            }
        }

        msg
    }
}
