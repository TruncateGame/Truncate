use eframe::egui::{self, Id};
use epaint::Color32;
use truncate_core::board::Coordinate;

use crate::utils::{
    depot::TruncateDepot,
    mapper::{MappedTile, MappedTileVariant},
    Lighten,
};

use super::{character::CharacterOrient, CharacterUI};

pub enum TilePlayer {
    Own,
    Enemy(usize),
}

pub struct TileUI {
    letter: Option<char>,
    player: TilePlayer,
    selected: bool,
    highlighted: bool,
    active: bool,
    hovered: bool,
    ghost: bool,
    added: bool,
    modified: bool,
    defeated: bool,
    truncated: bool,
    won: bool,
    victor: bool,
    id: Option<Id>,
}

impl TileUI {
    pub fn new(letter: Option<char>, player: TilePlayer) -> Self {
        Self {
            letter,
            player,
            selected: false,
            highlighted: false,
            active: true,
            hovered: false,
            ghost: false,
            added: false,
            modified: false,
            defeated: false,
            truncated: false,
            won: false,
            victor: false,
            id: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn hovered(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self
    }

    pub fn ghost(mut self, ghost: bool) -> Self {
        self.ghost = ghost;
        self
    }

    pub fn added(mut self, added: bool) -> Self {
        self.added = added;
        self
    }

    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    pub fn defeated(mut self, defeated: bool) -> Self {
        self.defeated = defeated;
        self
    }

    pub fn truncated(mut self, truncated: bool) -> Self {
        self.truncated = truncated;
        self
    }

    pub fn won(mut self, won: bool) -> Self {
        self.won = won;
        self
    }

    pub fn victor(mut self, victor: bool) -> Self {
        self.victor = victor;
        self
    }

    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
}

impl TileUI {
    fn tile_color(&self, hovered: bool, depot: &TruncateDepot) -> Color32 {
        let TruncateDepot {
            timing,
            gameplay,
            aesthetics,
            ..
        } = depot;

        if self.highlighted && timing.current_time.subsec_millis() > 500 {
            aesthetics.theme.selection.pastel()
        } else if self.won {
            aesthetics.theme.selection
        } else {
            let color = match (&self.player, hovered) {
                (TilePlayer::Own, false) => {
                    aesthetics.player_colors[gameplay.player_number as usize].pastel()
                }
                (TilePlayer::Own, true) => aesthetics.player_colors
                    [gameplay.player_number as usize]
                    .pastel()
                    .lighten(),
                (TilePlayer::Enemy(p), false) => aesthetics.player_colors[*p].pastel(),
                (TilePlayer::Enemy(p), true) => aesthetics.player_colors[*p].pastel().lighten(),
            };
            if self.defeated || self.truncated || !self.active {
                color.pastel()
            } else {
                color
            }
        }
    }

    pub fn render(
        mut self,
        coord: Option<Coordinate>,
        ui: &mut egui::Ui,
        capture_clicks: bool,
        rescale: Option<f32>,
        depot: &mut TruncateDepot,
    ) -> egui::Response {
        let TruncateDepot {
            timing, aesthetics, ..
        } = depot;

        let mut tile_gone = false;
        if timing.current_time > timing.prev_to_next_turn.0
            && timing.current_time < timing.prev_to_next_turn.1
        {
            let (from, to) = timing.prev_to_next_turn;
            let dur = timing.current_time.saturating_sub(from);
            let total = to.saturating_sub(from);
            let proportion = dur.as_secs_f32() / total.as_secs_f32();
            if proportion < 0.15 && self.defeated {
                self.defeated = false;
            }
            if proportion < 0.5 && self.truncated {
                self.truncated = false;
            }
            if proportion > 0.6 && self.defeated {
                tile_gone = true;
            }
            if proportion > 0.75 && self.truncated {
                tile_gone = true;
            }
        } else if self.defeated || self.truncated {
            tile_gone = true;
        } else {
            self.victor = false;
        }

        let theme = rescale
            .map(|v| aesthetics.theme.rescale(v))
            .unwrap_or_else(|| aesthetics.theme.clone());

        // TODO: Remove magic number somehow (currently 2px/16px for tile sprite border)
        let tile_margin = theme.grid_size * 0.125;

        let (mut base_rect, _) = ui.allocate_exact_size(
            egui::vec2(theme.grid_size, theme.grid_size),
            egui::Sense::hover(),
        );

        let mut tile_rect = base_rect.shrink(tile_margin);
        let tile_sense = if capture_clicks {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let mut response = ui.allocate_rect(tile_rect, tile_sense);

        if let Some(id) = self.id {
            response = ui.interact(tile_rect, id, egui::Sense::click_and_drag());
        }

        let hovered = (response.hovered() || self.hovered) && (!self.truncated && !self.defeated);
        let tile_color = self.tile_color(hovered, &depot);
        let TruncateDepot {
            aesthetics,
            interactions,
            ..
        } = depot;

        if hovered {
            if !self.ghost {
                base_rect = base_rect.translate(egui::vec2(0.0, tile_margin * -1.0));
                tile_rect = tile_rect.translate(egui::vec2(0.0, tile_margin * -1.0));
            }
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
            if let Some(coord) = coord {
                interactions.hovered_tile_on_board = Some(coord);
            }
        }

        if ui.is_rect_visible(base_rect) {
            let outline = if self.added {
                Some(theme.addition)
            } else if self.modified {
                Some(theme.modification)
            } else if self.selected || self.victor {
                Some(theme.selection)
            } else {
                None
            };

            let variant = if tile_gone {
                MappedTileVariant::Gone
            } else if self.defeated {
                MappedTileVariant::Dead
            } else if self.truncated {
                MappedTileVariant::Dying
            } else {
                MappedTileVariant::Healthy
            };
            let mapped_tile = if self.ghost {
                MappedTile::new(
                    variant,
                    self.letter.unwrap_or(' '),
                    None,
                    Some(tile_color),
                    coord,
                    aesthetics.map_texture.clone(),
                )
            } else {
                MappedTile::new(
                    variant,
                    self.letter.unwrap_or(' '),
                    Some(tile_color),
                    outline,
                    coord,
                    aesthetics.map_texture.clone(),
                )
            };
            mapped_tile.render(base_rect, ui);

            let mut char_rect = tile_rect.clone();
            char_rect.set_height(char_rect.height() - tile_margin * 0.5);

            if let Some(char) = self.letter {
                CharacterUI::new(
                    char,
                    match self.player {
                        TilePlayer::Own => CharacterOrient::North,
                        TilePlayer::Enemy(_) => CharacterOrient::South,
                    },
                )
                .hovered(hovered)
                .selected(self.selected)
                .active(self.active)
                .ghost(self.ghost)
                .defeated(self.defeated)
                .truncated(self.truncated)
                .gone(tile_gone)
                .render(ui, char_rect, &theme);
            }
        }

        // let outline = if self.selected {
        //     Some(theme.selection)
        // } else if self.added {
        //     Some(theme.addition)
        // } else if self.modified {
        //     Some(theme.modification)
        // } else {
        //     None
        // };

        // if let Some(outline) = outline {
        //     ui.painter().rect_stroke(
        //         tile_rect.expand(theme.tile_margin * 0.5),
        //         theme.rounding * 1.3,
        //         Stroke::new(theme.tile_margin, outline),
        //     )
        // }

        response
    }
}
