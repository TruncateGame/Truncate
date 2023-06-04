use eframe::egui;
use epaint::{pos2, vec2, Color32, Mesh, Rect, Shape, TextureHandle, TextureId};
use truncate_core::board::Square;

use crate::regions::lobby::BoardEditingMode;

#[derive(Debug, Copy, Clone)]
pub struct Tex {
    tile: usize,
    tint: Option<Color32>,
}

pub type TexQuad = [Tex; 4];

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

// TODO: Generate this impl with codegen from aseprite
impl Tex {
    const fn single(tile: usize) -> Self {
        Self { tile, tint: None }
    }

    const fn quad(nw_tile: usize, ne_tile: usize, se_tile: usize, sw_tile: usize) -> [Self; 4] {
        [
            Self {
                tile: nw_tile,
                tint: None,
            },
            Self {
                tile: ne_tile,
                tint: None,
            },
            Self {
                tile: se_tile,
                tint: None,
            },
            Self {
                tile: sw_tile,
                tint: None,
            },
        ]
    }

    pub const MAX_TILE: usize = 148;

    pub const NONE: Self = Tex::single(0);
    pub const DEBUG: Self = Tex::single(77);

    pub const GRASS1: Self = Tex::single(21);
    pub const GRASS2: Self = Tex::single(22);
    pub const GRASS3: Self = Tex::single(23);
    pub const GRASS4: Self = Tex::single(24);

    pub const WATER: Self = Tex::single(8);

    // Water with land on one edge
    pub const LAND_SE: Self = Tex::single(4);
    pub const LAND_S: Self = Tex::single(5);
    pub const LAND_SW: Self = Tex::single(6);
    pub const LAND_W: Self = Tex::single(11);
    pub const LAND_NW: Self = Tex::single(17);
    pub const LAND_N: Self = Tex::single(16);
    pub const LAND_NE: Self = Tex::single(15);
    pub const LAND_E: Self = Tex::single(10);

    // Water with land on two edges
    pub const LAND_N_E: Self = Tex::single(25);
    pub const LAND_S_E: Self = Tex::single(18);
    pub const LAND_S_W: Self = Tex::single(19);
    pub const LAND_N_W: Self = Tex::single(26);

    // Transparent island in water
    pub const ISLAND: TexQuad = Tex::quad(69, 70, 72, 71);
    // Transparent lake on land
    pub const LAKE: TexQuad = Tex::quad(73, 74, 76, 75);

    // Tiles
    pub const GAME_TILE: TexQuad = Tex::quad(53, 54, 55, 56);

    // Tiles for buttons
    pub const GAME_TILE_NW: Self = Tex::single(53);
    pub const GAME_TILE_SW: Self = Tex::single(56);
    pub const GAME_TILE_N: Self = Tex::single(147);
    pub const GAME_TILE_S: Self = Tex::single(148);
    pub const GAME_TILE_NE: Self = Tex::single(54);
    pub const GAME_TILE_SE: Self = Tex::single(55);

    // Highlight rings for tiles
    pub const HIGHLIGHT: TexQuad = Tex::quad(65, 66, 68, 67);

    // Grass cover over tiles
    pub const TILE_SE_GRASS1: Self = Tex::single(58);
    pub const TILE_SE_GRASS2: Self = Tex::single(60);
    pub const TILE_SE_GRASS3: Self = Tex::single(62);
    pub const TILE_SE_GRASS4: Self = Tex::single(64);

    pub const TILE_SW_GRASS1: Self = Tex::single(57);
    pub const TILE_SW_GRASS2: Self = Tex::single(59);
    pub const TILE_SW_GRASS3: Self = Tex::single(61);
    pub const TILE_SW_GRASS4: Self = Tex::single(63);

    // HOUSES
    pub const HOUSE1: Self = Tex::single(36);
    pub const HOUSE2: Self = Tex::single(37);
    pub const HOUSE3: Self = Tex::single(38);
    pub const HOUSE4: Self = Tex::single(39);

    // ROOFS
    pub const ROOF1: Self = Tex::single(32);
    pub const ROOF2: Self = Tex::single(33);
    pub const ROOF3: Self = Tex::single(34);
    pub const ROOF4: Self = Tex::single(35);

    // DOCKS
    pub const DOCK_NORTH: TexQuad = Tex::quad(79, 81, 82, 80);
    pub const DOCK_NORTH_SAIL: TexQuad = Tex::quad(95, 96, 0, 0);

    pub const DOCK_EAST: TexQuad = Tex::quad(87, 88, 90, 89);
    pub const DOCK_EAST_SAIL: TexQuad = Tex::quad(99, 100, 108, 107);

