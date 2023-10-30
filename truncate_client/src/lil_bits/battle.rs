use std::sync::Arc;

use eframe::{
    egui::{self, CursorIcon, FontId, Id, Layout, RichText, Sense},
    emath::Align,
};
use epaint::{hex_color, pos2, vec2, Color32, Galley, Pos2, Rect, Stroke, Vec2};
use truncate_core::reporting::{BattleReport, BattleWord};

use crate::{
    regions::active_game::GameCtx,
    utils::{
        glyph_meaure::GlyphMeasure, tex::paint_dialog_background, text::TextHelper, Darken, Lighten,
    },
};

pub struct BattleUI<'a> {
    battle: &'a BattleReport,
    latest: bool,
}

impl<'a> BattleUI<'a> {
    pub fn new(battle: &'a BattleReport, latest: bool) -> Self {
        Self { battle, latest }
    }
}

fn get_galleys<'a>(
    battle_words: &'a Vec<BattleWord>,
    transparent: bool,
    ctx: &GameCtx,
    ui: &mut egui::Ui,
) -> Vec<Arc<Galley>> {
    let dot = ui.painter().layout_no_wrap(
        "• ".into(),
        FontId::new(
            ctx.theme.letter_size * 0.75,
            egui::FontFamily::Name("Truncate-Heavy".into()),
        ),
        ctx.theme.outlines.darken(),
    );

    let mut words: Vec<_> = battle_words
        .iter()
        .flat_map(|w| {
            [
                ui.painter().layout_no_wrap(
                    w.resolved_word.clone(),
                    FontId::new(
                        ctx.theme.letter_size * 0.75,
                        egui::FontFamily::Name("Truncate-Heavy".into()),
                    ),
                    match (transparent, w.valid) {
                        (true, _) => Color32::TRANSPARENT,
                        (false, Some(true)) => ctx.theme.addition.darken(),
                        (false, Some(false)) => ctx.theme.defeated.darken(),
                        (false, None) => ctx.theme.outlines.darken().darken(),
                    },
                ),
                dot.clone(),
            ]
        })
        .collect();
    words.pop();
    words
}

fn paint_galleys<'a>(
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

                ui.painter().galley(word_pt, galley);

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

    ui.allocate_rect(battle_rect, Sense::click())
}

#[derive(Clone, Copy, PartialEq)]
struct BattleUIState {
    hovered_last_frame: bool,
    size_last_frame: Vec2,
    /// If None, we defer to showing when it is the latest battle
    show_details: Option<bool>,
}

