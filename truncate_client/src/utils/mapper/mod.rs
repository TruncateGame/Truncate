use std::collections::VecDeque;

use eframe::egui;
use epaint::{hex_color, pos2, vec2, Color32, ColorImage, Mesh, Rect, Shape, TextureHandle};
use truncate_core::{
    board::{Board, Coordinate, Direction, Square},
    reporting::Change,
};

use crate::{
    app_outer::{TextureMeasurement, GLYPH_IMAGE, TEXTURE_IMAGE, TEXTURE_MEASUREMENT},
    utils::tex::FGTexType,
};

use self::image_manipulation::ImageMusher;

use super::{
    depot::{AestheticDepot, GameplayDepot, InteractionDepot},
    glyph_utils::BaseTileGlyphs,
    macros::tr_log,
    tex::{self, tiles, BGTexType, Tex, TexLayers, TexQuad},
};

mod image_manipulation;

#[derive(Clone)]
struct ResolvedTextureLayers {
    terrain: TextureHandle,
    structures: TextureHandle,
    pieces: TextureHandle,
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
            pieces: ctx.load_texture(
                format!("board_layer_pieces"),
                layer_base.clone(),
                egui::TextureOptions::NEAREST,
            ),
        }
    }
}

#[derive(Clone, PartialEq)]
struct MapState {
    prev_board: Board,
    prev_tick: u64,
    prev_selected: Option<Coordinate>,
    prev_hover: Option<Coordinate>,
    prev_changes: Vec<Change>,
}

#[derive(Clone)]
pub struct MappedBoard {
    layer_memory: Vec<Vec<TexLayers>>,
    state_memory: Option<MapState>,
    resolved_textures: Option<ResolvedTextureLayers>,
    map_seed: usize,
    inverted: bool,
    for_player: usize,
    last_tick: u64,
    forecasted_wind: u8,
    incoming_wind: u8,
    winds: VecDeque<u8>,
}

