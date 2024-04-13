use epaint::{emath::Align2, vec2};

use truncate_core::messages::PlayerMessage;

use eframe::{
    egui::{self, Layout, Order, Sense},
    emath::Align,
};

use crate::{
    lil_bits::DictionaryUI,
    utils::{text::TextHelper, urls::back_to_menu},
};

use super::{ActiveGame, GameLocation};

impl ActiveGame {
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

                    // TODO: Resigning is largely implented for multiplayer games as well, but we need to:
                    // - Resolve why the update isn't being sent from the server
                    // - Show the confirmation modal inside active_game (we only show it in single player)
                    //   otherwise this button is an immediate resign.
                    // This intentionally excludes the tutorial
                    if matches!(self.location, GameLocation::Local) {
                        ui.add_space(menu_spacing);
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
                    }

                    ui.add_space(menu_spacing);

                    let text = TextHelper::heavy("BACK TO MENU", 14.0, None, ui);
                    if text
                        .button(
                            self.depot.aesthetics.theme.button_primary,
                            self.depot.aesthetics.theme.text,
                            &self.depot.aesthetics.map_texture,
                            ui,
                        )
                        .clicked()
                    {
                        back_to_menu();
                    }
                    ui.add_space(menu_spacing);
                });
            });
        });

        msg
    }
}
