use epaint::{
    emath::{Align, Align2},
    hex_color, vec2, Color32, Stroke, TextureHandle, Vec2,
};
use truncate_core::{
    board::Board,
    messages::{LobbyPlayerMessage, PlayerMessage, RoomCode},
};

use eframe::egui::{self, Frame, Layout, Margin, Order, RichText, ScrollArea, Sense};

use crate::{
    lil_bits::EditorUI,
    utils::{
        mapper::MappedBoard,
        tex::{render_tex_quads, Tex, TexQuad},
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
    pub room_code: RoomCode,
    pub players: Vec<LobbyPlayerMessage>,
    pub player_index: u64,
    pub player_colors: Vec<Color32>,
    pub mapped_board: MappedBoard,
    pub map_texture: TextureHandle,
    pub editing_mode: BoardEditingMode,
}

impl Lobby {
    pub fn new(
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
        Self {
            room_code,
            mapped_board: MappedBoard::new(&board, map_texture.clone(), false, &player_colors),
            players,
            player_index,
            player_colors,
            map_texture,
            board,
            editing_mode: BoardEditingMode::None,
        }
    }

    pub fn update_board(&mut self, board: Board) {
        self.mapped_board.remap(&board, &self.player_colors);
        self.board = board;
    }

    pub fn render_sidebar(&mut self, ui: &mut egui::Ui, theme: &Theme) -> Option<PlayerMessage> {
        let mut msg = None;

        let area = egui::Area::new(egui::Id::new("lobby_sidebar_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::RIGHT_TOP, vec2(0.0, 0.0));

        let sidebar_padding = 8.0;

        let mut outer_sidebar_area = ui.max_rect().shrink2(vec2(0.0, sidebar_padding));
        outer_sidebar_area.set_right(outer_sidebar_area.right() - sidebar_padding);
        let inner_sidebar_area = outer_sidebar_area.shrink(sidebar_padding);

        let resp = area.show(ui.ctx(), |ui| {
            ui.painter()
                .rect_filled(outer_sidebar_area, 4.0, hex_color!("#111111aa"));

            ui.allocate_ui_at_rect(inner_sidebar_area, |ui| {
                ui.style_mut().spacing.item_spacing = Vec2::splat(6.0);
                ScrollArea::new([false, true]).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Room Code:").color(Color32::WHITE));
                        let text = TextHelper::heavy(&self.room_code, 14.0, ui);
                        text.paint(Color32::WHITE, ui);
                    });

                    let text = TextHelper::heavy("START GAME", 14.0, ui);
                    if text
                        .full_button(
                            theme.selection.lighten().lighten(),
                            theme.text,
                            &self.map_texture,
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

                    if matches!(self.editing_mode, BoardEditingMode::None) {
                        let text = TextHelper::heavy("EDIT BOARD", 10.0, ui);
                        if text
                            .button(
                                Color32::WHITE.diaphanize(),
                                theme.text,
                                &self.map_texture,
                                ui,
                            )
                            .clicked()
                        {
                            self.editing_mode = BoardEditingMode::Land;
                        }
                    }

                    if !matches!(self.editing_mode, BoardEditingMode::None) {
                        let mut highlights = [None; 5];
                        match self.editing_mode {
                            BoardEditingMode::Land => highlights[0] = Some(theme.selection),
                            BoardEditingMode::Town(0) => highlights[1] = Some(theme.selection),
                            BoardEditingMode::Town(1) => highlights[2] = Some(theme.selection),
                            BoardEditingMode::Dock(0) => highlights[3] = Some(theme.selection),
                            BoardEditingMode::Dock(1) => highlights[4] = Some(theme.selection),
                            _ => unreachable!(
                                "Unknown board editing mode — player count has likely increased"
                            ),
                        }

                        let tiled_button = |quads: Vec<TexQuad>, ui: &mut egui::Ui| {
                            let (mut rect, resp) =
                                ui.allocate_exact_size(Vec2::splat(48.0), Sense::click());
                            if resp.hovered() {
                                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                                rect = rect.translate(vec2(0.0, -2.0));
                            }
                            render_tex_quads(&quads, rect, &self.map_texture, ui);
                            resp
                        };

                        let pcol = |pnum: usize| self.player_colors.get(pnum).copied();

                        ui.label(RichText::new("Land & Water").color(Color32::WHITE));
                        if tiled_button(Tex::land_button(highlights[0]), ui).clicked() {
                            self.editing_mode = BoardEditingMode::Land;
                        }

                        ui.label(RichText::new("Towns").color(Color32::WHITE));
                        ui.horizontal(|ui| {
                            if tiled_button(Tex::town_button(pcol(0), highlights[1]), ui).clicked()
                            {
                                self.editing_mode = BoardEditingMode::Town(0);
                            }

                            if tiled_button(Tex::town_button(pcol(1), highlights[2]), ui).clicked()
                            {
                                self.editing_mode = BoardEditingMode::Town(1);
                            }
                        });

                        ui.label(RichText::new("Docks").color(Color32::WHITE));
                        ui.horizontal(|ui| {
                            if tiled_button(Tex::dock_button(pcol(0), highlights[3]), ui).clicked()
                            {
                                self.editing_mode = BoardEditingMode::Dock(0);
                            }

                            if tiled_button(Tex::dock_button(pcol(1), highlights[4]), ui).clicked()
                            {
                                self.editing_mode = BoardEditingMode::Dock(1);
                            }
                        });

                        ui.label(RichText::new("Actions").color(Color32::WHITE));

                        let text = TextHelper::heavy("GROW BOARD", 10.0, ui);
                        if text
                            .button(Color32::WHITE, theme.text, &self.map_texture, ui)
                            .clicked()
                        {
                            self.board.grow();
                            msg = Some(PlayerMessage::EditBoard(self.board.clone()));
                        }

                        let text = TextHelper::heavy("STOP EDITING BOARD", 10.0, ui);
                        if text
                            .button(
                                Color32::RED.lighten().lighten().lighten(),
                                theme.text,
                                &self.map_texture,
                                ui,
                            )
                            .clicked()
                        {
                            self.editing_mode = BoardEditingMode::None;
                        }
                    };
                });
            });
        });

        msg
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme) -> Option<PlayerMessage> {
        let mut msg = None;

        let mut board_space = ui.available_rect_before_wrap();
        let mut sidebar_space = board_space.clone();
        sidebar_space.set_left(sidebar_space.right() - 300.0);

        if ui.available_size().x >= theme.mobile_breakpoint {
            board_space.set_right(board_space.right() - 300.0);
        }

        let mut sidebar_space_ui = ui.child_ui(sidebar_space, Layout::top_down(Align::LEFT));
        if let Some(board_update) = self.render_sidebar(&mut sidebar_space_ui, theme) {
            msg = Some(board_update);
        }

        let board_space_ui = ui.child_ui(board_space, Layout::top_down(Align::LEFT));

        {
            let mut ui = board_space_ui;

            if let Some(board_update) = EditorUI::new(
                &mut self.board,
                &self.mapped_board,
                &mut self.editing_mode,
            )
            .render(true, &mut ui, theme, &self.map_texture)
            {
                msg = Some(board_update);
            }
        }

        msg
    }
}
