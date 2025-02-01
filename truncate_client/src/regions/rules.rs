use std::fmt::Alignment;

use eframe::egui::{self, Align2, Sense};
use epaint::{hex_color, vec2, Color32, TextureHandle};
use instant::Duration;
use truncate_core::{
    board::Direction,
    game::{Game, GAME_COLORS},
    moves::Move,
    reporting::{BoardChange, BoardChangeAction, BoardChangeDetail, Change},
};

use crate::{
    app_outer::{Backchannel, GLYPHER},
    utils::{
        depot::{AestheticDepot, GameplayDepot, TimingDepot},
        game_evals::get_main_dict,
        includes::RuleCard,
        mapper::{ImageMusher, MappedBoard, MappedTileVariant},
        tex::{
            self, render_texs_clockwise, BGTexType, PieceLayer, Tex, TexLayers, TexQuad,
            TileDecoration,
        },
        text::TextHelper,
        timing::get_qs_tick,
        urls::back_to_menu,
        Lighten, Theme,
    },
};

pub const RULE_PLAYER_COLORS: [Color32; 2] = [
    Color32::from_rgb(GAME_COLORS[1].0, GAME_COLORS[1].1, GAME_COLORS[1].2),
    Color32::from_rgb(GAME_COLORS[0].0, GAME_COLORS[0].1, GAME_COLORS[0].2),
];

#[derive(Debug, Clone)]
pub struct ParsedRuleCard {
    pub sections: Vec<ParsedRuleCardSection>,
}

