use instant::Duration;
use truncate_core::{messages::PlayerMessage, player::Hand};

use eframe::egui::{self, CursorIcon, Id, Order, Sense};
use epaint::{emath::Align2, pos2, vec2, Rect, Vec2};

use crate::utils::{
    depot::{HoveredRegion, TruncateDepot},
    mapper::{MappedTile, MappedTileVariant, MappedTiles},
};

use super::HandSquareUI;

pub struct HandUI<'a> {
    hand: &'a mut Hand,
    active: bool,
    interactive: bool,
}

impl<'a> HandUI<'a> {
    pub fn new(hand: &'a mut Hand) -> Self {
        Self {
            hand,
            active: true,
            interactive: true,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
}

impl<'a> HandUI<'a> {
    pub fn render(
        self,
        ui: &mut egui::Ui,
        depot: &mut TruncateDepot,
        mapped_tiles: &mut MappedTiles,
    ) -> Option<PlayerMessage> {
        let TruncateDepot {
            interactions,
            aesthetics,
            gameplay,
            ..
        } = depot;

        let mut msg = None;
        let mut started_interaction = false;
        let mut rearrange = None;
        let mut next_selection = None;
        let mut highlights = interactions.highlight_tiles.clone();
        let hand_animation_generation = interactions.hand_animation_generation;
        interactions.hovered_tile_in_hand = None;
        if !ui.memory(|m| m.is_anything_being_dragged()) {
            interactions.rearranging_tiles = false;
        }

        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        let (_, mut margin, theme) = aesthetics.theme.calc_rescale(
            &ui.available_rect_before_wrap(),
            self.hand.len(),
            1,
            0.5..1.3,
            (0.0, 0.0),
        );

        depot.ui_state.hand_height_last_frame = theme.grid_size;

        let old_theme = aesthetics.theme.clone();
        aesthetics.theme = theme;

        margin.top = 0.0;
        margin.bottom = 0.0;

        let outer_frame = egui::Frame::none().inner_margin(margin);

        let mut dragging_tile: Option<usize> = None;

        let render_slots: Vec<_> = outer_frame
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    self.hand
                        .iter()
                        .map(|_| {
                            let (base_rect, _) = ui.allocate_exact_size(
                                egui::vec2(aesthetics.theme.grid_size, aesthetics.theme.grid_size),
                                egui::Sense::hover(),
                            );

                            base_rect
                        })
                        .collect()
                })
                .inner
            })
            .inner;

        let hovered_slot = if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            render_slots.iter().position(|s| s.contains(pointer_pos))
        } else {
            None
        };

        for (i, char) in self.hand.iter().enumerate() {
            let tile_id = Id::new("Hand")
                .with(i)
                .with(char)
                .with(hand_animation_generation);
            let raw_dragged = ui.memory(|mem| mem.is_being_dragged(tile_id));
            let is_decidedly_dragging = ui.ctx().input(|inp| inp.pointer.is_decidedly_dragging());

            // Bail out of a drag if we're not "decidedly dragging",
            // as this could instead be just a click.
            let is_being_dragged = if raw_dragged {
                is_decidedly_dragging
            } else {
                false
            };
            if is_being_dragged {
                dragging_tile = Some(i);
            }
        }

        if self.interactive {
            for (i, char) in self.hand.iter().enumerate() {
                let tile_id = Id::new("Hand")
                    .with(i)
                    .with(char)
                    .with(hand_animation_generation);

                // TODO: Remove?
                let _highlight = if let Some(highlights) = highlights.as_mut() {
                    if let Some(c) = highlights.iter().position(|c| c == char) {
                        highlights.remove(c);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                let base_rect = render_slots[i].clone();

                // TODO: Remove magic number somehow (currently 2px/16px for tile sprite border)
                let tile_margin = aesthetics.theme.grid_size * 0.125;
                let tile_rect = base_rect.shrink(tile_margin);
                let tile_response = ui.interact(tile_rect, tile_id, Sense::click_and_drag());
                if tile_response.hovered() {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                    interactions.hovered_tile_in_hand = Some((i, *char));
                }

                if tile_response.drag_started() {
                    if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                        let delta = pointer_pos - tile_response.rect.center();
                        ui.memory_mut(|mem| {
                            mem.data.insert_temp(tile_id, delta);
                            mem.data.insert_temp(tile_id, depot.timing.current_time);
                        });
                    }
                    ui.ctx()
                        .animate_value_with_time(tile_id.with("initial_offset"), 0.0, 0.0);

                    started_interaction = true;
                } else if tile_response.drag_released() && dragging_tile.is_some() {
                    interactions.hand_animation_generation += 1;
                    if let Some(HoveredRegion {
                        coord: Some(coord), ..
                    }) = interactions.hovered_unoccupied_square_on_board
                    {
                        interactions.released_tile = Some((i, *char, coord));
                    }

                    if let Some(dropped_slot) = hovered_slot {
                        if dropped_slot != i {
                            rearrange = Some((i, dropped_slot));
                        }
                    }
                }

                if tile_response.clicked() {
                    if matches!(
                        interactions.selected_tile_in_hand,
                        Some((selected_index, selected_char))
                            if selected_index == i && selected_char == self.hand.0[i])
                    {
                        next_selection = Some(None);
                    } else {
                        next_selection = Some(Some((i, self.hand.0[i])));
                    }

                    started_interaction = true;
                }
            }
        }

        let selected = interactions.selected_tile_in_hand;
        let hovered = interactions.hovered_tile_in_hand;
        mapped_tiles.remap_texture(
            ui.ctx(),
            self.hand
                .0
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let hovered =
                        dragging_tile.is_none() && matches!(hovered, Some((p, _)) if p == i);
                    let selected =
                        dragging_tile.is_none() && matches!(selected, Some((p, _)) if p == i);

                    let color = if self.active {
                        aesthetics.player_colors[gameplay.player_number as usize]
                    } else {
                        aesthetics.theme.faded
                    };

                    MappedTile {
                        variant: MappedTileVariant::Healthy,
                        character: *c,
                        color: Some(color),
                        highlight: if selected && hovered {
                            Some(aesthetics.theme.ring_selected_hovered)
                        } else if selected {
                            Some(aesthetics.theme.ring_selected)
                        } else if hovered {
                            Some(aesthetics.theme.ring_hovered)
                        } else {
                            None
                        },
                        orientation: truncate_core::board::Direction::South,
                    }
                })
                .collect(),
            &aesthetics,
            Some(interactions),
        );

        for (i, char) in self.hand.iter().enumerate() {
            let tile_id = Id::new("Hand")
                .with(i)
                .with(char)
                .with(hand_animation_generation);
            let mut base_rect = render_slots[i].clone();

            if let (Some(hovered_slot), Some(dragging_tile)) = (hovered_slot, dragging_tile) {
                if dragging_tile > i && i >= hovered_slot {
                    interactions.rearranging_tiles = true;
                    base_rect = render_slots
                        .get(i + 1)
                        .expect("shouldn't be able to drag from a tile beyond the last")
                        .clone();
                } else if dragging_tile < i && i <= hovered_slot {
                    interactions.rearranging_tiles = true;
                    base_rect = render_slots
                        .get(i - 1)
                        .expect("shouldn't be able to drag from a tile beyond the last")
                        .clone();
                }
            }

            let animated_position = pos2(
                ui.ctx().animate_value_with_time(
                    tile_id.with("hand_delta_x"),
                    base_rect.left_top().x,
                    aesthetics.theme.animation_time,
                ),
                ui.ctx().animate_value_with_time(
                    tile_id.with("hand_delta_y"),
                    base_rect.left_top().y,
                    aesthetics.theme.animation_time,
                ),
            );
            let animated_rect = Rect::from_min_size(animated_position, base_rect.size());

            if dragging_tile != Some(i) {
                mapped_tiles.render_tile_to_rect(i, animated_rect, ui);
            }
        }

        if let Some(dragging_tile) = dragging_tile {
            next_selection = Some(None);

            let tile_id = Id::new("Hand")
                .with(dragging_tile)
                .with(self.hand.get(dragging_tile))
                .with(hand_animation_generation);

            let area = egui::Area::new(tile_id.with("floating"))
                .movable(false)
                .order(Order::Tooltip)
                .anchor(Align2::LEFT_TOP, vec2(0.0, 0.0));

            area.show(ui.ctx(), |ui| {
                let ideal_width =
                    if let Some(region) = &interactions.hovered_unoccupied_square_on_board {
                        region.rect.width()
                    } else {
                        aesthetics.theme.grid_size
                    };

                let bouncy_width = ui.ctx().animate_value_with_time(
                    tile_id.with("width"),
                    ideal_width,
                    aesthetics.theme.animation_time,
                );

                let mut snap_to_rect = interactions
                    .hovered_unoccupied_square_on_board
                    .as_ref()
                    .map(|region| region.rect);

                if interactions.rearranging_tiles {
                    let hovered_hand_rect = hovered_slot.map(|hovered_slot| {
                        let rect = render_slots[hovered_slot].clone();
                        rect.translate(vec2(0.0, -(rect.height() / 4.0)))
                    });

                    snap_to_rect = snap_to_rect.or(hovered_hand_rect);
                }

                let position = if let Some(snap) = snap_to_rect {
                    snap.left_top()
                } else if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                    let drag_offset = if depot.ui_state.is_touch { -50.0 } else { 0.0 };
                    let bounce_offset = ui.ctx().animate_value_with_time(
                        tile_id.with("initial_offset"),
                        drag_offset,
                        aesthetics.theme.animation_time,
                    );
                    let original_delta: Vec2 =
                        ui.memory(|mem| mem.data.get_temp(tile_id).unwrap_or_default());
                    pointer_pos + vec2(0.0, bounce_offset)
                        - original_delta
                        - Vec2::splat(bouncy_width / 2.0)
                } else {
                    pos2(0.0, 0.0)
                };

                let animated_position = pos2(
                    ui.ctx().animate_value_with_time(
                        tile_id.with("delta_x"),
                        position.x,
                        aesthetics.theme.animation_time,
                    ),
                    ui.ctx().animate_value_with_time(
                        tile_id.with("delta_y"),
                        position.y,
                        aesthetics.theme.animation_time,
                    ),
                );

                mapped_tiles.render_tile_to_rect(
                    dragging_tile,
                    Rect::from_min_size(animated_position, Vec2::splat(bouncy_width)),
                    ui,
                );
            })
            .response;

            ui.ctx()
                .output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
        }

        if let Some((from, to)) = rearrange {
            self.hand.rearrange(from, to);
            msg = Some(PlayerMessage::RearrangeHand(self.hand.clone()));
        }

        if let Some(new_selection) = next_selection {
            interactions.selected_tile_in_hand = new_selection;
            interactions.selected_tile_on_board = None;
        }

        aesthetics.theme = old_theme;

        if started_interaction {
            depot.ui_state.dictionary_open = false;
            depot.ui_state.dictionary_focused = false;
            interactions.selected_square_on_board = None;
            interactions.selected_tile_on_board = None;
        }

        msg
    }
}
