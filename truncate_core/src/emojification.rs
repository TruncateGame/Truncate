use crate::{board::Board, game::Game};

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
    pub fn emojify(&self, won: Option<usize>) -> String {
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

        // Remove all non-tile rows from the top
        while grid
            .first()
            .is_some_and(|row| !row.iter().any(|s| s == &tile))
        {
            trim_grid(&mut grid, D::Top);
        }

        // Remove all non-tile rows from the bottom
        while grid
            .last()
            .is_some_and(|row| !row.iter().any(|s| s == &tile))
        {
            trim_grid(&mut grid, D::Bottom);
        }

        // Remove all non-tile columns from the left
        while !grid.iter().any(|row| row.first() == Some(&tile)) {
            trim_grid(&mut grid, D::Left);
        }

        // Remove all non-tile columns from the right
        while !grid.iter().any(|row| row.last() == Some(&tile)) {
            trim_grid(&mut grid, D::Right);
        }

        // Keep thinning the sides until we get to 7 emoji
        while grid.first().is_some_and(|row| row.len() > 7) {
            let left = grid.iter().filter(|row| row.first() == Some(&tile)).count();
            let right = grid.iter().filter(|row| row.first() == Some(&tile)).count();
            trim_grid(&mut grid, if left < right { D::Left } else { D::Right });
        }

        let joined_grid = grid
            .into_iter()
            .map(|row| row.into_iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        if won == Some(0) {
            format!("Truncate — won in ... turns\n{}\n", joined_grid)
        } else {
            format!("Truncate — lost in ... turns\n{}\n", joined_grid)
        }
    }
}