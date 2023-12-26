use std::{
    collections::VecDeque,
    ops::{Range, RangeInclusive},
};

use eframe::egui::{self, Sense};
use epaint::{pos2, vec2, Color32, Mesh, Pos2, Rect, Shape, TextureHandle, TextureId, Vec2};
use truncate_core::board::Square;

use crate::{app_outer::TEXTURE_MEASUREMENT, regions::lobby::BoardEditingMode};

use super::mapper::{quickrand, MappedTileVariant};

pub mod tiles;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Tex {
    tile: usize,
    tint: Option<Color32>,
}

pub type TexQuad = [Tex; 4];

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct TexLayers {
    pub terrain: Option<TexQuad>,
    pub structures: Option<TexQuad>,
    pub tinted: Option<(TexQuad, Option<Color32>)>,
    pub overlay: Option<TexQuad>,
}

impl TexLayers {
    fn terrain(mut self, quad: TexQuad) -> Self {
        self.terrain = Some(quad);
        self
    }

    fn structures(mut self, quad: TexQuad) -> Self {
        self.structures = Some(quad);
        self
    }

    fn tinted(mut self, quad: TexQuad, tint: Option<Color32>) -> Self {
        self.tinted = Some((quad, tint));
        self
    }

    fn overlay(mut self, quad: TexQuad) -> Self {
        self.overlay = Some(quad);
        self
    }

