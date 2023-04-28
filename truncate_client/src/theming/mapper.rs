use eframe::egui;
use epaint::{Color32, Rect, TextureHandle};
use truncate_core::board::{Board, Coordinate, Square};

use crate::theming::tex::FGTexType;

use super::tex::{render_tex_quads, BGTexType, Tex, TexQuad};

#[derive(Clone)]
pub struct MappedBoard {
    resolved_tex: Vec<Vec<Vec<TexQuad>>>,
    map_texture: TextureHandle,
    map_seed: usize,
    inverted: bool, // TODO: Handle any transpose
}

impl MappedBoard {
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

    pub fn remap(&mut self, board: &Board) {
        fn base_type(sq: &Square) -> BGTexType {
            match sq {
                truncate_core::board::Square::Water => BGTexType::Water,
                truncate_core::board::Square::Land => BGTexType::Land,
                truncate_core::board::Square::Town(_) => BGTexType::Land,
                truncate_core::board::Square::Dock(_) => BGTexType::Water,
                truncate_core::board::Square::Occupied(_, _) => BGTexType::Land,
            }
        }

        fn layer_type(sq: &Square) -> Option<(FGTexType, Color32)> {
            match sq {
                Square::Water => None,
                Square::Land => None,
                Square::Town(owner) => Some((
                    FGTexType::Town,
                    if *owner == 0 {
                        Color32::DARK_RED
                    } else {
                        Color32::LIGHT_BLUE
                    },
                )),
                Square::Dock(owner) => Some((
                    FGTexType::Dock,
                    if *owner == 0 {
                        Color32::DARK_RED
                    } else {
                        Color32::LIGHT_BLUE
                    },
                )),
                Square::Occupied(_, _) => None,
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
                        let tile_layer_type = layer_type(square);

                        Tex::resolve_bg_tex(
                            tile_base_type,
                            tile_layer_type.map(|l| l.0),
                            neighbor_base_types,
                            tile_layer_type.map(|l| l.1),
                            self.map_seed + (coord.x * coord.y + coord.y),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
    }

    pub fn new(board: &Board, map_texture: TextureHandle, invert: bool) -> Self {
        let secs = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("We are living in the future")
            .as_secs();

        let mut mapper = Self {
            resolved_tex: Vec::with_capacity(board.squares.len()),
            map_texture,
            map_seed: (secs % 100000) as usize,
            inverted: invert,
        };

        mapper.remap(board);

        mapper
    }
}

#[derive(Clone)]
pub struct MappedTile {
    resolved_tex: Vec<TexQuad>,
    map_texture: TextureHandle,
}

impl MappedTile {
    pub fn new(
        color: Option<Color32>,
        highlight: Option<Color32>,
        coord: Option<Coordinate>,
        map_texture: TextureHandle,
    ) -> Self {
        let resolved_tex = if let Some(coord) = coord {
            Tex::resolve_board_tile_tex(color, highlight, coord.x * 99 + coord.y)
        } else {
            Tex::resolve_tile_tex(color, highlight)
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
