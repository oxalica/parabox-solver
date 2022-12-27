use std::str::FromStr;

use anyhow::{bail, ensure, Context, Result};

use crate::{Board, BoardId, Cell, Config, Game, GlobalPos, State, Vec2};

impl FromStr for Game {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines().map(|line| line.trim());

        let mut boards = Vec::new();
        let mut player = None;
        let mut player_target = None;
        let mut box_targets = Vec::new();
        let mut max_board_id = BoardId(0);

        while let Some(id_line) = lines.next() {
            let board_id = id_line.parse::<u8>()?;
            ensure!(
                board_id as usize == boards.len(),
                "Invalid board id: {board_id}"
            );
            let board_id = BoardId(board_id);

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
                            let board_id = BoardId(ch as u8 - b'0');
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
                    board_id.0,
                );
                parse_line(height, line)?;
                height += 1;
            }

            boards.push(Board {
                height: height as _,
                width: width as _,
                grid: grid.into(),
            });
        }

        ensure!(
            (max_board_id.0 as usize) < boards.len(),
            "Board id {} out of bound {}",
            max_board_id.0,
            boards.len(),
        );

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
