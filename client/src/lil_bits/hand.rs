use core::player::Hand;

use eframe::egui;

use crate::theming::Theme;

use super::{tile::TilePlayer, SquareUI, TileUI};

pub struct HandUI<'a> {
    hand: &'a mut Hand,
}

impl<'a> HandUI<'a> {
    pub fn new(hand: &'a mut Hand) -> Self {
        Self { hand }
    }
}

impl<'a> HandUI<'a> {
    pub fn render(
        self,
        selected_tile: Option<usize>,
        ui: &mut egui::Ui,
        theme: &Theme,
    ) -> Option<Option<usize>> {
        let mut rearrange = None;
        let mut next_selection = None;

        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);
        ui.horizontal(|ui| {
            for (i, char) in self.hand.iter().enumerate() {
                SquareUI::new().render(ui, theme, |ui, theme| {
                    if TileUI::new(*char, TilePlayer::Own)
                        .selected(Some(i) == selected_tile)
                        .render(ui, theme)
                        .clicked()
                    {
                        if let Some(selected) = selected_tile {
                            next_selection = Some(None);
                            if selected != i {
                                rearrange = Some((selected, i));
                            }
                        } else {
                            next_selection = Some(Some(i));
                        }
                    }
                });
            }
        });

        if let Some((from, to)) = rearrange {
            self.hand.rearrange(from, to);
        }

        next_selection
    }
}
