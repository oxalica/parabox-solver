use anyhow::{ensure, Context};
use parabox_solver::{solve, Game};

use crate::common::*;

mod common;

fn main() {
    run_tests("solve", false, |content| {
        let map = content
            .split_once(SEPARATOR)
            .map_or(content, |(input, _)| input)
            .trim();
        let mut game = map.parse::<Game>().context("Invalid map")?;

        let steps = solve::bfs(game.clone(), || {}).context("No solution")?;

        // Validate.
        for &dir in &steps {
            game.state.go(dir).context("Invalid move")?;
        }
        ensure!(game.is_success(), "Invalid solution");

        let steps = steps.into_iter().map(fmt_direction).collect::<String>();

        Ok(format!("{map}\n\n{SEPARATOR}{steps}\n"))
    });
}