#[derive(Debug, Clone)]
pub struct ParsedRuleCardSection {
    title: String,
    examples: Vec<ParsedRuleCardExample>,
    started_animation_at: Option<usize>,
    description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRuleCardExample {
    textures: Vec<Vec<TexLayers>>,
}

fn parse_rule_card(rules: RuleCard) -> ParsedRuleCard {
    ParsedRuleCard {
        sections: rules
            .sections
            .into_iter()
            .map(|section| ParsedRuleCardSection {
                title: section.title,
                examples: section
                    .examples
                    .into_iter()
                    .map(parse_rule_example)
                    .collect(),
                started_animation_at: None,
                description: section.description,
            })
            .collect(),
    }
}

#[derive(Clone)]
pub struct RulesState {
    map_texture: TextureHandle,
    theme: Theme,
    aesthetics: AestheticDepot,
    rules: ParsedRuleCard,
}

impl RulesState {
    pub fn new(
        ctx: &egui::Context,
        map_texture: TextureHandle,
        theme: Theme,
        rules: RuleCard,
    ) -> Self {
        let aesthetics = AestheticDepot {
            theme: Theme::day(),
            qs_tick: 0,
            map_texture: map_texture.clone(),
            player_colors: vec![Color32::from_rgb(255, 0, 0), Color32::from_rgb(0, 255, 0)],
            destruction_tick: 0.05,
            destruction_duration: 0.6,
        };

        let parsed_rules = parse_rule_card(rules);

        Self {
            map_texture,
            theme,
            aesthetics,
            rules: parsed_rules,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme, current_time: Duration) {
        let glypher = GLYPHER.get().expect("Glypher should have been initialized");
        let cur_animation_tick = get_qs_tick(current_time) as usize;

        let bg = ui.available_rect_before_wrap();
        ui.painter().rect_filled(bg, 0.0, hex_color!("#c8ecff"));

        let heading_size = if bg.width() > 700.0 { 24.0 } else { 18.0 };
        let text_size = if bg.width() > 700.0 { 32.0 } else { 24.0 };
        let sprite_size = if bg.width() > 700.0 { 48.0 } else { 32.0 };

        ui.spacing_mut().item_spacing.x = 0.0;

        egui::ScrollArea::new([false, true]).show(ui, |ui| {
            ui.expand_to_include_x(ui.available_rect_before_wrap().right());

            ui.add_space(bg.height() * 0.2);

            for rule in &mut self.rules.sections {
                let t = TextHelper::heavy_centered(
                    &rule.title,
                    heading_size,
                    Some((ui.available_width() - 16.0).max(0.0)),
                    ui,
                );

                let (heading_rect, _) = ui.allocate_at_least(
                    vec2(ui.available_width(), t.mesh_size().y * 2.0),
                    Sense::hover(),
                );
                t.paint_within(heading_rect, Align2::CENTER_CENTER, theme.text, ui);

                // if ui.button("replay").clicked() {
                //     rule.started_animation_at = None;
                // }

                let started_at = rule.started_animation_at.get_or_insert(cur_animation_tick);

                let cur_example = (cur_animation_tick - *started_at).min(rule.examples.len() - 1);
                for row in rule.examples[cur_example].textures.iter() {
                    ui.horizontal(|ui| {
                        let row_width = row.len() as f32 * sprite_size;
                        let buffer = (ui.available_width() - row_width) / 2.0;
                        ui.add_space(buffer);

                        for slot in row.iter() {
                            let (rect, _) = ui.allocate_exact_size(
                                vec2(sprite_size, sprite_size),
                                Sense::hover(),
                            );
                            if let Some(structures) = slot.structures {
                                render_texs_clockwise(
                                    structures.to_vec(),
                                    rect,
                                    &self.map_texture,
                                    ui,
                                );
                            }
                            for piece in &slot.pieces {
                                match piece {
                                    PieceLayer::Texture(texs, _) => {
                                        render_texs_clockwise(
                                            texs.to_vec(),
                                            rect,
                                            &self.map_texture,
                                            ui,
                                        );
                                    }
                                    PieceLayer::Character(char, color, is_flipped, y_offset) => {
                                        assert!(false);
                                    }
                                }
                            }
                        }
                    });
                }

                let description = TextHelper::light_centered(
                    &rule.description,
                    text_size,
                    Some((ui.available_width() - 16.0).max(0.0)),
                    ui,
                );
                let final_size = description.mesh_size();
                let (text_area, _) = ui.allocate_at_least(
                    vec2(ui.available_width(), final_size.y + text_size * 2.0),
                    Sense::hover(),
                );
                description.paint_within(text_area, Align2::CENTER_CENTER, theme.text, ui);

                ui.add_space(heading_size * 5.0);
            }
        });

        let text = TextHelper::heavy("rules :-)", 12.0, None, ui);
        text.paint_within(
            ui.available_rect_before_wrap(),
            Align2::CENTER_CENTER,
            Color32::KHAKI,
            ui,
        );
    }
}

fn parse_rule_example(example: String) -> ParsedRuleCardExample {
    let textures = example
        .split('\n')
        .map(|row| {
            row.split_whitespace()
                .enumerate()
                .map(|(i, square)| match square {
                    "~" => TexLayers::default(),
                    "$0" | "$1" => {
                        let color = if square == "$0" {
                            RULE_PLAYER_COLORS[0]
                        } else {
                            RULE_PLAYER_COLORS[1]
                        };

                        Tex::artifact(color, vec![BGTexType::Land; 4], 0)
                    }
                    "+0" | "+1" => {
                        let color = if square == "+0" {
                            RULE_PLAYER_COLORS[0]
                        } else {
                            RULE_PLAYER_COLORS[1]
                        };

                        Tex::town(color, i, 0, 0)
                    }
                    c => {
                        let mut chars = c.chars();
                        let tile = chars.next().unwrap();
                        let modifier = chars.next();
                        let player = if tile.is_uppercase() { 1 } else { 0 };
                        let mut color = RULE_PLAYER_COLORS[player];
                        let orientation = if player == 0 {
                            Direction::North
                        } else {
                            Direction::South
                        };
                        let mut variant = MappedTileVariant::Healthy;
                        let mut highlight = None;

                        match modifier {
                            Some('*') => {
                                variant = MappedTileVariant::Dead;
                                color = Color32::GRAY;
                            }
                            Some('^') => highlight = Some(Color32::GREEN),
                            Some(c) => panic!("Unknown modifier {c}"),
                            None => {}
                        }

                        Tex::board_game_tile(
                            variant,
                            tile.to_ascii_uppercase(),
                            orientation,
                            Some(color.lighten()),
                            highlight,
                            TileDecoration::None,
                            i,
                        )
                    }
                })
                .collect()
        })
        .collect();

    ParsedRuleCardExample { textures }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn parse() {
        let example =
            "~  w  ~  ~  ~  ~  ~  ~  ~  ~  ~\nW  O*  R^  $0 $1  ~  +0 +1 r^  o*  w".to_string();
        let parsed = parse_rule_example(example);

        assert!(matches!(
            parsed.textures[0][1].pieces[1],
            PieceLayer::Character('W', _, true, _),
        ));
        assert!(matches!(
            parsed.textures[1][0].pieces[1],
            PieceLayer::Character('W', _, false, _),
        ));
        assert!(matches!(
            parsed.textures[1][1].pieces[1],
            PieceLayer::Character('O', _, false, _),
        ));
        assert!(matches!(
            parsed.textures[1][2].pieces[1],
            PieceLayer::Character('R', _, false, _),
        ));
        assert!(matches!(
            parsed.textures[1][3].structures,
            Some(tex::tiles::quad::ARTIFACT),
        ));
        assert!(matches!(
            parsed.textures[1][4].structures,
            Some(tex::tiles::quad::ARTIFACT),
        ));
        assert!(matches!(parsed.textures[1][5].terrain, None,));
        assert!(parsed.textures[1][6]
            .structures
            .is_some_and(|texs| texs.iter().any(|t| matches!(
                t,
                &tex::tiles::HOUSE_0
                    | &tex::tiles::HOUSE_1
                    | &tex::tiles::HOUSE_2
                    | &tex::tiles::HOUSE_3
            ))));
        assert!(parsed.textures[1][7]
            .structures
            .is_some_and(|texs| texs.iter().any(|t| matches!(
                t,
                &tex::tiles::HOUSE_0
                    | &tex::tiles::HOUSE_1
                    | &tex::tiles::HOUSE_2
                    | &tex::tiles::HOUSE_3
            ))));
        assert!(matches!(
            parsed.textures[1][8].pieces[1],
            PieceLayer::Character('R', _, true, _),
        ));
        assert!(matches!(
            parsed.textures[1][9].pieces[1],
            PieceLayer::Character('O', _, true, _),
        ));
        assert!(matches!(
            parsed.textures[1][10].pieces[1],
            PieceLayer::Character('W', _, true, _),
        ));
    }
}
