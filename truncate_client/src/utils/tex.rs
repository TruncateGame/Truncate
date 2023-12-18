use std::{
    collections::VecDeque,
    ops::{Range, RangeInclusive},
};

use eframe::egui::{self, Sense};
use epaint::{pos2, vec2, Color32, Mesh, Pos2, Rect, Shape, TextureHandle, TextureId, Vec2};
use truncate_core::board::Square;

use crate::{app_outer::TEXTURE_MEASUREMENT, regions::lobby::BoardEditingMode};

use super::mapper::{quickrand, MappedTileVariant};

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

const fn t(tile: usize) -> Tex {
    Tex { tile, tint: None }
}

const fn quad(nw_tile: usize, ne_tile: usize, se_tile: usize, sw_tile: usize) -> [Tex; 4] {
    [
        Tex {
            tile: nw_tile,
            tint: None,
        },
        Tex {
            tile: ne_tile,
            tint: None,
        },
        Tex {
            tile: se_tile,
            tint: None,
        },
        Tex {
            tile: sw_tile,
            tint: None,
        },
    ]
}

// TODO: Generate this impl with codegen from aseprite
impl Tex {
    pub const MAX_TILE: usize = 265;

    pub const NONE: Self = t(0);
    pub const DEBUG: Self = t(77);

    pub const GRASS1: Self = t(21);
    pub const GRASS2: Self = t(22);
    pub const GRASS3: Self = t(23);
    pub const GRASS4: Self = t(24);

    pub const GRASS1_WIND0: Self = t(157);
    pub const GRASS2_WIND0: Self = t(158);
    pub const GRASS3_WIND0: Self = t(159);
    pub const GRASS4_WIND0: Self = t(160);

    pub const GRASS1_WIND1: Self = t(149);
    pub const GRASS2_WIND1: Self = t(150);
    pub const GRASS3_WIND1: Self = t(151);
    pub const GRASS4_WIND1: Self = t(152);

    pub const GRASS1_WIND2: Self = t(153);
    pub const GRASS2_WIND2: Self = t(154);
    pub const GRASS3_WIND2: Self = t(155);
    pub const GRASS4_WIND2: Self = t(156);

    pub const WATER: Self = t(8);

    // Water with land on one edge
    pub const LAND_SE: Self = t(4);
    pub const LAND_S: Self = t(5);
    pub const LAND_SW: Self = t(6);
    pub const LAND_W: Self = t(11);
    pub const LAND_NW: Self = t(17);
    pub const LAND_N: Self = t(16);
    pub const LAND_NE: Self = t(15);
    pub const LAND_E: Self = t(10);

    // Water with land on two edges
    pub const LAND_N_E: Self = t(25);
    pub const LAND_S_E: Self = t(18);
    pub const LAND_S_W: Self = t(19);
    pub const LAND_N_W: Self = t(26);

    // Transparent island in water
    pub const ISLAND: TexQuad = quad(69, 70, 72, 71);
    // Transparent lake on land
    pub const LAKE: TexQuad = quad(73, 74, 76, 75);

    // Tiles
    pub const GAME_TILE: TexQuad = quad(53, 54, 55, 56);

    // Tile cracks
    pub const TILE_CRACK_1: TexQuad = quad(216, 217, 219, 218);
    pub const TILE_CRACK_2: TexQuad = quad(220, 221, 223, 222);
    pub const TILE_CRACK_3: TexQuad = quad(224, 225, 227, 226);
    pub const TILE_CRACK_4: TexQuad = quad(228, 229, 231, 230);
    pub const TILE_CRACK_5: TexQuad = quad(232, 233, 235, 234);

    // Tile Remnants
    pub const TILE_REMNANT_1: TexQuad = quad(236, 237, 238, 239);
    pub const TILE_REMNANT_2: TexQuad = quad(240, 241, 242, 243);
    pub const TILE_REMNANT_3: TexQuad = quad(244, 245, 246, 247);

