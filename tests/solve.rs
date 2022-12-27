use anyhow::Context;
use parabox_solver::{solve, State};

use crate::common::*;

mod common;

fn main() {
    run_tests("solve", |content| {
        let map = content
            .split_once(SEPARATOR)
            .map_or(content, |(input, _)| input)
            .trim();
        let state = map.parse::<State>().context("Invalid map")?;

        let steps = solve::bfs(state, |_| {})
            .context("No solution")?
            .into_iter()
            .map(fmt_direction)
            .collect::<String>();

        Ok(format!("{map}\n\n{SEPARATOR}{steps}\n"))
    });
}
