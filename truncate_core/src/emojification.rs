use crate::{
    board::{Board, Square},
    game::Game,
    generation::BoardSeed,
};

pub const SQ_BLUE: &str = "🟦";
pub const SQ_GREEN: &str = "🟩";
pub const SQ_BROWN: &str = "🟫";
pub const SQ_RED: &str = "🟥";
pub const SQ_ORANGE: &str = "🟧";
pub const SQ_PURPLE: &str = "🟪";
pub const SQ_YELLOW: &str = "🟨";
pub const SQ_WHITE: &str = "⬜";
pub const SQ_BLACK: &str = "⬛";
pub const SQ_BLACK_IN_WHITE: &str = "🔳";
pub const SQ_WHITE_IN_BLACK: &str = "🔲";
pub const SQ_ERR: &str = "🆘";

impl Board {
    pub fn emojify(
        &self,
        player: usize,
        won: Option<usize>,
        game: Option<&Game>,
        seed: Option<BoardSeed>,
        attempts: Option<usize>,
        url_prefix: String,
    ) -> String {
        let player_won = won == Some(player);
        let water = SQ_BLUE;
        let land = if player_won { SQ_GREEN } else { SQ_BROWN };
        let tile = if player_won { SQ_YELLOW } else { SQ_PURPLE };

        let emoji_for_square = |sq: &Square| match sq {
            crate::board::Square::Water => water,
            crate::board::Square::Fog => water,
            crate::board::Square::Land => land,
            crate::board::Square::Town { .. } => land,
            crate::board::Square::Dock(_) => water,
            crate::board::Square::Occupied(player, _) if won == Some(*player) => tile,
            crate::board::Square::Occupied(_, _) => land,
        };

        let mut grid = if player == 0 {
            self.squares
                .iter()
                .rev()
                .map(|row| row.iter().rev().map(emoji_for_square).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        } else {
            self.squares
                .iter()
                .map(|row| row.iter().map(emoji_for_square).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };

        enum D {
            Top,
            Bottom,
            Left,
            Right,
        }
        fn trim_grid(grid: &mut Vec<Vec<&str>>, dir: D) {
            match dir {
                D::Top => {
                    grid.remove(0);
                }
                D::Bottom => {
                    grid.remove(grid.len() - 1);
                }
                D::Left => grid.iter_mut().for_each(|row| {
                    row.remove(0);
                }),
                D::Right => grid.iter_mut().for_each(|row| {
                    row.remove(row.len() - 1);
                }),
            };
        }

        // Remove all non-water rows from the top
        while grid
            .first()
            .is_some_and(|row| row.iter().all(|s| s == &water))
        {
            trim_grid(&mut grid, D::Top);
        }

        // Remove all non-water rows from the bottom
        while grid
            .last()
            .is_some_and(|row| row.iter().all(|s| s == &water))
        {
            trim_grid(&mut grid, D::Bottom);
        }

        // Remove all non-water columns from the left
        while grid.iter().all(|row| row.first() == Some(&water)) {
            trim_grid(&mut grid, D::Left);
        }

        // Remove all non-water columns from the right
        while grid.iter().all(|row| row.last() == Some(&water)) {
            trim_grid(&mut grid, D::Right);
        }

        let joined_grid = grid
            .into_iter()
            .map(|row| row.into_iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        let url = if let Some(seed) = seed.clone() {
            format!(
                "Play Puzzle: {url_prefix}PUZZLE:{}:{}:{}\n",
                seed.generation, seed.seed, player
            )
        } else {
            "".to_string()
        };

        let attempt_str = match attempts {
            Some(0) => format!(" first try"),
            Some(n) => format!(" on attempt #{}", n + 1),
            None => "".to_string(),
        };

        let counts = if let Some(game) = game {
            format!(
                "{} in {} move{}",
                attempt_str,
                game.player_turn_count[player],
                if game.player_turn_count[player] == 1 {
                    ""
                } else {
                    "s"
                },
            )
        } else {
            "".to_string()
        };

        if let Some(day) = seed.map(|s| s.day).flatten() {
            if player_won {
                format!("Truncate Town Day #{day}\nWon{counts}.\n{joined_grid}\n")
            } else {
                format!("Truncate Town Day #{day}\nLost{counts}.\n{joined_grid}\n")
            }
        } else {
            if player_won {
                format!("Truncate Town Random Puzzle\nWon{counts}.\n{url}{joined_grid}\n")
            } else {
                format!("Truncate Town Random Puzzle\nLost{counts}.\n{url}{joined_grid}\n")
            }
        }
    }
}
