use std::sync::Arc;

use eframe::{
    egui::{self, FontId, Id, Layout, Sense},
    emath::Align,
};
use epaint::{vec2, Color32, Galley, Rect, Vec2};
use truncate_core::reporting::{BattleReport, BattleWord};

use crate::utils::{
    depot::{AestheticDepot, TruncateDepot},
    tex::paint_dialog_background,
    text::TextHelper,
    Lighten,
};

pub struct BattleUI<'a> {
    battle: &'a BattleReport,
    show_headline: bool,
}

impl<'a> BattleUI<'a> {
    pub fn new(battle: &'a BattleReport, show_headline: bool) -> Self {
        Self {
            battle,
            show_headline,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct BattleUIState {
    size_last_frame: Vec2,
    /// If None, we defer to showing when it is the latest battle
    /// TODO: For now this is always true until we re-implement closing these dialogs
    show_details: Option<bool>,
}

impl<'a> BattleUI<'a> {
    fn get_galleys(
        &self,
        battle_words: &'a Vec<BattleWord>,
        transparent: bool,
        ui: &mut egui::Ui,
        aesthetics: &AestheticDepot,
    ) -> Vec<Arc<Galley>> {
        let dot = ui.painter().layout_no_wrap(
            "• ".into(),
            FontId::new(
                aesthetics.theme.letter_size * 0.75,
                egui::FontFamily::Name("Truncate-Heavy".into()),
            ),
            aesthetics.theme.text,
        );

        let mut words: Vec<_> = battle_words
            .iter()
            .flat_map(|w| {
                [
                    if &w.original_word == "#" || &w.original_word == "|" {
                        let label = if &w.original_word == "#" {
                            "A TOWN"
                        } else {
                            "AN ARTIFACT"
                        };
                        ui.painter().layout_no_wrap(
                            // TODO: It would be nice to phrase this as <player_name>'s town,
                            // but we don't have player data in the battle UI yet so this is a good first step.
                            label.into(),
                            FontId::new(
                                aesthetics.theme.letter_size * 0.75,
                                egui::FontFamily::Name("Truncate-Heavy".into()),
                            ),
                            match (transparent, w.valid) {
                                (true, _) => Color32::TRANSPARENT,
                                (false, Some(true)) => aesthetics.theme.word_valid,
                                (false, Some(false)) => aesthetics.theme.word_invalid,
                                (false, None) => aesthetics.theme.text,
                            },
                        )
                    } else {
                        ui.painter().layout_no_wrap(
                            w.resolved_word.clone(),
                            FontId::new(
                                aesthetics.theme.letter_size * 0.75,
                                egui::FontFamily::Name("Truncate-Heavy".into()),
                            ),
                            match (transparent, w.valid) {
                                (true, _) => Color32::TRANSPARENT,
                                (false, Some(true)) => aesthetics.theme.word_valid,
                                (false, Some(false)) => aesthetics.theme.word_invalid,
                                (false, None) => aesthetics.theme.text,
                            },
                        )
                    },
                    dot.clone(),
                ]
            })
            .collect();
        words.pop();
        words
    }

    fn paint_galleys(
        &self,
        mut galleys: Vec<Arc<Galley>>,
        ui: &mut egui::Ui,
        centered: bool,
    ) -> egui::Response {
        galleys.reverse();

        let mut staging_galleys: Vec<Arc<Galley>> = vec![];

        let avail_width = ui.available_width();
        let origin = ui.next_widget_position();
        let word_height = galleys.first().unwrap().mesh_bounds.height();
        let word_pad = 4.0;
        let word_offset = 8.0;

        let mut current_row = 0;
        let mut current_width = 0.0;

        while !galleys.is_empty() || !staging_galleys.is_empty() {
            let last_galley = galleys.last().map(|galley| galley.mesh_bounds.width());
            let next_width = last_galley.unwrap_or_default() + word_pad * 2.0;

            if last_galley.is_none()
                || (current_width > 0.0 && current_width + next_width > avail_width)
            {
                let padding = if centered {
                    (avail_width - current_width) / 2.0
                } else {
                    0.0
                };
                let mut total_x = 0.0;
                for galley in staging_galleys.drain(0..) {
                    let Vec2 { x, y } = galley.mesh_bounds.size();
                    let word_pt = origin
                        + vec2(
                            padding + total_x + word_pad + word_offset,
                            current_row as f32 * y
                                + current_row as f32 * word_pad
                                + galley.mesh_bounds.height() * 0.25,
                        );

                    ui.painter().galley(word_pt, galley, Color32::BLACK);

                    total_x += x + word_pad * 2.0;
                }
                current_row += 1;
                current_width = 0.0;
            } else {
                let galley = galleys.pop().unwrap();
                current_width += galley.mesh_bounds.width() + word_pad * 2.0;
                staging_galleys.push(galley);
            }
        }

        let total_height = word_height * current_row as f32;
        let battle_rect = epaint::Rect::from_min_size(origin, vec2(avail_width, total_height));

        ui.allocate_rect(battle_rect, Sense::hover())
    }

    pub fn render(self, ui: &mut egui::Ui, depot: &mut TruncateDepot) {
        let TruncateDepot { aesthetics, .. } = depot;

        let battle_id = Id::new("battle").with(self.battle.battle_number.unwrap_or_default());
        let prev_battle_storage: Option<BattleUIState> = ui.memory(|m| m.data.get_temp(battle_id));

        // Paint the background dialog based on the size of the battle last frame
        if let Some(BattleUIState {
            size_last_frame,
            show_details,
        }) = prev_battle_storage
        {
            let (dialog_rect, _) = paint_dialog_background(
                true,
                false,
                false,
                size_last_frame,
                aesthetics.theme.water.lighten(),
                &aesthetics.map_texture,
                ui,
            );
            let offset = (dialog_rect.height() - size_last_frame.y) / 2.0;
            let mut dialog_ui = ui.child_ui(dialog_rect, Layout::top_down(Align::Min));
            dialog_ui.add_space(offset);

            let battle_rect = self.render_innards(
                show_details.unwrap_or(true),
                prev_battle_storage,
                &mut dialog_ui,
                depot,
            );

            // Save the sizing of our box for the next render pass to draw the background
            let new_state = BattleUIState {
                size_last_frame: battle_rect.size(),
                show_details,
            };
            if prev_battle_storage != Some(new_state) {
                ui.memory_mut(|m| m.data.insert_temp(battle_id, new_state));
                ui.ctx().request_repaint();
            }
        } else {
            // If we have no info on sizing, we can't paint the dialog background.
            // Instead, we render everything transparent and trigger an immediate re-render.
            let battle_rect = self.render_innards(false, prev_battle_storage, ui, depot);
            // Save the sizing of our box for the next render pass to draw the background
            // TODO: We can (maybe?) use Memory::area_rect now instead of tracking sizes ourselves
            ui.memory_mut(|m| {
                m.data.insert_temp(
                    battle_id,
                    BattleUIState {
                        show_details: None,
                        size_last_frame: battle_rect.size(),
                    },
                )
            });
            ui.ctx().request_repaint();
        }
    }

    fn render_innards(
        &self,
        active: bool,
        prev_battle_storage: Option<BattleUIState>,
        ui: &mut egui::Ui,
        depot: &mut TruncateDepot,
    ) -> Rect {
        let TruncateDepot { aesthetics, .. } = depot;

        let mut theme = aesthetics.theme.rescale(0.5);
        theme.tile_margin = 0.0;
        let render_transparent = prev_battle_storage.is_none();

        let mut battle_rect = Rect::NOTHING;

        if self.show_headline {
            if !self.battle.attackers.is_empty() {
                ui.allocate_ui_with_layout(
                    vec2(ui.available_size_before_wrap().x, 0.0),
                    Layout::left_to_right(Align::Center).with_main_wrap(true),
                    |ui| {
                        let words = self.get_galleys(
                            &self.battle.attackers,
                            render_transparent,
                            ui,
                            &aesthetics,
                        );
                        battle_rect = battle_rect.union(self.paint_galleys(words, ui, false).rect);
                    },
                );
                ui.add_space(5.0);
            }

            if !self.battle.attackers.is_empty() && !self.battle.defenders.is_empty() {
                let (msg, _) = match self.battle.outcome {
                    truncate_core::judge::Outcome::AttackerWins(_) => {
                        ("won an attack against", aesthetics.theme.word_valid)
                    }
                    truncate_core::judge::Outcome::DefenderWins => {
                        ("failed an attack against", aesthetics.theme.word_invalid)
                    }
                };

                let galley = ui.painter().layout_no_wrap(
                    msg.to_string(),
                    FontId::new(
                        aesthetics.theme.letter_size * 0.3,
                        egui::FontFamily::Name("Truncate-Heavy".into()),
                    ),
                    if render_transparent {
                        Color32::TRANSPARENT
                    } else {
                        aesthetics.theme.text
                    },
                );
                battle_rect = battle_rect.union(self.paint_galleys(vec![galley], ui, false).rect);
                ui.add_space(5.0);
            }

            if !self.battle.defenders.is_empty() {
                ui.allocate_ui_with_layout(
                    vec2(ui.available_size_before_wrap().x, 0.0),
                    Layout::left_to_right(Align::Center).with_main_wrap(true),
                    |ui| {
                        let words = self.get_galleys(
                            &self.battle.defenders,
                            render_transparent,
                            ui,
                            &aesthetics,
                        );
                        battle_rect = battle_rect.union(self.paint_galleys(words, ui, false).rect);
                    },
                );
            }

            if !active {
                return battle_rect;
            }

            ui.add_space(8.0);
        }

        let definition_space = ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                for word in self
                    .battle
                    .attackers
                    .iter()
                    .chain(self.battle.defenders.iter())
                {
                    if &word.original_word == "#" || &word.original_word == "|" {
                        continue;
                    }
                    ui.add_space(12.0);
                    TextHelper::heavy(
                        &word.resolved_word,
                        aesthetics.theme.letter_size * 0.5,
                        None,
                        ui,
                    )
                    .paint(aesthetics.theme.text, ui, false);

                    match (word.valid, &word.meanings) {
                        (Some(true), Some(meanings)) if !meanings.is_empty() => TextHelper::light(
                            &if meanings[0].pos.is_empty() {
                                format!("{}", meanings[0].defs[0])
                            } else {
                                format!("{}: {}", meanings[0].pos, meanings[0].defs[0])
                            },
                            24.0,
                            Some(ui.available_width()),
                            ui,
                        )
                        .paint(aesthetics.theme.text, ui, false),
                        (Some(true), _) => TextHelper::light(
                            "Definition not found",
                            24.0,
                            Some(ui.available_width()),
                            ui,
                        )
                        .paint(aesthetics.theme.text, ui, false),
                        (Some(false), _) => {
                            TextHelper::light("Invalid word", 24.0, Some(ui.available_width()), ui)
                                .paint(aesthetics.theme.text, ui, false)
                        }
                        (None, _) => {
                            TextHelper::light("Unchecked", 24.0, Some(ui.available_width()), ui)
                                .paint(aesthetics.theme.text, ui, false)
                        }
                    };

                    ui.add_space(12.0);
                }
            })
        });
        battle_rect = battle_rect.union(definition_space.response.rect);

        battle_rect
    }
}
