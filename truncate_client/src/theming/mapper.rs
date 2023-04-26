use eframe::egui;
use epaint::{vec2, Color32, Rect, TextureHandle};
use truncate_core::board::{Board, Coordinate};

use super::tex::{render_tex_quad, BGTexType, Tex, TexQuad};

#[derive(Clone)]
pub struct MappedBoard {
    resolved_tex: Vec<Vec<TexQuad>>,
    map_texture: TextureHandle,
    map_seed: usize,
    inverted: bool, // TODO: Handle any transpose
}

impl MappedBoard {
    pub fn get(&self, coord: Coordinate) -> TexQuad {
        match self
            .resolved_tex
            .get(coord.y)
            .and_then(|row| row.get(coord.x))
        {
            Some(texs) => *texs,
            None => [Tex::DEBUG; 4],
        }
    }

    pub fn render_coord(&self, coord: Coordinate, rect: Rect, ui: &mut egui::Ui) {
        render_tex_quad(self.get(coord), rect, &self.map_texture, ui);
    }

    pub fn remap(&mut self, board: &Board) {
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
                            .map(|sq| {
                                if sq.is_some() {
                                    BGTexType::Land
                                } else {
                                    BGTexType::Water
                                }
                            })
                            .collect();

                        let tile_base_type = if square.is_some() {
                            BGTexType::Land
                        } else {
                            BGTexType::Water
                        };

                        Tex::resolve_bg_tex(
                            tile_base_type,
                            neighbor_base_types,
                            board.width() * coord.x * self.map_seed + board.height() + coord.y,
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
        color: Color32,
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
        for tex in &self.resolved_tex {
            render_tex_quad(*tex, rect, &self.map_texture, ui);
        }
    }
}
