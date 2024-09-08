use time::Duration;
use truncate_core::{board::Direction, player::Hand};

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
    local_player_index: usize,
}

impl<'a> HandUI<'a> {
    pub fn new(hand: &'a mut Hand, local_player_index: usize) -> Self {
        Self {
            hand,
            active: true,
            interactive: true,
            local_player_index,
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
    ) {
        let TruncateDepot {
            interactions,
            aesthetics,
            gameplay,
            ..
        } = depot;

        let interactions = &mut interactions[self.local_player_index];

        let selected = interactions.selected_tile_in_hand;
        let hovered = interactions.hovered_tile_in_hand;

        mapped_tiles.remap_texture(
            ui.ctx(),
            self.hand
                .0
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    // NB: Hovering and selecting here will be delayed by one frame since
                    // we remap before handling interactions.
                    let hovered = matches!(hovered, Some((p, _)) if p == i);
                    let selected = matches!(selected, Some((p, _)) if p == i);

                    let color = if self.active {
                        aesthetics.player_colors
                            [gameplay.player_numbers[self.local_player_index] as usize]
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
                        orientation: truncate_core::board::Direction::North,
                    }
                })
                .collect(),
            &aesthetics,
            Some(interactions),
        );

        let mut started_interaction = false;
        let rearrange = None;
        let mut next_selection = None;
        let mut highlights = interactions.highlight_tiles.clone();
        interactions.hovered_tile_in_hand = None;

        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        let (_, mut margin, theme) = aesthetics.theme.calc_rescale(
            &ui.available_rect_before_wrap(),
            self.hand.len(),
            1,
            0.5..1.3,
            (0, 0),
        );

        depot.ui_state.hand_height_last_frame = theme.grid_size;

        let old_theme = aesthetics.theme.clone();
        aesthetics.theme = theme;

        margin.top = 0.0;
        margin.bottom = 0.0;

        let outer_frame = egui::Frame::none().inner_margin(margin);

        let player_dir = match self.local_player_index {
            0 => Direction::North,
            1 => Direction::South,
            _ => unimplemented!(),
        };

        let tiles: Vec<_> = if matches!(player_dir, Direction::South) {
            self.hand.0.iter().cloned().enumerate().rev().collect()
        } else {
            self.hand.0.iter().cloned().enumerate().collect()
        };