    pub const DOCK_SOUTH: TexQuad = Tex::quad(83, 85, 86, 84);
    pub const DOCK_SOUTH_SAIL: TexQuad = Tex::quad(97, 0, 0, 105);

    pub const DOCK_WEST: TexQuad = Tex::quad(91, 92, 94, 93);
    pub const DOCK_WEST_SAIL: TexQuad = Tex::quad(101, 102, 0, 0);

    pub const DOCK_DISCONNECTED: TexQuad = Tex::quad(139, 140, 142, 141);
    pub const DOCK_DISCONNECTED_SAIL: TexQuad = Tex::quad(143, 144, 146, 145);

    // PATHS
    pub const PATH1: Self = Tex::single(40);
    pub const PATH2: Self = Tex::single(41);
    pub const PATH3: Self = Tex::single(42);
    pub const PATH4: Self = Tex::single(43);
    pub const PATH5: Self = Tex::single(44);
    pub const PATH6: Self = Tex::single(45);
    pub const PATH7: Self = Tex::single(46);
    pub const PATH8: Self = Tex::single(47);
    pub const PATH9: Self = Tex::single(48);

    // DECOR
    pub const DECOR_PLANTER1: Self = Tex::single(111);
    pub const DECOR_PLANTER1_COLOR: Self = Tex::single(116);
    pub const DECOR_PLANTER2: Self = Tex::single(112);
    pub const DECOR_PLANTER2_COLOR: Self = Tex::single(117);
    pub const DECOR_BUSH: Self = Tex::single(113);
    pub const DECOR_BUSH_COLOR: Self = Tex::single(118);
    pub const DECOR_WHEAT: Self = Tex::single(114);
    pub const DECOR_WHEAT_COLOR: Self = Self::NONE;
    pub const DECOR_WELL: Self = Tex::single(115);
    pub const DECOR_WELL_COLOR: Self = Self::NONE;

    // BUTTONS
    pub const BUTTON_TOWN: TexQuad = Tex::quad(119, 120, 126, 125);
    pub const BUTTON_TOWN_COLOR: TexQuad = Tex::quad(131, 132, 134, 133);

    pub const BUTTON_DOCK: TexQuad = Tex::quad(121, 122, 128, 127);
    pub const BUTTON_DOCK_COLOR: TexQuad = Tex::quad(135, 136, 138, 137);

    pub const BUTTON_LAND: TexQuad = Tex::quad(123, 124, 130, 129);
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

impl Tex {
    pub fn game_tile(color: Option<Color32>, highlight: Option<Color32>) -> Vec<TexQuad> {
        let mut tex = vec![];

        if let Some(color) = color {
            tex.push(Self::GAME_TILE.tint(color));
        }

        if let Some(highlight) = highlight {
            tex.push(Self::HIGHLIGHT.tint(highlight))
        }
        tex
    }

    pub fn board_game_tile(
        color: Option<Color32>,
        highlight: Option<Color32>,
        seed: usize,
    ) -> Vec<TexQuad> {
        let mut texs = Tex::game_tile(color, highlight);
        texs.push([
            Self::NONE,
            Self::NONE,
            match quickrand(seed) {
                0..=25 => Self::TILE_SE_GRASS1,
                26..=50 => Self::TILE_SE_GRASS2,
                51..=75 => Self::TILE_SE_GRASS3,
                _ => Self::TILE_SE_GRASS4,
            },
            match quickrand(seed + 678) {
                0..=25 => Self::TILE_SW_GRASS1,
                26..=50 => Self::TILE_SW_GRASS2,
                51..=75 => Self::TILE_SW_GRASS3,
                _ => Self::TILE_SW_GRASS4,
            },
        ]);
        texs
    }

    pub fn town_button(color: Option<Color32>, highlight: Option<Color32>) -> Vec<TexQuad> {
        let mut t = vec![
            Self::BUTTON_TOWN,
            if let Some(color) = color {
                Self::BUTTON_TOWN_COLOR.tint(color)
            } else {
                Self::BUTTON_TOWN_COLOR
            },
        ];
        if let Some(highlight) = highlight {
            t.push(Self::HIGHLIGHT.tint(highlight));
        };
        t
    }

    pub fn dock_button(color: Option<Color32>, highlight: Option<Color32>) -> Vec<TexQuad> {
        let mut t = vec![
            Self::BUTTON_DOCK,
            if let Some(color) = color {
                Self::BUTTON_DOCK_COLOR.tint(color)
            } else {
                Self::BUTTON_DOCK_COLOR
            },
        ];
        if let Some(highlight) = highlight {
            t.push(Self::HIGHLIGHT.tint(highlight));
        };
        t
    }

    pub fn land_button(highlight: Option<Color32>) -> Vec<TexQuad> {
        if let Some(highlight) = highlight {
            vec![Self::BUTTON_LAND, Self::HIGHLIGHT.tint(highlight)]
        } else {
            vec![Self::BUTTON_LAND]
        }
    }

