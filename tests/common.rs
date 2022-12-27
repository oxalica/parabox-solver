use std::path::Path;

use anyhow::{bail, Result};
use parabox_solver::Direction;

pub const SEPARATOR: &str = "================\n";
pub const TEST_DIR: &str = "tests";
pub const EXTENTION: &str = "map";

#[allow(unused)]
pub fn parse_direction(ch: char) -> Result<Direction> {
    Ok(match ch {
        'L' => Direction::Left,
        'R' => Direction::Right,
        'U' => Direction::Up,
        'D' => Direction::Down,
        _ => bail!("Invalid action: {ch:?}"),
    })
}

#[allow(unused)]
pub fn fmt_direction(dir: Direction) -> &'static str {
    match dir {
        Direction::Right => "R",
        Direction::Down => "D",
        Direction::Left => "L",
        Direction::Up => "U",
    }
}

pub fn run_tests(subdir: &str, mut f: impl FnMut(&str) -> Result<String>) {
    let mut tests = std::fs::read_dir(Path::new(TEST_DIR).join(subdir))
        .unwrap()
        .filter_map(|ent| {
            let path = ent.unwrap().path();
            if path.extension().map_or(true, |ext| ext != EXTENTION) {
                return None;
            }
            let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
            Some((name, path))
        })
        .collect::<Vec<_>>();
    tests.sort();

    let do_update_tests = std::env::var("UPDATE_EXPECT").map_or(false, |v| v == "1");

    let mut failed_cnt = 0;
    for (name, path) in &tests {
        eprint!("{name}: ");
        let content = std::fs::read_to_string(path).unwrap();
        match f(&content) {
            Ok(got) if got == content => eprintln!("\x1B[32mOK\x1B[0m"),
            Ok(got) if do_update_tests => {
                std::fs::write(path, got).unwrap();
                eprintln!("\x1B[33mUpdated\x1B[0m");
            }
            Ok(_) => eprintln!("\x1B[31mFAILED\x1B[0m"),
            Err(err) => {
                eprintln!("\x1B[31mFAILED\x1B[0m\n{:?}", err);
                failed_cnt += 1;
            }
        }
    }

    if failed_cnt != 0 {
        eprintln!("{failed_cnt}/{} tests failed", tests.len());
        std::process::exit(1);
    }
}
