use eframe::{
    egui,
    epaint::{Color32, Rect, Stroke, TextShape, Vec2},
};

use std::f32;

use super::GameClient;
use core::{
    board::{Board, Coordinate, Square},
    hand::Hand,
    messages::{GameMessage, GameStateMessage, PlayerMessage},
};

type RoomCode = String;

#[derive(Debug, Clone)]
pub struct ActiveGame {
    room_code: RoomCode,
    player_number: u64,
    next_player_number: u64,
    board: Board,
    hand: Hand,
    selected_tile_in_hand: Option<usize>,
    selected_square_on_board: Option<Coordinate>,
    playing_tile: Option<char>,
    error_msg: Option<String>,
}

impl ActiveGame {
    fn new(
        room_code: RoomCode,
        player_number: u64,
        next_player_number: u64,
        board: Board,
        hand: Hand,
    ) -> Self {
        Self {
            room_code,
            player_number,
            next_player_number,
            board,
            hand,
            selected_tile_in_hand: None,
            selected_square_on_board: None,
            playing_tile: None,
            error_msg: None,
        }
    }
}

#[derive(Debug)]
pub enum GameStatus {
    None(RoomCode),
    PendingJoin(RoomCode),
    PendingCreate,
    PendingStart(RoomCode),
    Active(ActiveGame),
    Concluded(ActiveGame, u64),
}

pub fn render(client: &mut GameClient, ui: &mut egui::Ui) {
    let GameClient {
        name,
        game_status,
        rx_game,
        tx_player,
    } = client;

    if matches!(game_status, GameStatus::None(_)) {
        ui.horizontal(|ui| {
            ui.label("Name: ");
            ui.text_edit_singleline(name);
        });
    } else {
        ui.label(format!("Playing as {name}"));
    }

    // TODO: Option to join an existing game

    let mut new_game_status = None;
    match game_status {
        GameStatus::None(room_code) => {
            if ui.button("New Game").clicked() {
                // TODO: Send player name in NewGame message
                tx_player.send(PlayerMessage::NewGame).unwrap();
                new_game_status = Some(GameStatus::PendingCreate);
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(room_code);
                if ui.button("Join Game").clicked() {
                    tx_player
                        .send(PlayerMessage::JoinGame(room_code.clone()))
                        .unwrap();
                    new_game_status = Some(GameStatus::PendingJoin(room_code.clone()));
                }
            });
        }
        GameStatus::PendingJoin(room_code) => {
            ui.label(format!("Waiting to join room {room_code}"));
        }
        GameStatus::PendingCreate => {
            ui.label("Waiting for a new game to be created . . .");
        }
        GameStatus::PendingStart(game_id) => {
            // TODO: Make this state exist
            ui.label(format!("Playing in game {game_id}"));
            ui.label("Waiting for the game to start . . .");
            if ui.button("Start game").clicked() {
                tx_player.send(PlayerMessage::StartGame).unwrap();
            }
        }
        GameStatus::Active(game) => {
            // TODO: All actual board/game state
            ui.label(format!("Playing in game {}", game.room_code));

            if game.player_number == game.next_player_number {
                ui.label("It is your turn! :)");
            } else {
                ui.label("It is not your turn :(");
            }

            if let Some(error) = &game.error_msg {
                ui.label(error);
            } else {
                ui.label("");
            }

            if let Some(msg) = render_board(game, ui) {
                tx_player.send(msg).unwrap();
            }
            render_hand(game, ui);
        }
        GameStatus::Concluded(game, winner) => {
            ui.label(format!("Game {} has concluded", game.room_code));
            ui.label(format!("Player {winner} won"));
            render_board(game, ui);
            // TODO: Reset state and play again
        }
    }
    if let Some(new_game_status) = new_game_status {
        *game_status = new_game_status;
    }

    while let Ok(msg) = rx_game.try_recv() {
        match msg {
            GameMessage::JoinedGame(id) => {
                *game_status = GameStatus::PendingStart(id.to_uppercase())
            }
            GameMessage::StartedGame(GameStateMessage {
                room_code,
                player_number,
                next_player_number,
                board,
                hand,
            }) => {
                *game_status = GameStatus::Active(ActiveGame::new(
                    room_code.to_uppercase(),
                    player_number,
                    next_player_number,
                    board,
                    hand,
                ));
                println!("Starting a game")
            }
            GameMessage::GameUpdate(GameStateMessage {
                room_code: _,
                player_number: _,
                next_player_number,
                board,
                hand: mut new_hand,
            }) => {
                match game_status {
                    GameStatus::Active(game) => {
                        // assert_eq!(game.room_code, room_code);
                        // assert_eq!(game.player_number, player_number);
                        game.board = board;
                        game.next_player_number = next_player_number;
                        // TODO: Remove all of this logic and return hand updates from the server
                        if let Some(playing) = game.playing_tile {
                            game.hand.remove(
                                game.hand
                                    .iter()
                                    .enumerate()
                                    .find(|(_, t)| **t == playing)
                                    .unwrap()
                                    .0,
                            );
                            for tile in &game.hand {
                                if let Some((i, _)) =
                                    new_hand.iter().enumerate().find(|(_, t)| **t == *tile)
                                {
                                    new_hand.remove(i);
                                }
                            }
                            game.hand.extend(new_hand);
                        }

                        game.playing_tile = None;
                        game.error_msg = None;
                    }
                    _ => todo!("Game update hit an unknown state"),
                }
            }
            GameMessage::GameEnd(
                GameStateMessage {
                    room_code: _,
                    player_number: _,
                    next_player_number: _,
                    board,
                    hand: _,
                },
                winner,
            ) => match game_status {
                GameStatus::Active(game) => {
                    // assert_eq!(game.room_code, id);
                    // assert_eq!(game.player_number, num);
                    game.board = board;
                    *game_status = GameStatus::Concluded(game.clone(), winner);
                }
                _ => todo!("Game error hit an unknown state"),
            },
            GameMessage::GameError(id, num, err) => match game_status {
                GameStatus::Active(game) => {
                    // assert_eq!(game.room_code, id);
                    // assert_eq!(game.player_number, num);
                    game.error_msg = Some(err);
                }
                _ => todo!("Game error hit an unknown state"),
            },
        }
    }
}