    pub fn text_button(ratio: f32) -> Vec<Tex> {
        let extra_tiles = ratio as usize;
        [
            vec![Self::GAME_TILE_NW],
            vec![Self::GAME_TILE_N; extra_tiles],
            vec![Self::GAME_TILE_NE, Self::GAME_TILE_SE],
            vec![Self::GAME_TILE_S; extra_tiles],
            vec![Self::GAME_TILE_SW],
        ]
        .concat()
    }

    fn dock(color: Option<Color32>, neighbors: Vec<BGTexType>) -> Vec<TexQuad> {
        // TODO: Render docks with multiple edges somehow.
        let mut dock = if matches!(neighbors[1], BGTexType::Land) {
            vec![Self::DOCK_SOUTH, Self::DOCK_SOUTH_SAIL]
        } else if matches!(neighbors[5], BGTexType::Land) {
            vec![Self::DOCK_NORTH, Self::DOCK_NORTH_SAIL]
        } else if matches!(neighbors[3], BGTexType::Land) {
            vec![Self::DOCK_WEST, Self::DOCK_WEST_SAIL]
        } else if matches!(neighbors[7], BGTexType::Land) {
            vec![Self::DOCK_EAST, Self::DOCK_EAST_SAIL]
        } else {
            vec![Self::DOCK_DISCONNECTED, Self::DOCK_DISCONNECTED_SAIL]
        };
        if let Some(color) = color {
            dock[1] = dock[1].tint(color);
        }
        dock
    }

    fn town(color: Option<Color32>, seed: usize) -> Vec<TexQuad> {
        let rand_house = |n: usize| match quickrand(n) {
            0..=25 => (Self::HOUSE1, Self::ROOF1),
            26..=50 => (Self::HOUSE3, Self::ROOF3),
            51..=75 => (Self::HOUSE2, Self::ROOF2),
            _ => (Self::HOUSE4, Self::ROOF4),
        };

        let rand_house_colored = |n: usize| {
            let mut h = rand_house(n);
            if let Some(color) = &color {
                h.1 = h.1.tint(*color);
            }
            h
        };

        let rand_decor = |n: usize| match quickrand(n) {
            0..=20 => (Self::DECOR_PLANTER1, Self::DECOR_PLANTER1_COLOR),
            21..=40 => (Self::DECOR_PLANTER2, Self::DECOR_PLANTER2_COLOR),
            41..=60 => (Self::DECOR_BUSH, Self::DECOR_BUSH_COLOR),
            61..=80 => (Self::DECOR_WHEAT, Self::DECOR_WHEAT_COLOR),
            _ => (Self::DECOR_WELL, Self::DECOR_WELL_COLOR),
        };

        let rand_decor_colored = |n: usize| {
            let mut d = rand_decor(n);
            if let Some(color) = &color {
                d.1 = d.1.tint(*color);
            }
            d
        };

        let rand_path = |n: usize| match quickrand(n) {
            0..=20 => Self::PATH1,
            21..=30 => Self::PATH3,
            31..=40 => Self::PATH4,
            41..=50 => Self::PATH5,
            51..=60 => Self::PATH6,
            61..=70 => Self::PATH7,
            71..=80 => Self::PATH8,
            81..=90 => Self::PATH9,
            _ => Self::PATH2,
        };

        let numhouses = match quickrand(seed + 345) {
            0..=50 => 1,
            _ => 2,
        };

        let numdecor = match quickrand(seed + 23465) {
            0..=70 => 0,
            _ => 1,
        };

        let mut texs = vec![
            [
                rand_path(seed + 4),
                rand_path(seed + 44),
                rand_path(seed + 444),
                rand_path(seed + 4444),
            ],
            [Self::NONE, Self::NONE, Self::NONE, Self::NONE],
        ];

        for d in 0..numdecor {
            let decorpos = quickrand(seed + 454 + d + d) % 4;
            let (decor, layer) = rand_decor_colored(seed + 646 * d);

            texs[0][decorpos] = decor;
            texs[1][decorpos] = layer;
        }

        // These may bowl each other over but that's fine,
        // it just skews the average house number down slightly.
        for h in 0..numhouses {
            let housepos = quickrand(seed + 45 * h) % 4;
            let (house, roof) = rand_house_colored(seed + 6 * h);

            texs[0][housepos] = house;
            texs[1][housepos] = roof;
        }

        texs
    }

