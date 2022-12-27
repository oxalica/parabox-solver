use std::fmt::Write;

use anyhow::{bail, ensure, Context};
use common::*;
use parabox_solver::{Direction, State};

mod common;

fn main() {
    run_tests("move", |content| {
        let input = content
            .split_once(SEPARATOR)
            .map_or(content, |(input, _)| input)
            .trim();
        let (actions, map) = input.split_once('\n').context("No actions")?;
        ensure!(!actions.is_empty(), "No actions");

        let mut state = map.parse::<State>().context("Invalid map")?;
        let mut got = format!("{input}\n\n{SEPARATOR}");
        for (ch, i) in actions.chars().zip(1..) {
            (|| {
                let dir = match ch {
                    'L' => Direction::Left,
                    'R' => Direction::Right,
                    'U' => Direction::Up,
                    'D' => Direction::Down,
                    _ => bail!("Invalid action: {ch:?}"),
                };
                state.go(dir).context("Move failed")
            })()
            .with_context(|| format!("Failed to perform step {i} {ch}"))?;
            write!(got, "{state}{SEPARATOR}").unwrap();
        }

        Ok(got)
    });
}