fn render_board(game: &mut ActiveGame, ui: &mut egui::Ui) -> Option<PlayerMessage> {
    let mut msg = None;
    let is_flipped = game.player_number == 0;
    ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

    // This is super gross but I'm just powering through
    let mut render = |rows: Box<dyn Iterator<Item = (usize, &Vec<Option<Square>>)>>| {
        let mut render_row = |rownum, row: Box<dyn Iterator<Item = (usize, &Option<Square>)>>| {
            ui.horizontal(|ui| {
                for (colnum, square) in row {
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                    if ui.is_rect_visible(rect) {
                        if square.is_some() && response.hovered() {
                            ui.painter().rect_filled(rect, 0.0, Color32::LIGHT_YELLOW);
                        }
                        if let Some(selected) = game.selected_square_on_board {
                            if selected.eq(&(colnum, rownum)) {
                                ui.painter().rect_filled(rect, 0.0, Color32::KHAKI);
                            }
                        }
                        match square {
                            Some(Square::Empty) => {
                                ui.painter().rect_stroke(
                                    rect,
                                    0.0,
                                    Stroke::new(1.0, Color32::GOLD),
                                );
                            }
                            Some(Square::Occupied(player, char)) => {
                                let is_self = *player as u64 == game.player_number;
                                let color = if is_self {
                                    Color32::LIGHT_GREEN
                                } else {
                                    Color32::RED
                                };
                                ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, color));
                                render_char(char, !is_self, rect, ui, color);
                            }
                            None => {}
                        };
                    }
                    if response.clicked() {
                        let coord = Coordinate::new(colnum, rownum);
                        if let Some(tile) = game.selected_tile_in_hand {
                            msg = Some(PlayerMessage::Place(coord, game.hand[tile]));
                            game.playing_tile = Some(game.hand[tile]);
                            game.selected_tile_in_hand = None;
                        } else if let Some(selected_coord) = game.selected_square_on_board {
                            if selected_coord != coord {
                                msg = Some(PlayerMessage::Swap(coord, selected_coord));
                            }
                            game.selected_square_on_board = None;
                        } else {
                            game.selected_square_on_board = Some(coord);
                        }
                    }
                }
            });
        };

        for (rownum, row) in rows {
            if is_flipped {
                render_row(rownum, Box::new(row.iter().enumerate().rev()));
            } else {
                render_row(rownum, Box::new(row.iter().enumerate()));
            }
        }
    };
    if is_flipped {
        render(Box::new(game.board.squares.iter().enumerate().rev()));
    } else {
        render(Box::new(game.board.squares.iter().enumerate()));
    }
    msg
}

fn render_hand(game: &mut ActiveGame, ui: &mut egui::Ui) {
    let mut rearrange = None;
    ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);
    ui.separator();
    ui.horizontal(|ui| {
        for (i, char) in game.hand.iter().enumerate() {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
            if ui.is_rect_visible(rect) {
                ui.painter()
                    .rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GOLD));
                if response.hovered() {
                    ui.painter().rect_filled(rect, 0.0, Color32::LIGHT_YELLOW);
                }
                if game.selected_tile_in_hand == Some(i) {
                    ui.painter().rect_filled(rect, 0.0, Color32::KHAKI);
                }
                render_char(char, false, rect, ui, Color32::LIGHT_GREEN);
            }
            if response.clicked() {
                if let Some(selected) = game.selected_tile_in_hand {
                    game.selected_tile_in_hand = None;
                    if selected != i {
                        rearrange = Some((selected, i));
                    }
                } else {
                    game.selected_tile_in_hand = Some(i);
                }
                game.selected_square_on_board = None;
            }
        }
    });
    if let Some((from, to)) = rearrange {
        let c = game.hand.remove(from);
        game.hand.insert(to, c);
    }
}

fn render_char(char: &char, inverted: bool, rect: Rect, ui: &mut egui::Ui, color: Color32) {
    let angle = if inverted { f32::consts::PI } else { 0.0 };
    let pos = if inverted {
        rect.right_bottom()
    } else {
        rect.left_top()
    };

    let galley = ui.painter().layout_no_wrap(
        char.to_uppercase().to_string(),
        egui::FontId::new(20.0, egui::FontFamily::Name("Tile".into())),
        color,
    );

    let shift = Vec2::new(
        (rect.width() - galley.size().x) / if inverted { -2.0 } else { 2.0 },
        if inverted { 4.0 } else { -4.0 }, // TODO: Fix magic number for font alignment
    );

    ui.painter().add(TextShape {
        angle,
        override_text_color: Some(color),
        ..TextShape::new(pos + shift, galley)
    });
}
