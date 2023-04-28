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

    pub const MAX_TILE: usize = 94;

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

    pub const DOCK_EAST_NW: Self = Tex::index(87);
    pub const DOCK_EAST_NE: Self = Tex::index(88);
    pub const DOCK_EAST_SE: Self = Tex::index(90);
    pub const DOCK_EAST_SW: Self = Tex::index(89);

    pub const DOCK_SOUTH_NW: Self = Tex::index(83);
    pub const DOCK_SOUTH_NE: Self = Tex::index(85);
    pub const DOCK_SOUTH_SE: Self = Tex::index(86);
    pub const DOCK_SOUTH_SW: Self = Tex::index(84);

    pub const DOCK_WEST_NW: Self = Tex::index(91);
    pub const DOCK_WEST_NE: Self = Tex::index(92);
    pub const DOCK_WEST_SE: Self = Tex::index(94);
    pub const DOCK_WEST_SW: Self = Tex::index(93);
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
        let rand = |mut n: usize| {
            n ^= n << 13;
            n ^= n >> 7;
            n ^= n << 17;
            n % 100
        };

        let mut texs = Tex::resolve_tile_tex(color, highlight);
        texs.push([
            Self::NONE,
            Self::NONE,
            match rand(seed) {
                0..=25 => Self::TILE_SE_GRASS1,
                26..=50 => Self::TILE_SE_GRASS2,
                51..=75 => Self::TILE_SE_GRASS3,
                _ => Self::TILE_SE_GRASS4,
            },
            match rand(seed + 678) {
                0..=25 => Self::TILE_SW_GRASS1,
                26..=50 => Self::TILE_SW_GRASS2,
                51..=75 => Self::TILE_SW_GRASS3,
                _ => Self::TILE_SW_GRASS4,
            },
        ]);
        texs
    }

    fn resolve_dock_tex(neighbors: Vec<BGTexType>) -> TexQuad {
        // TODO: Render docks with multiple edges somehow.
        if matches!(neighbors[1], BGTexType::Land) {
            [
                Self::DOCK_SOUTH_NW,
                Self::DOCK_SOUTH_NE,
                Self::DOCK_SOUTH_SE,
                Self::DOCK_SOUTH_SW,
            ]
        } else if matches!(neighbors[5], BGTexType::Land) {
            [
                Self::DOCK_NORTH_NW,
                Self::DOCK_NORTH_NE,
                Self::DOCK_NORTH_SE,
                Self::DOCK_NORTH_SW,
            ]
        } else if matches!(neighbors[3], BGTexType::Land) {
            [
                Self::DOCK_WEST_NW,
                Self::DOCK_WEST_NE,
                Self::DOCK_WEST_SE,
                Self::DOCK_WEST_SW,
            ]
        } else if matches!(neighbors[7], BGTexType::Land) {
            [
                Self::DOCK_EAST_NW,
                Self::DOCK_EAST_NE,
                Self::DOCK_EAST_SE,
                Self::DOCK_EAST_SW,
            ]
        } else {
            // TODO: Render disconnected docks somehow
            [Self::DEBUG, Self::DEBUG, Self::DEBUG, Self::DEBUG]
        }
    }

    /// Determine the tiles to use based on a given square and its neighbors,
    /// provided clockwise from northwest.
    pub fn resolve_bg_tex(
        base_type: BGTexType,
        layer_type: Option<FGTexType>,
        neighbors: Vec<BGTexType>,
        seed: usize,
    ) -> Vec<TexQuad> {
        debug_assert_eq!(neighbors.len(), 8);
        if neighbors.len() != 8 {
            return vec![[Self::DEBUG, Self::DEBUG, Self::DEBUG, Self::DEBUG]];
        }

        let rand_grass = |mut n: usize| {
            n ^= n << 13;
            n ^= n >> 7;
            n ^= n << 17;

            match n % 100 {
                0..=70 => Self::GRASS1,
                71..=85 => Self::GRASS2,
                86..=94 => Self::GRASS3,
                _ => Self::GRASS4,
            }
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
                FGTexType::Town => {
                    texs.push([Self::HOUSE1, Self::HOUSE2, Self::HOUSE3, Self::HOUSE4])
                }
                FGTexType::Dock => texs.push(Tex::resolve_dock_tex(neighbors)),
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