    // Tiles for buttons
    pub const GAME_TILE_NW: Self = t(53);
    pub const GAME_TILE_SW: Self = t(56);
    pub const GAME_TILE_N: Self = t(147);
    pub const GAME_TILE_S: Self = t(148);
    pub const GAME_TILE_NE: Self = t(54);
    pub const GAME_TILE_SE: Self = t(55);

    // Highlight rings for tiles
    pub const HIGHLIGHT: TexQuad = quad(65, 66, 68, 67);

    // Grass cover over tiles
    pub const TILE_SE_GRASS1: Self = t(58);
    pub const TILE_SE_GRASS2: Self = t(60);
    pub const TILE_SE_GRASS3: Self = t(62);
    pub const TILE_SE_GRASS4: Self = t(64);

    pub const TILE_SW_GRASS1: Self = t(57);
    pub const TILE_SW_GRASS2: Self = t(59);
    pub const TILE_SW_GRASS3: Self = t(61);
    pub const TILE_SW_GRASS4: Self = t(63);

    // HOUSES
    pub const HOUSE1: Self = t(36);
    pub const HOUSE2: Self = t(37);
    pub const HOUSE3: Self = t(38);
    pub const HOUSE4: Self = t(39);

    // ROOFS
    pub const ROOF1: Self = t(32);
    pub const ROOF2: Self = t(33);
    pub const ROOF3: Self = t(34);
    pub const ROOF4: Self = t(35);

    // SMOKES
    pub const ROOF1_SMOKE: [Self; 5] = [t(185), t(186), t(187), t(188), t(189)];
    pub const ROOF1_SMOKE_WIND0: [Self; 5] = [t(190), t(191), t(192), t(193), t(194)];
    pub const ROOF1_SMOKE_WIND1: [Self; 5] = [t(195), t(196), t(197), t(198), t(199)];

    pub const ROOF2_SMOKE: [Self; 5] = [t(200), t(201), t(202), t(203), t(204)];
    pub const ROOF2_SMOKE_WIND0: [Self; 5] = [t(205), t(206), t(207), t(208), t(209)];
    pub const ROOF2_SMOKE_WIND1: [Self; 5] = [t(210), t(211), t(212), t(213), t(214)];

    // DOCKS
    pub const DOCK_NORTH: TexQuad = quad(79, 81, 82, 80);
    pub const DOCK_NORTH_SAIL: TexQuad = quad(95, 96, 0, 0);
    pub const DOCK_NORTH_SAIL_WIND0: TexQuad = quad(161, 162, 0, 0);
    pub const DOCK_NORTH_SAIL_WIND1: TexQuad = quad(163, 164, 0, 0);

    pub const DOCK_EAST: TexQuad = quad(87, 88, 90, 89);
    pub const DOCK_EAST_SAIL: TexQuad = quad(99, 100, 108, 107);
    pub const DOCK_EAST_SAIL_WIND0: TexQuad = quad(168, 169, 182, 170);
    pub const DOCK_EAST_SAIL_WIND1: TexQuad = quad(171, 172, 173, 183);

    pub const DOCK_SOUTH: TexQuad = quad(83, 85, 86, 84);
    pub const DOCK_SOUTH_SAIL: TexQuad = quad(97, 0, 0, 105);
    pub const DOCK_SOUTH_SAIL_WIND0: TexQuad = quad(165, 0, 0, 184);
    pub const DOCK_SOUTH_SAIL_WIND1: TexQuad = quad(166, 0, 0, 167);

    pub const DOCK_WEST: TexQuad = quad(91, 92, 94, 93);
    pub const DOCK_WEST_SAIL: TexQuad = quad(101, 102, 0, 0);
    pub const DOCK_WEST_SAIL_WIND0: TexQuad = quad(174, 175, 0, 0);
    pub const DOCK_WEST_SAIL_WIND1: TexQuad = quad(176, 177, 0, 0);

