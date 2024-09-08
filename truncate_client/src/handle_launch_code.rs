use eframe::egui;

use truncate_core::{
    board::Board,
    game,
    generation::{generate_board, BoardSeed},
    messages::LobbyPlayerMessage,
    npc::scoring::NPCPersonality,
    rules::GameRules,
};

use crate::{
    app_inner::GameStatus,
    regions::{
        active_game::HeaderType, lobby::Lobby, single_player::SinglePlayerState,
        tutorial::TutorialState,
    },
    utils::{self, daily::get_puzzle_day},
};

use super::OuterApplication;
use truncate_core::messages::PlayerMessage;

pub fn handle_launch_code(
    launch_code: &String,
    outer: &mut OuterApplication,
    ui: &mut egui::Ui,
) -> Option<GameStatus> {
    let mut send_to_server = |msg| {
        outer.tx_player.try_send(msg).unwrap();
    };

    match launch_code.as_str() {
        "__REJOIN__" => match &mut outer.game_status {
            GameStatus::None(_, Some(token)) => {
                send_to_server(PlayerMessage::RejoinGame(token.to_string()));
                return Some(GameStatus::PendingJoin("...".into()));
            }
            _ => return Some(GameStatus::HardError(vec!["Could not rejoin".to_string()])),
        },
        "TUTORIAL_RULES" => {
            return Some(GameStatus::Tutorial(TutorialState::new(
                "rules".to_string(),
                utils::includes::rules(),
                ui.ctx(),
                outer.map_texture.clone(),
                &outer.theme,
                outer.event_dispatcher.clone(),
            )));
        }
        "TUTORIAL_EXAMPLE" => {
            return Some(GameStatus::Tutorial(TutorialState::new(
                "example_game".to_string(),
                utils::includes::example_game(),
                ui.ctx(),
                outer.map_texture.clone(),
                &outer.theme,
                outer.event_dispatcher.clone(),
            )));
        }
        "SINGLE_PLAYER" => {
            outer.event_dispatcher.event("single_player_lobby");
            let mut board = Board::new(9, 9);
            board.grow();
            return Some(GameStatus::PendingSinglePlayer(Lobby::new(
                ui.ctx(),
                "Single Player".into(),
                vec![
                    LobbyPlayerMessage {
                        name: "You".into(),
                        index: 0,
                        color: (128, 128, 255),
                    },
                    LobbyPlayerMessage {
                        name: "Computer".into(),
                        index: 1,
                        color: (255, 80, 80),
                    },
                ],
                0,
                board,
                outer.map_texture.clone(),
            )));
        }
        "DAILY_PUZZLE" => {
            let day = get_puzzle_day(game::now());
            if let Some(token) = &outer.logged_in_as {
                send_to_server(PlayerMessage::LoadDailyPuzzle(token.clone(), day));
            }

            return Some(GameStatus::PendingDaily);
        }
        "RANDOM_PUZZLE" => {
            let seed = (game::now().whole_microseconds() % 243985691) as u32;
            let board_seed = BoardSeed::new(seed);
            let board = generate_board(board_seed.clone())
                .expect("Common seeds can be reasonably expected to produce a board")
                .board;
            let header = HeaderType::Summary {
                title: format!("Regular Puzzle"),
                attempt: None,
            };
            let rules_generation = GameRules::latest().0;
            let puzzle_game = SinglePlayerState::new(
                "jet".to_string(),
                ui.ctx(),
                outer.map_texture.clone(),
                outer.theme.clone(),
                board,
                Some(board_seed),
                rules_generation,
                true,
                header,
                NPCPersonality::jet(),
                outer.event_dispatcher.clone(),
            );
            return Some(GameStatus::SinglePlayer(puzzle_game));
        }
        "RANDOM_EASY_PUZZLE" => {
            let seed = (game::now().whole_microseconds() % 243985691) as u32;
            let board_seed = BoardSeed::new(seed);
            let board = generate_board(board_seed.clone())
                .expect("Common seeds can be reasonably expected to produce a board")
                .board;
            let header = HeaderType::Summary {
                title: format!("Easy Puzzle"),
                attempt: None,
            };
            let rules_generation = GameRules::latest().0;
            let puzzle_game = SinglePlayerState::new(
                "mellite".to_string(),
                ui.ctx(),
                outer.map_texture.clone(),
                outer.theme.clone(),
                board,
                Some(board_seed),
                rules_generation,
                true,
                header,
                NPCPersonality::mellite(),
                outer.event_dispatcher.clone(),
            );
            return Some(GameStatus::SinglePlayer(puzzle_game));
        }
        "DEBUG_BEHEMOTH" => {
            let behemoth_board = Board::from_string(include_str!("../tutorials/test_board.txt"));
            let seed_for_hand_tiles = BoardSeed::new_with_generation(0, 1);
            let rules_generation = GameRules::latest().0;
            let behemoth_game = SinglePlayerState::new(
                "behemoth".to_string(),
                ui.ctx(),
                outer.map_texture.clone(),
                outer.theme.clone(),
                behemoth_board,
                Some(seed_for_hand_tiles),
                rules_generation,
                true,
                HeaderType::Timers,
                NPCPersonality::jet(),
                outer.event_dispatcher.clone(),
            );
            outer.log_frames = true;
            return Some(GameStatus::SinglePlayer(behemoth_game));
        }
        _ => {}
    };

    if launch_code.starts_with("PUZZLE:") {
        let url_segments = launch_code.chars().filter(|c| *c == ':').count();
        let has_board_generation = url_segments >= 3;
        let has_npc_id = url_segments >= 4;
        let has_rules_generation = url_segments >= 5;

        let mut parts = launch_code.split(':').skip(1);
        let board_generation = if has_board_generation {
            parts.next().map(str::parse::<u32>)
        } else {
            Some(Ok(0))
        };
        let npc = if has_npc_id {
            parts
                .next()
                .map(|p| NPCPersonality::from_id(p.to_ascii_lowercase()))
        } else {
            Some(Some(NPCPersonality::jet()))
        };
        let rules_generation = if has_rules_generation {
            parts.next().map(str::parse::<u32>)
        } else {
            Some(Ok(0))
        };
        let seed = parts.next().map(str::parse::<u32>);
        let player = parts
            .next()
            .map(|p| p.parse::<usize>().unwrap_or(0))
            .unwrap_or(0);

        if let (
            Some(Ok(board_generation)),
            Some(Some(npc)),
            Some(Ok(rules_generation)),
            Some(Ok(seed)),
        ) = (board_generation, npc, rules_generation, seed)
        {
            let board_seed = BoardSeed::new_with_generation(board_generation, seed);
            let board = generate_board(board_seed.clone())
                .expect("Common seeds can be reasonably expected to produce a board")
                .board;
            let header = HeaderType::Summary {
                title: format!("Truncate Puzzle"),
                attempt: None,
            };
            outer
                .event_dispatcher
                .event(format!("linked_puzzle_{launch_code}"));
            let puzzle_game = SinglePlayerState::new(
                npc.name.clone(),
                ui.ctx(),
                outer.map_texture.clone(),
                outer.theme.clone(),
                board,
                Some(board_seed),
                rules_generation,
                player == 0,
                header,
                npc,
                outer.event_dispatcher.clone(),
            );
            return Some(GameStatus::SinglePlayer(puzzle_game));
        } else {
            return Some(GameStatus::HardError(vec![
                "Sorry, that puzzle URL".to_string(),
                "doesn't look right!".to_string(),
            ]));
        }
    }

    if launch_code.starts_with("REPLAY:") {
        if let Some(id) = launch_code.split(':').skip(1).next() {
            send_to_server(PlayerMessage::LoadReplay(id.to_string()));
            return Some(GameStatus::PendingReplay);
        } else {
            return Some(GameStatus::HardError(vec![
                "Sorry, that replay URL".to_string(),
                "doesn't look right!".to_string(),
            ]));
        }
    }

    // No room code means we start a new game.
    if launch_code.is_empty() {
        send_to_server(PlayerMessage::NewGame(outer.name.clone()));
        return Some(GameStatus::PendingCreate);
    }

    // Finally, if nothing matched, we try to join a lobby with the given code.

    let token = if let GameStatus::None(_, token) = &outer.game_status {
        token.clone()
    } else {
        None
    };

    send_to_server(PlayerMessage::JoinGame(
        launch_code.clone(),
        outer.name.clone(),
        token,
    ));

    Some(GameStatus::PendingJoin(launch_code.clone()))
}
