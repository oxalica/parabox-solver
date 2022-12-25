use std::mem;
use std::ops::{Index, IndexMut};

use anyhow::{Context, Result};
use console::{Key, Term};

mod fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    player: GlobalPos,
    player_target: GlobalPos,
    box_targets: Box<[GlobalPos]>,
    boards: Box<[Board]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Board {
    height: u8,
    width: u8,
    grid: Box<[Cell]>,
}

impl Index<Vec2> for Board {
    type Output = Cell;
    fn index(&self, pos: Vec2) -> &Self::Output {
        let idx = pos.0 as usize * self.width as usize + pos.1 as usize;
        &self.grid[idx]
    }
}
impl IndexMut<Vec2> for Board {
    fn index_mut(&mut self, pos: Vec2) -> &mut Self::Output {
        let idx = pos.0 as usize * self.width as usize + pos.1 as usize;
        &mut self.grid[idx]
    }
}

impl Board {
    pub fn cells(&self) -> impl Iterator<Item = (Vec2, Cell)> + '_ {
        let idx_iter = std::iter::successors(Some(Vec2(0, 0)), |&Vec2(x, y)| {
            Some(if y + 1 < self.width {
                Vec2(x, y + 1)
            } else {
                Vec2(x + 1, 0)
            })
        });
        idx_iter.zip(self.grid.iter().copied())
    }

    pub fn sibling_pos(&self, pos: Vec2, dir: Direction) -> Option<Vec2> {
        const DIRECTIONS: [(i8, i8); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        let x = pos.0.checked_add_signed(DIRECTIONS[dir as usize].0)?;
        let y = pos.1.checked_add_signed(DIRECTIONS[dir as usize].1)?;
        if self.height <= x || self.width <= y {
            return None;
        }
        Some(Vec2(x, y))
    }

    pub fn inner_sibling_pos(&self, push_dir: Direction) -> Vec2 {
        match push_dir {
            Direction::Right => Vec2(self.height / 2, 0),
            Direction::Down => Vec2(0, self.width / 2),
            Direction::Left => Vec2(self.height / 2, self.width - 1),
            Direction::Up => Vec2(self.height - 1, self.width / 2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GlobalPos {
    board_id: u8,
    pos: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Vec2(u8, u8);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    #[default]
    Empty,
    Wall,
    Box,
    Board(u8),
}

impl Cell {
    pub fn is_box_like(&self) -> bool {
        matches!(self, Self::Box | Self::Board(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Right = 0,
    Down,
    Left,
    Up,
}

impl Index<GlobalPos> for State {
    type Output = Cell;
    fn index(&self, gpos: GlobalPos) -> &Self::Output {
        &self.boards[gpos.board_id as usize][gpos.pos]
    }
}
impl IndexMut<GlobalPos> for State {
    fn index_mut(&mut self, gpos: GlobalPos) -> &mut Self::Output {
        &mut self.boards[gpos.board_id as usize][gpos.pos]
    }
}

impl State {
    pub fn is_finished(&self) -> bool {
        self.player_target == self.player
            && self
                .box_targets
                .iter()
                .all(|&gpos| self[gpos].is_box_like())
    }

    pub fn get_board_box_pos(&self, target_board: u8) -> Option<GlobalPos> {
        self.boards.iter().zip(0..).find_map(|(board, board_id)| {
            let (pos, _) = board
                .cells()
                .find(|(_, cell)| *cell == Cell::Board(target_board))?;
            Some(GlobalPos { board_id, pos })
        })
    }

    pub fn sibling(&self, gpos: GlobalPos, dir: Direction) -> Option<GlobalPos> {
        if let Some(pos) = self.boards[gpos.board_id as usize].sibling_pos(gpos.pos, dir) {
            return Some(GlobalPos { pos, ..gpos });
        };
        let board_box_gpos = self.get_board_box_pos(gpos.board_id)?;
        if let Some(pos) =
            self.boards[board_box_gpos.board_id as usize].sibling_pos(board_box_gpos.pos, dir)
        {
            return Some(GlobalPos { pos, ..gpos });
        }
        todo!();
    }

    pub fn go(&mut self, dir: Direction) -> Result<(), ()> {
        let start_gpos = self.player;
        let mut gpos = start_gpos;
        let mut blocks = Vec::new();
        loop {
            blocks.push(gpos);
            gpos = self.sibling(gpos, dir).ok_or(())?;
            let cell = self[gpos];
            match cell {
                Cell::Box | Cell::Board(_) => {}
                // Push sequence.
                Cell::Empty => {
                    let mut cell = Cell::Empty;
                    blocks.push(gpos);
                    for &gpos in &blocks {
                        cell = mem::replace(&mut self[gpos], cell);
                    }
                    self.player = blocks[1];
                    return Ok(());
                }
                Cell::Wall => break,
            }
        }

        // Push aganst the wall.
        if blocks.len() == 1 {
            return Err(());
        }

        // Back pressure.
        while let Some(gpos) = blocks.pop() {
            let cell = self[gpos];
            match cell {
                Cell::Empty => unreachable!(),
                Cell::Wall | Cell::Box => continue,
                Cell::Board(board_id) => {
                    let board = &self.boards[board_id as usize];
                    // board.
                }
            }
        }

        todo!()
    }

    fn inner_sibling(&self, board_id: u8, push_dir: Direction) -> GlobalPos {
        let board = &self.boards[board_id as usize];
        let pos = board.inner_sibling_pos(push_dir);
        GlobalPos { board_id, pos }
    }
}

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
    let init_state = map_data
        .parse::<State>()
        .context("Failed to parse the map")?;

    let mut state = init_state.clone();
    let mut history = Vec::new();

    let term = Term::stderr();
    loop {
        eprintln!("{state}");

        let action = loop {
            if let Ok(action) = Action::try_from(term.read_key()?) {
                break action;
            }
        };

        match action {
            Action::Exit => break,
            Action::Go(dir) => {
                let mut new_state = state.clone();
                if new_state.go(dir).is_ok() {
                    history.push(state);
                    state = new_state;
                }
            }
            Action::Undo => {
                if let Some(last_state) = history.pop() {
                    state = last_state;
                }
            }
            Action::Reset => {
                history.push(state);
                state = init_state.clone();
            }
        }
    }

    Ok(())
}