impl MappedBoard {
    pub fn new(
        ctx: &egui::Context,
        aesthetics: &AestheticDepot,
        board: &Board,
        for_player: usize,
    ) -> Self {
        let secs = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("We are living in the future")
            .as_secs();

        let mut mapper = Self {
            layer_memory: vec![
                vec![TexLayers::default(); board.squares[0].len()];
                board.squares.len()
            ],
            state_memory: None,
            resolved_textures: None,
            map_seed: (secs % 100000) as usize,
            inverted: for_player == 0,
            for_player,
            last_tick: 0,
            forecasted_wind: 0,
            incoming_wind: 0,
            winds: vec![0; board.width() + board.height()].into(),
        };

        mapper.remap_texture(ctx, aesthetics, None, None, board);

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
            paint(tex.pieces.id(), Color32::WHITE);
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
        tiles: &BaseTileGlyphs,
        interactions: Option<&InteractionDepot>,
        gameplay: Option<&GameplayDepot>,
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
        let seed_at_coord = self.map_seed + (coord.x * coord.y + coord.y);

        let mut layers = Tex::terrain(
            tile_base_type,
            tile_layer_type,
            neighbor_base_types,
            seed_at_coord,
            tick,
            wind_at_coord,
        );

        let orient = |player: usize| {
            if player == self.for_player {
                Direction::North
            } else {
                Direction::South
            }
        };

        let mut tile_was_added = false;
        let mut tile_was_swapped = false;
        let mut tile_was_victor = false;

        if let Some(gameplay) = gameplay {
            use truncate_core::reporting::BoardChangeAction;
            use Square::*;

            if let Some(change) = gameplay.changes.iter().find_map(|c| match c {
                Change::Board(b) if b.detail.coordinate == coord => Some(b),
                _ => None,
            }) {
                match change.action {
                    BoardChangeAction::Added => {
                        tile_was_added = true;
                    }
                    BoardChangeAction::Swapped => {
                        tile_was_swapped = true;
                    }
                    BoardChangeAction::Victorious => {
                        tile_was_victor = true;
                    }
                    BoardChangeAction::Defeated => {
                        if let Occupied(player, char) = change.detail.square {
                            let tile_layers = Tex::board_game_tile(
                                MappedTileVariant::Gone,
                                char,
                                orient(player),
                                None,
                                None,
                                seed_at_coord,
                            );
                            layers = layers.merge(tile_layers);
                        }
                    }
                    BoardChangeAction::Truncated => {
                        if let Occupied(player, char) = change.detail.square {
                            let tile_layers = Tex::board_game_tile(
                                MappedTileVariant::Gone,
                                char,
                                orient(player),
                                None,
                                None,
                                seed_at_coord,
                            );
                            layers = layers.merge(tile_layers);
                        }
                    }
                    BoardChangeAction::Exploded => {
                        if let Occupied(player, char) = change.detail.square {
                            let tile_layers = Tex::board_game_tile(
                                MappedTileVariant::Gone,
                                char,
                                orient(player),
                                None,
                                None,
                                seed_at_coord,
                            );
                            layers = layers.merge(tile_layers);
                        }
                    }
                }
            }
        }

        match square {
            Square::Occupied(player, character) => {
                let mut highlight = None;
                if let Some(interactions) = interactions {
                    let selected = interactions.selected_tile_on_board == Some(coord);
                    let hovered = interactions.hovered_tile_on_board == Some(coord);

                    highlight = match (selected, hovered) {
                        (true, true) => Some(hex_color!("#ff00ff")),
                        (true, false) => Some(hex_color!("#ff0000")),
                        (false, true) => Some(hex_color!("#00ffff")),
                        (false, false) => None,
                    };
                }

                if highlight.is_none() {
                    if tile_was_added {
                        highlight = Some(hex_color!("#00ff00"));
                    } else if tile_was_swapped {
                        highlight = Some(hex_color!("#ff00ff"));
                    } else if tile_was_victor {
                        highlight = Some(Color32::GOLD);
                    }
                }

                let tile_layers = Tex::board_game_tile(
                    MappedTileVariant::Healthy,
                    *character,
                    orient(*player),
                    player_colors.get(*player).cloned(),
                    highlight,
                    seed_at_coord,
                );
                layers = layers.merge(tile_layers);
            }
            _ => {}
        }

        let cached = self
            .layer_memory
            .get_mut(coord.y)
            .unwrap()
            .get_mut(coord.x)
            .unwrap();

        if *cached == layers {
            return;
        }

        let tile_dims = [measures.inner_tile_width_px, measures.inner_tile_height_px];
        let dest_pos = [
            dest_col * (measures.inner_tile_width_px * 2),
            dest_row * (measures.inner_tile_height_px * 2),
        ];

        let paint_quad = |quad: [Tex; 4], canvas: &mut TextureHandle| {
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
                canvas.set_partial(
                    [dest_pos[0] + sub_loc[0], dest_pos[1] + sub_loc[1]],
                    tex.slice_as_image(tileset),
                    egui::TextureOptions::NEAREST,
                );
            }
        };

        let erase = |img: &mut TextureHandle| {
            paint_quad([tex::tiles::NONE; 4], img);
        };

        if cached.terrain != layers.terrain {
            if cached.terrain.is_some() && layers.terrain.is_none() {
                erase(&mut resolved_textures.terrain);
            } else if let Some(terrain) = layers.terrain {
                paint_quad(terrain, &mut resolved_textures.terrain);
            }
        }

        if cached.structures != layers.structures {
            if cached.structures.is_some() && layers.structures.is_none() {
                erase(&mut resolved_textures.structures);
            } else if let Some(structures) = layers.structures {
                paint_quad(structures, &mut resolved_textures.structures);
            }
        }

