use eframe::egui;
use gilrs::{Button, Event, EventType, Gilrs};
use truncate_core::{board::Board, messages::PlayerMessage, player::Hand};

use super::{ensure_board_selection, move_selection};
use crate::utils::depot::TruncateDepot;

pub struct GamepadInput {
    mgr: Gilrs,
}

impl GamepadInput {
    pub fn new() -> Self {
        Self {
            mgr: Gilrs::new().unwrap(),
        }
    }

    pub fn handle_input(
        &mut self,
        ctx: &egui::Context,
        board: &Board,
        hand: &Hand,
        depot: &mut TruncateDepot,
    ) -> Option<PlayerMessage> {
        let mut msg = None;

        while let Some(event) = self.mgr.next_event() {
            match event.event {
                // Tile slot 0
                EventType::ButtonPressed(Button::Start, _) => {
                    let current_selection = ensure_board_selection(depot, board);

                    if let Some(char) = hand.get(0) {
                        msg = Some(PlayerMessage::Place(current_selection, *char))
                    }
                }
                // Tile slot 1
                EventType::ButtonPressed(Button::LeftTrigger2, _) => {
                    let current_selection = ensure_board_selection(depot, board);

                    if let Some(char) = hand.get(1) {
                        msg = Some(PlayerMessage::Place(current_selection, *char))
                    }
                }
                // Tile slot 2
                EventType::ButtonPressed(Button::RightTrigger, _) => {
                    let current_selection = ensure_board_selection(depot, board);

                    if let Some(char) = hand.get(2) {
                        msg = Some(PlayerMessage::Place(current_selection, *char))
                    }
                }
                // Tile slot 3
                EventType::ButtonPressed(Button::South, _) => {
                    let current_selection = ensure_board_selection(depot, board);

                    if let Some(char) = hand.get(3) {
                        msg = Some(PlayerMessage::Place(current_selection, *char))
                    }
                }
                EventType::AxisChanged(dir, amt, _) => {
                    match dir {
                        gilrs::Axis::LeftStickX if amt < 0.0 => {
                            move_selection(depot, [-1, 0], board);
                            ctx.request_repaint();
                        }
                        gilrs::Axis::LeftStickX if amt > 0.0 => {
                            move_selection(depot, [1, 0], board);
                            ctx.request_repaint();
                        }
                        gilrs::Axis::LeftStickY if amt < 0.0 => {
                            move_selection(depot, [0, 1], board);
                            ctx.request_repaint();
                        }
                        gilrs::Axis::LeftStickY if amt > 0.0 => {
                            move_selection(depot, [0, -1], board);
                            ctx.request_repaint();
                        }
                        _ => { /* ignored */ }
                    }
                }
                _ => {}
            }
            println!("{event:#?}");
        }

        msg
    }
}
