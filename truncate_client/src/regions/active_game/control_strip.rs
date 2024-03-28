use epaint::{emath::Align2, hex_color, vec2, Rect, Vec2};

use truncate_core::messages::PlayerMessage;

use eframe::{
    egui::{self, CursorIcon, Layout, Order, Sense},
    emath::Align,
};

use crate::{
    lil_bits::HandUI,
    utils::{
        tex::{render_tex_quad, tiles},
        text::TextHelper,
    },
};

use super::{ActiveGame, GameLocation, HeaderType};

impl ActiveGame {
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

                            ui.add_space(20.0);
                        }
                        if matches!(self.location, GameLocation::Local) {
                            let text = TextHelper::heavy("VIEW RESULTS", 12.0, None, ui);
                            if text
                                .centered_button(
                                    self.depot.aesthetics.theme.button_primary,
                                    self.depot.aesthetics.theme.text,
                                    &self.depot.aesthetics.map_texture,
                                    ui,
                                )
                                .clicked()
                            {
                                msg = Some(PlayerMessage::Resign);
                            }

                            ui.add_space(20.0);
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
}
