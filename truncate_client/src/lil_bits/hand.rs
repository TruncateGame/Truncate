use instant::Duration;
use truncate_core::player::Hand;

use eframe::egui::{self, CursorIcon, Id, LayerId, Order};
use epaint::{emath::Align2, vec2, Rect, Vec2};

use crate::regions::active_game::{GameCtx, HoveredRegion};

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
    pub fn render(self, ctx: &mut GameCtx, ui: &mut egui::Ui) {
        let mut rearrange = None;
        let mut next_selection = None;
        let mut highlights = ctx.highlight_tiles.clone();

        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        let (_, mut margin, theme) = ctx.theme.calc_rescale(
            &ui.available_rect_before_wrap(),
            self.hand.len(),
            1,
            0.5..1.3,
            (2, 0),
        );

        let old_theme = ctx.theme.clone();
        ctx.theme = theme;

        margin.top = 0.0;
        margin.bottom = 0.0;

        let outer_frame = egui::Frame::none().inner_margin(margin);

        outer_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, char) in self.hand.iter().enumerate() {
                    HandSquareUI::new().render(ui, ctx, |ui, ctx| {
                        let tile_id = Id::new("Hand").with(i).with(char);
                        let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(tile_id));

                        let highlight = if let Some(highlights) = highlights.as_mut() {
                            if let Some(c) = highlights.iter().position(|c| c == char) {
                                highlights.remove(c);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        let tile_response = TileUI::new(*char, TilePlayer::Own)
                            .id(tile_id)
                            .active(self.active)
                            .ghost(is_being_dragged)
                            .selected(Some(i) == ctx.selected_tile_in_hand)
                            .highlighted(highlight)
                            .render(None, ui, ctx, true, None);

                        if tile_response.drag_started() {
                            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                let delta = pointer_pos - tile_response.rect.center();
                                ui.memory_mut(|mem| {
                                    mem.data.insert_temp(tile_id, delta);
                                    mem.data.insert_temp(tile_id, ctx.current_time);
                                });
                            }
                            next_selection = Some(None);
                            ctx.dragging_tile = true;
                            ui.ctx().animate_value_with_time(
                                tile_id.with("initial_offset"),
                                0.0,
                                0.0,
                            );
                        } else if tile_response.drag_released() {
                            if let Some(HoveredRegion {
                                coord: Some(coord), ..
                            }) = ctx.hovered_tile_on_board
                            {
                                ctx.released_tile = Some((i, coord));
                            }
                            ctx.dragging_tile = false;
                        }

                        if is_being_dragged {
                            let drag_id: Duration = ui
                                .memory_mut(|mem| mem.data.get_temp(tile_id))
                                .unwrap_or_default();

                            // There is definitely a better way to do this, but this works for now.
                            // The issue with using a layer directly, or with aligning this area to a corner,
                            // is that the Tile is clipped to the screen bounds before we translate the layer.
                            // So if we paint our tile on this layer at the bottom of the screen,
                            // and then drag it over the board which might scale it up,
                            // the bottom of our tile can be cut off due to it clipping based on its original location.
                            let area = egui::Area::new(tile_id.with("floating").with(drag_id))
                                .movable(false)
                                .order(Order::Tooltip)
                                .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0));

                            let response = area
                                .show(ui.ctx(), |ui| {
                                    let hover_scale =
                                        if let Some(region) = &ctx.hovered_tile_on_board {
                                            region.rect.width() / ctx.theme.grid_size
                                        } else {
                                            1.0
                                        };
                                    let bouncy_scale = ui.ctx().animate_value_with_time(
                                        area.layer().id,
                                        hover_scale,
                                        ctx.theme.animation_time,
                                    );
                                    TileUI::new(*char, TilePlayer::Own)
                                        .active(self.active)
                                        .selected(false)
                                        .hovered(true)
                                        .ghost(ctx.hovered_tile_on_board.is_some())
                                        .render(None, ui, ctx, false, Some(bouncy_scale));
                                })
                                .response;

                            let snap_to_rect =
                                ctx.hovered_tile_on_board.as_ref().map(|region| region.rect);

                            let delta = if let Some(snap_rect) = snap_to_rect {
                                snap_rect.center() - response.rect.center()
                            } else if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                let drag_offset = if ctx.is_touch { -50.0 } else { 0.0 };

                                let bounce_offset = ui.ctx().animate_value_with_time(
                                    tile_id.with("initial_offset"),
                                    drag_offset,
                                    ctx.theme.animation_time,
                                );

                                let delta =
                                    pointer_pos + vec2(0.0, bounce_offset) - response.rect.center();
                                let original_delta: Vec2 = ui.memory_mut(|mem| {
                                    mem.data.get_temp(tile_id).unwrap_or_default()
                                });
                                delta - original_delta
                            } else {
                                vec2(0.0, 0.0)
                            };

                            let animated_delta = vec2(
                                ui.ctx().animate_value_with_time(
                                    area.layer().id.with("delta_x"),
                                    delta.x,
                                    ctx.theme.animation_time,
                                ),
                                ui.ctx().animate_value_with_time(
                                    area.layer().id.with("delta_y"),
                                    delta.y,
                                    ctx.theme.animation_time,
                                ),
                            );
                            ui.ctx()
                                .translate_layer(area.layer(), animated_delta.round());

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
    }
}
