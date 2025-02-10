use std::fmt::Alignment;

use eframe::egui::{self, Align2, Layout, Sense};
use epaint::{hex_color, vec2, Color32, Pos2, Rect, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    board::Direction,
    game::{Game, GAME_COLORS},
    moves::Move,
    reporting::{BoardChange, BoardChangeAction, BoardChangeDetail, Change},
};

use crate::{
    app_outer::{Backchannel, EventDispatcher, GLYPHER},
    utils::{
        depot::{AestheticDepot, GameplayDepot, TimingDepot},
        game_evals::get_main_dict,
        includes::RuleCard,
        mapper::{ImageMusher, MappedBoard, MappedTileVariant},
        tex::{
            self, render_texs_clockwise, BGTexType, PieceLayer, Tex, TexLayers, TexQuad,
            TileDecoration, Tint,
        },
        text::TextHelper,
        timing::get_qs_tick,
        urls::back_to_menu,
        Diaphanize, Lighten, Theme,
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
    ended_animation_at: Option<usize>,
    seen: bool,
    description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRuleCardExample {
    textures: Vec<Vec<TexLayers>>,
}

fn parse_rule_card(rules: RuleCard, theme: &Theme) -> ParsedRuleCard {
    ParsedRuleCard {
        sections: rules
            .sections
            .into_iter()
            .map(|section| ParsedRuleCardSection {
                title: section.title,
                examples: section
                    .examples
                    .into_iter()
                    .map(|r| parse_rule_example(r, theme))
                    .collect(),
                started_animation_at: None,
                ended_animation_at: None,
                seen: false,
                description: section.description,
            })
            .collect(),
    }
}

#[derive(Clone)]
pub struct RulesState {
    map_texture: TextureHandle,
    aesthetics: AestheticDepot,
    rules: ParsedRuleCard,
    active_rule: usize,
    finished: bool,
    event_dispatcher: EventDispatcher,
}

impl RulesState {
    pub fn new(
        ctx: &egui::Context,
        map_texture: TextureHandle,
        theme: Theme,
        rules: RuleCard,
        mut event_dispatcher: EventDispatcher,
    ) -> Self {
        let aesthetics = AestheticDepot {
            theme,
            qs_tick: 0,
            map_texture: map_texture.clone(),
            player_colors: vec![Color32::from_rgb(255, 0, 0), Color32::from_rgb(0, 255, 0)],
            destruction_tick: 0.05,
            destruction_duration: 0.6,
        };

        event_dispatcher.event(format!("tutorial_core_rules"));

        let parsed_rules = parse_rule_card(rules, &aesthetics.theme);

        Self {
            map_texture,
            aesthetics,
            rules: parsed_rules,
            active_rule: 0,
            finished: false,
            event_dispatcher,
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
        let screen_height = ui.available_height();
        let screen_width = ui.available_width();
        let rulebox = (screen_height / 2.0).max(310.0);
        let text_padding = 20.0;

        egui::ScrollArea::new([false, true]).show(ui, |ui| {
            ui.expand_to_include_x(ui.available_rect_before_wrap().right());

            let scroll_pos = ui.next_widget_position().y * -1.0;
            let ideal_active_rule = ((scroll_pos + (rulebox / 2.0)) / rulebox).max(0.0) as usize;

            if self.active_rule != ideal_active_rule {
                let backlash = (scroll_pos + (rulebox / 2.0)) % rulebox;

                if backlash > 50.0 && backlash < (rulebox - 50.0) {
                    self.active_rule = ideal_active_rule;
                }
            }

            let (top_rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), rulebox / 2.0), Sense::hover());

            let t = TextHelper::heavy_centered(
                "Truncate, abridged.",
                heading_size,
                Some((ui.available_width() - 16.0).max(0.0)),
                ui,
            );
            t.paint_within(top_rect, Align2::CENTER_CENTER, theme.text, ui);

            for (rulenum, rule) in self.rules.sections.iter_mut().enumerate() {
                let is_active = self.active_rule == rulenum;
                if is_active {
                    if !rule.seen {
                        self.event_dispatcher
                            .event(format!("tutorial_core_rules_seen_{}", rulenum));
                        rule.seen = true;
                    }
                }
                let texture_gamma = if is_active { 1.0 } else { 0.2 };
                let animated_gamma =
                    ui.ctx()
                        .animate_value_with_time(ui.id().with(rulenum), texture_gamma, 0.3);
                if animated_gamma != texture_gamma {
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
                }

                let text_color = theme.text.gamma_multiply(animated_gamma);

                let (rule_rect, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), rulebox), Sense::hover());

                let started_at = rule.started_animation_at.unwrap_or(cur_animation_tick);
                if is_active && rule.started_animation_at.is_none() {
                    rule.started_animation_at = Some(started_at);
                }

                let final_stage = rule.examples.len() - 1;
                let current_stage =
                    (cur_animation_tick.saturating_sub(started_at)).min(final_stage);

                if current_stage == final_stage && rule.ended_animation_at.is_none() {
                    rule.ended_animation_at = Some(cur_animation_tick);
                }

                let ended_ago = rule
                    .ended_animation_at
                    .map(|e| cur_animation_tick.saturating_sub(e))
                    .unwrap_or_default();

                let target_retrigger_gamma = if ended_ago > 6 { 0.0 } else { 1.0 };
                let retrigger_gamma = ui.ctx().animate_value_with_time(
                    ui.id().with(rulenum).with("retrigger"),
                    target_retrigger_gamma,
                    0.3,
                );
                if retrigger_gamma != target_retrigger_gamma {
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
                }
                let blended_gamma = animated_gamma * retrigger_gamma;

                if ended_ago > 9 {
                    rule.started_animation_at = if is_active {
                        Some(cur_animation_tick + 2)
                    } else {
                        None
                    };
                    rule.ended_animation_at = None;
                }

                let cur_example = &rule.examples[current_stage];
                let example_height = (cur_example.textures.len() as f32 * sprite_size);
                let mut example_corner = rule_rect.left_center();
                example_corner.y -= example_height / 2.0;

                for (rownum, row) in cur_example.textures.iter().enumerate() {
                    let row_width = row.len() as f32 * sprite_size;
                    let left_buffer = (ui.available_width() - row_width) / 2.0;
                    let top_buffer = rownum as f32 * sprite_size;

                    for (colnum, slot) in row.iter().enumerate() {
                        let rect = Rect::from_min_size(
                            Pos2::new(
                                example_corner.x + left_buffer + colnum as f32 * sprite_size,
                                example_corner.y + top_buffer,
                            ),
                            Vec2::splat(sprite_size),
                        );

                        if let Some(structures) = slot.structures {
                            let structures = structures
                                .iter()
                                .map(|s| {
                                    s.tint(
                                        s.current_tint()
                                            .unwrap_or(Color32::WHITE)
                                            .gamma_multiply(blended_gamma),
                                    )
                                })
                                .collect();
                            render_texs_clockwise(structures, rect, &self.map_texture, ui);
                        }
                        for piece in &slot.pieces {
                            match piece {
                                PieceLayer::Texture(texs, _) => {
                                    let texs = texs
                                        .iter()
                                        .map(|t| {
                                            t.tint(
                                                t.current_tint()
                                                    .unwrap_or(Color32::WHITE)
                                                    .gamma_multiply(blended_gamma),
                                            )
                                        })
                                        .collect();

                                    render_texs_clockwise(texs, rect, &self.map_texture, ui);
                                }
                                PieceLayer::Character(char, color, is_flipped, y_offset) => {
                                    assert!(false);
                                }
                            }
                        }
                    }
                }

                let t = TextHelper::heavy_centered(
                    &rule.title,
                    heading_size,
                    Some((ui.available_width() - 16.0).max(0.0)),
                    ui,
                );
                // Adding an extra smidge of height for visual centering
                let heading_height = t.mesh_size().y + 5.0;
                let heading_rect = Rect::from_min_size(
                    Pos2::new(
                        example_corner.x,
                        example_corner.y - text_padding - heading_height,
                    ),
                    vec2(screen_width, heading_height),
                );

                t.paint_within(heading_rect, Align2::CENTER_TOP, text_color, ui);

                let description = TextHelper::light_centered(
                    &rule.description,
                    text_size,
                    Some((ui.available_width() - 16.0).max(0.0)),
                    ui,
                );
                let text_height = description.mesh_size().y;
                let text_area = Rect::from_min_size(
                    Pos2::new(
                        example_corner.x,
                        example_corner.y + example_height + text_padding,
                    ),
                    vec2(screen_width, text_height),
                );

                description.paint_within(text_area, Align2::CENTER_TOP, text_color, ui);
            }
            ui.add_space(rulebox * 0.5);

            let is_post_rules = self.active_rule >= self.rules.sections.len();

            if is_post_rules && !self.finished {
                self.event_dispatcher
                    .event(format!("tutorial_core_rules_finished"));
                self.finished = true;
            }

            let end_gamma = if is_post_rules { 1.0 } else { 0.2 };
            let animated_gamma =
                ui.ctx()
                    .animate_value_with_time(ui.id().with("post_rules"), end_gamma, 0.3);
            if animated_gamma != end_gamma {
                ui.ctx().request_repaint_after(Duration::from_millis(16));
            }
            let text_color = theme.text.gamma_multiply(animated_gamma);

            let t = TextHelper::heavy_centered(
                "Next steps",
                heading_size,
                Some((ui.available_width() - 16.0).max(0.0)),
                ui,
            );
            let (heading_rect, _) =
                ui.allocate_at_least(vec2(ui.available_width(), t.mesh_size().y), Sense::hover());
            t.paint_within(heading_rect, Align2::CENTER_CENTER, text_color, ui);
            ui.add_space(text_padding);

            let desc = [
                "That covers the core mechanics of Truncate.",
                "If you want to learn more, you can explore the in-depth Tutorial,",
                "or, you can jump straight into a game and learn as you play!",
            ]
            .join("\n");
            let d = TextHelper::light_centered(
                &desc,
                heading_size,
                Some((ui.available_width() - 16.0).max(0.0)),
                ui,
            );
            let (desc_rect, _) =
                ui.allocate_at_least(vec2(ui.available_width(), d.mesh_size().y), Sense::hover());
            d.paint_within(desc_rect, Align2::CENTER_CENTER, text_color, ui);
            ui.add_space(text_padding * 2.0);

            let back_text = TextHelper::heavy("RETURN TO MENU", 14.0, None, ui);
            if back_text
                .centered_button(theme.water.lighten(), theme.text, &self.map_texture, ui)
                .clicked()
            {
                back_to_menu();
            }
            ui.add_space(text_padding);

            let tut_text = TextHelper::heavy("PLAY TUTORIAL", 14.0, None, ui);
            if tut_text
                .centered_button(theme.water.lighten(), theme.text, &self.map_texture, ui)
                .clicked()
            {
                back_to_menu();
            }

            ui.add_space(rulebox * 0.5);
        });
    }
}

fn parse_rule_example(example: String, theme: &Theme) -> ParsedRuleCardExample {
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
                        let mut text_color = None;
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
                            Some('-') => {
                                variant = MappedTileVariant::Healthy;
                                color = Color32::GRAY.gamma_multiply(0.3);
                                text_color = Some(Color32::GRAY.gamma_multiply(0.6));
                            }
                            Some('^') => highlight = Some(theme.ring_added),
                            Some('<') => highlight = Some(theme.ring_modified),
                            Some(c) => panic!("Unknown modifier {c}"),
                            None => {}
                        }

                        Tex::board_game_tile(
                            variant,
                            tile.to_ascii_uppercase(),
                            orientation,
                            Some(color.lighten()),
                            text_color,
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
        let theme = Theme::day();
        let example =
            "~  w  ~  ~  ~  ~  ~  ~  ~  ~  ~\nW  O*  R^  $0 $1  ~  +0 +1 r^  o*  w".to_string();
        let parsed = parse_rule_example(example, &theme);

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
