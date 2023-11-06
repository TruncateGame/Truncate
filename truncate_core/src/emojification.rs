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
const SQ_TOWN: &str = "🏘";
const SQ_DESTROYED: &str = "🏚";
const SQ_STAR: &str = "🌟";
const SQ_BOAT: &str = "⛵️";

impl Board {
    pub fn emojify(&self, won: Option<usize>) -> String {
        let mut grid = self
            .squares
            .iter()
            .rev()
            .map(|row| {
                row.iter()
                    .rev()
                    .map(|sq| match sq {
                        crate::board::Square::Water => SQ_BLACK,
                        crate::board::Square::Land => SQ_GREEN,
                        crate::board::Square::Town { defeated, .. } if *defeated => SQ_DESTROYED,
                        crate::board::Square::Town { defeated, .. } if !*defeated => SQ_TOWN,
                        crate::board::Square::Town { .. } => SQ_ERR,
                        crate::board::Square::Dock(_) => SQ_BOAT,
                        crate::board::Square::Occupied(player, _) if *player == 0 => SQ_YELLOW,
                        crate::board::Square::Occupied(player, _) if *player == 1 => SQ_BROWN,
                        crate::board::Square::Occupied(_, _) => SQ_ERR,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        while grid.iter().all(|row| row.first() == Some(&SQ_BLACK)) {
            grid.iter_mut().for_each(|row| {
                row.remove(0);
            });
        }

        while grid.iter().all(|row| row.last() == Some(&SQ_BLACK)) {
            grid.iter_mut().for_each(|row| {
                row.remove(row.len() - 1);
            });
        }

        while grid
            .first()
            .is_some_and(|row| row.iter().all(|s| s == &SQ_BLACK))
        {
            grid.remove(0);
        }

        while grid
            .last()
            .is_some_and(|row| row.iter().all(|s| s == &SQ_BLACK))
        {
            grid.remove(grid.len() - 1);
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
