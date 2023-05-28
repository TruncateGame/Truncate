use eframe::egui;
use epaint::{pos2, vec2, Color32, Mesh, Rect, Shape, TextureHandle, TextureId};

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
    const fn index(tile: usize) -> Self {
        Self { tile, tint: None }
    }

    pub const MAX_TILE: usize = 118;

    pub const NONE: Self = Tex::index(0);
    pub const DEBUG: Self = Tex::index(77);

    pub const GRASS1: Self = Tex::index(21);
    pub const GRASS2: Self = Tex::index(22);
    pub const GRASS3: Self = Tex::index(23);
    pub const GRASS4: Self = Tex::index(24);

    pub const WATER: Self = Tex::index(8);

    // Water with land on one edge
    pub const LAND_SE: Self = Tex::index(4);
    pub const LAND_S: Self = Tex::index(5);
    pub const LAND_SW: Self = Tex::index(6);
    pub const LAND_W: Self = Tex::index(11);
    pub const LAND_NW: Self = Tex::index(17);
    pub const LAND_N: Self = Tex::index(16);
    pub const LAND_NE: Self = Tex::index(15);
    pub const LAND_E: Self = Tex::index(10);

    // Water with land on two edges
    pub const LAND_N_E: Self = Tex::index(25);
    pub const LAND_S_E: Self = Tex::index(18);
    pub const LAND_S_W: Self = Tex::index(19);
    pub const LAND_N_W: Self = Tex::index(26);

    // Transparent island in water
    pub const ISLAND_NW: Self = Tex::index(69);
    pub const ISLAND_NE: Self = Tex::index(70);
    pub const ISLAND_SE: Self = Tex::index(72);
    pub const ISLAND_SW: Self = Tex::index(71);
    // Transparent cracks on land
    pub const CRACKS_NW: Self = Tex::index(73);
    pub const CRACKS_NE: Self = Tex::index(74);
    pub const CRACKS_SE: Self = Tex::index(76);
    pub const CRACKS_SW: Self = Tex::index(75);

    // Tiles
    pub const TILE_NW: Self = Tex::index(53);
    pub const TILE_NE: Self = Tex::index(54);
    pub const TILE_SE: Self = Tex::index(55);
    pub const TILE_SW: Self = Tex::index(56);

    // Highlight rings for tiles
    pub const HIGHLIGHT_NW: Self = Tex::index(65);
    pub const HIGHLIGHT_NE: Self = Tex::index(66);
    pub const HIGHLIGHT_SE: Self = Tex::index(68);
    pub const HIGHLIGHT_SW: Self = Tex::index(67);

    // Grass cover over tiles
    pub const TILE_SE_GRASS1: Self = Tex::index(58);
    pub const TILE_SE_GRASS2: Self = Tex::index(60);
    pub const TILE_SE_GRASS3: Self = Tex::index(62);
    pub const TILE_SE_GRASS4: Self = Tex::index(64);

    pub const TILE_SW_GRASS1: Self = Tex::index(57);
    pub const TILE_SW_GRASS2: Self = Tex::index(59);
    pub const TILE_SW_GRASS3: Self = Tex::index(61);
    pub const TILE_SW_GRASS4: Self = Tex::index(63);

    // HOUSES
    pub const HOUSE1: Self = Tex::index(36);
    pub const HOUSE2: Self = Tex::index(37);
    pub const HOUSE3: Self = Tex::index(38);
    pub const HOUSE4: Self = Tex::index(39);

    // ROOFS
    pub const ROOF1: Self = Tex::index(32);
    pub const ROOF2: Self = Tex::index(33);
    pub const ROOF3: Self = Tex::index(34);
    pub const ROOF4: Self = Tex::index(35);

    // DOCKS
    pub const DOCK_NORTH_NW: Self = Tex::index(79);
    pub const DOCK_NORTH_NE: Self = Tex::index(81);
    pub const DOCK_NORTH_SE: Self = Tex::index(82);
    pub const DOCK_NORTH_SW: Self = Tex::index(80);

    pub const DOCK_NORTH_NW_SAIL: Self = Tex::index(95);
    pub const DOCK_NORTH_NE_SAIL: Self = Tex::index(96);
    pub const DOCK_NORTH_SE_SAIL: Self = Self::NONE;
    pub const DOCK_NORTH_SW_SAIL: Self = Self::NONE;

    pub const DOCK_EAST_NW: Self = Tex::index(87);
    pub const DOCK_EAST_NE: Self = Tex::index(88);
    pub const DOCK_EAST_SE: Self = Tex::index(90);
    pub const DOCK_EAST_SW: Self = Tex::index(89);

    pub const DOCK_EAST_NW_SAIL: Self = Tex::index(99);
    pub const DOCK_EAST_NE_SAIL: Self = Tex::index(100);
    pub const DOCK_EAST_SE_SAIL: Self = Tex::index(108);
    pub const DOCK_EAST_SW_SAIL: Self = Tex::index(107);

    pub const DOCK_SOUTH_NW: Self = Tex::index(83);
    pub const DOCK_SOUTH_NE: Self = Tex::index(85);
    pub const DOCK_SOUTH_SE: Self = Tex::index(86);
    pub const DOCK_SOUTH_SW: Self = Tex::index(84);

    pub const DOCK_SOUTH_NW_SAIL: Self = Tex::index(97);
    pub const DOCK_SOUTH_NE_SAIL: Self = Self::NONE;
    pub const DOCK_SOUTH_SE_SAIL: Self = Self::NONE;
    pub const DOCK_SOUTH_SW_SAIL: Self = Tex::index(105);

    pub const DOCK_WEST_NW: Self = Tex::index(91);
    pub const DOCK_WEST_NE: Self = Tex::index(92);
    pub const DOCK_WEST_SE: Self = Tex::index(94);
    pub const DOCK_WEST_SW: Self = Tex::index(93);

    pub const DOCK_WEST_NW_SAIL: Self = Tex::index(101);
    pub const DOCK_WEST_NE_SAIL: Self = Tex::index(102);
    pub const DOCK_WEST_SE_SAIL: Self = Self::NONE;
    pub const DOCK_WEST_SW_SAIL: Self = Self::NONE;

    // PATHS
    pub const PATH1: Self = Tex::index(40);
    pub const PATH2: Self = Tex::index(41);
    pub const PATH3: Self = Tex::index(42);
    pub const PATH4: Self = Tex::index(43);
    pub const PATH5: Self = Tex::index(44);
    pub const PATH6: Self = Tex::index(45);
    pub const PATH7: Self = Tex::index(46);
    pub const PATH8: Self = Tex::index(47);
    pub const PATH9: Self = Tex::index(48);

    // DECOR
    pub const DECOR_PLANTER1: Self = Tex::index(111);
    pub const DECOR_PLANTER1_COLOR: Self = Tex::index(116);
    pub const DECOR_PLANTER2: Self = Tex::index(112);
    pub const DECOR_PLANTER2_COLOR: Self = Tex::index(117);
    pub const DECOR_BUSH: Self = Tex::index(113);
    pub const DECOR_BUSH_COLOR: Self = Tex::index(118);
    pub const DECOR_WHEAT: Self = Tex::index(114);
    pub const DECOR_WHEAT_COLOR: Self = Self::NONE;
    pub const DECOR_WELL: Self = Tex::index(115);
    pub const DECOR_WELL_COLOR: Self = Self::NONE;
}

