use std::{cmp::Ordering, collections::HashMap};

use eframe::egui;
use gilrs::{Axis, Button, Event, EventType, GamepadId, Gilrs};
use serde::{Deserialize, Serialize};
use truncate_core::{
    board::{Board, Square},
    messages::{AssignedPlayerMessage, PlayerMessage},
    player::Hand,
};

use super::{
    ensure_board_selection, move_selection, GamepadMappings, IdentifiedGamepadMappings,
    InputAction, PlayerInputAction,
};
use crate::utils::depot::TruncateDepot;

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug, Hash)]
pub enum GamepadEvent {
    Button { b: gilrs::Button },
    Axis { a: gilrs::Axis, positive: bool },
}

impl PartialOrd for GamepadEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

impl Ord for GamepadEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (GamepadEvent::Button { b: b1 }, GamepadEvent::Button { b: b2 }) => {
                (*b1 as u16).cmp(&(*b2 as u16))
            }
            (GamepadEvent::Axis { a: a1, .. }, GamepadEvent::Axis { a: a2, .. }) => {
                (*a1 as u16).cmp(&(*a2 as u16))
            }
            (GamepadEvent::Axis { .. }, GamepadEvent::Button { .. }) => Ordering::Greater,
            (GamepadEvent::Button { .. }, GamepadEvent::Axis { .. }) => Ordering::Less,
        }
    }
}

pub struct GamepadManager {
    mgr: Gilrs,
}

impl GamepadManager {
    pub fn new() -> Self {
        Self {
            mgr: Gilrs::new().unwrap(),
        }
    }

    // how to store which player is which gamepad?
    // We need to bake in the two mappings, but also
    // use one of those buttons as the "identification" button
    // which inserts the actual map events into the mappings used

    pub fn get_gp_action(
        &mut self,
        ctx: &egui::Context,
        raw_mappings: &GamepadMappings,
        mappings: &mut IdentifiedGamepadMappings,
    ) -> Vec<PlayerInputAction> {
        let mut msgs = vec![];

        while let Some(event) = self.mgr.next_event() {
            let Event {
                id: gamepad_id,
                event,
                time,
            } = event;

            match event {
                EventType::ButtonPressed(b, _) => {
                    let mut events = GamepadManager::map_button(gamepad_id, &b, true, mappings);

                    if events.is_empty() {
                        let unassigned_select_pressed =
                            raw_mappings.gamepad.iter().enumerate().find(|(_, m)| {
                                m.get(&GamepadEvent::Button { b })
                                    .is_some_and(|a| *a == InputAction::SelectSwap)
                            });
                        if let Some((identified_player, raw_mapping)) = unassigned_select_pressed {
                            // If a select button was hit, we now know which player a gamepad is,
                            // so we can add all of its input as mappings
                            let idenfitied_mapping: HashMap<_, _> = raw_mapping
                                .iter()
                                .map(|(event, action)| ((gamepad_id, event.clone()), *action))
                                .collect();
                            mappings
                                .gamepad
                                .insert(identified_player, idenfitied_mapping);

                            println!("Assigning a set of mappings to player {identified_player}");

                            events = GamepadManager::map_button(gamepad_id, &b, true, mappings)
                        }
                    }

                    msgs.extend(events);
                }
                EventType::ButtonReleased(b, _) => {
                    msgs.extend(GamepadManager::map_button(gamepad_id, &b, false, mappings));
                }
                EventType::AxisChanged(a, v, _) => {
                    msgs.extend(GamepadManager::map_axis(gamepad_id, &a, v, mappings));
                }
                EventType::ButtonRepeated(_, _) => { /* no-op */ }
                EventType::ButtonChanged(_, _, _) => { /* no-op */ }
                EventType::Connected => { /* no-op */ }
                EventType::Disconnected => { /* no-op */ }
                EventType::Dropped => { /* no-op */ }
            }
        }

        msgs
    }

    fn map_button(
        gamepad: GamepadId,
        button: &Button,
        pressed: bool,
        mappings: &IdentifiedGamepadMappings,
    ) -> Vec<PlayerInputAction> {
        mappings
            .gamepad
            .iter()
            .flat_map(|(player, map)| {
                let Some(action) = map.get(&(gamepad, GamepadEvent::Button { b: *button })) else {
                    return None;
                };

                return Some(PlayerInputAction {
                    action: *action,
                    start: pressed,
                    player: *player,
                });
            })
            .collect()
    }

    fn map_axis(
        gamepad: GamepadId,
        axis: &Axis,
        value: f32,
        mappings: &IdentifiedGamepadMappings,
    ) -> Vec<PlayerInputAction> {
        let mut msgs = vec![];

        mappings.gamepad.iter().for_each(|(player, map)| {
            if value.abs() < 0.1 {
                if let Some(action) = map.get(&(
                    gamepad,
                    GamepadEvent::Axis {
                        a: *axis,
                        positive: true,
                    },
                )) {
                    msgs.push(PlayerInputAction {
                        action: *action,
                        start: false,
                        player: *player,
                    });
                };

                if let Some(action) = map.get(&(
                    gamepad,
                    GamepadEvent::Axis {
                        a: *axis,
                        positive: false,
                    },
                )) {
                    msgs.push(PlayerInputAction {
                        action: *action,
                        start: false,
                        player: *player,
                    });
                };
            } else {
                if let Some(action) = map.get(&(
                    gamepad,
                    GamepadEvent::Axis {
                        a: *axis,
                        positive: value > 0.0,
                    },
                )) {
                    msgs.push(PlayerInputAction {
                        action: *action,
                        start: true,
                        player: *player,
                    });
                };
            }
        });

        msgs
    }
}
