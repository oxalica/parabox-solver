use std::fmt::Write;
use std::path::Path;

use anyhow::{bail, ensure, Context, Result};
use parabox_solver::{Direction, State};

const SEPARATOR: &str = "================\n";
const TEST_DIR: &str = "tests/move";
const EXTENTION: &str = "map";

fn main() {
    let mut test_paths = std::fs::read_dir(TEST_DIR)
        .unwrap()
        .filter_map(|ent| {
            let path = ent.unwrap().path();
            path.extension()
                .map_or(false, |ext| ext == EXTENTION)
                .then_some(path)
        })
        .collect::<Vec<_>>();
    test_paths.sort();

    let mut failed = 0;
    for path in &test_paths {
        let name = path.file_name().unwrap().to_str().unwrap();
        eprint!("{name}: ");
        match run_test(path) {
            Ok(false) => eprintln!("\x1B[32mOK\x1B[0m"),
            Ok(true) => eprintln!("\x1B[33mUpdated\x1B[0m"),
            Err(err) => {
                eprintln!("\x1B[31mFAILED\x1B[0m\n{:?}", err);
                failed += 1;
            }
        }
    }

    if failed != 0 {
        eprintln!("{failed}/{} tests failed", test_paths.len());
        std::process::exit(1);
    }
}

fn run_test(path: &Path) -> Result<bool> {
    let content = std::fs::read_to_string(path)?;
    let (input, expect) = content.split_once(SEPARATOR).unwrap_or((&content, ""));
    let (actions, map) = input.split_once('\n').context("No actions")?;
    ensure!(!actions.is_empty(), "No actions");

    let mut state = map.parse::<State>().context("Invalid map")?;
    let mut got = String::new();
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

    if got != expect {
        if std::env::var("UPDATE_EXPECT").is_err() {
            bail!("Test failed");
        }
        let new_content = format!("{}\n\n{SEPARATOR}{got}", input.trim());
        std::fs::write(path, new_content).context("Failed to update tests")?;
        return Ok(true);
    }

    Ok(false)
}
