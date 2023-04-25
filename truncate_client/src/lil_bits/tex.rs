use eframe::egui;
use epaint::{pos2, Color32, Mesh, Rect, Shape, TextureId};

#[derive(Debug)]
pub struct Tex {
    tile: usize,
}

#[derive(Debug, Copy, Clone)]
pub enum BGTexType {
    Land,
    Water,
}

// TODO: Generate this impl with codegen from aseprite
impl Tex {
    const fn index(tile: usize) -> Self {
        Self { tile }
    }

    pub const TILE_SIZE: f32 = 1.0 / 52.0;

    // TODO: Make an actual debug tile
    pub const DEBUG: Self = Tex::index(50);

    pub const GRASS1: Self = Tex::index(21);
    pub const GRASS2: Self = Tex::index(22);
    pub const GRASS3: Self = Tex::index(23);
    pub const GRASS4: Self = Tex::index(24);

    pub const WATER: Self = Tex::index(8);

    // Land on one edge
    pub const LAND_SE: Self = Tex::index(4);
    pub const LAND_S: Self = Tex::index(5);
    pub const LAND_SW: Self = Tex::index(6);
    pub const LAND_W: Self = Tex::index(11);
    pub const LAND_NW: Self = Tex::index(17);
    pub const LAND_N: Self = Tex::index(16);
    pub const LAND_NE: Self = Tex::index(15);
    pub const LAND_E: Self = Tex::index(10);

    // Land on all edges
    pub const LAND_N_E: Self = Tex::index(25);
    pub const LAND_S_E: Self = Tex::index(18);
    pub const LAND_S_W: Self = Tex::index(19);
    pub const LAND_N_W: Self = Tex::index(26);
}

impl Tex {
    /// Determine the tiles to use based on a given square and its neighbors,
    /// provided clockwise from northwest.
    pub fn resolve_bg_tile(
        tex_type: BGTexType,
        neighbors: Vec<BGTexType>,
        seed: usize,
    ) -> [Self; 4] {
        debug_assert_eq!(neighbors.len(), 8);
        if neighbors.len() != 8 {
            return [Self::DEBUG, Self::DEBUG, Self::DEBUG, Self::DEBUG];
        }

        let rand_tile = |mut n: usize| {
            n ^= n << 13;
            n ^= n >> 17;
            n ^= n << 5;
            match n % 100 {
                0..=70 => Self::GRASS1,
                71..=85 => Self::GRASS2,
                86..=94 => Self::GRASS3,
                _ => Self::GRASS4,
            }
        };

        use BGTexType::*;
        let top_left = match tex_type {
            Land => rand_tile(seed),
            Water => match (neighbors[7], neighbors[0], neighbors[1]) {
                (Land, Land | Water, Land) => Self::LAND_N_W,
                (Land, Land | Water, Water) => Self::LAND_W,
                (Water, Land | Water, Land) => Self::LAND_N,
                (Water, Land, Water) => Self::LAND_NW,
                (Water, Water, Water) => Self::WATER,
            },
        };

        let top_right = match tex_type {
            Land => rand_tile(seed + 1),
            Water => match (neighbors[1], neighbors[2], neighbors[3]) {
                (Land, Land | Water, Land) => Self::LAND_N_E,
                (Land, Land | Water, Water) => Self::LAND_N,
                (Water, Land | Water, Land) => Self::LAND_E,
                (Water, Land, Water) => Self::LAND_NE,
                (Water, Water, Water) => Self::WATER,
            },
        };

        let bottom_right = match tex_type {
            Land => rand_tile(seed + 2),
            Water => match (neighbors[3], neighbors[4], neighbors[5]) {
                (Land, Land | Water, Land) => Self::LAND_S_E,
                (Land, Land | Water, Water) => Self::LAND_E,
                (Water, Land | Water, Land) => Self::LAND_S,
                (Water, Land, Water) => Self::LAND_SE,
                (Water, Water, Water) => Self::WATER,
            },
        };

        let bottom_left = match tex_type {
            Land => rand_tile(seed + 3),
            Water => match (neighbors[5], neighbors[6], neighbors[7]) {
                (Land, Land | Water, Land) => Self::LAND_S_W,
                (Land, Land | Water, Water) => Self::LAND_S,
                (Water, Land | Water, Land) => Self::LAND_W,
                (Water, Land, Water) => Self::LAND_SW,
                (Water, Water, Water) => Self::WATER,
            },
        };

        [top_left, top_right, bottom_right, bottom_left]
    }
}

impl Tex {
    pub fn render(self, map_texture: TextureId, rect: Rect, ui: &mut egui::Ui) {
        let mut mesh = Mesh::with_texture(map_texture);
        let uv = Rect::from_min_max(
            pos2(Tex::TILE_SIZE * ((self.tile - 1) as f32), 0.0),
            pos2(Tex::TILE_SIZE * ((self.tile) as f32), 1.0),
        );
        mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
        ui.painter().add(Shape::mesh(mesh));
    }
}