    fn merge(mut self, other: &TexLayers) -> Self {
        Self {
            terrain: self.terrain.or(other.terrain),
            structures: self.structures.or(other.structures),
            tinted: self.tinted.or(other.tinted),
            overlay: self.overlay.or(other.overlay),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum BGTexType {
    Land,
    Water,
}

#[derive(Debug, Copy, Clone)]
pub enum FGTexType {
    Town,
    Dock,
}

// Wind levels
macro_rules! calm {
    () => {
        0..=56
    };
}
macro_rules! breeze {
    () => {
        57..=69
    };
}
macro_rules! wind {
    () => {
        70..=85
    };
}

pub trait Tint {
    fn tint(self, color: Color32) -> Self;
}

impl Tint for Tex {
    fn tint(mut self, color: Color32) -> Self {
        self.tint = Some(color);
        self
    }
}

impl Tint for TexQuad {
    fn tint(mut self, color: Color32) -> Self {
        for i in 0..4 {
            self[i].tint = Some(color);
        }
        self
    }
}

impl Tint for Vec<Tex> {
    fn tint(mut self, color: Color32) -> Self {
        for tex in &mut self {
            tex.tint = Some(color);
        }
        self
    }
}

impl Tint for Vec<Vec<Tex>> {
    fn tint(mut self, color: Color32) -> Self {
        for row in &mut self {
            for tex in row {
                tex.tint = Some(color);
            }
        }
        self
    }
}

impl Tex {
    pub fn game_tile(color: Option<Color32>, highlight: Option<Color32>) -> Vec<TexQuad> {
        let mut tex = vec![];

        if let Some(color) = color {
            tex.push(tiles::quad::GAME_PIECE.tint(color));
        }

        if let Some(highlight) = highlight {
            tex.push(tiles::quad::HIGHLIGHT.tint(highlight));
        }
        tex
    }

    pub fn board_game_tile(
        variant: MappedTileVariant,
        color: Option<Color32>,
        highlight: Option<Color32>,
        seed: usize,
    ) -> Vec<TexQuad> {
        let mut texs = Tex::game_tile(color, highlight);
        texs.push([
            tiles::NONE,
            tiles::NONE,
            match quickrand(seed) % 100 {
                0..=25 => tiles::GAME_PIECE_GRASS_0_SE,
                26..=50 => tiles::GAME_PIECE_GRASS_1_SE,
                51..=75 => tiles::GAME_PIECE_GRASS_2_SE,
                _ => tiles::GAME_PIECE_GRASS_3_SE,
            },
            match quickrand(seed + 678) % 100 {
                0..=25 => tiles::GAME_PIECE_GRASS_0_SW,
                26..=50 => tiles::GAME_PIECE_GRASS_1_SW,
                51..=75 => tiles::GAME_PIECE_GRASS_2_SW,
                _ => tiles::GAME_PIECE_GRASS_3_SW,
            },
        ]);
        match variant {
            MappedTileVariant::Healthy => {}
            MappedTileVariant::Dying => {
                texs.push(
                    match quickrand(seed) % 100 {
                        0..=19 => tiles::quad::GAME_PIECE_CRACKS_0,
                        20..=39 => tiles::quad::GAME_PIECE_CRACKS_1,
                        40..=59 => tiles::quad::GAME_PIECE_CRACKS_2,
                        60..=79 => tiles::quad::GAME_PIECE_CRACKS_3,
                        _ => tiles::quad::GAME_PIECE_CRACKS_4,
                    }
                    .tint(color.unwrap_or_default()),
                );
            }
            MappedTileVariant::Dead => {
                for i in 0..2 {
                    texs.push(
                        match quickrand(seed + i) % 100 {
                            0..=19 => tiles::quad::GAME_PIECE_CRACKS_0,
                            20..=39 => tiles::quad::GAME_PIECE_CRACKS_1,
                            40..=59 => tiles::quad::GAME_PIECE_CRACKS_2,
                            60..=79 => tiles::quad::GAME_PIECE_CRACKS_3,
                            _ => tiles::quad::GAME_PIECE_CRACKS_4,
                        }
                        .tint(color.unwrap_or_default()),
                    );
                }
            }
            MappedTileVariant::Gone => {
                texs = vec![[
                    match quickrand(seed + 345) % 100 {
                        0..=33 => tiles::GAME_PIECE_RUBBLE_0_NW,
                        34..=66 => tiles::GAME_PIECE_RUBBLE_1_NW,
                        _ => tiles::GAME_PIECE_RUBBLE_2_NW,
                    },
                    match quickrand(seed + 757) % 100 {
                        0..=33 => tiles::GAME_PIECE_RUBBLE_0_NE,
                        34..=66 => tiles::GAME_PIECE_RUBBLE_1_NE,
                        _ => tiles::GAME_PIECE_RUBBLE_2_NE,
                    },
                    match quickrand(seed + 8447) % 100 {
                        0..=33 => tiles::GAME_PIECE_RUBBLE_0_SE,
                        34..=66 => tiles::GAME_PIECE_RUBBLE_1_SE,
                        _ => tiles::GAME_PIECE_RUBBLE_2_SE,
                    },
                    match quickrand(seed + 477387) % 100 {
                        0..=33 => tiles::GAME_PIECE_RUBBLE_0_SW,
                        34..=66 => tiles::GAME_PIECE_RUBBLE_1_SW,
                        _ => tiles::GAME_PIECE_RUBBLE_2_SW,
                    },
                ]];
            }
        }
        texs
    }

    pub fn town_button(color: Option<Color32>, highlight: Option<Color32>) -> Vec<TexQuad> {
        let mut t = vec![
            tiles::quad::TOWN_BUTTON,
            if let Some(color) = color {
                tiles::quad::TOWN_BUTTON_ROOF.tint(color)
            } else {
                tiles::quad::TOWN_BUTTON_ROOF
            },
        ];
        if let Some(highlight) = highlight {
            t.push(tiles::quad::HIGHLIGHT.tint(highlight));
        };
        t
    }

    pub fn dock_button(color: Option<Color32>, highlight: Option<Color32>) -> Vec<TexQuad> {
        let mut t = vec![
            tiles::quad::DOCK_BUTTON,
            if let Some(color) = color {
                tiles::quad::DOCK_BUTTON_SAIL.tint(color)
            } else {
                tiles::quad::DOCK_BUTTON_SAIL
            },
        ];
        if let Some(highlight) = highlight {
            t.push(tiles::quad::HIGHLIGHT.tint(highlight));
        };
        t
    }

    pub fn land_button(highlight: Option<Color32>) -> Vec<TexQuad> {
        if let Some(highlight) = highlight {
            vec![
                tiles::quad::TERRAIN_BUTTON,
                tiles::quad::HIGHLIGHT.tint(highlight),
            ]
        } else {
            vec![tiles::quad::TERRAIN_BUTTON]
        }
    }

    pub fn text_button(ratio: f32) -> Vec<Tex> {
        let extra_tiles = ratio as usize;
        [
            vec![tiles::GAME_PIECE_NW],
            vec![tiles::GAME_PIECE_N; extra_tiles],
            vec![tiles::GAME_PIECE_NE, tiles::GAME_PIECE_SE],
            vec![tiles::GAME_PIECE_S; extra_tiles],
            vec![tiles::GAME_PIECE_SW],
        ]
        .concat()
    }

    pub fn text_dialog(x_tiles: usize, y_tiles: usize) -> Vec<Vec<Tex>> {
        let middle_x_tiles = x_tiles.saturating_sub(2);
        let middle_y_tiles = y_tiles.saturating_sub(2);

        [
            vec![[
                vec![tiles::DIALOG_NW],
                vec![tiles::DIALOG_N; middle_x_tiles],
                vec![tiles::DIALOG_NE],
            ]
            .concat()],
            vec![
                [
                    vec![tiles::DIALOG_W],
                    vec![tiles::DIALOG_CENTER; middle_x_tiles],
                    vec![tiles::DIALOG_E]
                ]
                .concat();
                middle_y_tiles
            ],
            vec![[
                vec![tiles::DIALOG_SW],
                vec![tiles::DIALOG_S; middle_x_tiles],
                vec![tiles::DIALOG_SE],
            ]
            .concat()],
        ]
        .concat()
    }

    fn dock(color: Option<Color32>, neighbors: Vec<BGTexType>, wind_at_coord: u8) -> TexLayers {
        // TODO: Render docks with multiple edges somehow.

        let mut dock = if matches!(neighbors[1], BGTexType::Land) {
            TexLayers::default()
                .structures(tiles::quad::SOUTH_DOCK)
                .tinted(
                    match wind_at_coord {
                        calm!() => tiles::quad::SOUTH_DOCK_SAIL_WIND_0,
                        breeze!() => tiles::quad::SOUTH_DOCK_SAIL_WIND_1,
                        _ => tiles::quad::SOUTH_DOCK_SAIL_WIND_2,
                    },
                    color,
                )
        } else if matches!(neighbors[5], BGTexType::Land) {
            TexLayers::default()
                .structures(tiles::quad::NORTH_DOCK)
                .tinted(
                    match wind_at_coord {
                        calm!() => tiles::quad::NORTH_DOCK_SAIL_WIND_0,
                        breeze!() => tiles::quad::NORTH_DOCK_SAIL_WIND_1,
                        _ => tiles::quad::NORTH_DOCK_SAIL_WIND_2,
                    },
                    color,
                )
        } else if matches!(neighbors[3], BGTexType::Land) {
            TexLayers::default()
                .structures(tiles::quad::WEST_DOCK)
                .tinted(
                    match wind_at_coord {
                        calm!() => tiles::quad::WEST_DOCK_SAIL_WIND_0,
                        breeze!() => tiles::quad::WEST_DOCK_SAIL_WIND_1,
                        _ => tiles::quad::WEST_DOCK_SAIL_WIND_2,
                    },
                    color,
                )
        } else if matches!(neighbors[7], BGTexType::Land) {
            TexLayers::default()
                .structures(tiles::quad::EAST_DOCK)
                .tinted(
                    match wind_at_coord {
                        calm!() => tiles::quad::EAST_DOCK_SAIL_WIND_0,
                        breeze!() => tiles::quad::EAST_DOCK_SAIL_WIND_1,
                        _ => tiles::quad::EAST_DOCK_SAIL_WIND_2,
                    },
                    color,
                )
        } else {
            TexLayers::default()
                .structures(tiles::quad::FLOATING_DOCK)
                .tinted(
                    match wind_at_coord {
                        calm!() => tiles::quad::FLOATING_DOCK_SAIL_WIND_0,
                        breeze!() => tiles::quad::FLOATING_DOCK_SAIL_WIND_1,
                        _ => tiles::quad::FLOATING_DOCK_SAIL_WIND_2,
                    },
                    color,
                )
        };
        dock
    }

    fn town(color: Option<Color32>, seed: usize, tick: u64, wind_at_coord: u8) -> TexLayers {
        let anim_index = (quickrand(seed + 3) + tick as usize) % 30;
        let rand_house = |n: usize| match quickrand(n) {
            0..=25 => (
                tiles::HOUSE_0,
                tiles::ROOF_0,
                // TODO: revive the smoke animations
                // match (anim_index, wind_at_coord) {
                //     (5.., _) => tiles::NONE,
                //     (_, calm!()) => tiles::ROOF1_SMOKE[anim_index],
                //     (_, breeze!()) => tiles::ROOF1_SMOKE_WIND0[anim_index],
                //     _ => tiles::ROOF1_SMOKE_WIND1[anim_index],
                // },
            ),
            26..=50 => (tiles::HOUSE_2, tiles::ROOF_2),
            51..=75 => (
                tiles::HOUSE_1,
                tiles::ROOF_1,
                // TODO: revive the smoke animations
                // match (anim_index, wind_at_coord) {
                //     (5.., _) => tiles::NONE,
                //     (_, calm!()) => tiles::ROOF2_SMOKE[anim_index],
                //     (_, breeze!()) => tiles::ROOF2_SMOKE_WIND0[anim_index],
                //     _ => tiles::ROOF2_SMOKE_WIND1[anim_index],
                // },
            ),
            _ => (tiles::HOUSE_3, tiles::ROOF_3),
        };

        let rand_house_colored = |n: usize| {
            let mut h = rand_house(n);
            if let Some(color) = &color {
                h.1 = h.1.tint(*color);
            }
            h
        };

        let rand_decor = |n: usize| match quickrand(n) {
            0..=20 => (tiles::BUSH_0, tiles::BUSH_FLOWERS_0),
            21..=40 => (tiles::BUSH_1, tiles::BUSH_FLOWERS_1),
            41..=60 => (tiles::BUSH_2, tiles::BUSH_FLOWERS_2),
            61..=80 => (
                match wind_at_coord {
                    calm!() | breeze!() => tiles::WHEAT_WIND_0,
                    _ => tiles::WHEAT_WIND_1,
                },
                tiles::NONE,
            ),
            _ => (tiles::WELL, tiles::NONE),
        };

        let rand_decor_colored = |n: usize| {
            let mut d = rand_decor(n);
            if let Some(color) = &color {
                d.1 = d.1.tint(*color);
            }
            d
        };

        let rand_path = |n: usize| match quickrand(n) {
            0..=20 => tiles::MISC_COBBLE_0,
            21..=30 => tiles::MISC_COBBLE_1,
            31..=40 => tiles::MISC_COBBLE_2,
            41..=50 => tiles::MISC_COBBLE_3,
            51..=60 => tiles::MISC_COBBLE_4,
            61..=70 => tiles::MISC_COBBLE_5,
            71..=80 => tiles::MISC_COBBLE_6,
            81..=90 => tiles::MISC_COBBLE_7,
            _ => tiles::MISC_COBBLE_8,
        };

        let numhouses = match quickrand(seed + 345) {
            0..=50 => 1,
            _ => 2,
        };

        let numdecor = match quickrand(seed + 23465) {
            0..=70 => 0,
            _ => 1,
        };

        let mut structures = [
            rand_path(seed + 4),
            rand_path(seed + 44),
            rand_path(seed + 444),
            rand_path(seed + 4444),
        ];
        let mut tinted = [tiles::NONE, tiles::NONE, tiles::NONE, tiles::NONE];

        for d in 0..numdecor {
            let decorpos = quickrand(seed + 454 + d + d) % 4;
            let (decor, layer) = rand_decor_colored(seed + 646 * d);

            structures[decorpos] = decor;
            tinted[decorpos] = layer;
        }

        // These may bowl each other over but that's fine,
        // it just skews the average house number down slightly.
        for h in 0..numhouses {
            let housepos = quickrand(seed + 45 * h) % 4;
            let (house, roof /*, smoke*/) = rand_house_colored(seed + 6 * h);

            structures[housepos] = house;
            tinted[housepos] = roof;
            // texs[2][housepos] = smoke;
        }

        TexLayers::default()
            .structures(structures)
            .tinted(tinted, color)
    }

    /// Determine the tiles to use based on a given square and its neighbors,
    /// provided clockwise from northwest.
    pub fn terrain(
        base_type: BGTexType,
        layer_type: Option<FGTexType>,
        neighbors: Vec<BGTexType>,
        color: Option<Color32>,
        seed: usize,
        tick: u64,
        wind_at_coord: u8,
    ) -> TexLayers {
        debug_assert_eq!(neighbors.len(), 8);
        if neighbors.len() != 8 {
            return TexLayers::default().terrain([
                tiles::DEBUG,
                tiles::DEBUG,
                tiles::DEBUG,
                tiles::DEBUG,
            ]);
        }

        let grasses = match wind_at_coord {
            calm!() => [
                tiles::BASE_GRASS,
                tiles::GRASS_0_WIND_0,
                tiles::GRASS_1_WIND_0,
                tiles::GRASS_2_WIND_0,
            ],
            breeze!() => [
                tiles::BASE_GRASS,
                tiles::GRASS_0_WIND_1,
                tiles::GRASS_1_WIND_1,
                tiles::GRASS_2_WIND_1,
            ],
            wind!() => [
                tiles::BASE_GRASS,
                tiles::GRASS_0_WIND_2,
                tiles::GRASS_1_WIND_2,
                tiles::GRASS_2_WIND_2,
            ],
            _ => [
                tiles::BASE_GRASS,
                tiles::GRASS_0_WIND_3,
                tiles::GRASS_1_WIND_3,
                tiles::GRASS_2_WIND_3,
            ],
        };

        let rand_grass = |n: usize| match quickrand(n) {
            0..=70 => grasses[0],
            71..=85 => grasses[1],
            86..=94 => grasses[2],
            _ => grasses[3],
        };

        use BGTexType::*;
        let top_left = match base_type {
            Land => rand_grass(seed),
            Water => match (neighbors[7], neighbors[0], neighbors[1]) {
                (Land, Land | Water, Land) => tiles::LAND_WITH_WATER_SE,
                (Land, Land | Water, Water) => tiles::WATER_WITH_LAND_W,
                (Water, Land | Water, Land) => tiles::WATER_WITH_LAND_N,
                (Water, Land, Water) => tiles::WATER_WITH_LAND_NW,
                (Water, Water, Water) => tiles::BASE_WATER,
            },
        };

        let top_right = match base_type {
            Land => rand_grass(seed + 1),
            Water => match (neighbors[1], neighbors[2], neighbors[3]) {
                (Land, Land | Water, Land) => tiles::LAND_WITH_WATER_SW,
                (Land, Land | Water, Water) => tiles::WATER_WITH_LAND_N,
                (Water, Land | Water, Land) => tiles::WATER_WITH_LAND_E,
                (Water, Land, Water) => tiles::WATER_WITH_LAND_NE,
                (Water, Water, Water) => tiles::BASE_WATER,
            },
        };

        let bottom_right = match base_type {
            Land => rand_grass(seed + 2),
            Water => match (neighbors[3], neighbors[4], neighbors[5]) {
                (Land, Land | Water, Land) => tiles::LAND_WITH_WATER_NW,
                (Land, Land | Water, Water) => tiles::WATER_WITH_LAND_E,
                (Water, Land | Water, Land) => tiles::WATER_WITH_LAND_S,
                (Water, Land, Water) => tiles::WATER_WITH_LAND_SE,
                (Water, Water, Water) => tiles::BASE_WATER,
            },
        };

        let bottom_left = match base_type {
            Land => rand_grass(seed + 3),
            Water => match (neighbors[5], neighbors[6], neighbors[7]) {
                (Land, Land | Water, Land) => tiles::LAND_WITH_WATER_NE,
                (Land, Land | Water, Water) => tiles::WATER_WITH_LAND_S,
                (Water, Land | Water, Land) => tiles::WATER_WITH_LAND_W,
                (Water, Land, Water) => tiles::WATER_WITH_LAND_SW,
                (Water, Water, Water) => tiles::BASE_WATER,
            },
        };

        let mut layers =
            TexLayers::default().terrain([top_left, top_right, bottom_right, bottom_left]);

        if let Some(layer) = layer_type {
            match layer {
                FGTexType::Town => {
                    layers = layers.merge(&Tex::town(color, seed, tick, wind_at_coord))
                }
                FGTexType::Dock => {
                    layers = layers.merge(&Tex::dock(color, neighbors, wind_at_coord))
                }
            }
        }

        layers
    }

    pub fn landscaping(from: &Square, action: &BoardEditingMode) -> Option<TexQuad> {
        match (action, from) {
            (
                BoardEditingMode::Land | BoardEditingMode::Town(_),
                Square::Water | Square::Dock(_),
            ) => Some(tiles::quad::ISLAND),
            (
                BoardEditingMode::Land | BoardEditingMode::Dock(_),
                Square::Land | Square::Town { .. },
            ) => Some(tiles::quad::LAKE),
            _ => None,
        }
    }
}

impl Tex {
    pub fn render(self, map_texture: TextureId, rect: Rect, ui: &mut egui::Ui) {
        let measures = TEXTURE_MEASUREMENT
            .get()
            .expect("Texture should be loaded and measured");

        let row = (self.tile / measures.num_tiles_x) as f32;
        let col = (self.tile % measures.num_tiles_x) as f32;

        let left = measures.outer_tile_width * col + measures.x_padding_pct;
        let top = measures.outer_tile_height * row + measures.y_padding_pct;

        let uv = Rect::from_min_max(
            pos2(
                // Index to our tile, and skip over the leading column padding
                left, top,
            ),
            pos2(
                // Index to our next tile, and skip behind our trailing column padding
                left + measures.inner_tile_width,
                top + measures.inner_tile_height,
            ),
        );

        let mut mesh = Mesh::with_texture(map_texture);
        mesh.add_rect_with_uv(rect, uv, self.tint.unwrap_or(Color32::WHITE));
        ui.painter().add(Shape::mesh(mesh));
    }

    pub fn get_source_position(&self) -> Pos2 {
        let measures = TEXTURE_MEASUREMENT
            .get()
            .expect("Texture should be loaded and measured");

        let row = (self.tile / measures.num_tiles_x) as f32;
        let col = (self.tile % measures.num_tiles_x) as f32;

        let left = measures.outer_tile_width * col + measures.x_padding_pct;
        let top = measures.outer_tile_height * row + measures.y_padding_pct;

        pos2(
            // Index to our tile, and skip over the leading column padding
            left, top,
        )
    }
}

pub fn render_tex_quad(texs: TexQuad, rect: Rect, map_texture: &TextureHandle, ui: &mut egui::Ui) {
    let ts = rect.width() * 0.25;
    let tile_rect = rect.shrink(ts);

    for (tex, translate) in texs
        .into_iter()
        .zip([vec2(-ts, -ts), vec2(ts, -ts), vec2(ts, ts), vec2(-ts, ts)].into_iter())
    {
        tex.render(map_texture.id(), tile_rect.translate(translate), ui);
    }
}

pub fn render_tex_quads(
    texs: &[TexQuad],
    rect: Rect,
    map_texture: &TextureHandle,
    ui: &mut egui::Ui,
) {
    for tex in texs.iter() {
        render_tex_quad(*tex, rect, map_texture, ui);
    }
}

pub fn render_texs_clockwise(
    texs: Vec<Tex>,
    rect: Rect,
    map_texture: &TextureHandle,
    ui: &mut egui::Ui,
) {
    let h_tex_count = texs.len() / 2;
    let tsize = rect.width() / h_tex_count as f32;
    let mut tile_rect = rect.clone();
    tile_rect.set_width(tsize);
    tile_rect.set_height(rect.height() / 2.0);

    for toptex in 0..h_tex_count {
        let tex = texs[toptex];
        tex.render(
            map_texture.id(),
            tile_rect.translate(vec2(tile_rect.width() * toptex as f32, 0.0)),
            ui,
        );
    }
    for bottex in h_tex_count..texs.len() {
        let tex = texs[bottex];
        tex.render(
            map_texture.id(),
            tile_rect.translate(vec2(
                tile_rect.width() * (h_tex_count - (bottex - h_tex_count) - 1) as f32,
                tile_rect.height(),
            )),
            ui,
        );
    }
}

pub fn render_tex_rows(
    texs: Vec<Vec<Tex>>,
    rect: Rect,
    map_texture: &TextureHandle,
    ui: &mut egui::Ui,
) {
    let region = rect.size();
    let tile_height = region.y / texs.len() as f32;

    let mut tile_rect = rect.clone();
    tile_rect.set_height(tile_height);

    for (rownum, row) in texs.into_iter().enumerate() {
        let tile_width = region.x / row.len() as f32;
        tile_rect.set_width(tile_width);

        for (colnum, tex) in row.into_iter().enumerate() {
            tex.render(
                map_texture.id(),
                tile_rect.translate(vec2(
                    tile_width * colnum as f32,
                    tile_height * rownum as f32,
                )),
                ui,
            );
        }
    }
}

pub fn paint_dialog_background(
    full_width: bool,
    full_height: bool,
    centered: bool,
    size_to_pos: Vec2,
    background_color: Color32,
    map_texture: &TextureHandle,
    ui: &mut egui::Ui,
) -> (Rect, egui::Response) {
    let text_size = size_to_pos;
    let dialog_width = if full_width {
        ui.available_width()
    } else {
        text_size.x
    };

    let width_in_tiles = (dialog_width / 16.0) as usize;
    let dialog_tile_size = dialog_width / width_in_tiles as f32;

    let height_in_tiles = if full_height {
        (ui.available_height() / dialog_tile_size) as usize
    } else {
        (text_size.y / dialog_tile_size).ceil() as usize + 2
    };
    let dialog_height = height_in_tiles as f32 * dialog_tile_size;

    let dialog_texs = Tex::text_dialog(width_in_tiles, height_in_tiles);

    let (dialog_rect, dialog_resp) = if centered {
        ui.horizontal(|ui| {
            let centered_offset = (ui.available_width() - dialog_width) * 0.5;
            ui.add_space(centered_offset);
            ui.allocate_exact_size(vec2(dialog_width, dialog_height), Sense::click())
        })
        .inner
    } else {
        ui.allocate_exact_size(vec2(dialog_width, dialog_height), Sense::click())
    };

    render_tex_rows(
        dialog_texs.tint(background_color),
        dialog_rect,
        map_texture,
        ui,
    );

    let inner_dialog_rect = dialog_rect.shrink(dialog_tile_size / 2.0);
    (inner_dialog_rect, dialog_resp)
}
