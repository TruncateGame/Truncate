use instant::Duration;
use truncate_core::player::Hand;

use eframe::egui::{self, CursorIcon, Id, LayerId, Order};
use epaint::{vec2, Vec2};

use crate::active_game::GameCtx;

use super::{tile::TilePlayer, HandSquareUI, TileUI};

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
    pub fn render(self, ctx: &mut GameCtx, ui: &mut egui::Ui) -> Option<usize> {
        let mut rearrange = None;
        let mut next_selection = None;
        let mut released_drag = None;

        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);
        let (margin, theme) = ctx.theme.calc_rescale(
            &ui.available_rect_before_wrap(),
            self.hand.len(),
            1,
            0.5..1.3,
        );

        let old_theme = ctx.theme.clone();
        ctx.theme = theme;

        let outer_frame = egui::Frame::none().inner_margin(margin);

        outer_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, char) in self.hand.iter().enumerate() {
                    HandSquareUI::new().render(ui, ctx, |ui, ctx| {
                        let tile_id = Id::new("Hand").with(i).with(char);
                        let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(tile_id));

                        let tile_response = TileUI::new(*char, TilePlayer::Own)
                            .id(tile_id)
                            .active(self.active)
                            .ghost(is_being_dragged)
                            .selected(Some(i) == ctx.selected_tile_in_hand)
                            .render(None, ui, ctx, None);

                        if tile_response.drag_started() {
                            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                let delta = pointer_pos - tile_response.rect.center();
                                ui.memory_mut(|mem| {
                                    mem.data.insert_temp(tile_id, delta);
                                    mem.data.insert_temp(tile_id, ctx.current_time);
                                });
                            }
                            next_selection = Some(None);
                        } else if tile_response.drag_released() {
                            released_drag = Some(i);
                        }

                        if is_being_dragged {
                            let drag_id: Duration = ui
                                .memory_mut(|mem| mem.data.get_temp(tile_id))
                                .unwrap_or_default();
                            let layer_id = LayerId::new(
                                Order::Tooltip,
                                tile_id.with("floating").with(drag_id),
                            );
                            let response = ui
                                .with_layer_id(layer_id, |ui| {
                                    let hover_scale =
                                        if let Some(region) = &ctx.hovered_tile_on_board {
                                            region.rect.width() / ctx.theme.grid_size
                                        } else {
                                            1.0
                                        };
                                    let bouncy_scale = ui.ctx().animate_value_with_time(
                                        layer_id.id,
                                        hover_scale,
                                        ctx.theme.animation_time,
                                    );
                                    TileUI::new(*char, TilePlayer::Own)
                                        .active(self.active)
                                        .selected(false)
                                        .hovered(true)
                                        .ghost(ctx.hovered_tile_on_board.is_some())
                                        .render(None, ui, ctx, Some(bouncy_scale));
                                })
                                .response;

                            let snap_to_rect =
                                ctx.hovered_tile_on_board.as_ref().map(|region| region.rect);

                            let delta = if let Some(snap_rect) = snap_to_rect {
                                snap_rect.center() - response.rect.center()
                            } else if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                let delta = pointer_pos - response.rect.center();
                                let original_delta: Vec2 = ui.memory_mut(|mem| {
                                    mem.data.get_temp(tile_id).unwrap_or_default()
                                });
                                delta - original_delta
                            } else {
                                vec2(0.0, 0.0)
                            };

                            let animated_delta = vec2(
                                ui.ctx().animate_value_with_time(
                                    layer_id.id.with("delta_x"),
                                    delta.x,
                                    ctx.theme.animation_time,
                                ),
                                ui.ctx().animate_value_with_time(
                                    layer_id.id.with("delta_y"),
                                    delta.y,
                                    ctx.theme.animation_time,
                                ),
                            );
                            ui.ctx().translate_layer(layer_id, animated_delta.round());

                            ui.ctx()
                                .output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                        }

                        if tile_response.clicked() {
                            if let Some(selected) = ctx.selected_tile_in_hand {
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

        if let Some(new_selection) = next_selection {
            ctx.selected_tile_in_hand = new_selection;
            ctx.selected_square_on_board = None;
        }

        ctx.theme = old_theme;

        released_drag
    }
}
