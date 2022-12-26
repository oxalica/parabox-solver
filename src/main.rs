use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::mem;
use std::ops::{Index, IndexMut};

use anyhow::{Context, Result};
use console::{Key, Term};
use indicatif::{ProgressBar, ProgressStyle};

mod fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum GoError {
    Stuck,
    Unmovable,
    OutOfInfinity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct BoardId(u8);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct State {
    player: GlobalPos,
    player_target: GlobalPos,
    box_targets: Box<[GlobalPos]>,
    boards: Box<[Board]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlobalPos {
    board_id: BoardId,
    pos: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Vec2(u8, u8);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Cell {
    #[default]
    Empty,
    Wall,
    Box,
    Board(BoardId),
}

impl Cell {
    pub fn is_box_like(&self) -> bool {
        matches!(self, Self::Box | Self::Board(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Direction {
    Right = 0,
    Down,
    Left,
    Up,
}

impl Direction {
    const ALL: [Self; 4] = [Self::Right, Self::Down, Self::Left, Self::Up];

    pub fn reversed(self) -> Self {
        match self {
            Direction::Right => Direction::Left,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Up => Direction::Down,
        }
    }
}

impl Index<BoardId> for State {
    type Output = Board;
    fn index(&self, idx: BoardId) -> &Self::Output {
        &self.boards[idx.0 as usize]
    }
}
impl IndexMut<BoardId> for State {
    fn index_mut(&mut self, idx: BoardId) -> &mut Self::Output {
        &mut self.boards[idx.0 as usize]
    }
}

impl Index<GlobalPos> for State {
    type Output = Cell;
    fn index(&self, gpos: GlobalPos) -> &Self::Output {
        &self[gpos.board_id][gpos.pos]
    }
}
impl IndexMut<GlobalPos> for State {
    fn index_mut(&mut self, gpos: GlobalPos) -> &mut Self::Output {
        &mut self[gpos.board_id][gpos.pos]
    }
}

impl State {
    pub fn is_success(&self) -> bool {
        self.player_target == self.player
            && self
                .box_targets
                .iter()
                .all(|&gpos| self[gpos].is_box_like())
    }

    pub fn get_board_box_pos(&self, target_board: BoardId) -> Option<GlobalPos> {
        self.boards.iter().zip(0..).find_map(|(board, id)| {
            let (pos, _) = board
                .cells()
                .find(|(_, cell)| *cell == Cell::Board(target_board))?;
            Some(GlobalPos {
                board_id: BoardId(id),
                pos,
            })
        })
    }

    pub fn sibling(&self, mut gpos: GlobalPos, dir: Direction) -> Option<GlobalPos> {
        let mut visited = Vec::new();
        loop {
            if let Some(pos) = self[gpos.board_id].sibling_pos(gpos.pos, dir) {
                return Some(GlobalPos {
                    pos,
                    board_id: gpos.board_id,
                });
            };
            gpos = self.get_board_box_pos(gpos.board_id)?;
            if visited.contains(&gpos) {
                // TODO: Infinity.
                return None;
            }
            visited.push(gpos);
        }
    }

    pub fn go(&mut self, dir: Direction) -> Result<(), GoError> {
        let start_gpos = self.player;
        let mut cur_gpos = start_gpos;
        let mut cur_dir = dir;
        let mut push_seq = Vec::new();
        let mut cnt = 0;
        'try_push: loop {
            cnt += 1;
            // FIXME
            if cnt > 1000 {
                return Err(GoError::Stuck);
            }

            match self[cur_gpos] {
                // Accumulate the push sequence.
                Cell::Box | Cell::Board(_) => push_seq.push(cur_gpos),
                // Push.
                Cell::Empty => {
                    let mut cell = Cell::Empty;
                    push_seq.push(cur_gpos);
                    for &gpos in &push_seq {
                        cell = mem::replace(&mut self[gpos], cell);
                    }
                    self.player = push_seq[1];
                    return Ok(());
                }
                // Back pressure for entering.
                Cell::Wall => loop {
                    // Push aganst the wall.
                    if push_seq.len() <= 1 {
                        return Err(GoError::Unmovable);
                    }

                    let last_gpos = push_seq.pop().unwrap();
                    let is_cur_edible = match self[last_gpos] {
                        Cell::Empty => unreachable!(),
                        // Non-enterable and non-edible.
                        Cell::Wall => false,
                        // Non-enterable but edible.
                        Cell::Box => true,
                        // Enter.
                        Cell::Board(board_id) => match self.inner_sibling(board_id, cur_dir) {
                            // Enterable (preferred).
                            InnerSibling::NonWall(gpos) => {
                                cur_gpos = gpos;
                                continue 'try_push;
                            }
                            // Non-enterable but edible.
                            InnerSibling::Wall => true,
                        },
                    };

                    // If the current box is edible and the previous box is enterable in the
                    // inversed direction, enqueue it in the reversed direction.
                    if is_cur_edible {
                        if let Cell::Board(board_id) = self[*push_seq.last().unwrap()] {
                            let dir_rev = cur_dir.reversed();
                            if let InnerSibling::NonWall(eater_gpos) =
                                self.inner_sibling(board_id, dir_rev)
                            {
                                push_seq.push(last_gpos);
                                cur_gpos = eater_gpos;
                                cur_dir = dir_rev;
                                continue 'try_push;
                            }
                        }
                    }
                },
            }
            cur_gpos = self
                .sibling(cur_gpos, cur_dir)
                .ok_or(GoError::OutOfInfinity)?;
        }
    }

    fn inner_sibling(&self, board_id: BoardId, push_dir: Direction) -> InnerSibling {
        let board = &self[board_id];
        let pos = board.inner_sibling_pos(push_dir);
        match board[pos] {
            Cell::Wall => InnerSibling::Wall,
            Cell::Empty | Cell::Box | Cell::Board(_) => {
                InnerSibling::NonWall(GlobalPos { board_id, pos })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InnerSibling {
    Wall,
    NonWall(GlobalPos),
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

    if std::env::args().nth(2).as_deref() == Some("--solve") {
        let pb = ProgressBar::new_spinner()
            .with_style(ProgressStyle::with_template("{spinner} {pos} {per_sec}").unwrap());
        let ret = solve(init_state, |len| {
            pb.set_position(len as _);
        });
        eprintln!("{:?}", ret);
        return Ok(());
    }

    let mut state = init_state.clone();
    let mut history = Vec::new();

    let term = Term::stderr();
    loop {
        eprintln!("{state}");

        if state.is_success() {
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

fn solve(init_state: State, mut on_step: impl FnMut(usize)) -> Option<Vec<Direction>> {
    #[derive(Clone)]
    struct Node {
        state: State,
        steps: u32,
        parent: usize,
        dir: Direction,
    }

    let mut visited = HashMap::new();
    let mut queue = vec![Node {
        state: init_state,
        steps: 0,
        // Unused.
        parent: 0,
        // Unused.
        dir: Direction::Right,
    }];

    let mut cur = 0;
    let mut success_dir = None;
    'bfs: while cur != queue.len() {
        on_step(queue.len());
        for dir in Direction::ALL {
            let mut state = queue[cur].state.clone();
            if state.go(dir).is_err() {
                continue;
            }
            if state.is_success() {
                success_dir = Some(dir);
                break 'bfs;
            }
            let Entry::Vacant(ent) = visited.entry(state) else { continue };
            queue.push(Node {
                state: ent.key().clone(),
                steps: queue[cur].steps + 1,
                parent: cur,
                dir,
            });
            ent.insert(queue.len() - 1);
        }
        cur += 1;
    }

    let success_dir = success_dir?;
    let mut ret = std::iter::successors(Some(cur), |&i| Some(queue[i].parent))
        .take_while(|&i| i != 0)
        .map(|i| queue[i].dir)
        .collect::<Vec<_>>();
    ret.reverse();
    ret.push(success_dir);
    Some(ret)
}
