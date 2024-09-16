use eframe::egui::{self, Key, Modifiers};
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::{AssignedPlayerMessage, PlayerMessage},
    player::Hand,
};

use crate::utils::depot::TruncateDepot;

use super::{
    ensure_board_selection, move_selection, InputAction, KeyboardMappings, PlayerInputAction,
};

pub fn get_kb_action(ctx: &egui::Context, mappings: &KeyboardMappings) -> Vec<PlayerInputAction> {
    let mut msgs = vec![];

    ctx.input_mut(|input| {
        for event in &input.raw.events {
            match event {
                egui::Event::Key {
                    key,
                    physical_key,
                    pressed,
                    repeat,
                    modifiers,
                } => {
                    if *repeat {
                        continue;
                    }
                    msgs.extend(map_key(key, pressed, mappings));
                }
                _ => {}
            }
        }
    });

    msgs
}

fn map_key(key: &Key, pressed: &bool, mappings: &KeyboardMappings) -> Vec<PlayerInputAction> {
    mappings
        .keyboard
        .iter()
        .enumerate()
        .flat_map(|(player, map)| {
            let Some(action) = map.get(key) else {
                return None;
            };

            return Some(PlayerInputAction {
                action: *action,
                start: *pressed,
                player,
            });
        })
        .collect()
}
