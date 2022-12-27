use std::str::FromStr;

use anyhow::{anyhow, bail, ensure, Context, Result};

use crate::{
    Board, BoardId, Cell, Config, Game, GlobalPos, State, Vec2, MAX_BOARD_CNT, MAX_BOARD_WIDTH,
};

impl FromStr for Game {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines().map(|line| line.trim());

        let mut boards = Vec::new();
        let mut player = None;
        let mut player_target = None;
        let mut box_targets = Vec::new();
        let mut max_board_id = BoardId::default();

        while let Some(id_line) = lines.next() {
            let board_id = id_line
                .parse::<usize>()?
                .try_into()
                .map_err(|()| anyhow!("Too many boards"))?;
            ensure!(
                board_id as usize == boards.len(),
                "Invalid board id: {board_id}"
            );

            let line = lines.next().context("Missing board content")?;
            let width = line.chars().count();

            let mut grid = Vec::new();
            let mut parse_line = |i: usize, line: &str| -> Result<_> {
                for (j, ch) in line.chars().enumerate() {
                    let gpos = GlobalPos {
                        board_id,
                        pos: Vec2(i as _, j as _),
                    };
                    let cell = match ch {
                        '.' => Cell::Empty,
                        '#' => Cell::Wall,
                        'b' => Cell::Box,
                        'p' => {
                            ensure!(player.is_none(), "Multiple players");
                            player = Some(gpos);
                            Cell::Box
                        }
                        '_' => {
                            box_targets.push(gpos);
                            Cell::Empty
                        }
                        '=' => {
                            ensure!(player_target.is_none(), "Multiple player targets");
                            player_target = Some(gpos);
                            Cell::Empty
                        }
                        '0'..='9' => {
                            let board_id = BoardId::try_from(ch as usize - b'0' as usize).unwrap();
                            max_board_id = max_board_id.max(board_id);
                            Cell::Board(board_id)
                        }
                        _ => bail!("Invalid cell: {ch:?}",),
                    };
                    grid.push(cell);
                }
                Ok(())
            };

            parse_line(0, line)?;
            let mut height = 1;
            while let Some(line) = lines.next().filter(|line| !line.is_empty()) {
                ensure!(
                    line.chars().count() == width,
                    "Width mismatch of board {}, line {height}, expecting width {width}",
                    board_id,
                );
                parse_line(height, line)?;
                height += 1;
            }

            ensure!(
                width < MAX_BOARD_WIDTH && height < MAX_BOARD_WIDTH,
                "Board too big",
            );

            boards.push(Board {
                height: height as _,
                width: width as _,
                grid: grid.into(),
            });
        }

        ensure!(
            (max_board_id as usize) < boards.len(),
            "Board id {} out of bound {}",
            max_board_id,
            boards.len(),
        );

        ensure!(boards.len() < MAX_BOARD_CNT, "Too many boards");

        let config = Config {
            player_target: player_target.context("Missing player target")?,
            box_targets: box_targets.into(),
        };
        let state = State {
            player: player.context("Missing player")?,
            boards: boards.into(),
        };
        Ok(Game { config, state })
    }
}
