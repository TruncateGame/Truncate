use std::{collections::VecDeque, f32::consts::PI};

use eframe::egui;
use epaint::{
    emath::Rot2, pos2, vec2, Color32, ColorImage, Mesh, Rect, Shape, TextureHandle, Vec2,
};
use truncate_core::board::{Board, Coordinate, Square};

use crate::{
    app_outer::{TEXTURE_IMAGE, TEXTURE_MEASUREMENT},
    utils::tex::FGTexType,
};

use super::tex::{render_tex_quads, BGTexType, Tex, TexQuad};

#[derive(Clone)]
pub struct MappedBoard {
    resolved_tex: Vec<Vec<Vec<TexQuad>>>,
    resolved_textures: Vec<TextureHandle>,
    map_texture: TextureHandle,
    map_seed: usize,
    inverted: bool, // TODO: Handle any transpose
    last_tick: u64,
    forecasted_wind: u8,
    incoming_wind: u8,
    winds: VecDeque<u8>,
}

impl MappedBoard {
    pub fn new(
        board: &Board,
        map_texture: TextureHandle,
        invert: bool,
        player_colors: &Vec<Color32>,
    ) -> Self {
        let secs = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("We are living in the future")
            .as_secs();

        let mut mapper = Self {
            resolved_tex: Vec::with_capacity(board.squares.len()),
            resolved_textures: vec![],
            map_texture,
            map_seed: (secs % 100000) as usize,
            inverted: invert,
            last_tick: 0,
            forecasted_wind: 0,
            incoming_wind: 0,
            winds: vec![0; board.width() + board.height()].into(),
        };

        mapper.remap(board, player_colors, 0);

        mapper
    }

    pub fn get(&self, coord: Coordinate) -> &[TexQuad] {
        match self
            .resolved_tex
            .get(coord.y)
            .and_then(|row| row.get(coord.x))
        {
            Some(texs) => texs,
            None => &[[Tex::DEBUG; 4]],
        }
    }

    pub fn render_coord(&self, coord: Coordinate, rect: Rect, ui: &mut egui::Ui) {
        render_tex_quads(self.get(coord), rect, &self.map_texture, ui);
    }

    pub fn render_entire(&self, rect: Rect, ui: &mut egui::Ui) {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

        for layer in &self.resolved_textures {
            let mut mesh = Mesh::with_texture(layer.id());
            mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
            ui.painter().add(Shape::mesh(mesh));
        }
    }

