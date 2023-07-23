use std::collections::VecDeque;

use eframe::egui;
use epaint::{Color32, Rect, TextureHandle};
use truncate_core::board::{Board, Coordinate, Square};

use crate::utils::tex::FGTexType;

use super::tex::{render_tex_quads, BGTexType, Tex, TexQuad};

#[derive(Clone)]
pub struct MappedBoard {
    resolved_tex: Vec<Vec<Vec<TexQuad>>>,
    map_texture: TextureHandle,
    map_seed: usize,
    inverted: bool, // TODO: Handle any transpose
    last_tick: u64,
    forecasted_wind: u8,
    incoming_wind: u8,
    winds: VecDeque<u8>,
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

    pub fn remap(&mut self, board: &Board, player_colors: &Vec<Color32>, tick: u64) {
        fn base_type(sq: &Square) -> BGTexType {
            match sq {
                truncate_core::board::Square::Water => BGTexType::Water,
                truncate_core::board::Square::Land => BGTexType::Land,
                truncate_core::board::Square::Town(_) => BGTexType::Land,
                truncate_core::board::Square::Dock(_) => BGTexType::Water,
                truncate_core::board::Square::Occupied(_, _) => BGTexType::Land,
            }
        }

        fn layer_type(sq: &Square, player_colors: &Vec<Color32>) -> Option<(FGTexType, Color32)> {
            match sq {
                Square::Water => None,
                Square::Land => None,
                Square::Town(owner) => Some((
                    FGTexType::Town,
                    *player_colors.get(*owner).unwrap_or(&Color32::WHITE),
                )),
                Square::Dock(owner) => Some((
                    FGTexType::Dock,
                    *player_colors.get(*owner).unwrap_or(&Color32::WHITE),
                )),
                Square::Occupied(_, _) => None,
            }
        }

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
                        let tile_layer_type = layer_type(square, &player_colors);

                        let wind_at_coord =
                            self.winds.get(colnum + rownum).cloned().unwrap_or_default();

                        Tex::terrain(
                            tile_base_type,
                            tile_layer_type.map(|l| l.0),
                            neighbor_base_types,
                            tile_layer_type.map(|l| l.1),
                            self.map_seed + (coord.x * coord.y + coord.y),
                            tick,
                            wind_at_coord,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
    }

    pub fn new(
        board: &Board,
        map_texture: TextureHandle,
        invert: bool,
        player_colors: &Vec<Color32>,
    ) -> Self {
        let secs = instant::SystemTime::now()
            .duration_since(instant::SystemTime::UNIX_EPOCH)
            .expect("We are living in the future")
            .as_secs();

        let mut mapper = Self {
            resolved_tex: Vec::with_capacity(board.squares.len()),
            map_texture,
            map_seed: (secs % 100000) as usize,
            inverted: invert,
            last_tick: 0,
            forecasted_wind: 0,
            incoming_wind: 0,
            winds: vec![0; board.width() + board.height()].into(),
        };

        mapper.remap(board, player_colors, 0);

        mapper
    }
}

#[derive(Clone)]
pub enum MappedTileVariant {
    Healthy,
    Dying,
    Dead,
}

#[derive(Clone)]
pub struct MappedTile {
    resolved_tex: Vec<TexQuad>,
    map_texture: TextureHandle,
}

impl MappedTile {
    pub fn new(
        variant: MappedTileVariant,
        color: Option<Color32>,
        highlight: Option<Color32>,
        coord: Option<Coordinate>,
        map_texture: TextureHandle,
    ) -> Self {
        let resolved_tex = if let Some(coord) = coord {
            Tex::board_game_tile(variant, color, highlight, coord.x * 99 + coord.y)
        } else {
            Tex::game_tile(color, highlight)
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

pub fn quickrand(mut n: usize) -> usize {
    n ^= n << 13;
    n ^= n >> 7;
    n ^= n << 17;
    n % 100
}
