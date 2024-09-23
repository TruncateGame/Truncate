use std::{
    collections::{BTreeMap, HashMap},
    f32::consts::PI,
};

use eframe::egui::Key;
use gilrs::GamepadId;
use serde::{Deserialize, Serialize};
use time::Duration;
use truncate_core::{
    board::{Board, Coordinate, Square},
    messages::{AssignedPlayerMessage, PlayerMessage},
    player::Hand,
};

use super::depot::TruncateDepot;

const KEYBOARD_MAPPINGS_FILE: &'static str = "truncate_kb_mappings.json";
const GAMEPAD_MAPPINGS_FILE: &'static str = "truncate_gp_mappings.json";

const INITIAL_MOVE_DELAY: Duration = Duration::milliseconds(300);
const MOVE_REPEAT_DELAY: Duration = Duration::milliseconds(50);

#[cfg(not(target_arch = "wasm32"))]
pub mod gamepad;
pub mod keyboard;

type Movement = [isize; 2];
type MovementMap = HashMap<(Movement, usize), MovingDirection>;

struct MovingDirection {
    started_move: Duration,
    ticked_moves: usize,
}

pub struct Switchboard {
    moving: MovementMap,
    keyboard_mappings: KeyboardMappings,
    #[cfg(not(target_arch = "wasm32"))]
    gamepad_mappings: GamepadMappings,
    #[cfg(not(target_arch = "wasm32"))]
    identified_gamepad_mappings: IdentifiedGamepadMappings,
    #[cfg(not(target_arch = "wasm32"))]
    gamepad: gamepad::GamepadManager,
}

impl Switchboard {
    pub fn load() -> Self {
        Self {
            moving: HashMap::new(),
            keyboard_mappings: load_from_disk(KEYBOARD_MAPPINGS_FILE).unwrap_or_default(),
            #[cfg(not(target_arch = "wasm32"))]
            gamepad_mappings: load_from_disk(GAMEPAD_MAPPINGS_FILE).unwrap_or_default(),
            #[cfg(not(target_arch = "wasm32"))]
            identified_gamepad_mappings: IdentifiedGamepadMappings::default(),
            #[cfg(not(target_arch = "wasm32"))]
            gamepad: gamepad::GamepadManager::new(),
        }
    }

    pub fn store(&self) {
        save_to_disk(&self.keyboard_mappings, KEYBOARD_MAPPINGS_FILE);

        #[cfg(not(target_arch = "wasm32"))]
        save_to_disk(&self.gamepad_mappings, GAMEPAD_MAPPINGS_FILE);
    }

    pub fn operate(
        &mut self,
        ctx: &eframe::egui::Context,
        board: &Board,
        hands: &Vec<Hand>,
        depot: &mut TruncateDepot,
    ) -> Vec<AssignedPlayerMessage> {
        let mut msgs = vec![];

        let actions = keyboard::get_kb_action(ctx, &self.keyboard_mappings);

        msgs.extend(
            actions
                .into_iter()
                .flat_map(|a| self.handle_action(a, ctx, board, hands, depot)),
        );

        #[cfg(not(target_arch = "wasm32"))]
        {
            let actions = self.gamepad.get_gp_action(
                ctx,
                &self.gamepad_mappings,
                &mut self.identified_gamepad_mappings,
            );

            msgs.extend(
                actions
                    .into_iter()
                    .flat_map(|a| self.handle_action(a, ctx, board, hands, depot)),
            );
        }

        self.handle_repeat(board, depot);

        if !self.moving.is_empty() {
            ctx.request_repaint();
        }

        msgs
    }