        outer_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, char) in tiles {
                    HandSquareUI::new().render(ui, depot, |ui, depot| {
                        let tile_id = Id::new("Hand")
                            .with(self.local_player_index)
                            .with(i)
                            .with(char);
                        let mut is_being_dragged = ui.memory(|mem| mem.is_being_dragged(tile_id));
                        let is_decidedly_dragging =
                            ui.ctx().input(|inp| inp.pointer.is_decidedly_dragging());

                        // Bail out of a drag if we're not "decidedly dragging",
                        // as this could instead be just a click.
                        if is_being_dragged && !is_decidedly_dragging {
                            is_being_dragged = false;
                        }

                        let TruncateDepot {
                            interactions,
                            aesthetics,
                            timing,
                            ui_state,
                            ..
                        } = depot;
                        let interactions = &mut interactions[self.local_player_index];

                        let _highlight = if let Some(highlights) = highlights.as_mut() {
                            if let Some(c) = highlights.iter().position(|c| *c == char) {
                                highlights.remove(c);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        let (base_rect, _) = ui.allocate_exact_size(
                            egui::vec2(aesthetics.theme.grid_size, aesthetics.theme.grid_size),
                            egui::Sense::hover(),
                        );
                        mapped_tiles.render_tile_to_rect(i, base_rect, player_dir, ui);

                        if !self.interactive {
                            return;
                        }

                        // TODO: Remove magic number somehow (currently 2px/16px for tile sprite border)
                        let tile_margin = aesthetics.theme.grid_size * 0.125;
                        let tile_rect = base_rect.shrink(tile_margin);
                        let tile_response =
                            ui.interact(tile_rect, tile_id, Sense::click_and_drag());
                        if tile_response.hovered() {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                            interactions.hovered_tile_in_hand = Some((i, char));
                        }

                        if tile_response.drag_started() {
                            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                let delta = pointer_pos - tile_response.rect.center();
                                ui.memory_mut(|mem| {
                                    mem.data.insert_temp(tile_id, delta);
                                    mem.data.insert_temp(tile_id, timing.current_time);
                                });
                            }
                            ui.ctx().animate_value_with_time(
                                tile_id.with("initial_offset"),
                                0.0,
                                0.0,
                            );

                            started_interaction = true;
                        } else if tile_response.drag_released() && is_decidedly_dragging {
                            if let Some(HoveredRegion {
                                coord: Some(coord), ..
                            }) = interactions.hovered_unoccupied_square_on_board
                            {
                                interactions.released_tile = Some((i, coord));
                            }
                        }

                        if is_being_dragged {
                            next_selection = Some(None);

                            let drag_id: Duration = ui
                                .memory(|mem| mem.data.get_temp(tile_id))
                                .unwrap_or_default();

                            let area = egui::Area::new(tile_id.with("floating").with(drag_id))
                                .movable(false)
                                .order(Order::Tooltip)
                                .anchor(Align2::LEFT_TOP, vec2(0.0, 0.0));

                            area.show(ui.ctx(), |ui| {
                                let interactions = &mut depot.interactions[self.local_player_index];

                                let ideal_width = if let Some(region) =
                                    &interactions.hovered_unoccupied_square_on_board
                                {
                                    region.rect.width()
                                } else {
                                    depot.aesthetics.theme.grid_size
                                };

                                let bouncy_width = ui.ctx().animate_value_with_time(
                                    area.layer().id.with("width"),
                                    ideal_width,
                                    depot.aesthetics.theme.animation_time,
                                );

                                let snap_to_rect = interactions
                                    .hovered_unoccupied_square_on_board
                                    .as_ref()
                                    .map(|region| region.rect);

                                let position = if let Some(snap) = snap_to_rect {
                                    snap.left_top()
                                } else if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                    let drag_offset = if ui_state.is_touch { -50.0 } else { 0.0 };
                                    let bounce_offset = ui.ctx().animate_value_with_time(
                                        tile_id.with("initial_offset"),
                                        drag_offset,
                                        depot.aesthetics.theme.animation_time,
                                    );
                                    let original_delta: Vec2 = ui.memory(|mem| {
                                        mem.data.get_temp(tile_id).unwrap_or_default()
                                    });
                                    pointer_pos + vec2(0.0, bounce_offset)
                                        - original_delta
                                        - Vec2::splat(bouncy_width / 2.0)
                                } else {
                                    tile_rect.left_top()
                                };

                                let animated_position = pos2(
                                    ui.ctx().animate_value_with_time(
                                        area.layer().id.with("delta_x"),
                                        position.x,
                                        depot.aesthetics.theme.animation_time,
                                    ),
                                    ui.ctx().animate_value_with_time(
                                        area.layer().id.with("delta_y"),
                                        position.y,
                                        depot.aesthetics.theme.animation_time,
                                    ),
                                );

                                mapped_tiles.render_tile_to_rect(
                                    i,
                                    Rect::from_min_size(
                                        animated_position,
                                        Vec2::splat(bouncy_width),
                                    ),
                                    player_dir,
                                    ui,
                                );
                            })
                            .response;

                            ui.ctx()
                                .output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                        }

                        if tile_response.clicked() {
                            if matches!(
                                depot.interactions[self.local_player_index].selected_tile_in_hand,
                                Some((selected_index, selected_char))
                                    if selected_index == i && selected_char == self.hand.0[i])
                            {
                                next_selection = Some(None);
                            } else {
                                next_selection = Some(Some((i, self.hand.0[i])));
                            }

                            started_interaction = true;
                        }
                    });
                }
            });
        });

        if let Some((from, to)) = rearrange {
            self.hand.rearrange(from, to);
        }

        if let Some(new_selection) = next_selection {
            depot.interactions[self.local_player_index].selected_tile_in_hand = new_selection;
            depot.interactions[self.local_player_index].selected_tile_on_board = None;
        }

        depot.aesthetics.theme = old_theme;

        if started_interaction {
            depot.ui_state.dictionary_open = false;
            depot.ui_state.dictionary_focused = false;
            depot.interactions[self.local_player_index].selected_square_on_board = None;
            depot.interactions[self.local_player_index].selected_tile_on_board = None;
        }
    }
}
