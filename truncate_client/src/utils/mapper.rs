use std::{collections::VecDeque, f32::consts::PI};

use eframe::egui;
use epaint::{
    emath::Rot2, pos2, vec2, Color32, ColorImage, Mesh, Rect, Shape, TextureHandle, Vec2,
};
use truncate_core::board::{Board, Coordinate, Square};

use crate::{
    app_outer::{TextureMeasurement, TEXTURE_IMAGE, TEXTURE_MEASUREMENT},
    utils::tex::FGTexType,
};

use super::tex::{render_tex_quads, tiles, BGTexType, Tex, TexLayers, TexQuad};

#[derive(Clone)]
struct ResolvedTextureLayers {
    terrain: TextureHandle,
    structures: TextureHandle,
    tinted: Vec<(TextureHandle, Option<Color32>)>,
    overlay: TextureHandle,
}

impl ResolvedTextureLayers {
    fn new(board: &Board, measures: &TextureMeasurement, ctx: &egui::Context) -> Self {
        let final_width = measures.inner_tile_width_px * board.width() * 2;
        let final_height = measures.inner_tile_height_px * board.height() * 2;
        let layer_base = ColorImage::new([final_width, final_height], Color32::TRANSPARENT);

        Self {
            terrain: ctx.load_texture(
                format!("board_layer_terrain"),
                layer_base.clone(),
                egui::TextureOptions::NEAREST,
            ),
            structures: ctx.load_texture(
                format!("board_layer_structures"),
                layer_base.clone(),
                egui::TextureOptions::NEAREST,
            ),
            tinted: vec![],
            overlay: ctx.load_texture(
                format!("board_layer_overlay"),
                layer_base,
                egui::TextureOptions::NEAREST,
            ),
        }
    }
}

#[derive(Clone)]
pub struct MappedBoard {
    memory: Vec<Vec<TexLayers>>,
    resolved_textures: Option<ResolvedTextureLayers>,
    map_seed: usize,
    inverted: bool, // TODO: Handle any transpose
    last_tick: u64,
    forecasted_wind: u8,
    incoming_wind: u8,
    winds: VecDeque<u8>,
}

impl MappedBoard {
    pub fn new(
        ctx: &egui::Context,
        board: &Board,
        invert: bool,
        player_colors: &Vec<Color32>,
    ) -> Self {
        let secs = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("We are living in the future")
            .as_secs();

        let mut mapper = Self {
            memory: vec![vec![TexLayers::default(); board.squares[0].len()]; board.squares.len()],
            resolved_textures: None,
            map_seed: (secs % 100000) as usize,
            inverted: invert,
            last_tick: 0,
            forecasted_wind: 0,
            incoming_wind: 0,
            winds: vec![0; board.width() + board.height()].into(),
        };

        mapper.remap_texture(ctx, board, player_colors, 0);

        mapper
    }

    pub fn render_to_rect(&self, rect: Rect, ui: &mut egui::Ui) {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

        let paint = |id: epaint::TextureId, color: Color32| {
            let mut mesh = Mesh::with_texture(id);
            mesh.add_rect_with_uv(rect, uv, color);
            ui.painter().add(Shape::mesh(mesh));
        };

        if let Some(tex) = &self.resolved_textures {
            paint(tex.terrain.id(), Color32::WHITE);
            paint(tex.structures.id(), Color32::WHITE);
            for (layer, color) in &tex.tinted {
                paint(layer.id(), color.unwrap_or(Color32::WHITE));
            }
            paint(tex.overlay.id(), Color32::WHITE);
        }
    }

    fn wind_vane(&mut self, tick: u64) {
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
    }