    fn handle_action(
        &mut self,
        action: PlayerInputAction,
        ctx: &eframe::egui::Context,
        board: &Board,
        hands: &Vec<Hand>,
        depot: &mut TruncateDepot,
    ) -> Vec<AssignedPlayerMessage> {
        let PlayerInputAction {
            action,
            start,
            player,
        } = action;
        let mut msgs = vec![];

        if depot.ui_state.dictionary_open {
            if !matches!(action, InputAction::Dictionary | InputAction::Escape) {
                return msgs;
            }
        }

        let moving = |other_movements: &MovementMap, depot: &TruncateDepot| {
            let now = depot.timing.current_time;
            let earliest_start = other_movements
                .values()
                .min_by_key(|m| m.started_move)
                .map(|m| m.started_move)
                .unwrap_or(now);

            // If we are already fast-moving, we don't want an additional
            // axis to have to wait for its delay. e.g. while moving up
            // if you start a rightward input we go straight to fast-moving right.
            let remapped_start = if now - earliest_start > INITIAL_MOVE_DELAY {
                now - INITIAL_MOVE_DELAY
            } else {
                earliest_start
            };

            MovingDirection {
                started_move: remapped_start,
                ticked_moves: 0,
            }
        };

        match (action, start) {
            (InputAction::MoveUp, true) => {
                move_selection(depot, player, [0, -1], board);
                self.moving
                    .insert(([0, -1], player), moving(&self.moving, depot));
                ctx.request_repaint();
            }
            (InputAction::MoveUp, false) => {
                self.moving.remove(&([0, -1], player));
            }
            (InputAction::MoveRight, true) => {
                move_selection(depot, player, [1, 0], board);
                self.moving
                    .insert(([1, 0], player), moving(&self.moving, depot));
                ctx.request_repaint();
            }
            (InputAction::MoveRight, false) => {
                self.moving.remove(&([1, 0], player));
            }
            (InputAction::MoveDown, true) => {
                move_selection(depot, player, [0, 1], board);
                self.moving
                    .insert(([0, 1], player), moving(&self.moving, depot));
                ctx.request_repaint();
            }
            (InputAction::MoveDown, false) => {
                self.moving.remove(&([0, 1], player));
            }
            (InputAction::MoveLeft, true) => {
                move_selection(depot, player, [-1, 0], board);
                self.moving
                    .insert(([-1, 0], player), moving(&self.moving, depot));
                ctx.request_repaint();
            }
            (InputAction::MoveLeft, false) => {
                self.moving.remove(&([-1, 0], player));
            }
            (InputAction::SelectSwap, true) => {
                let current_selection = ensure_board_selection(depot, player, board);
                if matches!(board.get(current_selection), Ok(Square::Occupied { .. })) {
                    if let Some((already_selected_tile, _)) =
                        depot.interactions[player].selected_tile_on_board
                    {
                        if already_selected_tile == current_selection {
                            depot.interactions[player].selected_tile_on_board = None;
                        } else {
                            msgs.push(AssignedPlayerMessage {
                                message: PlayerMessage::Swap(
                                    already_selected_tile,
                                    current_selection,
                                ),
                                player_id: Some(player as _),
                            });
                            depot.interactions[player].selected_tile_on_board = None;
                        }
                    } else {
                        depot.interactions[player].selected_tile_on_board =
                            Some((current_selection, board.get(current_selection).unwrap()));
                    }
                } else {
                    depot.interactions[player].selected_tile_on_board = None;
                }
            }
            (InputAction::Dictionary, true) => {
                if !depot.ui_state.dictionary_open {
                    depot.ui_state.dictionary_open = true;
                    depot.ui_state.dictionary_opened_by_keyboard = true;
                } else if depot.ui_state.dictionary_opened_by_keyboard {
                    depot.ui_state.dictionary_open = false;
                    depot.ui_state.dictionary_opened_by_keyboard = false;
                }
            }
            (InputAction::Escape, true) => {
                if depot.ui_state.dictionary_open {
                    depot.ui_state.dictionary_open = false;
                    depot.ui_state.dictionary_focused = false;
                }
            }
            (InputAction::Slot1, true) => {
                if let Some(msg) = tile_play_message(player, 0, board, hands, depot) {
                    msgs.push(msg);
                }
            }
            (InputAction::Slot2, true) => {
                if let Some(msg) = tile_play_message(player, 1, board, hands, depot) {
                    msgs.push(msg);
                }
            }
            (InputAction::Slot3, true) => {
                if let Some(msg) = tile_play_message(player, 2, board, hands, depot) {
                    msgs.push(msg);
                }
            }
            (InputAction::Slot4, true) => {
                if let Some(msg) = tile_play_message(player, 3, board, hands, depot) {
                    msgs.push(msg);
                }
            }
            (InputAction::Slot5, true) => {
                if let Some(msg) = tile_play_message(player, 4, board, hands, depot) {
                    msgs.push(msg);
                }
            }
            (InputAction::Slot6, true) => {
                if let Some(msg) = tile_play_message(player, 5, board, hands, depot) {
                    msgs.push(msg);
                }
            }
            (InputAction::Slot7, true) => {
                if let Some(msg) = tile_play_message(player, 6, board, hands, depot) {
                    msgs.push(msg);
                }
            }
            (InputAction::Slot8, true) => {
                if let Some(msg) = tile_play_message(player, 7, board, hands, depot) {
                    msgs.push(msg);
                }
            }
            (InputAction::Slot9, true) => {
                if let Some(msg) = tile_play_message(player, 8, board, hands, depot) {
                    msgs.push(msg);
                }
            }
            (InputAction::Slot10, true) => {
                if let Some(msg) = tile_play_message(player, 9, board, hands, depot) {
                    msgs.push(msg);
                }
            }
            (_, false) => { /* no-op for most buttons on release */ }
        }

        msgs
    }

