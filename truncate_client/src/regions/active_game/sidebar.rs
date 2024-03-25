use epaint::{emath::Align2, hex_color, vec2, Color32, FontId, Rect, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    board::{Board, Coordinate},
    generation::BoardSeed,
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage, RoomCode},
    npc::scoring::{NPCParams, NPCPersonality},
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
        urls::back_to_menu,
        Lighten, Theme,
    },
};

use super::ActiveGame;

impl ActiveGame {
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
                            ui.painter()
                                .galley(r.min, room, self.depot.aesthetics.theme.text);
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
}