    pub const DOCK_DISCONNECTED: TexQuad = quad(139, 140, 142, 141);
    pub const DOCK_DISCONNECTED_SAIL: TexQuad = quad(143, 144, 146, 145);
    pub const DOCK_DISCONNECTED_SAIL_WIND0: TexQuad = quad(178, 179, 0, 0);
    pub const DOCK_DISCONNECTED_SAIL_WIND1: TexQuad = quad(180, 181, 0, 0);

    // PATHS
    pub const PATH1: Self = t(40);
    pub const PATH2: Self = t(41);
    pub const PATH3: Self = t(42);
    pub const PATH4: Self = t(43);
    pub const PATH5: Self = t(44);
    pub const PATH6: Self = t(45);
    pub const PATH7: Self = t(46);
    pub const PATH8: Self = t(47);
    pub const PATH9: Self = t(48);

    // DECOR
    pub const DECOR_PLANTER1: Self = t(111);
    pub const DECOR_PLANTER1_COLOR: Self = t(116);
    pub const DECOR_PLANTER2: Self = t(112);
    pub const DECOR_PLANTER2_COLOR: Self = t(117);
    pub const DECOR_BUSH: Self = t(113);
    pub const DECOR_BUSH_COLOR: Self = t(118);
    pub const DECOR_WHEAT: Self = t(114);
    pub const DECOR_WHEAT_WIND0: Self = t(215);
    pub const DECOR_WHEAT_COLOR: Self = Self::NONE;
    pub const DECOR_WELL: Self = t(115);
    pub const DECOR_WELL_COLOR: Self = Self::NONE;

    // BUTTONS
    pub const BUTTON_TOWN: TexQuad = quad(119, 120, 126, 125);
    pub const BUTTON_TOWN_COLOR: TexQuad = quad(131, 132, 134, 133);

    pub const BUTTON_DOCK: TexQuad = quad(121, 122, 128, 127);
    pub const BUTTON_DOCK_COLOR: TexQuad = quad(135, 136, 138, 137);

    pub const BUTTON_LAND: TexQuad = quad(123, 124, 130, 129);

    pub const BUTTON_INFO: TexQuad = quad(257, 258, 260, 259);
    pub const BUTTON_CLOSE: TexQuad = quad(261, 262, 264, 263);
    pub const BUTTON_NOTIF: TexQuad = quad(0, 0, 265, 0);

