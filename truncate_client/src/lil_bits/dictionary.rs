use eframe::egui::{self, CursorIcon, Layout, Sense};
use epaint::{
    emath::{Align, Align2},
    hex_color, vec2, Color32, Stroke,
};

use std::{collections::HashMap, f32};
use truncate_core::{
    judge::Outcome,
    messages::PlayerMessage,
    reporting::{BattleReport, BattleWord, WordMeaning},
};

use crate::utils::{
    depot::TruncateDepot,
    game_evals::get_main_dict,
    tex::{render_tex_quad, tiles},
    text::TextHelper,
    Lighten,
};

use super::BattleUI;

#[derive(Clone)]
pub struct DictionaryUI {
    current_word: String,
    is_valid: bool,
    focus_in_n_frames: usize,
    definitions: HashMap<String, Option<Vec<WordMeaning>>>,
    prefix_matches: Vec<String>,
    suffix_matches: Vec<String>,
}

impl DictionaryUI {
    pub fn new(initial_focus: bool) -> Self {
        Self {
            current_word: String::new(),
            is_valid: false,
            focus_in_n_frames: if initial_focus { 2 } else { 0 },
            definitions: HashMap::new(),
            prefix_matches: Vec::new(),
            suffix_matches: Vec::new(),
        }
    }

    fn find_prefix_matches(&mut self, dict: &HashMap<String, impl std::any::Any>) {
        if self.current_word.is_empty() {
            self.prefix_matches.clear();
            return;
        }

        self.prefix_matches = dict
            .keys()
            .filter(|word| word.starts_with(&self.current_word))
            .filter(|word| *word != &self.current_word)
            .collect::<Vec<_>>()
            .into_iter()
            .min_by_key(|word| word.len())
            .into_iter()
            .cloned()
            .collect();
    }

    fn find_suffix_matches(&mut self, dict: &HashMap<String, impl std::any::Any>) {
        if self.current_word.is_empty() {
            self.suffix_matches.clear();
            return;
        }

        self.suffix_matches = dict
            .keys()
            .filter(|word| word.ends_with(&self.current_word))
            .filter(|word| *word != &self.current_word)
            .collect::<Vec<_>>()
            .into_iter()
            .min_by_key(|word| word.len())
            .into_iter()
            .cloned()
            .collect();
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

        let desired_input_width = ui.available_width().min(550.0);

        let desired_input_height = input_fz * 2.0;
        let fill_height = depot
            .regions
            .headers_total_rect
            .map(|r| r.height())
            .unwrap_or(input_fz * 3.0);

        let spacing = fill_height - desired_input_height;
        if spacing > 4.0 {
            ui.add_space(spacing - 4.0);
        }

        let inset = (ui.available_width() - desired_input_width) / 2.0;
        let (input_band, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), desired_input_height),
            Sense::hover(),
        );
        let input_band_inner = input_band.shrink2(vec2(inset, 0.0));

        let mut close_button = input_band_inner.clone();
        close_button.set_left(close_button.right() - 48.0);
        if close_button.height() < 48.0 {
            close_button = close_button.expand2(vec2(0.0, (48.0 - close_button.height()) / 2.0));
        }

        let button_resp = ui.interact(close_button, ui.id().with("close"), Sense::click());

        if button_resp.hovered() {
            close_button = close_button.translate(vec2(0.0, -2.0));
            ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
        }
        render_tex_quad(
            tiles::quad::CLOSE_BUTTON,
            close_button,
            &depot.aesthetics.map_texture,
            ui,
        );

        if button_resp.clicked() {
            depot.ui_state.dictionary_open = false;
            depot.ui_state.dictionary_focused = false;
            return None;
        }

        let mut input_inner = input_band_inner.clone();
        input_inner.set_right(close_button.left() - 16.0);
        input_inner.set_left(input_inner.left() + 6.0);

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

        if self.focus_in_n_frames > 0 {
            if self.focus_in_n_frames == 1 {
                input.response.request_focus();
                self.focus_in_n_frames = 0;
            } else {
                self.focus_in_n_frames -= 1;
            }
        }

        if (input.response.gained_focus() || input.response.has_focus())
            && !ui.input(|i| i.pointer.any_down())
        {
            depot.ui_state.dictionary_focused = true;
        } else if !input.response.has_focus() {
            depot.ui_state.dictionary_focused = false;
        }

        if input.response.changed() {
            self.current_word = self.current_word.to_ascii_lowercase();

            let dict_lock = get_main_dict();
            let dict = dict_lock.as_ref().unwrap();

            self.is_valid = dict.contains_key(&self.current_word);
            self.find_prefix_matches(dict);
            self.find_suffix_matches(dict);

            if self.is_valid && !self.definitions.contains_key(&self.current_word) {
                msg = Some(PlayerMessage::RequestDefinitions(vec![self
                    .current_word
                    .clone()]));
            }
        }

        if !self.current_word.is_empty() {
            depot.ui_state.dictionary_showing_definition = true;

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

            let mut defenders = vec![BattleWord {
                original_word: self.current_word.clone(),
                resolved_word: self.current_word.clone(),
                meanings,
                valid: Some(self.is_valid),
            }];

            if let Some(prefix_word) = self.prefix_matches.first() {
                defenders.push(BattleWord {
                    original_word: prefix_word.clone(),
                    resolved_word: prefix_word.clone(),
                    meanings: Some(vec![WordMeaning {
                        pos: "".to_string(),
                        defs: vec!["Possible prefix match".to_string()],
                    }]),
                    valid: Some(true),
                });
            }

            if let Some(suffix_word) = self.suffix_matches.first() {
                defenders.push(BattleWord {
                    original_word: suffix_word.clone(),
                    resolved_word: suffix_word.clone(),
                    meanings: Some(vec![WordMeaning {
                        pos: "".to_string(),
                        defs: vec!["Possible suffix match".to_string()],
                    }]),
                    valid: Some(true),
                });
            }

            let report = BattleReport {
                battle_number: None,
                attackers: vec![],
                defenders,
                outcome: Outcome::DefenderWins,
            };

            let desired_battle_width = ui.available_width().min(550.0);

            ui.add_space(20.0);

            let inset = (ui.available_width() - desired_battle_width) / 2.0;
            let (battle_band, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), ui.available_height()),
                Sense::hover(),
            );
            let battle_inner = battle_band.shrink2(vec2(inset, 0.0));
            let mut battle_ui = ui.child_ui(battle_inner, Layout::top_down(Align::LEFT));

            depot.aesthetics.theme.letter_size = 48.0;
            BattleUI::new(&report, false).render(&mut battle_ui, depot);
        } else {
            depot.ui_state.dictionary_showing_definition = false;

            if !depot.ui_state.dictionary_focused && self.focus_in_n_frames == 0 {
                let text = TextHelper::heavy("Search", 20.0, None, ui);
                text.paint_within(
                    input.response.rect,
                    Align2::CENTER_CENTER,
                    Color32::WHITE.gamma_multiply(0.8),
                    ui,
                );
            }
        }

        msg
    }
}