    /// Determine the tiles to use based on a given square and its neighbors,
    /// provided clockwise from northwest.
    pub fn terrain(
        base_type: BGTexType,
        layer_type: Option<FGTexType>,
        neighbors: Vec<BGTexType>,
        color: Option<Color32>,
        seed: usize,
    ) -> Vec<TexQuad> {
        debug_assert_eq!(neighbors.len(), 8);
        if neighbors.len() != 8 {
            return vec![[Self::DEBUG, Self::DEBUG, Self::DEBUG, Self::DEBUG]];
        }

        let rand_grass = |n: usize| match quickrand(n) {
            0..=70 => Self::GRASS1,
            71..=85 => Self::GRASS2,
            86..=94 => Self::GRASS3,
            _ => Self::GRASS4,
        };

        use BGTexType::*;
        let top_left = match base_type {
            Land => rand_grass(seed),
            Water => match (neighbors[7], neighbors[0], neighbors[1]) {
                (Land, Land | Water, Land) => Self::LAND_N_W,
                (Land, Land | Water, Water) => Self::LAND_W,
                (Water, Land | Water, Land) => Self::LAND_N,
                (Water, Land, Water) => Self::LAND_NW,
                (Water, Water, Water) => Self::WATER,
            },
        };

        let top_right = match base_type {
            Land => rand_grass(seed + 1),
            Water => match (neighbors[1], neighbors[2], neighbors[3]) {
                (Land, Land | Water, Land) => Self::LAND_N_E,
                (Land, Land | Water, Water) => Self::LAND_N,
                (Water, Land | Water, Land) => Self::LAND_E,
                (Water, Land, Water) => Self::LAND_NE,
                (Water, Water, Water) => Self::WATER,
            },
        };

        let bottom_right = match base_type {
            Land => rand_grass(seed + 2),
            Water => match (neighbors[3], neighbors[4], neighbors[5]) {
                (Land, Land | Water, Land) => Self::LAND_S_E,
                (Land, Land | Water, Water) => Self::LAND_E,
                (Water, Land | Water, Land) => Self::LAND_S,
                (Water, Land, Water) => Self::LAND_SE,
                (Water, Water, Water) => Self::WATER,
            },
        };

        let bottom_left = match base_type {
            Land => rand_grass(seed + 3),
            Water => match (neighbors[5], neighbors[6], neighbors[7]) {
                (Land, Land | Water, Land) => Self::LAND_S_W,
                (Land, Land | Water, Water) => Self::LAND_S,
                (Water, Land | Water, Land) => Self::LAND_W,
                (Water, Land, Water) => Self::LAND_SW,
                (Water, Water, Water) => Self::WATER,
            },
        };

        let mut texs = vec![[top_left, top_right, bottom_right, bottom_left]];

        if let Some(layer) = layer_type {
            match layer {
                FGTexType::Town => texs.extend(Tex::town(color, seed)),
                FGTexType::Dock => texs.extend(Tex::dock(color, neighbors)),
            }
        }

        texs
    }

    pub fn landscaping(from: &Square, action: &BoardEditingMode) -> Option<TexQuad> {
        match (action, from) {
            (
                BoardEditingMode::Land | BoardEditingMode::Town(_),
                Square::Water | Square::Dock(_),
            ) => Some(Self::ISLAND),
            (
                BoardEditingMode::Land | BoardEditingMode::Dock(_),
                Square::Land | Square::Town(_),
            ) => Some(Self::LAKE),
            _ => None,
        }
    }
}

impl Tex {
    pub fn render(self, map_texture: TextureId, rect: Rect, ui: &mut egui::Ui) {
        // TODO: Move these calcs into a lazy static since they can't be const :c
        // (or codegen them)

        let num_tiles = Tex::MAX_TILE + 1;
        // Outer tiles ate 18px wide with padding
        let uv_outer_tile_size = 1.0 / (num_tiles) as f32;
        // Padding is 1px per side out of the 18px
        let uv_tile_padding_size = uv_outer_tile_size / 18.0;

        // Texture has 1px padding above and below tiles that we can uniformly trim out
        let tile_y_uv = (0.0625, 0.9375);

        let mut mesh = Mesh::with_texture(map_texture);
        let uv = Rect::from_min_max(
            pos2(
                // Index to our tile, and skip over the leading column padding
                uv_outer_tile_size * ((self.tile) as f32) + uv_tile_padding_size,
                tile_y_uv.0,
            ),
            pos2(
                // Index to our next tile, and skip behind our trailing column padding
                uv_outer_tile_size * ((self.tile + 1) as f32) - uv_tile_padding_size,
                tile_y_uv.1,
            ),
        );

        mesh.add_rect_with_uv(rect, uv, self.tint.unwrap_or(Color32::WHITE));
        ui.painter().add(Shape::mesh(mesh));
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

fn quickrand(mut n: usize) -> usize {
    n ^= n << 13;
    n ^= n >> 7;
    n ^= n << 17;
    n % 100
}