        if cached.pieces != layers.pieces {
            if !cached.pieces.is_empty() && layers.pieces.is_empty() {
                erase(&mut resolved_textures.pieces);
            } else if !layers.pieces.is_empty() {
                let mut target =
                    ColorImage::new([tile_dims[0] * 2, tile_dims[1] * 2], Color32::TRANSPARENT);
                for piece in layers.pieces.iter() {
                    match piece {
                        tex::PieceLayer::Texture(texs, tint) => {
                            if *tint == Some(hex_color!("#ff0000")) {
                                tr_log!({ "Painting a highlight ring!" });
                            }
                            for (tex, sub_loc) in texs.iter().zip([
                                [0, 0],
                                [tile_dims[0], 0],
                                [tile_dims[0], tile_dims[1]],
                                [0, tile_dims[1]],
                            ]) {
                                let mut image = tex.slice_as_image(tileset);
                                if let Some(tint) = tint {
                                    image.tint(tint);
                                }
                                target.hard_overlay(&image, sub_loc);
                            }
                        }
                        tex::PieceLayer::Character(char, color, is_flipped) => {
                            let (_, achar) = tiles.glyphs.iter().find(|(c, _)| c == char).unwrap();
                            let mut letter = achar.clone();
                            let mut offset = [
                                (target.width() - letter.width()) / 2 + 1, // small shift to center character
                                (target.height() - letter.height()) / 2,
                            ];
                            if *is_flipped {
                                letter.pixels.reverse();
                                offset[0] -= 2; // Small shifts to center inverted characters
                                offset[1] -= 2; // Small shifts to center inverted characters
                            }
                            letter.recolor(color);
                            target.hard_overlay(&letter, offset);
                        }
                    }
                }

                resolved_textures.pieces.set_partial(
                    dest_pos,
                    target,
                    egui::TextureOptions::NEAREST,
                );
            }
        }

        *cached = layers;
    }

    pub fn remap_texture(
        &mut self,
        ctx: &egui::Context,
        aesthetics: &AestheticDepot,
        interactions: Option<&InteractionDepot>,
        gameplay: Option<&GameplayDepot>,
        board: &Board,
    ) {
        let mut tick_eq = true;
        let selected = interactions.map(|i| i.selected_tile_on_board).flatten();
        let hover = interactions.map(|i| i.hovered_tile_on_board).flatten();

        if let Some(memory) = self.state_memory.as_mut() {
            let board_eq = memory.prev_board == *board;
            let selected_eq = memory.prev_selected == selected;
            let hover_eq = memory.prev_hover == hover;
            if memory.prev_tick != aesthetics.qs_tick {
                tick_eq = false;
            }

            if board_eq && tick_eq && selected_eq && hover_eq {
                return;
            }

            if !board_eq {
                memory.prev_board = board.clone();
            }
            if !selected_eq {
                memory.prev_selected = selected;
            }
            if !hover_eq {
                memory.prev_hover = hover;
            }
            if !tick_eq {
                memory.prev_tick = aesthetics.qs_tick;
            }
        } else {
            self.state_memory = Some(MapState {
                prev_board: board.clone(),
                prev_tick: aesthetics.qs_tick,
                prev_selected: selected,
                prev_hover: hover,
                prev_changes: vec![],
            });
            tick_eq = false;
        }

        if !tick_eq {
            self.wind_vane(aesthetics.qs_tick);
        }

        let measures = TEXTURE_MEASUREMENT
            .get()
            .expect("Base texture should have been measured");
        let tileset = TEXTURE_IMAGE
            .get()
            .expect("Base image should have been loaded");
        let tile_glyphs = GLYPH_IMAGE
            .get()
            .expect("Glyph image should have been loaded");

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
            self.layer_memory =
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
                                &aesthetics.player_colors,
                                aesthetics.qs_tick,
                                source_row,
                                source_col,
                                dest_row,
                                dest_col,
                                square,
                                measures,
                                tileset,
                                tile_glyphs,
                                interactions,
                                gameplay,
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
                        &aesthetics.player_colors,
                        aesthetics.qs_tick,
                        rownum,
                        colnum,
                        rownum,
                        colnum,
                        square,
                        measures,
                        tileset,
                        tile_glyphs,
                        interactions,
                        gameplay,
                    );
                });
            });
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum MappedTileVariant {
    Healthy,
    Dying,
    Dead,
    Gone,
}

#[derive(Clone, PartialEq)]
pub struct MappedTile {
    pub variant: MappedTileVariant,
    pub character: char,
    pub color: Option<Color32>,
    pub highlight: Option<Color32>,
    pub orientation: Direction,
}

#[derive(Clone)]
pub struct MappedTiles {
    tile_texture: TextureHandle,
    slots: Vec<MappedTile>,
    capacity: usize,
}

