use std::collections::VecDeque;

use eframe::egui;
use epaint::{hex_color, pos2, Color32, ColorImage, Mesh, Rect, Shape, TextureHandle};
use instant::Duration;
use truncate_core::{
    board::{Board, BoardDistances, Coordinate, Direction, SignedCoordinate, Square},
    reporting::Change,
};

use crate::{
    app_outer::{TextureMeasurement, GLYPHER, TEXTURE_IMAGE, TEXTURE_MEASUREMENT},
    utils::tex::FGTexType,
};

use self::image_manipulation::alpha_blend;
pub use self::image_manipulation::ImageMusher;

use super::{
    depot::{
        AestheticDepot, GameplayDepot, HoveredRegion, InteractionDepot, TimingDepot, UIStateDepot,
    },
    glyph_utils::Glypher,
    tex::{self, BGTexType, PieceLayer, Tex, TexLayers, TileDecoration},
    Lighten,
};

mod image_manipulation;

type WantsRepaint = bool;

#[derive(Clone)]
struct ResolvedTextureLayers {
    terrain: TextureHandle,
    checkerboard: TextureHandle,
    structures: TextureHandle,
    pieces: TextureHandle,
    pieces_validity: TextureHandle,
    mist: TextureHandle,
    fog: TextureHandle,
}

