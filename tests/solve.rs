use anyhow::Context;
use parabox_solver::{solve, Game};

use crate::common::*;

mod common;

fn main() {
    run_tests("solve", false, |content| {
        let map = content
            .split_once(SEPARATOR)
            .map_or(content, |(input, _)| input)
            .trim();
        let game = map.parse::<Game>().context("Invalid map")?;

        let steps = solve::bfs(game, |_| {})
            .context("No solution")?
            .into_iter()
            .map(fmt_direction)
            .collect::<String>();

        Ok(format!("{map}\n\n{SEPARATOR}{steps}\n"))
    });
}
