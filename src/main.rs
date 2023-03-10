use std::time::Instant;

use anyhow::{Context, Result};
use console::{Key, Term};
use indicatif::{ProgressBar, ProgressStyle};
use parabox_solver::{solve, Direction, Game};

enum Action {
    Exit,
    Go(Direction),
    Undo,
    Reset,
}

impl TryFrom<Key> for Action {
    type Error = ();

    fn try_from(key: Key) -> Result<Self, Self::Error> {
        Ok(match key {
            Key::ArrowLeft | Key::Char('a') => Self::Go(Direction::Left),
            Key::ArrowRight | Key::Char('d') => Self::Go(Direction::Right),
            Key::ArrowUp | Key::Char('w') => Self::Go(Direction::Up),
            Key::ArrowDown | Key::Char('s') => Self::Go(Direction::Down),
            Key::Escape | Key::Char('q') => Self::Exit,
            Key::Char('z') => Self::Undo,
            Key::Char('r') => Self::Reset,
            _ => return Err(()),
        })
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .context("Missing map file argument")?;
    let map_data = std::fs::read_to_string(path).context("Failed to read the map")?;
    let game = map_data
        .parse::<Game>()
        .context("Failed to parse the map")?;

    if std::env::args().nth(2).as_deref() == Some("--solve") {
        let style = ProgressStyle::with_template(
            "{spinner} Elapsed: {elapsed_precise} Searched: {human_pos} Speed: {per_sec}",
        )
        .unwrap();
        let pb = ProgressBar::new_spinner().with_style(style);

        const BULK: u64 = 1 << 16;
        let mut counter = 0u64;
        let inst = Instant::now();
        let ret = solve::bfs(game, || {
            counter += 1;
            if counter % BULK == 0 {
                pb.set_position(counter);
            }
        });
        let elapsed = inst.elapsed();
        pb.set_position(counter);
        pb.finish();
        eprintln!("Finished in {:?}", elapsed);
        eprintln!("{:?}", ret);
        return Ok(());
    }

    let mut history = vec![game.state];

    let term = Term::stderr();
    loop {
        let mut state = history.last().cloned().unwrap();
        eprintln!("{}", state);

        if state.is_success_on(&game.config) {
            eprintln!("Success");
            break;
        }

        let action = loop {
            if let Ok(action) = Action::try_from(term.read_key()?) {
                break action;
            }
        };

        match action {
            Action::Exit => break,
            Action::Go(dir) => {
                let msg = match state.go(dir) {
                    Ok(pushed) => {
                        history.push(state);
                        pushed.to_string()
                    }
                    Err(err) => err.to_string(),
                };
                eprintln!("{msg}");
            }
            Action::Undo => {
                if history.len() >= 2 {
                    history.pop();
                }
            }
            Action::Reset => {
                history.push(history[0].clone());
            }
        }
    }

    Ok(())
}
