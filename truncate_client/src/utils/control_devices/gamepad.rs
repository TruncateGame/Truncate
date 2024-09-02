use eframe::egui;
use gilrs::{Button, Event, EventType, Gilrs};
use truncate_core::{
    board::Board,
    messages::{AssignedPlayerMessage, PlayerMessage},
    player::Hand,
};

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
        hands: &Vec<Hand>,
        depot: &mut TruncateDepot,
    ) -> Option<AssignedPlayerMessage> {
        let mut msg = None;

        while let Some(event) = self.mgr.next_event() {
            let player: usize = event.id.into();
            let m = |message: PlayerMessage| {
                Some(AssignedPlayerMessage {
                    message,
                    player_id: Some(player as u64),
                })
            };
            match event.event {
                // Tile slot 0
                EventType::ButtonPressed(Button::Start, _) => {
                    let current_selection = ensure_board_selection(depot, player, board);

                    if let Some(char) = hands[player].get(0) {
                        msg = m(PlayerMessage::Place(current_selection, *char))
                    }
                }
                // Tile slot 1
                EventType::ButtonPressed(Button::LeftTrigger2, _) => {
                    let current_selection = ensure_board_selection(depot, player, board);

                    if let Some(char) = hands[player].get(1) {
                        msg = m(PlayerMessage::Place(current_selection, *char))
                    }
                }
                // Tile slot 2
                EventType::ButtonPressed(Button::RightTrigger, _) => {
                    let current_selection = ensure_board_selection(depot, player, board);

                    if let Some(char) = hands[player].get(2) {
                        msg = m(PlayerMessage::Place(current_selection, *char))
                    }
                }
                // Tile slot 3
                EventType::ButtonPressed(Button::South, _) => {
                    let current_selection = ensure_board_selection(depot, player, board);

                    if let Some(char) = hands[player].get(3) {
                        msg = m(PlayerMessage::Place(current_selection, *char))
                    }
                }
                EventType::AxisChanged(dir, amt, _) => {
                    match dir {
                        gilrs::Axis::LeftStickX if amt < 0.0 => {
                            move_selection(depot, player, [-1, 0], board);
                            ctx.request_repaint();
                        }
                        gilrs::Axis::LeftStickX if amt > 0.0 => {
                            move_selection(depot, player, [1, 0], board);
                            ctx.request_repaint();
                        }
                        gilrs::Axis::LeftStickY if amt < 0.0 => {
                            move_selection(depot, player, [0, 1], board);
                            ctx.request_repaint();
                        }
                        gilrs::Axis::LeftStickY if amt > 0.0 => {
                            move_selection(depot, player, [0, -1], board);
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