    fn handle_repeat(&mut self, board: &Board, depot: &mut TruncateDepot) {
        for ((movement, player), direction) in self.moving.iter_mut() {
            let now = depot.timing.current_time;
            let total_dur = now.saturating_sub(direction.started_move);
            if total_dur < INITIAL_MOVE_DELAY {
                continue;
            }

            let target_tick_count =
                ((total_dur - INITIAL_MOVE_DELAY) / MOVE_REPEAT_DELAY) as usize + 1;

            while target_tick_count > direction.ticked_moves {
                move_selection(depot, *player, *movement, board);
                direction.ticked_moves += 1;
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct KeyboardMappings {
    keyboard: Vec<BTreeMap<Key, InputAction>>,
}

impl Default for KeyboardMappings {
    fn default() -> Self {
        Self {
            keyboard: Vec::from([
                BTreeMap::from([
                    (Key::ArrowUp, InputAction::MoveUp),
                    (Key::ArrowRight, InputAction::MoveRight),
                    (Key::ArrowDown, InputAction::MoveDown),
                    (Key::ArrowLeft, InputAction::MoveLeft),
                    (Key::Period, InputAction::SelectSwap),
                    (Key::Slash, InputAction::Dictionary),
                    (Key::Num7, InputAction::Slot1),
                    (Key::Num8, InputAction::Slot2),
                    (Key::Num9, InputAction::Slot3),
                    (Key::Num0, InputAction::Slot4),
                ]),
                BTreeMap::from([
                    (Key::W, InputAction::MoveUp),
                    (Key::D, InputAction::MoveRight),
                    (Key::S, InputAction::MoveDown),
                    (Key::A, InputAction::MoveLeft),
                    (Key::Q, InputAction::SelectSwap),
                    (Key::E, InputAction::Dictionary),
                    (Key::Num1, InputAction::Slot1),
                    (Key::Num2, InputAction::Slot2),
                    (Key::Num3, InputAction::Slot3),
                    (Key::Num4, InputAction::Slot4),
                ]),
            ]),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Serialize, Deserialize, Debug)]
struct GamepadMappings {
    gamepad: Vec<BTreeMap<gamepad::GamepadEvent, InputAction>>,
}

impl Default for GamepadMappings {
    fn default() -> Self {
        use gamepad::GamepadEvent::*;
        Self {
            gamepad: Vec::from([
                BTreeMap::from([
                    (
                        Axis {
                            a: gilrs::Axis::LeftStickX,
                            positive: false,
                        },
                        InputAction::MoveUp,
                    ),
                    (
                        Axis {
                            a: gilrs::Axis::LeftStickY,
                            positive: true,
                        },
                        InputAction::MoveRight,
                    ),
                    (
                        Axis {
                            a: gilrs::Axis::LeftStickX,
                            positive: true,
                        },
                        InputAction::MoveDown,
                    ),
                    (
                        Axis {
                            a: gilrs::Axis::LeftStickY,
                            positive: false,
                        },
                        InputAction::MoveLeft,
                    ),
                    (
                        Button {
                            b: gilrs::Button::RightThumb,
                        },
                        InputAction::SelectSwap,
                    ),
                    (
                        Button {
                            b: gilrs::Button::North,
                        },
                        InputAction::Slot1,
                    ),
                    (
                        Button {
                            b: gilrs::Button::East,
                        },
                        InputAction::Slot2,
                    ),
                    (
                        Button {
                            b: gilrs::Button::South,
                        },
                        InputAction::Slot3,
                    ),
                    (
                        Button {
                            b: gilrs::Button::West,
                        },
                        InputAction::Slot4,
                    ),
                    (
                        Button {
                            b: gilrs::Button::LeftTrigger,
                        },
                        InputAction::Slot5,
                    ),
                ]),
                BTreeMap::from([
                    (
                        Axis {
                            a: gilrs::Axis::LeftStickX,
                            positive: false,
                        },
                        InputAction::MoveUp,
                    ),
                    (
                        Axis {
                            a: gilrs::Axis::LeftStickY,
                            positive: true,
                        },
                        InputAction::MoveRight,
                    ),
                    (
                        Axis {
                            a: gilrs::Axis::LeftStickX,
                            positive: true,
                        },
                        InputAction::MoveDown,
                    ),
                    (
                        Axis {
                            a: gilrs::Axis::LeftStickY,
                            positive: false,
                        },
                        InputAction::MoveLeft,
                    ),
                    (
                        Button {
                            b: gilrs::Button::LeftThumb,
                        },
                        InputAction::SelectSwap,
                    ),
                    (
                        Button {
                            b: gilrs::Button::North,
                        },
                        InputAction::Slot1,
                    ),
                    (
                        Button {
                            b: gilrs::Button::East,
                        },
                        InputAction::Slot2,
                    ),
                    (
                        Button {
                            b: gilrs::Button::South,
                        },
                        InputAction::Slot3,
                    ),
                    (
                        Button {
                            b: gilrs::Button::West,
                        },
                        InputAction::Slot4,
                    ),
                    (
                        Button {
                            b: gilrs::Button::LeftTrigger,
                        },
                        InputAction::Slot5,
                    ),
                ]),
            ]),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Default)]
struct IdentifiedGamepadMappings {
    gamepad: HashMap<usize, HashMap<(GamepadId, gamepad::GamepadEvent), InputAction>>,
}

struct PlayerInputAction {
    action: InputAction,
    start: bool,
    player: usize,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
enum InputAction {
    MoveUp,
    MoveRight,
    MoveDown,
    MoveLeft,
    SelectSwap,
    Dictionary,
    Escape,
    Slot1,
    Slot2,
    Slot3,
    Slot4,
    Slot5,
    Slot6,
    Slot7,
    Slot8,
    Slot9,
    Slot10,
}

fn save_to_disk<M: Serialize>(mappings: &M, filename: &str) {
    let mapping_file = serde_json::to_string(mappings).expect("mappings should be serializable");
    std::fs::write(filename, mapping_file).expect("mappings file should be writeable");
}

fn load_from_disk<M: for<'de> Deserialize<'de>>(filename: &str) -> Option<M> {
    std::fs::read_to_string(filename)
        .ok()
        .map(|f| serde_json::from_str::<M>(&f).expect("if mappings file exists it should be valid"))
}

fn tile_play_message(
    player: usize,
    slot: usize,
    board: &Board,
    hands: &Vec<Hand>,
    depot: &mut TruncateDepot,
) -> Option<AssignedPlayerMessage> {
    let current_selection = ensure_board_selection(depot, player, board);

    hands[player].get(slot).map(|char| AssignedPlayerMessage {
        message: PlayerMessage::Place(current_selection, *char),
        player_id: Some(player as _),
    })
}

fn ensure_board_selection(
    depot: &mut TruncateDepot,
    local_player_index: usize,
    board: &Board,
) -> Coordinate {
    if let Some((coord, _)) = depot.interactions[local_player_index].selected_square_on_board {
        return coord;
    }
    if let Some((coord, sq)) =
        depot.interactions[local_player_index].previous_selected_square_on_board
    {
        depot.interactions[local_player_index].selected_square_on_board = Some((coord.clone(), sq));
        return coord;
    }
    let dock = board.docks.iter().find(|d| {
        board.get(**d).is_ok_and(
            |s| matches!(s, Square::Dock{ player, .. } if player == depot.gameplay.player_numbers[local_player_index] as usize),
        )
    });
    let coord = dock.cloned().unwrap_or_else(|| Coordinate::new(0, 0));
    depot.interactions[local_player_index].selected_square_on_board =
        Some((coord.clone(), board.get(coord).unwrap()));
    coord
}

fn move_selection(
    depot: &mut TruncateDepot,
    local_player_index: usize,
    mut movement: [isize; 2],
    board: &Board,
) {
    // If nothing is selected, the first interaction shouldn't move the cursor.
    // At the start of the game, it should select the dock,
    // and otherwise it should select the previously selected square.
    if depot.interactions[local_player_index]
        .selected_square_on_board
        .is_none()
    {
        ensure_board_selection(depot, local_player_index, board);
        return;
    }

    let current_selection = ensure_board_selection(depot, local_player_index, board);

    if depot.gameplay.player_numbers[local_player_index] == 0 {
        movement[0] *= -1;
        movement[1] *= -1;
    }

    let mut new_x = (current_selection.x as isize) + movement[0];
    let mut new_y = (current_selection.y as isize) + movement[1];

    new_x = new_x.min(board.width() as isize - 1);
    new_y = new_y.min(board.height() as isize - 1);

    new_x = new_x.max(0);
    new_y = new_y.max(0);

    let new_coord = Coordinate {
        x: new_x as usize,
        y: new_y as usize,
    };

    if let Ok(sq) = board.get(new_coord) {
        depot.interactions[local_player_index].selected_square_on_board = Some((new_coord, sq));
        depot.interactions[local_player_index].previous_selected_square_on_board =
            Some((new_coord, sq));
    }
}
