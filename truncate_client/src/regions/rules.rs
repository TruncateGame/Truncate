use std::collections::HashMap;

use eframe::egui::{self, Layout, Order, RichText, ScrollArea, Sense};
use epaint::{emath::Align, hex_color, vec2, Color32, TextureHandle};
use instant::Duration;
use serde::Deserialize;
use truncate_core::{game::Game, player::Hand};

use crate::{
    lil_bits::BoardUI,
    utils::{text::TextHelper, Diaphanize, Lighten, Theme},
};

use super::active_game::ActiveGame;

const RULES: &[u8] = include_bytes!("../../tutorials/rules.yml");

#[derive(Deserialize, Clone)]
struct Rules {
    rules: Vec<RuleSection>,
}

#[derive(Deserialize, Clone)]
struct RuleSection {
    section_title: String,
    rules: Vec<RuleBlock>,
}

#[derive(Deserialize, Clone)]
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

#[derive(Deserialize, Clone)]
struct RuleBoard {
    board: String,
    message: Option<String>,
    #[serde(skip)]
    game: Option<Game>,
    #[serde(skip)]
    active_game: Option<ActiveGame>,
}

pub struct RulesState {
    rules: Rules,
}

impl RuleBoard {
    fn hydrate(&mut self, map_texture: &TextureHandle, theme: &Theme) {
        let mut game = Game::from_string(&self.board);

        let mut active_game = ActiveGame::new(
            "RULES".into(),
            game.players.iter().map(Into::into).collect(),
            0,
            0,
            game.board.clone(),
            game.players[0].hand.clone(),
            map_texture.clone(),
            theme.clone(),
        );
        active_game.ctx.timers_visible = false;
        active_game.ctx.hand_visible = false;

        self.game = Some(Game::from_string(&self.board));
        self.active_game = Some(active_game);
    }
}

impl RuleBlock {
    fn hydrate(&mut self, map_texture: &TextureHandle, theme: &Theme) {
        match self {
            RuleBlock::CascadingRules { title, cascades } => {
                for block in cascades {
                    block.hydrate(map_texture, theme);
                }
            }
            RuleBlock::Rule { message, boards } => {
                for board in boards {
                    board.hydrate(map_texture, theme);
                }
            }
        }
    }
}

impl RulesState {
    pub fn new(map_texture: TextureHandle, theme: Theme) -> Self {
        let mut rules: Rules =
            serde_yaml::from_slice(RULES).expect("Rules should match Rules format");

        for section in &mut rules.rules {
            for rule in &mut section.rules {
                rule.hydrate(&map_texture, &theme);
            }
        }

        Self { rules }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme, current_time: Duration) {
        ui.label("I am the rules");

        for section in self.rules.rules.iter_mut() {
            ui.heading(&section.section_title);

            for rule in section.rules.iter_mut() {
                match rule {
                    RuleBlock::CascadingRules { title, cascades } => {
                        ui.label(title.clone());
                        for block in cascades.iter_mut() {
                            RulesState::render_rule(block, ui, theme, current_time);
                            break;
                        }
                    }
                    RuleBlock::Rule { .. } => {
                        RulesState::render_rule(rule, ui, theme, current_time);
                    }
                }
                break;
            }
        }
    }

    fn render_rule(
        block: &mut RuleBlock,
        ui: &mut egui::Ui,
        theme: &Theme,
        current_time: Duration,
    ) {
        let RuleBlock::Rule { message, boards } = block else {
            return;
        };

        ui.heading(message);
        for board in boards.iter_mut() {
            let (game_rect, _) = ui.allocate_exact_size(vec2(200.0, 200.0), Sense::hover());
            let mut game_ui = ui.child_ui(game_rect, Layout::left_to_right(Align::TOP));

            let border = game_rect.expand(5.0);
            ui.painter()
                .rect_filled(border, 10.0, hex_color!("#ff0000"));

            board
                .active_game
                .as_mut()
                .expect("Board was hydrated")
                .render(&mut game_ui, theme, None, current_time);

            if let Some(caption) = &board.message {
                ui.label(caption);
            }

            break;
        }
    }
}
