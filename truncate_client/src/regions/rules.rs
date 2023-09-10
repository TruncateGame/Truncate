use std::collections::HashMap;

use eframe::egui::{self, Layout, Order, RichText, ScrollArea};
use epaint::{hex_color, vec2, Color32, TextureHandle};
use instant::Duration;
use serde::Deserialize;
use truncate_core::player::Hand;

use crate::{
    lil_bits::BoardUI,
    utils::{text::TextHelper, Diaphanize, Lighten, Theme},
};

const RULES: &[u8] = include_bytes!("../../tutorials/rules.yml");

#[derive(Deserialize, Debug, Clone)]
struct Rules {
    rules: Vec<RuleSection>,
}

#[derive(Deserialize, Debug, Clone)]
struct RuleSection {
    section_title: String,
    rules: Vec<RuleBlock>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum RuleBlock {
    CascadingRules {
        title: String,
        cascades: Vec<RuleBlock>,
    },
    Rule {
        message: String,
        boards: Vec<RuleBoard>,
    },
}

#[derive(Deserialize, Debug, Clone)]
struct RuleBoard {
    board: String,
    message: Option<String>,
}

pub struct RulesState {
    rules: Rules,
}

impl RulesState {
    pub fn new(map_texture: TextureHandle, theme: Theme) -> Self {
        let rules: Rules = serde_yaml::from_slice(RULES).expect("Rules should match Rules format");

        Self { rules }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme, current_time: Duration) {
        ui.label("I am the rules");

        for section in &self.rules.rules {
            ui.heading(&section.section_title);

            for rule in &section.rules {
                match rule {
                    RuleBlock::CascadingRules { title, cascades } => {
                        ui.label(title);
                        for block in cascades {
                            RulesState::render_rule(block.clone(), ui, theme, current_time);
                        }
                    }
                    RuleBlock::Rule { .. } => {
                        RulesState::render_rule(rule.clone(), ui, theme, current_time);
                    }
                }
            }
        }
    }

    fn render_rule(block: RuleBlock, ui: &mut egui::Ui, theme: &Theme, current_time: Duration) {
        let RuleBlock::Rule { message, boards } = block else {
            return;
        };

        ui.heading(message);
        for board in boards {
            ui.label(board.board);
            if let Some(caption) = board.message {
                ui.small(caption);
            }
        }
    }
}