    fn paint_square_offscreen(
        &mut self,
        ctx: &egui::Context,
        board: &Board,
        player_colors: &Vec<Color32>,
        tick: u64,
        source_row: usize,
        source_col: usize,
        dest_row: usize,
        dest_col: usize,
        square: &Square,
        measures: &TextureMeasurement,
        tileset: &ColorImage,
    ) {
        let coord = Coordinate::new(source_col, source_row);
        let resolved_textures = self.resolved_textures.as_mut().unwrap();

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
            .map(|square| square.as_ref().map(Into::into).unwrap_or(BGTexType::Water))
            .collect();

        let tile_base_type = BGTexType::from(square);
        let tile_layer_type = FGTexType::from((square, player_colors));

        let wind_at_coord = self
            .winds
            .get(source_col + source_row)
            .cloned()
            .unwrap_or_default();

        let layers = Tex::terrain(
            tile_base_type,
            tile_layer_type,
            neighbor_base_types,
            self.map_seed + (coord.x * coord.y + coord.y),
            tick,
            wind_at_coord,
        );

        let cached = self
            .memory
            .get_mut(coord.y)
            .unwrap()
            .get_mut(coord.x)
            .unwrap();

        if *cached == layers {
            return;
        }

        let paint_quad = |quad: TexQuad, canvas: &mut TextureHandle| {
            println!("Painting some concrete image pixels to a texture canvas");
            for (tex, sub_loc) in quad.into_iter().zip(
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

                canvas.set_partial(dest_pos, tile_from_map, egui::TextureOptions::NEAREST);
            }
        };

        // For all layer types, we can skip the update if we have a cache hit
        // for this coordinate, as the target texture will already match.
        // We do need to check for the cache having a value while the new layer does not,
        // in which case we need to zero out this coordinate with blank tiles.

        if cached.terrain != layers.terrain {
            if let Some(terrain) = layers.terrain {
                paint_quad(terrain, &mut resolved_textures.terrain);
            } else if cached.terrain.is_some() {
                paint_quad([tiles::NONE; 4], &mut resolved_textures.terrain);
            }
        }

        if cached.structures != layers.structures {
            if let Some(structures) = layers.structures {
                paint_quad(structures, &mut resolved_textures.structures);
            } else if cached.structures.is_some() {
                paint_quad([tiles::NONE; 4], &mut resolved_textures.structures);
            }
        }

        if cached.overlay != layers.overlay {
            if let Some(overlay) = layers.overlay {
                paint_quad(overlay, &mut resolved_textures.overlay);
            } else if cached.overlay.is_some() {
                paint_quad([tiles::NONE; 4], &mut resolved_textures.overlay);
            }
        }

        // If there were cached tint layers that we aren't returning,
        // we need to find those tint layers and zero out this coordinate.
        if cached.tinted != layers.tinted {
            if let Some((_, tint)) = cached.tinted {
                let existing_tint = resolved_textures
                    .tinted
                    .iter_mut()
                    .find(|(_, layer_tint)| layer_tint == &tint);
                if let Some((existing_tint, _)) = existing_tint {
                    paint_quad([tiles::NONE; 4], existing_tint);
                }
            }
        }

        if let Some((tinted, tint)) = layers.tinted {
            let existing_tint = resolved_textures
                .tinted
                .iter_mut()
                .find(|(_, layer_tint)| layer_tint == &tint);
            if let Some((existing_tint, _)) = existing_tint {
                paint_quad(tinted, existing_tint);
            } else {
                let layer_base =
                    ColorImage::new(resolved_textures.terrain.size(), Color32::TRANSPARENT);
                let mut handle = ctx.load_texture(
                    format!("board_layer_tint"),
                    layer_base.clone(),
                    egui::TextureOptions::NEAREST,
                );
                paint_quad(tinted, &mut handle);
                resolved_textures.tinted.push((handle, tint));
            }
        }

        *cached = layers;
    }

    pub fn remap_texture(
        &mut self,
        ctx: &egui::Context,
        board: &Board,
        player_colors: &Vec<Color32>,
        tick: u64,
    ) {
        self.wind_vane(tick);

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
            .as_ref()
            .is_some_and(|t| t.terrain.size() == [final_width, final_height]);
        // Some game modes, or board editors, allow the board to resize.
        // For now we just throw our textures away if this happens, rather
        // than try to match old coordinates to new.
        if !sized_correct {
            self.resolved_textures = Some(ResolvedTextureLayers::new(board, measures, ctx));
            self.memory =
                vec![vec![TexLayers::default(); board.squares[0].len()]; board.squares.len()];
        }

        if self.inverted {
            board.squares.iter().enumerate().rev().enumerate().for_each(
                |(dest_row, (source_row, row))| {
                    row.iter().enumerate().rev().enumerate().for_each(
                        |(dest_col, (source_col, square))| {
                            self.paint_square_offscreen(
                                ctx,
                                board,
                                player_colors,
                                tick,
                                source_row,
                                source_col,
                                dest_row,
                                dest_col,
                                square,
                                measures,
                                tileset,
                            );
                        },
                    );
                },
            );
        } else {
            board.squares.iter().enumerate().for_each(|(rownum, row)| {
                row.iter().enumerate().for_each(|(colnum, square)| {
                    self.paint_square_offscreen(
                        ctx,
                        board,
                        player_colors,
                        tick,
                        rownum,
                        colnum,
                        rownum,
                        colnum,
                        square,
                        measures,
                        tileset,
                    );
                });
            });
        }
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