impl<'a> BattleUI<'a> {
    pub fn render(self, ctx: &GameCtx, ui: &mut egui::Ui) {
        let battle_id = Id::new("battle").with(self.battle.battle_number.unwrap_or_default());
        let prev_battle_storage: Option<BattleUIState> =
            ui.memory_mut(|m| m.data.get_temp(battle_id));

        // Paint the background dialog based on the size of the battle last frame
        if let Some(BattleUIState {
            hovered_last_frame,
            size_last_frame,
            mut show_details,
        }) = prev_battle_storage
        {
            let (dialog_rect, dialog_resp) = paint_dialog_background(
                true,
                false,
                false,
                size_last_frame,
                if hovered_last_frame {
                    Color32::WHITE
                } else {
                    ctx.theme.water.lighten()
                },
                &ctx.map_texture,
                ui,
            );
            let offset = (dialog_rect.height() - size_last_frame.y) / 2.0;
            let mut dialog_ui = ui.child_ui(dialog_rect, Layout::top_down(Align::Min));
            dialog_ui.add_space(offset);

            let battle_rect = self.render_innards(
                ui.rect_contains_pointer(dialog_rect),
                show_details.unwrap_or(self.latest),
                prev_battle_storage,
                ctx,
                &mut dialog_ui,
            );

            let resp = ui.interact(
                dialog_rect,
                ui.auto_id_with("battle_interact"),
                Sense::click(),
            );

            if resp.hovered() {
                ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
            }
            if resp.clicked() {
                show_details = Some(!(show_details.unwrap_or(self.latest)));
            }

            // Save the sizing of our box for the next render pass to draw the background
            let new_state = BattleUIState {
                hovered_last_frame: resp.hovered(),
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
            let battle_rect = self.render_innards(false, false, prev_battle_storage, ctx, ui);
            // Save the sizing of our box for the next render pass to draw the background
            ui.memory_mut(|m| {
                m.data.insert_temp(
                    battle_id,
                    BattleUIState {
                        hovered_last_frame: false,
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
        hovered: bool,
        active: bool,
        prev_battle_storage: Option<BattleUIState>,
        ctx: &GameCtx,
        ui: &mut egui::Ui,
    ) -> Rect {
        let mut theme = ctx.theme.rescale(0.5);
        theme.tile_margin = 0.0;
        let render_transparent = prev_battle_storage.is_none();

        let mut battle_rect = Rect::NOTHING;

        ui.allocate_ui_with_layout(
            vec2(ui.available_size_before_wrap().x, 0.0),
            Layout::left_to_right(Align::Center).with_main_wrap(true),
            |ui| {
                let words = get_galleys(&self.battle.attackers, render_transparent, ctx, ui);
                battle_rect = battle_rect.union(paint_galleys(words, ui, false).rect);
            },
        );
        ui.add_space(5.0);

        let (msg, border_color) = match self.battle.outcome {
            truncate_core::judge::Outcome::AttackerWins(_) => {
                ("won an attack against", ctx.theme.addition.darken())
            }
            truncate_core::judge::Outcome::DefenderWins => {
                ("failed an attack against", ctx.theme.defeated.darken())
            }
        };
        let galley = ui.painter().layout_no_wrap(
            msg.to_string(),
            FontId::new(
                ctx.theme.letter_size * 0.3,
                egui::FontFamily::Name("Truncate-Heavy".into()),
            ),
            if render_transparent {
                Color32::TRANSPARENT
            } else {
                ctx.theme.text
            },
        );
        battle_rect = battle_rect.union(paint_galleys(vec![galley], ui, false).rect);
        ui.add_space(5.0);

        ui.allocate_ui_with_layout(
            vec2(ui.available_size_before_wrap().x, 0.0),
            Layout::left_to_right(Align::Center).with_main_wrap(true),
            |ui| {
                let words = get_galleys(&self.battle.defenders, render_transparent, ctx, ui);
                battle_rect = battle_rect.union(paint_galleys(words, ui, false).rect);
            },
        );

        if !active {
            return battle_rect;
        }

        ui.add_space(8.0);

        let definition_space = ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                for word in self
                    .battle
                    .attackers
                    .iter()
                    .chain(self.battle.defenders.iter())
                {
                    ui.add_space(12.0);
                    TextHelper::heavy(&word.original_word, ctx.theme.letter_size * 0.5, None, ui)
                        .paint(ctx.theme.text, ui, false);

                    match (word.valid, &word.meanings) {
                        (Some(true), Some(meanings)) if !meanings.is_empty() => TextHelper::light(
                            &format!("{}: {}", meanings[0].pos, meanings[0].defs[0]),
                            24.0,
                            Some(ui.available_width()),
                            ui,
                        )
                        .paint(ctx.theme.text, ui, false),
                        (Some(true), _) => TextHelper::light(
                            "Definition unknown",
                            24.0,
                            Some(ui.available_width()),
                            ui,
                        )
                        .paint(ctx.theme.text, ui, false),
                        (Some(false), _) => {
                            TextHelper::light("Invalid word", 24.0, Some(ui.available_width()), ui)
                                .paint(ctx.theme.text, ui, false)
                        }
                        (None, _) => {
                            TextHelper::light("Unchecked", 24.0, Some(ui.available_width()), ui)
                                .paint(ctx.theme.text, ui, false)
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