    pub fn remap_texture(
        &mut self,
        ctx: &egui::Context,
        board: &Board,
        player_colors: &Vec<Color32>,
        tick: u64,
        remap_base: bool,
    ) {
        fn base_type(sq: &Square) -> BGTexType {
            match sq {
                truncate_core::board::Square::Water => BGTexType::Water,
                truncate_core::board::Square::Land => BGTexType::Land,
                truncate_core::board::Square::Town { .. } => BGTexType::Land,
                truncate_core::board::Square::Dock(_) => BGTexType::Water,
                truncate_core::board::Square::Occupied(_, _) => BGTexType::Land,
            }
        }

        fn layer_type(sq: &Square, player_colors: &Vec<Color32>) -> Option<(FGTexType, Color32)> {
            match sq {
                Square::Water => None,
                Square::Land => None,
                Square::Town { player, .. } => Some((
                    FGTexType::Town,
                    *player_colors.get(*player).unwrap_or(&Color32::WHITE),
                )),
                Square::Dock(player) => Some((
                    FGTexType::Dock,
                    *player_colors.get(*player).unwrap_or(&Color32::WHITE),
                )),
                Square::Occupied(_, _) => None,
            }
        }

        if self.last_tick != tick {
            self.last_tick = tick;
            if self.inverted {
                self.winds.pop_back();
            } else {
                self.winds.pop_front();
            }

            let off_target = self.incoming_wind.abs_diff(self.forecasted_wind);
            if off_target == 0 {
                self.forecasted_wind = (quickrand(tick as usize) % 100) as u8;
            } else if self.incoming_wind > self.forecasted_wind {
                self.incoming_wind -= (off_target / 3).clamp(1, 20);
            } else {
                self.incoming_wind += (off_target / 3).clamp(1, 20);
            }

            if self.inverted {
                self.winds.push_front(self.incoming_wind);
            } else {
                self.winds.push_back(self.incoming_wind);
            }
        }

        let measures = TEXTURE_MEASUREMENT
            .get()
            .expect("Base texture should have been measured");
        let tileset = TEXTURE_IMAGE
            .get()
            .expect("Base image should have been loaded");

        let final_width = measures.inner_tile_width_px * board.width() * 2;
        let final_height = measures.inner_tile_height_px * board.height() * 2;
        let sized_correct = self
            .resolved_textures
            .get(0)
            .is_some_and(|t| t.size() == [final_width, final_height]);
        if !sized_correct {
            self.resolved_textures = vec![];
        }

        let mut paint_square = |source_row: usize,
                                source_col: usize,
                                dest_row: usize,
                                dest_col: usize,
                                square: &Square| {
            let coord = Coordinate::new(source_col, source_row);

            let mut neighbor_squares: Vec<_> = coord
                .neighbors_8()
                .into_iter()
                .map(|pos| board.get(pos).ok())
                .collect();

            if self.inverted {
                neighbor_squares.rotate_left(4);
            }

            let neighbor_base_types: Vec<_> = neighbor_squares
                .iter()
                .map(|square| square.map(|sq| base_type(&sq)).unwrap_or(BGTexType::Water))
                .collect();

            let tile_base_type = base_type(square);
            let tile_layer_type = layer_type(square, &player_colors);

            let wind_at_coord = self
                .winds
                .get(source_col + source_row)
                .cloned()
                .unwrap_or_default();

            let layers = Tex::terrain(
                tile_base_type,
                tile_layer_type.map(|l| l.0),
                neighbor_base_types,
                tile_layer_type.map(|l| l.1),
                self.map_seed + (coord.x * coord.y + coord.y),
                tick,
                wind_at_coord,
            )
            .into_iter()
            .enumerate();

            // TODO: We should be able to skip remapping the base layer,
            // but it currently comes up blank.
            // if !remap_base && !self.resolved_textures.is_empty() {
            //     _ = layers.next();
            // }

            layers.for_each(|(layer, tex_quad)| {
                if self.resolved_textures.len() == layer {
                    let layer_base =
                        ColorImage::new([final_width, final_height], Color32::TRANSPARENT);
                    let handle = ctx.load_texture(
                        format!("board_layer_{layer}"),
                        layer_base,
                        egui::TextureOptions::NEAREST,
                    );
                    self.resolved_textures.push(handle);
                }

                let texture = self.resolved_textures.get_mut(layer).unwrap();
                for (tex, sub_loc) in tex_quad.into_iter().zip(
                    [
                        [0, 0],
                        [measures.inner_tile_width_px, 0],
                        [measures.inner_tile_width_px, measures.inner_tile_height_px],
                        [0, measures.inner_tile_height_px],
                    ]
                    .into_iter(),
                ) {
                    let source = tex.get_source_position();
                    let region = Rect::from_min_size(
                        pos2(
                            source.x * tileset.size[0] as f32,
                            source.y * tileset.size[1] as f32,
                        ),
                        vec2(
                            measures.inner_tile_width_px as f32,
                            measures.inner_tile_height_px as f32,
                        ),
                    );
                    let tile_from_map = tileset.region(&region, None);
                    let dest_pos = [
                        dest_col * (measures.inner_tile_width_px * 2) + sub_loc[0],
                        dest_row * (measures.inner_tile_height_px * 2) + sub_loc[1],
                    ];

                    texture.set_partial(dest_pos, tile_from_map, egui::TextureOptions::NEAREST);
                }
            });
        };

        if self.inverted {
            board.squares.iter().enumerate().rev().enumerate().for_each(
                |(dest_row, (source_row, row))| {
                    row.iter().enumerate().rev().enumerate().for_each(
                        |(dest_col, (source_col, square))| {
                            paint_square(source_row, source_col, dest_row, dest_col, square);
                        },
                    );
                },
            );
        } else {
            board.squares.iter().enumerate().for_each(|(rownum, row)| {
                row.iter().enumerate().for_each(|(colnum, square)| {
                    paint_square(rownum, colnum, rownum, colnum, square);
                });
            });
        }
    }

