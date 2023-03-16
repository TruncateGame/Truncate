use core::player::Hand;

use eframe::egui::{self, CursorIcon, Id, LayerId, Order};
use epaint::Vec2;

use crate::theming::Theme;

use super::{tile::TilePlayer, SquareUI, TileUI};

pub struct HandUI<'a> {
    hand: &'a mut Hand,
    active: bool,
}

impl<'a> HandUI<'a> {
    pub fn new(hand: &'a mut Hand) -> Self {
        Self { hand, active: true }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

impl<'a> HandUI<'a> {
    pub fn render(
        self,
        selected_tile: Option<usize>,
        ui: &mut egui::Ui,
        theme: &Theme,
    ) -> (Option<Option<usize>>, Option<usize>) {
        let mut rearrange = None;
        let mut next_selection = None;
        let mut released_drag = None;

        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);
        let (margin, theme) = theme.rescale(&ui.available_rect_before_wrap(), self.hand.len(), 1);
        let outer_frame = egui::Frame::none().inner_margin(margin);

        outer_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, char) in self.hand.iter().enumerate() {
                    SquareUI::new()
                        .decorated(false)
                        .render(ui, &theme, |ui, theme| {
                            let tile_id = Id::new("Hand").with(i).with(char);
                            let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(tile_id));

                            let tile_response = TileUI::new(*char, TilePlayer::Own)
                                .id(tile_id)
                                .active(self.active)
                                .ghost(is_being_dragged)
                                .selected(Some(i) == selected_tile)
                                .render(ui, theme);

                            if tile_response.drag_started() {
                                if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                    let delta = pointer_pos - tile_response.rect.center();
                                    ui.memory_mut(|mem| {
                                        mem.data.insert_temp(tile_id, delta);
                                    });
                                }
                                next_selection = Some(None);
                            } else if tile_response.drag_released() {
                                released_drag = Some(i);
                            }

                            if is_being_dragged {
                                let layer_id =
                                    LayerId::new(Order::Tooltip, tile_id.with("floating"));
                                let response = ui
                                    .with_layer_id(layer_id, |ui| {
                                        TileUI::new(*char, TilePlayer::Own)
                                            .active(self.active)
                                            .selected(false)
                                            .hovered(true)
                                            .render(ui, theme);
                                    })
                                    .response;

                                if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                    let delta = pointer_pos - response.rect.center();
                                    let original_delta: Vec2 = ui.memory_mut(|mem| {
                                        mem.data.get_temp(tile_id).unwrap_or_default()
                                    });
                                    ui.ctx().translate_layer(layer_id, delta - original_delta);
                                }

                                ui.ctx()
                                    .output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                            }

                            if tile_response.clicked() {
                                if let Some(selected) = selected_tile {
                                    next_selection = Some(None);
                                    if selected != i {
                                        rearrange = Some((selected, i));
                                    }
                                } else {
                                    next_selection = Some(Some(i));
                                }
                            }
                        });
                }
            });
        });

        if let Some((from, to)) = rearrange {
            self.hand.rearrange(from, to);
        }

        (next_selection, released_drag)
    }
}
