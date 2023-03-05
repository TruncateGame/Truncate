use eframe::egui::{self, Margin};
use epaint::{Color32, Stroke};

use crate::theming::Theme;

use super::{character::CharacterOrient, CharacterUI};

pub enum TilePlayer {
    Own,
    Enemy,
}

pub struct TileUI {
    letter: char,
    player: TilePlayer,
    selected: bool,
    ghost: bool,
    added: bool,
    modified: bool,
    defeated: bool,
    truncated: bool,
}

impl TileUI {
    pub fn new(letter: char, player: TilePlayer) -> Self {
        Self {
            letter,
            player,
            selected: false,
            ghost: false,
            added: false,
            modified: false,
            defeated: false,
            truncated: false,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
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
}

impl TileUI {
    fn edge_color(&self, theme: &Theme) -> Color32 {
        if self.defeated || self.truncated {
            theme.text.dark
        } else {
            match self.player {
                TilePlayer::Own => theme.friend.dark,
                TilePlayer::Enemy => theme.enemy.dark,
            }
        }
    }

    fn tile_color(&self, hovered: bool, theme: &Theme) -> Color32 {
        if self.defeated || self.truncated {
            theme.text.base
        } else {
            match (&self.player, hovered) {
                (TilePlayer::Own, false) => theme.friend.base,
                (TilePlayer::Own, true) => theme.friend.light,
                (TilePlayer::Enemy, false) => theme.enemy.base,
                (TilePlayer::Enemy, true) => theme.enemy.light,
            }
        }
    }

    pub fn render(self, ui: &mut egui::Ui, theme: &Theme) -> egui::Response {
        let frame = egui::Frame::none().inner_margin(Margin::same(theme.tile_margin));
        frame
            .show(ui, |ui| {
                let tile_size = theme.grid_size - theme.tile_margin * 2.0;
                let (mut rect, response) =
                    ui.allocate_exact_size(egui::vec2(tile_size, tile_size), egui::Sense::click());
                if response.hovered() {
                    if !self.ghost {
                        rect = rect.translate(egui::vec2(0.0, -2.0));
                    }
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                }

                let mut raised_rect = rect.clone();
                raised_rect.set_height(tile_size - 4.0);

                if ui.is_rect_visible(rect) {
                    if self.ghost {
                        ui.painter().rect_stroke(
                            rect,
                            10.0,
                            Stroke::new(1.0, self.edge_color(theme)),
                        );
                        ui.painter().rect_stroke(
                            raised_rect,
                            10.0,
                            Stroke::new(1.0, self.tile_color(response.hovered(), theme)),
                        );
                    } else {
                        ui.painter().rect_filled(rect, 10.0, self.edge_color(theme));
                        ui.painter().rect_filled(
                            raised_rect,
                            10.0,
                            self.tile_color(response.hovered(), theme),
                        );
                    }

                    CharacterUI::new(
                        self.letter,
                        match self.player {
                            TilePlayer::Own => CharacterOrient::North,
                            TilePlayer::Enemy => CharacterOrient::South,
                        },
                    )
                    .hovered(response.hovered())
                    .selected(self.selected)
                    .ghost(self.ghost)
                    .defeated(self.defeated)
                    .truncated(self.truncated)
                    .render(ui, raised_rect, theme);

                    let outline = if self.selected {
                        Some(theme.selection)
                    } else if self.added {
                        Some(theme.addition)
                    } else if self.modified {
                        Some(theme.modification)
                    } else {
                        None
                    };

                    if let Some(outline) = outline {
                        ui.painter()
                            .rect_stroke(rect.expand(2.0), 13.0, Stroke::new(4.0, outline))
                    }
                }

                response
            })
            .inner
    }
}