    // TODO: Delete.
    pub fn remap(&mut self, board: &Board, player_colors: &Vec<Color32>, tick: u64) {
        fn base_type(sq: &Square) -> BGTexType {
            match sq {
                truncate_core::board::Square::Water => BGTexType::Water,
                truncate_core::board::Square::Land => BGTexType::Land,
                truncate_core::board::Square::Town { .. } => BGTexType::Land,
                truncate_core::board::Square::Dock(_) => BGTexType::Water,
                truncate_core::board::Square::Occupied(_, _) => BGTexType::Land,
            }
        }

        fn layer_type(sq: &Square, player_colors: &Vec<Color32>) -> Option<(FGTexType, Color32)> {
            match sq {
                Square::Water => None,
                Square::Land => None,
                Square::Town { player, .. } => Some((
                    FGTexType::Town,
                    *player_colors.get(*player).unwrap_or(&Color32::WHITE),
                )),
                Square::Dock(player) => Some((
                    FGTexType::Dock,
                    *player_colors.get(*player).unwrap_or(&Color32::WHITE),
                )),
                Square::Occupied(_, _) => None,
            }
        }

        if self.last_tick != tick {
            self.last_tick = tick;
            if self.inverted {
                self.winds.pop_back();
            } else {
                self.winds.pop_front();
            }

            let off_target = self.incoming_wind.abs_diff(self.forecasted_wind);
            if off_target == 0 {
                self.forecasted_wind = (quickrand(tick as usize) % 100) as u8;
            } else if self.incoming_wind > self.forecasted_wind {
                self.incoming_wind -= (off_target / 3).clamp(1, 20);
            } else {
                self.incoming_wind += (off_target / 3).clamp(1, 20);
            }

            if self.inverted {
                self.winds.push_front(self.incoming_wind);
            } else {
                self.winds.push_back(self.incoming_wind);
            }
        }

        self.resolved_tex = board
            .squares
            .iter()
            .enumerate()
            .map(|(rownum, row)| {
                row.iter()
                    .enumerate()
                    .map(|(colnum, square)| {
                        let coord = Coordinate::new(colnum, rownum);

                        let mut neighbor_squares: Vec<_> = coord
                            .neighbors_8()
                            .into_iter()
                            .map(|pos| board.get(pos).ok())
                            .collect();

                        if self.inverted {
                            neighbor_squares.rotate_left(4);
                        }

                        let neighbor_base_types: Vec<_> = neighbor_squares
                            .iter()
                            .map(|square| {
                                square.map(|sq| base_type(&sq)).unwrap_or(BGTexType::Water)
                            })
                            .collect();

                        let tile_base_type = base_type(square);
                        let tile_layer_type = layer_type(square, &player_colors);

                        let wind_at_coord =
                            self.winds.get(colnum + rownum).cloned().unwrap_or_default();

                        Tex::terrain(
                            tile_base_type,
                            tile_layer_type.map(|l| l.0),
                            neighbor_base_types,
                            tile_layer_type.map(|l| l.1),
                            self.map_seed + (coord.x * coord.y + coord.y),
                            tick,
                            wind_at_coord,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
    }
}

#[derive(Clone)]
pub enum MappedTileVariant {
    Healthy,
    Dying,
    Dead,
    Gone,
}

#[derive(Clone)]
pub struct MappedTile {
    resolved_tex: Vec<TexQuad>,
    map_texture: TextureHandle,
}

impl MappedTile {
    pub fn new(
        variant: MappedTileVariant,
        color: Option<Color32>,
        highlight: Option<Color32>,
        coord: Option<Coordinate>,
        map_texture: TextureHandle,
    ) -> Self {
        let resolved_tex = if let Some(coord) = coord {
            Tex::board_game_tile(variant, color, highlight, coord.x * 99 + coord.y)
        } else {
            Tex::game_tile(color, highlight)
        };

        Self {
            resolved_tex,
            map_texture,
        }
    }

    pub fn render(&self, rect: Rect, ui: &mut egui::Ui) {
        render_tex_quads(&self.resolved_tex, rect, &self.map_texture, ui);
    }
}

pub fn quickrand(mut n: usize) -> usize {
    n ^= n << 13;
    n ^= n >> 7;
    n ^= n << 17;
    n % 100
}
