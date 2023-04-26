use eframe::egui;
use epaint::{vec2, Rect, TextureHandle};
use truncate_core::board::{Board, Coordinate};

use super::tex::{BGTexType, Tex, TexQuad};

#[derive(Clone)]
pub struct MappedBoard {
    resolved_tex: Vec<Vec<TexQuad>>,
    map_texture: TextureHandle,
    map_seed: usize,
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
        let ts = rect.width() * 0.25;
        let tile_rect = rect.shrink(ts);

        for (tex, translate) in self
            .get(coord)
            .into_iter()
            .zip([vec2(-ts, -ts), vec2(ts, -ts), vec2(ts, ts), vec2(-ts, ts)].into_iter())
        {
            tex.render(self.map_texture.id(), tile_rect.translate(translate), ui);
        }
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

                        let neighbor_squares: Vec<_> = coord
                            .neighbors_8()
                            .into_iter()
                            .map(|pos| board.get(pos).ok())
                            .collect();

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

                        Tex::resolve_bg_tile(
                            tile_base_type,
                            neighbor_base_types,
                            board.width() * coord.x * self.map_seed + board.height() + coord.y,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
    }

    pub fn map(board: &Board, map_texture: TextureHandle) -> Self {
        let secs = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("We are living in the future")
            .as_secs();

        let mut mapper = Self {
            resolved_tex: Vec::with_capacity(board.squares.len()),
            map_texture,
            map_seed: (secs % 100000) as usize,
        };

        mapper.remap(board);

        mapper
    }
}