    // Dialog
    pub const DIALOG_NW: Self = t(248);
    pub const DIALOG_N: Self = t(249);
    pub const DIALOG_NE: Self = t(250);
    pub const DIALOG_E: Self = t(253);
    pub const DIALOG_SE: Self = t(256);
    pub const DIALOG_S: Self = t(255);
    pub const DIALOG_SW: Self = t(254);
    pub const DIALOG_W: Self = t(251);
    pub const DIALOG_CENTER: Self = t(252);
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
            tex.push(Self::GAME_TILE.tint(color));
        }

        if let Some(highlight) = highlight {
            tex.push(Self::HIGHLIGHT.tint(highlight))
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
            Self::NONE,
            Self::NONE,
            match quickrand(seed) % 100 {
                0..=25 => Self::TILE_SE_GRASS1,
                26..=50 => Self::TILE_SE_GRASS2,
                51..=75 => Self::TILE_SE_GRASS3,
                _ => Self::TILE_SE_GRASS4,
            },
            match quickrand(seed + 678) % 100 {
                0..=25 => Self::TILE_SW_GRASS1,
                26..=50 => Self::TILE_SW_GRASS2,
                51..=75 => Self::TILE_SW_GRASS3,
                _ => Self::TILE_SW_GRASS4,
            },
        ]);
        match variant {
            MappedTileVariant::Healthy => {}
            MappedTileVariant::Dying => {
                texs.push(
                    match quickrand(seed) % 100 {
                        0..=19 => Self::TILE_CRACK_1,
                        20..=39 => Self::TILE_CRACK_2,
                        40..=59 => Self::TILE_CRACK_3,
                        60..=79 => Self::TILE_CRACK_4,
                        _ => Self::TILE_CRACK_5,
                    }
                    .tint(color.unwrap_or_default()),
                );
            }
            MappedTileVariant::Dead => {
                for i in 0..2 {
                    texs.push(
                        match quickrand(seed + i) % 100 {
                            0..=19 => Self::TILE_CRACK_1,
                            20..=39 => Self::TILE_CRACK_2,
                            40..=59 => Self::TILE_CRACK_3,
                            60..=79 => Self::TILE_CRACK_4,
                            _ => Self::TILE_CRACK_5,
                        }
                        .tint(color.unwrap_or_default()),
                    );
                }
            }
            MappedTileVariant::Gone => {
                texs = vec![[
                    match quickrand(seed + 345) % 100 {
                        0..=33 => Self::TILE_REMNANT_1[0],
                        34..=66 => Self::TILE_REMNANT_2[0],
                        _ => Self::TILE_REMNANT_3[0],
                    },
                    match quickrand(seed + 757) % 100 {
                        0..=33 => Self::TILE_REMNANT_1[1],
                        34..=66 => Self::TILE_REMNANT_2[1],
                        _ => Self::TILE_REMNANT_3[1],
                    },
                    match quickrand(seed + 8447) % 100 {
                        0..=33 => Self::TILE_REMNANT_1[2],
                        34..=66 => Self::TILE_REMNANT_2[2],
                        _ => Self::TILE_REMNANT_3[2],
                    },
                    match quickrand(seed + 477387) % 100 {
                        0..=33 => Self::TILE_REMNANT_1[3],
                        34..=66 => Self::TILE_REMNANT_2[3],
                        _ => Self::TILE_REMNANT_3[3],
                    },
                ]];
            }
        }
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

    pub fn text_dialog(x_tiles: usize, y_tiles: usize) -> Vec<Vec<Tex>> {
        let middle_x_tiles = x_tiles.saturating_sub(2);
        let middle_y_tiles = y_tiles.saturating_sub(2);

        [
            vec![[
                vec![Self::DIALOG_NW],
                vec![Self::DIALOG_N; middle_x_tiles],
                vec![Self::DIALOG_NE],
            ]
            .concat()],
            vec![
                [
                    vec![Self::DIALOG_W],
                    vec![Self::DIALOG_CENTER; middle_x_tiles],
                    vec![Self::DIALOG_E]
                ]
                .concat();
                middle_y_tiles
            ],
            vec![[
                vec![Self::DIALOG_SW],
                vec![Self::DIALOG_S; middle_x_tiles],
                vec![Self::DIALOG_SE],
            ]
            .concat()],
        ]
        .concat()
    }

    fn dock(color: Option<Color32>, neighbors: Vec<BGTexType>, wind_at_coord: u8) -> Vec<TexQuad> {
        // TODO: Render docks with multiple edges somehow.

        let mut dock = if matches!(neighbors[1], BGTexType::Land) {
            vec![
                Self::DOCK_SOUTH,
                match wind_at_coord {
                    calm!() => Self::DOCK_SOUTH_SAIL,
                    breeze!() => Self::DOCK_SOUTH_SAIL_WIND0,
                    _ => Self::DOCK_SOUTH_SAIL_WIND1,
                },
            ]
        } else if matches!(neighbors[5], BGTexType::Land) {
            vec![
                Self::DOCK_NORTH,
                match wind_at_coord {
                    calm!() => Self::DOCK_NORTH_SAIL,
                    breeze!() => Self::DOCK_NORTH_SAIL_WIND0,
                    _ => Self::DOCK_NORTH_SAIL_WIND1,
                },
            ]
        } else if matches!(neighbors[3], BGTexType::Land) {
            vec![
                Self::DOCK_WEST,
                match wind_at_coord {
                    calm!() => Self::DOCK_WEST_SAIL,
                    breeze!() => Self::DOCK_WEST_SAIL_WIND0,
                    _ => Self::DOCK_WEST_SAIL_WIND1,
                },
            ]
        } else if matches!(neighbors[7], BGTexType::Land) {
            vec![
                Self::DOCK_EAST,
                match wind_at_coord {
                    calm!() => Self::DOCK_EAST_SAIL,
                    breeze!() => Self::DOCK_EAST_SAIL_WIND0,
                    _ => Self::DOCK_EAST_SAIL_WIND1,
                },
            ]
        } else {
            vec![Self::DOCK_DISCONNECTED, Self::DOCK_DISCONNECTED_SAIL]
        };
        if let Some(color) = color {
            dock[1] = dock[1].tint(color);
        }
        dock
    }

    fn town(color: Option<Color32>, seed: usize, tick: u64, wind_at_coord: u8) -> Vec<TexQuad> {
        let anim_index = (quickrand(seed + 3) + tick as usize) % 30;
        let rand_house = |n: usize| match quickrand(n) {
            0..=25 => (
                Self::HOUSE1,
                Self::ROOF1,
                match (anim_index, wind_at_coord) {
                    (5.., _) => Self::NONE,
                    (_, calm!()) => Self::ROOF1_SMOKE[anim_index],
                    (_, breeze!()) => Self::ROOF1_SMOKE_WIND0[anim_index],
                    _ => Self::ROOF1_SMOKE_WIND1[anim_index],
                },
            ),
            26..=50 => (Self::HOUSE3, Self::ROOF3, Self::NONE),
            51..=75 => (
                Self::HOUSE2,
                Self::ROOF2,
                match (anim_index, wind_at_coord) {
                    (5.., _) => Self::NONE,
                    (_, calm!()) => Self::ROOF2_SMOKE[anim_index],
                    (_, breeze!()) => Self::ROOF2_SMOKE_WIND0[anim_index],
                    _ => Self::ROOF2_SMOKE_WIND1[anim_index],
                },
            ),
            _ => (Self::HOUSE4, Self::ROOF4, Self::NONE),
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
            61..=80 => (
                match wind_at_coord {
                    calm!() | breeze!() => Self::DECOR_WHEAT,
                    _ => Self::DECOR_WHEAT_WIND0,
                },
                Self::DECOR_WHEAT_COLOR,
            ),
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
            let (house, roof, smoke) = rand_house_colored(seed + 6 * h);

            texs[0][housepos] = house;
            texs[1][housepos] = roof;
            texs[2][housepos] = smoke;
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
        tick: u64,
        wind_at_coord: u8,
    ) -> Vec<TexQuad> {
        debug_assert_eq!(neighbors.len(), 8);
        if neighbors.len() != 8 {
            return vec![[Self::DEBUG, Self::DEBUG, Self::DEBUG, Self::DEBUG]];
        }

        let grasses = match wind_at_coord {
            calm!() => [Self::GRASS1, Self::GRASS2, Self::GRASS3, Self::GRASS4],
            breeze!() => [
                Self::GRASS1_WIND0,
                Self::GRASS2_WIND0,
                Self::GRASS3_WIND0,
                Self::GRASS4_WIND0,
            ],
            wind!() => [
                Self::GRASS1_WIND1,
                Self::GRASS2_WIND1,
                Self::GRASS3_WIND1,
                Self::GRASS4_WIND1,
            ],
            _ => [
                Self::GRASS1_WIND2,
                Self::GRASS2_WIND2,
                Self::GRASS3_WIND2,
                Self::GRASS4_WIND2,
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
                FGTexType::Town => texs.extend(Tex::town(color, seed, tick, wind_at_coord)),
                FGTexType::Dock => texs.extend(Tex::dock(color, neighbors, wind_at_coord)),
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
                Square::Land | Square::Town { .. },
            ) => Some(Self::LAKE),
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
