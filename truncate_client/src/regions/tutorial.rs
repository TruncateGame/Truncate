use std::collections::HashMap;

use eframe::egui;
use epaint::{vec2, TextureHandle};
use serde::Deserialize;
use time::OffsetDateTime;
use truncate_core::{
    bag::TileBag,
    board::{Board, Coordinate},
    game::Game,
    judge::Judge,
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage},
    moves::Move,
    player::{Hand, Player},
    rules::{GameRules, TileDistribution},
};

use crate::theming::Theme;

use super::active_game::ActiveGame;

const TUTORIAL_01: &[u8] = include_bytes!("../../tutorials/tutorial_01.yml");

#[derive(Deserialize, Debug)]
struct Tutorial {
    name: String,
    board: String,
    player_hand: String,
    computer_hand: String,
    dict: HashMap<String, String>,
    steps: Vec<TutorialStep>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum TutorialStep {
    OwnMove {
        you: String,
        gets: char,
        description: String,
    },
    ComputerMove {
        computer: String,
        gets: char,
        description: String,
    },
    Dialog {
        message: String,
    },
}

fn pos_to_coord(pos: &str) -> Option<Coordinate> {
    let (x, y) = pos.split_once(',')?;

    let x = x.parse::<usize>().ok()?;
    let y = y.parse::<usize>().ok()?;

    Some(Coordinate { x, y })
}

fn action_to_move(player: usize, action: &str) -> Move {
    let (from, to) = action
        .split_once(" -> ")
        .expect("Actions should be separated by ' -> '");
    let to_pos = pos_to_coord(to).expect("Coordinates should be separated by ','");
    if let Some(from_pos) = pos_to_coord(from) {
        Move::Swap {
            player,
            positions: [from_pos, to_pos],
        }
    } else if from.len() == 1 {
        Move::Place {
            player,
            tile: from.chars().next().unwrap(),
            position: to_pos,
        }
    } else {
        panic!("Couldn't parse tutorial action");
    }
}

impl PartialEq<Move> for TutorialStep {
    fn eq(&self, msg: &Move) -> bool {
        match self {
            TutorialStep::OwnMove { you, .. } => {
                return &action_to_move(0, you) == msg;
            }
            TutorialStep::ComputerMove { .. } => false,
            TutorialStep::Dialog { .. } => false,
        }
    }
}

pub struct TutorialState {
    game: Game,
    active_game: ActiveGame,
    stage: usize,
    tutorial: Tutorial,
}

impl TutorialState {
    pub fn new(map_texture: TextureHandle, theme: Theme) -> Self {
        let loaded_tutorial: Tutorial =
            serde_yaml::from_slice(TUTORIAL_01).expect("Tutorial should match Tutorial format");

        let game = Game {
            rules: GameRules::default(),
            players: vec![
                Player {
                    name: "You".into(),
                    index: 0,
                    hand: Hand(loaded_tutorial.player_hand.chars().collect()),
                    hand_capacity: loaded_tutorial.player_hand.len(),
                    allotted_time: None,
                    time_remaining: None,
                    turn_starts_at: Some(OffsetDateTime::now_utc()),
                    swap_count: 0,
                    penalties_incurred: 0,
                    color: (128, 128, 255),
                },
                Player {
                    name: "Computer".into(),
                    index: 1,
                    hand: Hand(loaded_tutorial.computer_hand.chars().collect()),
                    hand_capacity: loaded_tutorial.computer_hand.len(),
                    allotted_time: None,
                    time_remaining: None,
                    turn_starts_at: None,
                    swap_count: 0,
                    penalties_incurred: 0,
                    color: (255, 80, 80),
                },
            ],
            board: Board::from_string(loaded_tutorial.board.clone()),
            // TODO: Use some special infinite bag?
            bag: TileBag::new(&TileDistribution::Standard),
            judge: Judge::new(loaded_tutorial.dict.keys().cloned().collect()),
            recent_changes: vec![],
            started_at: None,
            next_player: 0,
            winner: None,
        };

        let active_game = ActiveGame::new(
            "TUTORIAL_01".into(),
            game.players.iter().map(Into::into).collect(),
            0,
            0,
            game.board.clone(),
            game.players[0].hand.clone(),
            map_texture,
            theme,
        );

        Self {
            game,
            active_game,
            stage: 0,
            tutorial: loaded_tutorial,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        let current_step = self.tutorial.steps.get(self.stage);
        let mut next_move = None;

        // Start the game after any leading dialogs
        if self.game.started_at.is_none()
            && matches!(
                current_step,
                Some(TutorialStep::OwnMove {
                    you: _,
                    gets: _,
                    description: _
                })
            )
        {
            self.game.start();
        }

        let mut dialog_pos = ui.available_rect_before_wrap();
        let max_width = f32::min(300.0, dialog_pos.width() * 0.8);
        let top_left = dialog_pos.right_bottom() - vec2(max_width, 200.0);
        dialog_pos.set_left(top_left.x);
        dialog_pos.set_top(top_left.y);

        // Render the dialog / helper
        egui::Window::new("Tutorial")
            .title_bar(false)
            .vscroll(true)
            .resizable(false)
            .fixed_size([dialog_pos.width(), dialog_pos.height()])
            .default_rect(dialog_pos)
            .show(ui.ctx(), |ui| {
                match current_step {
                    Some(step) => match step {
                        TutorialStep::OwnMove { description, .. } => {
                            ui.label(description);
                        }
                        TutorialStep::ComputerMove {
                            computer: action,
                            description,
                            ..
                        } => {
                            ui.label(description);
                            if ui.button("Next").clicked() {
                                next_move = Some(action_to_move(1, action));
                            }
                        }
                        TutorialStep::Dialog { message } => {
                            ui.label(message);
                            if ui.button("Next").clicked() {
                                self.stage += 1;
                            }
                        }
                    },
                    None => {
                        ui.label("Tutorial complete!");
                    }
                };
            });

        // Standard game helper
        if let Some(msg) = self.active_game.render(ui, theme, None) {
            let Some(game_move) = (match msg {
                PlayerMessage::Place(position, tile) => Some(Move::Place {
                    player: 0,
                    tile,
                    position,
                }),
                PlayerMessage::Swap(from, to) => Some(Move::Swap {
                    player: 0,
                    positions: [from, to],
                }),
                _ => None,
            }) else {
                return;
            };

            let Some(step) = current_step else { return };

            if step == &game_move {
                next_move = Some(game_move);
            } else {
                // TODO: Handle player doing the wrong tutorial thing
                println!("Expected {msg} to be {:?}", step);
            }
        }

        if let Some(game_move) = next_move {
            if let Some(next_tile) = match current_step {
                Some(TutorialStep::OwnMove { gets, .. }) => Some(gets),
                Some(TutorialStep::ComputerMove { gets, .. }) => Some(gets),
                _ => None,
            } {
                self.game.bag = TileBag::explicit(vec![*next_tile]);
            }

            match self.game.make_move(game_move, None) {
                Ok(changes) => {
                    let changes = changes
                        .into_iter()
                        .filter(|change| match change {
                            truncate_core::reporting::Change::Board(_) => true,
                            truncate_core::reporting::Change::Hand(hand_change) => {
                                hand_change.player == 0
                            }
                            truncate_core::reporting::Change::Battle(_) => true,
                            truncate_core::reporting::Change::Time(_) => true,
                        })
                        .collect();
                    let ctx = &self.active_game.ctx;
                    let state_message = GameStateMessage {
                        room_code: ctx.room_code.clone(),
                        players: self.game.players.iter().map(Into::into).collect(),
                        player_number: 0,
                        next_player_number: self.game.next_player as u64,
                        board: self.game.board.clone(),
                        hand: self.game.players[0].hand.clone(),
                        changes,
                    };
                    self.active_game.apply_new_state(state_message);
                    self.stage += 1;
                }
                Err(msg) => {
                    println!("Failed to make a move: {msg}");
                }
            }
        }
    }
}
