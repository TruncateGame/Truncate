use eframe::egui::{self, Layout, Sense};
use epaint::{
    emath::{Align, Align2},
    vec2, Color32, Stroke,
};

use std::{collections::HashMap, f32};
use truncate_core::{
    judge::Outcome,
    messages::PlayerMessage,
    reporting::{BattleReport, BattleWord, WordMeaning},
};

use crate::utils::{depot::TruncateDepot, game_evals::get_main_dict, text::TextHelper, Lighten};

use super::BattleUI;

#[derive(Clone)]
pub struct DictionaryUI {
    current_word: String,
    is_valid: bool,
    definitions: HashMap<String, Option<Vec<WordMeaning>>>,
}

impl DictionaryUI {
    pub fn new() -> Self {
        Self {
            current_word: String::new(),
            is_valid: false,
            definitions: HashMap::new(),
        }
    }

    pub fn load_definitions(&mut self, definitions: Vec<(String, Option<Vec<WordMeaning>>)>) {
        for (word, def) in definitions {
            self.definitions.insert(word, def);
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        depot: &mut TruncateDepot,
    ) -> Option<PlayerMessage> {
        let mut msg = None;

        let input_fz = 20.0;

        let desired_input_width = ui.available_width().min(500.0);
        let desired_input_height = input_fz * 2.0;

        ui.add_space(20.0);
        let inset = (ui.available_width() - desired_input_width) / 2.0;
        let (input_band, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), desired_input_height),
            Sense::hover(),
        );
        let input_inner = input_band.shrink2(vec2(inset, 0.0));
        let mut input_ui = ui.child_ui(input_inner, Layout::top_down(Align::LEFT));

        ui.painter().rect_filled(
            input_inner.expand(4.0),
            4.0,
            depot.aesthetics.theme.text.gamma_multiply(0.8),
        );
        ui.painter().rect_stroke(
            input_inner.expand(4.0),
            4.0,
            Stroke::new(2.0, Color32::WHITE.gamma_multiply(0.5)),
        );

        let input = egui::TextEdit::singleline(&mut self.current_word)
            .desired_width(f32::INFINITY)
            .frame(false)
            .margin(egui::vec2(4.0, 2.0))
            .min_size(vec2(0.0, desired_input_height))
            .text_color(if self.is_valid {
                depot.aesthetics.theme.word_valid.lighten()
            } else {
                depot.aesthetics.theme.word_invalid.lighten().lighten()
            })
            .horizontal_align(Align::Center)
            .vertical_align(Align::Center)
            .font(egui::FontId::new(
                input_fz,
                egui::FontFamily::Name("Truncate-Heavy".into()),
            ))
            .show(&mut input_ui);

        if input.response.changed() {
            self.current_word = self.current_word.to_ascii_lowercase();

            let dict_lock = get_main_dict();
            let dict = dict_lock.as_ref().unwrap();

            self.is_valid = dict.contains_key(&self.current_word);

            if self.is_valid && !self.definitions.contains_key(&self.current_word) {
                msg = Some(PlayerMessage::RequestDefinitions(vec![self
                    .current_word
                    .clone()]));
            }
        }

        if !self.current_word.is_empty() {
            let meanings = if self.is_valid {
                let loading_meaning = if !self.definitions.contains_key(&self.current_word) {
                    Some(vec![WordMeaning {
                        pos: "".to_string(),
                        defs: vec!["Loading definitions...".to_string()],
                    }])
                } else {
                    None
                };

                loading_meaning.or(self.definitions.get(&self.current_word).cloned().flatten())
            } else {
                Some(vec![WordMeaning {
                    pos: "".to_string(),
                    defs: vec!["Invalid word".to_string()],
                }])
            };

            let report = BattleReport {
                battle_number: None,
                attackers: vec![],
                defenders: vec![BattleWord {
                    original_word: self.current_word.clone(),
                    resolved_word: self.current_word.clone(),
                    meanings,
                    valid: Some(self.is_valid),
                }],
                outcome: Outcome::DefenderWins,
            };

            let desired_battle_width = ui.available_width().min(500.0);

            ui.add_space(20.0);

            let inset = (ui.available_width() - desired_battle_width) / 2.0;
            let (battle_band, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), ui.available_height()),
                Sense::hover(),
            );
            let battle_inner = battle_band.shrink2(vec2(inset, 0.0));
            let mut battle_ui = ui.child_ui(battle_inner, Layout::top_down(Align::LEFT));

            BattleUI::new(&report, false).render(&mut battle_ui, depot);
        } else {
            let text = TextHelper::heavy("Search", 20.0, None, ui);
            text.paint_within(
                input.response.rect,
                Align2::CENTER_CENTER,
                Color32::WHITE.gamma_multiply(0.8),
                ui,
            );

            ui.add_space(20.0);

            let (dialog_rect, _) = crate::utils::tex::paint_dialog_background(
                false,
                false,
                true,
                vec2(input.response.rect.width(), 200.0),
                depot.aesthetics.theme.water.lighten().lighten(),
                &depot.aesthetics.map_texture,
                ui,
            );

            let dialog_text = TextHelper::light(
                "Use the input above to check whether a word exists in Truncate's dictionary",
                32.0,
                Some(dialog_rect.shrink(16.0).width()),
                ui,
            );

            dialog_text.paint_within(
                dialog_rect,
                Align2::CENTER_CENTER,
                depot.aesthetics.theme.text,
                ui,
            );
        }

        msg
    }
}