impl ResolvedTextureLayers {
    fn new(
        board: &Board,
        measures: &TextureMeasurement,
        buffer: usize,
        ctx: &egui::Context,
    ) -> Self {
        let final_width = measures.inner_tile_width_px * (board.width() + buffer * 2) * 2;
        let final_height = measures.inner_tile_height_px * (board.height() + buffer * 2) * 2;
        let layer_base = ColorImage::new([final_width, final_height], Color32::TRANSPARENT);

        Self {
            terrain: ctx.load_texture(
                format!("board_layer_terrain"),
                layer_base.clone(),
                egui::TextureOptions::NEAREST,
            ),
            checkerboard: ctx.load_texture(
                format!("board_layer_checkerboard"),
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
            pieces_validity: ctx.load_texture(
                format!("board_layer_pieces_validity"),
                layer_base.clone(),
                egui::TextureOptions::NEAREST,
            ),
            mist: ctx.load_texture(
                format!("board_layer_mist"),
                layer_base.clone(),
                egui::TextureOptions::NEAREST,
            ),
            fog: ctx.load_texture(
                format!("board_layer_fog"),
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
    prev_winner: Option<usize>,
    prev_selected_tile: Option<(Coordinate, Square)>,
    prev_selected_square: Option<(Coordinate, Square)>,
    prev_tile_hover: Option<(Coordinate, Square)>,
    prev_dragging: Option<(Coordinate, Square)>,
    prev_occupied_hover: Option<HoveredRegion>,
    prev_square_hover: Option<HoveredRegion>,
    prev_changes: Vec<Change>,
    generic_tick: u32,
}

#[derive(Clone)]
pub struct MappedBoard {
    layer_memory: Vec<Vec<TexLayers>>,
    state_memory: Option<MapState>,
    /// Used to break cache and force a repaint
    generic_repaint_tick: u32,
    resolved_textures: Option<ResolvedTextureLayers>,
    /// Number of tiles to paint around the board in every direction
    map_buffer: usize,
    map_seed: usize,
    inverted: bool,
    for_player: usize,
    daytime: bool,
    last_tick: u64,
    forecasted_wind: u8,
    incoming_wind: u8,
    winds: VecDeque<u8>,
    distance_to_land: BoardDistances,
}

impl MappedBoard {
    pub fn new(
        ctx: &egui::Context,
        aesthetics: &AestheticDepot,
        board: &Board,
        map_buffer: usize,
        for_player: usize,
        daytime: bool,
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
            generic_repaint_tick: 0,
            resolved_textures: None,
            map_buffer,
            map_seed: (secs % 100000) as usize,
            inverted: for_player == 0,
            for_player,
            daytime,
            last_tick: 0,
            forecasted_wind: 0,
            incoming_wind: 0,
            winds: vec![0; board.width() + board.height()].into(),
            distance_to_land: board.flood_fill_water_from_land(),
        };

        mapper.remap_texture(ctx, aesthetics, &TimingDepot::default(), None, None, board);

        mapper
    }

    pub fn buffer(&self) -> usize {
        self.map_buffer
    }

    pub fn render_to_rect(&self, rect: Rect, ui_state: Option<&UIStateDepot>, ui: &mut egui::Ui) {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

        let paint = |id: epaint::TextureId, color: Color32| {
            let mut mesh = Mesh::with_texture(id);
            mesh.add_rect_with_uv(rect, uv, color);
            ui.painter().add(Shape::mesh(mesh));
        };

        if let Some(tex) = &self.resolved_textures {
            if ui_state.is_some_and(|s| s.dictionary_open) {
                paint(tex.terrain.id(), Color32::WHITE.gamma_multiply(0.2));
                paint(tex.structures.id(), Color32::WHITE.gamma_multiply(0.2));
                paint(tex.pieces.id(), Color32::WHITE.gamma_multiply(0.2));
                paint(tex.mist.id(), Color32::BLACK.gamma_multiply(0.7));
                paint(tex.pieces_validity.id(), Color32::WHITE);
            } else {
                paint(tex.terrain.id(), Color32::WHITE);
                paint(tex.checkerboard.id(), Color32::WHITE);
                paint(tex.structures.id(), Color32::WHITE);
                paint(tex.pieces.id(), Color32::WHITE);
                paint(tex.mist.id(), Color32::BLACK.gamma_multiply(0.7));
                paint(tex.fog.id(), Color32::BLACK);
            }
        }
    }

    pub fn render_coord_to_rect(&self, mut coord: Coordinate, rect: Rect, ui: &mut egui::Ui) {
        let Some(memory) = &self.state_memory else {
            return;
        };

        let tile_width = 1.0 / memory.prev_board.width() as f32;
        let tile_height = 1.0 / memory.prev_board.height() as f32;

        if self.inverted {
            coord.x = memory.prev_board.width() - coord.x - 1;
            coord.y = memory.prev_board.height() - coord.y - 1;
        }

        let uv = Rect::from_min_max(
            pos2(
                tile_width * (coord.x as f32),
                tile_height * (coord.y as f32),
            ),
            pos2(
                tile_width * (coord.x as f32) + tile_width,
                tile_height * (coord.y as f32) + tile_height,
            ),
        );

        if let Some(tex) = &self.resolved_textures {
            let mut mesh = Mesh::with_texture(tex.pieces.id());
            mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
            ui.painter().add(Shape::mesh(mesh));
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
        source_row: isize,
        source_col: isize,
        dest_row: usize,
        dest_col: usize,
        square: &Square,
        measures: &TextureMeasurement,
        tileset: &ColorImage,
        glypher: &Glypher,
        interactions: Option<&InteractionDepot>,
        gameplay: Option<&GameplayDepot>,
        aesthetics: &AestheticDepot,
        timing: &TimingDepot,
    ) -> WantsRepaint {
        let mut wants_repaint = false;
        let coord = SignedCoordinate::new(source_col, source_row);
        let dest_coord = Coordinate::new(dest_col, dest_row);
        let resolved_textures = self.resolved_textures.as_mut().unwrap();

        let mut neighbor_squares: Vec<_> = coord
            .neighbors_8()
            .into_iter()
            .flat_map(|c| c.map(|c| c.real_coord()))
            .map(|pos| pos.map(|p| board.get(p).ok()).flatten())
            .collect();

        if self.inverted {
            neighbor_squares.rotate_left(4);
        }

        let neighbor_base_types: Vec<_> = neighbor_squares
            .iter()
            .map(|square| {
                square
                    .as_ref()
                    .map(|sq| {
                        if aesthetics.theme.use_old_art {
                            if matches!(sq, Square::Artifact { .. }) {
                                return BGTexType::WaterOrFog;
                            }
                        }
                        sq.into()
                    })
                    .unwrap_or(BGTexType::WaterOrFog)
            })
            .collect();

        let mut tile_base_type = BGTexType::from(square);
        if aesthetics.theme.use_old_art {
            if matches!(square, Square::Artifact { .. }) {
                tile_base_type = BGTexType::WaterOrFog;
            }
        }
        let tile_layer_type = FGTexType::from((square, player_colors));

        let wind_at_coord = self
            .winds
            .get(dest_col + dest_row)
            .cloned()
            .unwrap_or_default();
        let seed_at_coord = self.map_seed + (dest_row * dest_col + dest_col);

        let mut layers = Tex::terrain(
            tile_base_type,
            tile_layer_type,
            neighbor_base_types,
            seed_at_coord,
            tick,
            wind_at_coord,
            coord,
            (board.width(), board.height()),
            &self.distance_to_land,
        );

        if square.is_foggy() {
            layers.mist = Some([tex::tiles::BASE_WATER; 4]);
        }

        let orient = |player: usize| {
            if player == self.for_player {
                Direction::South
            } else {
                Direction::North
            }
        };

        let square_is_highlighted = interactions.is_some_and(|i| {
            coord
                .real_coord()
                .is_some_and(|c| i.highlight_squares.as_ref().is_some_and(|s| s.contains(&c)))
        });

        let mut tile_was_added = false;
        let mut tile_was_swapped = false;
        let mut tile_was_victor = false;

        let base_destructo_time = (timing.current_time - timing.last_turn_change).as_secs_f32();
        let mut destructo_time = base_destructo_time;

        if let Some(gameplay) = gameplay {
            use truncate_core::reporting::BoardChangeAction;
            use Square::*;

            if let Some((battle_origin, coord)) =
                gameplay.last_battle_origin.zip(coord.real_coord())
            {
                let dist = coord.distance_to(&battle_origin) as f32;
                destructo_time -= dist * aesthetics.destruction_tick;
                if destructo_time < 0.0 {
                    destructo_time = 0.0;
                }
            }

            let changes = gameplay.changes.iter().filter_map(|c| match c {
                Change::Board(b)
                    if coord.real_coord().is_some_and(|c| c == b.detail.coordinate) =>
                {
                    Some(b)
                }
                _ => None,
            });

            let base_color = |player: usize| {
                player_colors
                    .get(player)
                    .cloned()
                    .map(|c| c.lighten())
                    .unwrap_or_default()
            };

            let mut animated_variant = |player: usize| {
                if destructo_time < aesthetics.destruction_duration {
                    wants_repaint = true;
                    (MappedTileVariant::Healthy, Some(base_color(player)))
                } else {
                    (MappedTileVariant::Gone, None)
                }
            };

            for change in changes {
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
                        // TODO: We could use `validity` below to show whether a tile
                        // lost on length or lost on being invalid.
                        if let Occupied {
                            player,
                            tile,
                            validity: _,
                            ..
                        } = change.detail.square
                        {
                            let validity_color =
                                if base_destructo_time < aesthetics.destruction_duration {
                                    let traj = ((aesthetics.destruction_duration
                                        - base_destructo_time)
                                        .clamp(0.0, 1.0)
                                        / aesthetics.destruction_duration)
                                        .sqrt();
                                    let color = alpha_blend(
                                        base_color(player),
                                        aesthetics.theme.word_invalid,
                                        Some(traj),
                                    );

                                    Some(color)
                                } else {
                                    None
                                };
                            let (variant, color) = animated_variant(player);

                            let tile_layers = Tex::board_game_tile(
                                variant,
                                tile,
                                orient(player),
                                validity_color.or(color),
                                None,
                                TileDecoration::Grass,
                                seed_at_coord,
                            );
                            layers = layers.merge_below_self(tile_layers);
                        }
                    }
                    BoardChangeAction::Truncated => {
                        // TODO: We could use `validity` below to show whether a tile
                        // lost on length or lost on being invalid.
                        if let Occupied {
                            player,
                            tile,
                            validity: _,
                            ..
                        } = change.detail.square
                        {
                            let (variant, color) = animated_variant(player);

                            let tile_layers = Tex::board_game_tile(
                                variant,
                                tile,
                                orient(player),
                                color,
                                None,
                                TileDecoration::Grass,
                                seed_at_coord,
                            );
                            layers = layers.merge_below_self(tile_layers);
                        }
                    }
                    BoardChangeAction::Exploded => {
                        // TODO: We could use `validity` below to show whether a tile
                        // lost on length or lost on being invalid.
                        if let Occupied {
                            player,
                            tile,
                            validity: _,
                            ..
                        } = change.detail.square
                        {
                            let (variant, color) = animated_variant(player);

                            let tile_layers = Tex::board_game_tile(
                                variant,
                                tile,
                                orient(player),
                                color,
                                None,
                                TileDecoration::Grass,
                                seed_at_coord,
                            );
                            layers = layers.merge_below_self(tile_layers);
                        }
                    }
                }
            }
        }

        match square {
            Square::Occupied {
                player,
                tile,
                validity,
                ..
            } => {
                let mut highlight = None;
                let mut being_dragged = false;

                let mut render_as_swap = None;

                if let Some((interactions, coord)) = interactions.zip(coord.real_coord()) {
                    let selected =
                        matches!(interactions.selected_tile_on_board, Some((c, _)) if c == coord);
                    let hovered =
                        matches!(interactions.hovered_tile_on_board, Some((c, _)) if c == coord);
                    let hovered_occupied = matches!(interactions.hovered_occupied_square_on_board, Some(HoveredRegion { coord: Some(c), .. }) if c == coord);
                    being_dragged =
                        matches!(interactions.dragging_tile_on_board, Some((c, _)) if c == coord);

                    highlight = match (selected, hovered) {
                        (true, true) => Some(aesthetics.theme.ring_selected_hovered),
                        (true, false) => Some(aesthetics.theme.ring_selected),
                        (false, true) => Some(aesthetics.theme.ring_hovered),
                        (false, false) => None,
                    };

                    // Preview click-to-swap from this tile to another
                    if selected && !hovered {
                        if let Some((
                            _,
                            Square::Occupied {
                                player: hovered_player,
                                tile: hovered_tile,
                                ..
                            },
                        )) = interactions.hovered_tile_on_board
                        {
                            if hovered_player == *player {
                                render_as_swap = Some(hovered_tile);
                            }
                        }
                    }

                    // Preview click-to-swap from another tile to this one
                    if hovered && !selected {
                        if let Some((
                            _,
                            Square::Occupied {
                                player: selected_player,
                                tile: selected_tile,
                                ..
                            },
                        )) = interactions.selected_tile_on_board
                        {
                            if selected_player == *player {
                                render_as_swap = Some(selected_tile);
                            }
                        }
                    }

                    // Preview drag-to-swap from this tile to another
                    // (the inverse is handled in the dragging logic itself within the board)
                    if being_dragged && !hovered_occupied {
                        if let Some(HoveredRegion {
                            square:
                                Some(Square::Occupied {
                                    player: hovered_player,
                                    tile: hovered_tile,
                                    ..
                                }),
                            ..
                        }) = interactions.hovered_occupied_square_on_board
                        {
                            if hovered_player == *player {
                                render_as_swap = Some(hovered_tile);
                            }
                        }
                    }
                }

                if highlight.is_none() {
                    if tile_was_added {
                        highlight = Some(aesthetics.theme.ring_added);
                    } else if tile_was_swapped {
                        highlight = Some(aesthetics.theme.ring_modified);
                    }
                }

                let mut color = if being_dragged || render_as_swap.is_some() {
                    Some(aesthetics.theme.ring_selected_hovered)
                } else {
                    player_colors.get(*player).cloned().map(|c| c.lighten())
                };

                if tile_was_victor && base_destructo_time < aesthetics.destruction_duration {
                    wants_repaint = true;
                    let traj = ((aesthetics.destruction_duration - base_destructo_time)
                        .clamp(0.0, 1.0)
                        / aesthetics.destruction_duration)
                        .sqrt();
                    color = color.map(|c| alpha_blend(c, aesthetics.theme.word_valid, Some(traj)));
                }

                if square_is_highlighted && (tick % 4 < 2) {
                    color = Some(aesthetics.theme.ring_selected_hovered);
                }

                let mut variant = MappedTileVariant::Healthy;
                if let Some(GameplayDepot {
                    winner: Some(winner),
                    ..
                }) = gameplay
                {
                    if winner != player {
                        color = None;
                        variant = MappedTileVariant::Gone;
                    }
                }

                let tile_layers = Tex::board_game_tile(
                    variant,
                    render_as_swap.unwrap_or(*tile),
                    orient(*player),
                    color,
                    highlight,
                    TileDecoration::Grass,
                    seed_at_coord,
                );
                layers = layers.merge_above_self(tile_layers);

                // TODO: colors
                let validity_color = match validity {
                    truncate_core::board::SquareValidity::Unknown => aesthetics.theme.faded,
                    truncate_core::board::SquareValidity::Valid => {
                        aesthetics.theme.word_valid.lighten()
                    }
                    truncate_core::board::SquareValidity::Invalid => {
                        aesthetics.theme.word_invalid.lighten().lighten()
                    }
                    truncate_core::board::SquareValidity::Partial => {
                        aesthetics.theme.button_primary
                    }
                };
                let validity_layers = Tex::board_game_tile(
                    MappedTileVariant::Healthy,
                    *tile,
                    orient(*player),
                    Some(validity_color),
                    None,
                    TileDecoration::None,
                    seed_at_coord,
                )
                .into_piece_validity();

                layers = layers.merge_above_self(validity_layers);
            }
            Square::Land { .. } => {
                if let Some((interactions, coord)) = interactions.zip(coord.real_coord()) {
                    if let Some((_, tile_char)) = interactions.selected_tile_in_hand {
                        // Don't show preview tiles if anything is being dragged (i.e. a tile from the hand)
                        if !ctx.memory(|m| m.is_anything_being_dragged())
                            && interactions
                                .hovered_unoccupied_square_on_board
                                .as_ref()
                                .is_some_and(|h| h.coord == Some(coord))
                        {
                            let self_color = gameplay
                                .map(|gameplay| {
                                    player_colors.get(gameplay.player_number as usize).cloned()
                                })
                                .flatten()
                                .unwrap_or(aesthetics.theme.ring_selected);

                            let tile_layers = Tex::board_game_tile(
                                MappedTileVariant::Healthy,
                                tile_char,
                                Direction::South,
                                Some(self_color.lighten()),
                                None,
                                TileDecoration::None,
                                seed_at_coord,
                            );
                            layers = layers.merge_above_self(tile_layers);
                        }
                    } else if !ctx.memory(|m| m.is_anything_being_dragged())
                        && interactions
                            .hovered_unoccupied_square_on_board
                            .is_some_and(|s| s.coord == Some(coord))
                    {
                        // TODO: paint something on empty tiles when hovered
                        // (maybe? the oly current interaction is to select it for keyboard control)
                        // layers = layers.merge_below_self(TexLayers {
                        //     terrain: None,
                        //     structures: None,
                        //     checkerboard: None,
                        //     piece_validities: vec![],
                        //     mist: None,
                        //     fog: None,
                        //     pieces: vec![PieceLayer::Texture(
                        //         tex::tiles::quad::CHECKERBOARD_HOVER,
                        //         None,
                        //     )],
                        // });
                    }
                }

                if square_is_highlighted && tick % 4 < 2 {
                    let tile_layers = Tex::board_game_tile(
                        MappedTileVariant::Healthy,
                        ' ',
                        Direction::North,
                        Some(aesthetics.theme.ring_selected_hovered),
                        None,
                        TileDecoration::Grass,
                        seed_at_coord,
                    );
                    layers = layers.merge_above_self(tile_layers);
                }
            }
            _ => {}
        }

        if let Some((interactions, coord)) = interactions.zip(coord.real_coord()) {
            if interactions
                .selected_square_on_board
                .is_some_and(|(c, _)| c == coord)
            {
                let spinner = match tick % 4 {
                    0 => tex::tiles::quad::SELECTION_SPINNER_1,
                    1 => tex::tiles::quad::SELECTION_SPINNER_2,
                    2 => tex::tiles::quad::SELECTION_SPINNER_3,
                    3.. => tex::tiles::quad::SELECTION_SPINNER_4,
                };
                let color = gameplay
                    .map(|g| player_colors.get(g.player_number as usize))
                    .flatten()
                    .unwrap_or(&Color32::GOLD);
                layers = layers.merge_above_self(TexLayers {
                    terrain: None,
                    structures: None,
                    checkerboard: None,
                    piece_validities: vec![],
                    mist: None,
                    fog: None,
                    pieces: vec![PieceLayer::Texture(spinner, Some(*color))],
                });
            }
        }

        let cached = self
            .layer_memory
            .get_mut(dest_coord.y)
            .unwrap()
            .get_mut(dest_coord.x)
            .unwrap();

        if *cached == layers {
            return wants_repaint;
        }

        let tile_dims = [measures.inner_tile_width_px, measures.inner_tile_height_px];
        let dest_pos = [
            dest_col * (measures.inner_tile_width_px * 2),
            dest_row * (measures.inner_tile_height_px * 2),
        ];

        let paint_quad = |quad: [Tex; 4], canvas: &mut TextureHandle| {
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

        if cached.checkerboard != layers.checkerboard {
            // Always erase the checkerboard if it needs to be repainted,
            // as it is transparent and will overlay otherwise.
            erase(&mut resolved_textures.checkerboard);
            if let Some(checkerboard) = layers.checkerboard {
                paint_quad(checkerboard, &mut resolved_textures.checkerboard);
            }
        }

        if cached.structures != layers.structures {
            if cached.structures.is_some() && layers.structures.is_none() {
                erase(&mut resolved_textures.structures);
            } else if let Some(structures) = layers.structures {
                paint_quad(structures, &mut resolved_textures.structures);
            }
        }

        let mut render_pieces = |cache: &Vec<PieceLayer>,
                                 layer: &Vec<PieceLayer>,
                                 target_tex: &mut TextureHandle| {
            if cache != layer {
                if !cache.is_empty() && layer.is_empty() {
                    erase(target_tex);
                } else if !layer.is_empty() {
                    let mut target =
                        ColorImage::new([tile_dims[0] * 2, tile_dims[1] * 2], Color32::TRANSPARENT);
                    for piece in layer.iter() {
                        match piece {
                            tex::PieceLayer::Texture(texs, tint) => {
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
                            tex::PieceLayer::Character(char, color, is_flipped, y_offset) => {
                                let mut glyph = glypher.paint(*char, 16);

                                if *is_flipped {
                                    glyph.flip_y();
                                }

                                let offset = [
                                    (target.width() - glyph.width()) / 2,
                                    ((target.height() - glyph.height()) / 2)
                                        .saturating_add_signed(*y_offset),
                                ];

                                glyph.recolor(color);
                                target.hard_overlay(&glyph, offset);
                            }
                        }
                    }

                    target_tex.set_partial(dest_pos, target, egui::TextureOptions::NEAREST);
                }
            }
        };

        render_pieces(
            &cached.pieces,
            &layers.pieces,
            &mut resolved_textures.pieces,
        );
        render_pieces(
            &cached.piece_validities,
            &layers.piece_validities,
            &mut resolved_textures.pieces_validity,
        );

        if cached.mist != layers.mist {
            if cached.mist.is_some() && layers.mist.is_none() {
                erase(&mut resolved_textures.mist);
            } else if let Some(mist) = layers.mist {
                paint_quad(mist, &mut resolved_textures.mist);
            }
        }

        if cached.fog != layers.fog {
            if cached.fog.is_some() && layers.fog.is_none() {
                erase(&mut resolved_textures.fog);
            } else if let Some(fog) = layers.fog {
                paint_quad(fog, &mut resolved_textures.fog);
            }
        }

        *cached = layers;

        wants_repaint
    }

    pub fn remap_texture(
        &mut self,
        ctx: &egui::Context,
        aesthetics: &AestheticDepot,
        timing: &TimingDepot,
        interactions: Option<&InteractionDepot>,
        gameplay: Option<&GameplayDepot>,
        board: &Board,
    ) {
        let mut tick_eq = true;
        let selected_tile = interactions.map(|i| i.selected_tile_on_board).flatten();
        let selected_square = interactions.map(|i| i.selected_square_on_board).flatten();
        let tile_hover = interactions.map(|i| i.hovered_tile_on_board).flatten();
        let dragging = interactions.map(|i| i.dragging_tile_on_board).flatten();
        let occupied_hover = interactions
            .map(|i| i.hovered_occupied_square_on_board)
            .flatten();
        let square_hover = interactions
            .map(|i| i.hovered_unoccupied_square_on_board.clone())
            .flatten();
        let generic_repaint_tick = self.generic_repaint_tick;
        let winner = gameplay.map(|g| g.winner).flatten();

        if let Some(memory) = self.state_memory.as_mut() {
            let board_eq = memory.prev_board == *board;
            let selected_tile_eq = memory.prev_selected_tile == selected_tile;
            let selected_square_eq = memory.prev_selected_square == selected_square;
            let tile_hover_eq = memory.prev_tile_hover == tile_hover;
            let dragging_eq = memory.prev_dragging == dragging;
            let occupied_hover_eq = memory.prev_occupied_hover == occupied_hover;
            let square_hover_eq = memory.prev_square_hover == square_hover;
            let generic_tick_eq = memory.generic_tick == generic_repaint_tick;
            let winner_eq = memory.prev_winner == winner;
            if memory.prev_tick != aesthetics.qs_tick {
                tick_eq = false;
            }

            if board_eq
                && tick_eq
                && selected_tile_eq
                && selected_square_eq
                && tile_hover_eq
                && dragging_eq
                && occupied_hover_eq
                && square_hover_eq
                && generic_tick_eq
                && winner_eq
            {
                return;
            }

            if !board_eq {
                memory.prev_board = board.clone();
                self.distance_to_land = board.flood_fill_water_from_land();
            }
            if !selected_tile_eq {
                memory.prev_selected_tile = selected_tile;
            }
            if !selected_square_eq {
                memory.prev_selected_square = selected_square;
            }
            if !tile_hover_eq {
                memory.prev_tile_hover = tile_hover;
            }
            if !dragging_eq {
                memory.prev_dragging = dragging;
            }
            if !occupied_hover_eq {
                memory.prev_occupied_hover = occupied_hover;
            }
            if !square_hover_eq {
                memory.prev_square_hover = square_hover;
            }
            if !tick_eq {
                memory.prev_tick = aesthetics.qs_tick;
            }
            if !generic_tick_eq {
                memory.generic_tick = generic_repaint_tick;
            }
            if !winner_eq {
                memory.prev_winner = winner;
            }
        } else {
            self.state_memory = Some(MapState {
                prev_board: board.clone(),
                prev_tick: aesthetics.qs_tick,
                prev_selected_tile: selected_tile,
                prev_selected_square: selected_square,
                prev_tile_hover: tile_hover,
                prev_dragging: dragging,
                prev_occupied_hover: occupied_hover,
                prev_square_hover: square_hover,
                prev_changes: vec![],
                generic_tick: 0,
                prev_winner: winner,
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
        let glypher = GLYPHER.get().expect("Glypher should have been initialized");

        let total_buffer = self.map_buffer * 2;

        let final_width = measures.inner_tile_width_px * (board.width() + total_buffer) * 2;
        let final_height = measures.inner_tile_height_px * (board.height() + total_buffer) * 2;
        let sized_correct = self
            .resolved_textures
            .as_ref()
            .is_some_and(|t| t.terrain.size() == [final_width, final_height]);
        // Some game modes, or board editors, allow the board to resize.
        // For now we just throw our textures away if this happens, rather
        // than try to match old coordinates to new.
        if !sized_correct {
            self.resolved_textures = Some(ResolvedTextureLayers::new(
                board,
                measures,
                self.map_buffer,
                ctx,
            ));
            self.layer_memory = vec![
                vec![TexLayers::default(); board.width() + total_buffer];
                board.height() + total_buffer
            ];
        }

        for dest_row in 0..(board.height() + total_buffer) {
            for dest_col in 0..(board.width() + total_buffer) {
                let mut source_col = dest_col as isize - self.map_buffer as isize;
                let mut source_row = dest_row as isize - self.map_buffer as isize;

                if self.inverted {
                    source_col = board.width() as isize - source_col - 1;
                    source_row = board.height() as isize - source_row - 1;
                }

                let source_coord = SignedCoordinate::new(source_col, source_row);

                let square = source_coord
                    .real_coord()
                    .and_then(|c| board.get(c).ok())
                    .unwrap_or_else(|| Square::Water { foggy: false });

                let wants_repaint = self.paint_square_offscreen(
                    ctx,
                    board,
                    &aesthetics.player_colors,
                    aesthetics.qs_tick,
                    source_row as _,
                    source_col as _,
                    dest_row,
                    dest_col,
                    &square,
                    measures,
                    tileset,
                    glypher,
                    interactions,
                    gameplay,
                    aesthetics,
                    timing,
                );

                if wants_repaint {
                    ctx.request_repaint_after(Duration::from_millis(16));
                    self.generic_repaint_tick += 1;
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
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
    last_tick: u64,
    last_selected: Option<Vec<char>>,
    capacity: usize,
}

impl MappedTiles {
    pub fn new(egui_ctx: &egui::Context, capacity: usize) -> Self {
        Self {
            tile_texture: MappedTiles::reset_texture(capacity, egui_ctx),
            slots: Vec::with_capacity(capacity),
            last_tick: 0,
            last_selected: None,
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

    pub fn remap_texture(
        &mut self,
        egui_ctx: &egui::Context,
        slots: Vec<MappedTile>,
        aesthetics: &AestheticDepot,
        interactions: Option<&InteractionDepot>,
    ) {
        let selected_tiles = interactions.map(|i| i.highlight_tiles.clone()).flatten();
        // We only animate if there are selected tiles,
        // otherwise we don't want the tick to trigger re-rendering.
        let tick = if selected_tiles.is_some() {
            aesthetics.qs_tick
        } else {
            0
        };
        if slots == self.slots && selected_tiles == self.last_selected && tick == self.last_tick {
            return;
        }

        self.slots = slots;
        self.last_selected = selected_tiles;
        self.last_tick = tick;

        if self.capacity < self.slots.len() {
            self.tile_texture = MappedTiles::reset_texture(self.slots.len(), egui_ctx);
        }

        let measures = TEXTURE_MEASUREMENT
            .get()
            .expect("Base texture should have been measured");
        let tileset = TEXTURE_IMAGE
            .get()
            .expect("Base image should have been loaded");
        let glypher = GLYPHER.get().expect("Glypher should have been initialized");

        let tile_dims = [measures.inner_tile_width_px, measures.inner_tile_height_px];

        for (i, slot) in self.slots.iter().enumerate() {
            let tile_is_highlighted = self
                .last_selected
                .as_ref()
                .is_some_and(|t| t.contains(&slot.character));

            let color = if tile_is_highlighted && tick % 4 < 2 {
                Some(hex_color!("#FFDE85"))
            } else {
                slot.color.map(|c| c.lighten())
            };

            let tile_layers = Tex::board_game_tile(
                slot.variant.clone(),
                slot.character,
                slot.orientation,
                color,
                slot.highlight,
                TileDecoration::None,
                0,
            );

            let mut target =
                ColorImage::new([tile_dims[0] * 2, tile_dims[1] * 2], Color32::TRANSPARENT);
            for piece in tile_layers.pieces.iter() {
                match piece {
                    tex::PieceLayer::Texture(texs, tint) => {
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
                    tex::PieceLayer::Character(char, color, is_flipped, y_offset) => {
                        let mut glyph = glypher.paint(*char, 16);

                        if *is_flipped {
                            glyph.flip_y();
                        }

                        let offset = [
                            (target.width() - glyph.width()) / 2,
                            ((target.height() - glyph.height()) / 2)
                                .saturating_add_signed(*y_offset),
                        ];

                        glyph.recolor(color);
                        target.hard_overlay(&glyph, offset);
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
