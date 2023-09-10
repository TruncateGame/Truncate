use std::collections::HashMap;

use eframe::egui::{self, Layout, Order, RichText, ScrollArea};
use epaint::{hex_color, vec2, Color32, TextureHandle};
use instant::Duration;
use serde::Deserialize;

use crate::utils::{text::TextHelper, Diaphanize, Lighten, Theme};

const RULES: &[u8] = include_bytes!("../../tutorials/rules.yml");

#[derive(Deserialize, Debug)]
struct Rules {
    rules: Vec<RuleSection>,
}

#[derive(Deserialize, Debug)]
struct RuleSection {
    section_title: String,
    rules: Vec<RuleBlock>,
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
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
    }
}