impl MappedTiles {
    pub fn new(egui_ctx: &egui::Context, capacity: usize) -> Self {
        Self {
            tile_texture: MappedTiles::reset_texture(capacity, egui_ctx),
            slots: Vec::with_capacity(capacity),
            capacity,
        }
    }

    #[must_use]
    fn reset_texture(capacity: usize, egui_ctx: &egui::Context) -> TextureHandle {
        let measures = TEXTURE_MEASUREMENT
            .get()
            .expect("Base texture should have been measured");

        let tile_base = ColorImage::new(
            [
                capacity * measures.inner_tile_width_px * 2,
                measures.inner_tile_height_px * 2,
            ],
            Color32::TRANSPARENT,
        );
        egui_ctx.load_texture(format!("tiles"), tile_base, egui::TextureOptions::NEAREST)
    }

    pub fn remap_texture(&mut self, egui_ctx: &egui::Context, slots: Vec<MappedTile>) {
        if slots == self.slots {
            return;
        }

        self.slots = slots;

        if self.capacity < self.slots.len() {
            self.tile_texture = MappedTiles::reset_texture(self.slots.len(), egui_ctx);
        }

        let measures = TEXTURE_MEASUREMENT
            .get()
            .expect("Base texture should have been measured");
        let tileset = TEXTURE_IMAGE
            .get()
            .expect("Base image should have been loaded");
        let tile_glyphs = GLYPH_IMAGE
            .get()
            .expect("Glyph image should have been loaded");

        let tile_dims = [measures.inner_tile_width_px, measures.inner_tile_height_px];

        for (i, slot) in self.slots.iter().enumerate() {
            let tile_layers = Tex::board_game_tile(
                slot.variant.clone(),
                slot.character,
                slot.orientation,
                slot.color,
                slot.highlight,
                0,
            );

            let mut target =
                ColorImage::new([tile_dims[0] * 2, tile_dims[1] * 2], Color32::TRANSPARENT);
            for piece in tile_layers.pieces.iter() {
                match piece {
                    tex::PieceLayer::Texture(texs, tint) => {
                        if *tint == Some(hex_color!("#ff0000")) {
                            tr_log!({ "Painting a highlight ring!" });
                        }
                        for (tex, sub_loc) in texs.iter().zip([
                            [0, 0],
                            [tile_dims[0], 0],
                            [tile_dims[0], tile_dims[1]],
                            [0, tile_dims[1]],
                        ]) {
                            let mut image = tex.slice_as_image(tileset);
                            if let Some(tint) = tint {
                                image.tint(tint);
                            }
                            target.hard_overlay(&image, sub_loc);
                        }
                    }
                    tex::PieceLayer::Character(char, color, is_flipped) => {
                        let (_, achar) =
                            tile_glyphs.glyphs.iter().find(|(c, _)| c == char).unwrap();
                        let mut letter = achar.clone();
                        let mut offset = [
                            (target.width() - letter.width()) / 2 + 1, // small shift to center character
                            (target.height() - letter.height()) / 2,
                        ];
                        if *is_flipped {
                            letter.pixels.reverse();
                            offset[0] -= 2; // Small shifts to center inverted characters
                            offset[1] -= 2; // Small shifts to center inverted characters
                        }
                        letter.recolor(color);
                        target.hard_overlay(&letter, offset);
                    }
                }
            }

            let dest_x = i * measures.inner_tile_width_px * 2;
            self.tile_texture
                .set_partial([dest_x, 0], target, egui::TextureOptions::NEAREST);
        }
    }

    pub fn render_tile_to_rect(&self, slot: usize, rect: Rect, ui: &mut egui::Ui) {
        let tile_width = 1.0 / self.capacity as f32;
        let uv = Rect::from_min_max(
            pos2(tile_width * (slot as f32), 0.0),
            pos2(tile_width * (slot as f32) + tile_width, 1.0),
        );

        let mut mesh = Mesh::with_texture(self.tile_texture.id());
        mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
        ui.painter().add(Shape::mesh(mesh));
    }
}

pub fn quickrand(mut n: usize) -> usize {
    n ^= n << 13;
    n ^= n >> 7;
    n ^= n << 17;
    n % 100
}
