use crate::{board::Board, game::Game, generation::BoardSeed};

const SQ_BLUE: &str = "🟦";
const SQ_GREEN: &str = "🟩";
const SQ_BROWN: &str = "🟫";
const SQ_RED: &str = "🟥";
const SQ_ORANGE: &str = "🟧";
const SQ_PURPLE: &str = "🟪";
const SQ_YELLOW: &str = "🟨";
const SQ_WHITE: &str = "⬜";
const SQ_BLACK: &str = "⬛";
const SQ_BLACK_IN_WHITE: &str = "🔳";
const SQ_WHITE_IN_BLACK: &str = "🔲";
const SQ_ERR: &str = "🆘";

impl Board {
    pub fn emojify(
        &self,
        won: Option<usize>,
        game: Option<&Game>,
        seed: Option<BoardSeed>,
        url_prefix: String,
    ) -> String {
        let water = if won == Some(0) { SQ_BLUE } else { SQ_BLACK };
        let land = if won == Some(0) { SQ_GREEN } else { SQ_BROWN };
        let tile = if won == Some(0) { SQ_YELLOW } else { SQ_ORANGE };

        let mut grid = self
            .squares
            .iter()
            .rev()
            .map(|row| {
                row.iter()
                    .rev()
                    .map(|sq| match sq {
                        crate::board::Square::Water => water,
                        crate::board::Square::Land => land,
                        crate::board::Square::Town { .. } => land,
                        crate::board::Square::Dock(_) => water,
                        crate::board::Square::Occupied(player, _) if won == Some(*player) => tile,
                        crate::board::Square::Occupied(_, _) => land,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

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
                "Play Puzzle: {url_prefix}PUZZLE:{}:{}\n",
                seed.generation, seed.seed
            )
        } else {
            "".to_string()
        };

        let counts = if let Some(game) = game {
            format!(
                " in {} turn{}, {} battle{}",
                game.player_turn_count[0],
                if game.player_turn_count[0] == 1 {
                    ""
                } else {
                    "s"
                },
                game.battle_count,
                if game.battle_count == 1 { "" } else { "s" },
            )
        } else {
            "".to_string()
        };

        if let Some(day) = seed.map(|s| s.day).flatten() {
            format!("🌟 Truncate Town Day #{day} 🌟\nWon{counts}.\n{joined_grid}\n")
        } else {
            if won == Some(0) {
                format!("Truncate Town Custom Puzzle\nWon{counts}.\n{url}{joined_grid}\n")
            } else {
                format!("Truncate Town Custom Puzzle\nLost{counts}.\n{url}{joined_grid}\n")
            }
        }
    }
}
