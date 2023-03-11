use core::player::Hand;

use eframe::egui;

use crate::theming::Theme;

use super::{tile::TilePlayer, SquareUI, TileUI};

pub struct HandUI<'a> {
    hand: &'a mut Hand,
    active: bool,
}

impl<'a> HandUI<'a> {
    pub fn new(hand: &'a mut Hand) -> Self {
        Self { hand, active: true }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
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
        let (margin, theme) = theme.rescale(&ui.available_rect_before_wrap(), self.hand.len(), 1);
        let outer_frame = egui::Frame::none().inner_margin(margin);

        outer_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, char) in self.hand.iter().enumerate() {
                    SquareUI::new()
                        .decorated(false)
                        .render(ui, &theme, |ui, theme| {
                            if TileUI::new(*char, TilePlayer::Own)
                                .active(self.active)
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
        });

        if let Some((from, to)) = rearrange {
            self.hand.rearrange(from, to);
        }

        next_selection
    }
}
