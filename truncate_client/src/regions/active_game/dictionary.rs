use epaint::{emath::Align2, vec2, Vec2};

use truncate_core::messages::PlayerMessage;

use eframe::{
    egui::{self, CursorIcon, Layout, Order, Sense},
    emath::Align,
};

use crate::utils::tex::{render_tex_quad, tiles};

use super::ActiveGame;

impl ActiveGame {
    pub fn render_dictionary(&mut self, ui: &mut egui::Ui) -> Option<PlayerMessage> {
        let mut msg = None;

        if let Some(dict_ui) = self.dictionary_ui.as_mut() {
            let area = egui::Area::new(egui::Id::new("dict_layer"))
                .movable(false)
                .order(Order::Foreground)
                .anchor(Align2::RIGHT_TOP, vec2(0.0, 0.0));

            let mut dict_alloc = ui.max_rect();
            if let Some(hand_rect) = self.depot.regions.hand_total_rect {
                dict_alloc.set_bottom(hand_rect.top());
            }
            let inner_dict_area = dict_alloc.shrink2(vec2(10.0, 0.0));
            let button_size = 48.0;

            area.show(ui.ctx(), |ui| {
                if self.depot.ui_state.dictionary_focused {
                    ui.painter().clone().rect_filled(
                        dict_alloc,
                        0.0,
                        self.depot.aesthetics.theme.water.gamma_multiply(0.9),
                    );
                }

                ui.allocate_ui_at_rect(inner_dict_area, |ui| {
                    ui.expand_to_include_rect(inner_dict_area);

                    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                        msg = dict_ui.render(ui, &mut self.depot);
                    });
                });
            });
        }

        // Guard for dictionary being closed (since it can't destroy itself)
        if !self.depot.ui_state.dictionary_open {
            self.depot.ui_state.dictionary_focused = false;
            self.dictionary_ui = None;
        }

        msg
    }
}