impl Tex {
    fn tint(mut self, color: Color32) -> Self {
        self.tint = Some(color);
        self
    }
}

impl Tex {
    pub fn resolve_tile_tex(color: Option<Color32>, highlight: Option<Color32>) -> Vec<TexQuad> {
        let mut tex = vec![];

        if let Some(color) = color {
            tex.push([
                Self::TILE_NW.tint(color),
                Self::TILE_NE.tint(color),
                Self::TILE_SE.tint(color),
                Self::TILE_SW.tint(color),
            ]);
        }

        if let Some(highlight) = highlight {
            tex.push([
                Self::HIGHLIGHT_NW.tint(highlight),
                Self::HIGHLIGHT_NE.tint(highlight),
                Self::HIGHLIGHT_SE.tint(highlight),
                Self::HIGHLIGHT_SW.tint(highlight),
            ])
        }
        tex
    }

    pub fn resolve_board_tile_tex(
        color: Option<Color32>,
        highlight: Option<Color32>,
        seed: usize,
    ) -> Vec<TexQuad> {
        let mut texs = Tex::resolve_tile_tex(color, highlight);
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

    fn resolve_dock_tex(color: Option<Color32>, neighbors: Vec<BGTexType>) -> Vec<TexQuad> {
        // TODO: Render docks with multiple edges somehow.
        let mut dock = if matches!(neighbors[1], BGTexType::Land) {
            vec![
                [
                    Self::DOCK_SOUTH_NW,
                    Self::DOCK_SOUTH_NE,
                    Self::DOCK_SOUTH_SE,
                    Self::DOCK_SOUTH_SW,
                ],
                [
                    Self::DOCK_SOUTH_NW_SAIL,
                    Self::DOCK_SOUTH_NE_SAIL,
                    Self::DOCK_SOUTH_SE_SAIL,
                    Self::DOCK_SOUTH_SW_SAIL,
                ],
            ]
        } else if matches!(neighbors[5], BGTexType::Land) {
            vec![
                [
                    Self::DOCK_NORTH_NW,
                    Self::DOCK_NORTH_NE,
                    Self::DOCK_NORTH_SE,
                    Self::DOCK_NORTH_SW,
                ],
                [
                    Self::DOCK_NORTH_NW_SAIL,
                    Self::DOCK_NORTH_NE_SAIL,
                    Self::DOCK_NORTH_SE_SAIL,
                    Self::DOCK_NORTH_SW_SAIL,
                ],
            ]
        } else if matches!(neighbors[3], BGTexType::Land) {
            vec![
                [
                    Self::DOCK_WEST_NW,
                    Self::DOCK_WEST_NE,
                    Self::DOCK_WEST_SE,
                    Self::DOCK_WEST_SW,
                ],
                [
                    Self::DOCK_WEST_NW_SAIL,
                    Self::DOCK_WEST_NE_SAIL,
                    Self::DOCK_WEST_SE_SAIL,
                    Self::DOCK_WEST_SW_SAIL,
                ],
            ]
        } else if matches!(neighbors[7], BGTexType::Land) {
            vec![
                [
                    Self::DOCK_EAST_NW,
                    Self::DOCK_EAST_NE,
                    Self::DOCK_EAST_SE,
                    Self::DOCK_EAST_SW,
                ],
                [
                    Self::DOCK_EAST_NW_SAIL,
                    Self::DOCK_EAST_NE_SAIL,
                    Self::DOCK_EAST_SE_SAIL,
                    Self::DOCK_EAST_SW_SAIL,
                ],
            ]
        } else {
            // TODO: Render disconnected docks somehow
            vec![[Self::DEBUG, Self::DEBUG, Self::DEBUG, Self::DEBUG]; 2]
        };
        if let Some(color) = color {
            for p in 0..4 {
                dock[1][p] = dock[1][p].tint(color);
            }
        }
        dock
    }

    fn resolve_town_tex(color: Option<Color32>, seed: usize) -> Vec<TexQuad> {
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
    pub fn resolve_bg_tex(
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
                FGTexType::Town => texs.extend(Tex::resolve_town_tex(color, seed)),
                FGTexType::Dock => texs.extend(Tex::resolve_dock_tex(color, neighbors)),
            }
        }

        texs
    }

    pub fn resolve_landscaping_tex(adding: bool) -> TexQuad {
        if adding {
            [
                Self::ISLAND_NW,
                Self::ISLAND_NE,
                Self::ISLAND_SE,
                Self::ISLAND_SW,
            ]
        } else {
            [
                Self::CRACKS_NW,
                Self::CRACKS_NE,
                Self::CRACKS_SE,
                Self::CRACKS_SW,
            ]
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

fn quickrand(mut n: usize) -> usize {
    n ^= n << 13;
    n ^= n >> 7;
    n ^= n << 17;
    n % 100
}
