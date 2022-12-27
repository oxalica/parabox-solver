use std::mem;
use std::ops::{Index, IndexMut};

use anyhow::Result;

mod fmt;
mod parse;
pub mod solve;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GoError {
    Stuck,
    Unmovable,
    OutOfInfinity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoardId(pub u8);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    player: GlobalPos,
    player_target: GlobalPos,
    box_targets: Box<[GlobalPos]>,
    boards: Box<[Board]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Board {
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
    fn cells(&self) -> impl Iterator<Item = (Vec2, Cell)> + '_ {
        let idx_iter = std::iter::successors(Some(Vec2(0, 0)), |&Vec2(x, y)| {
            Some(if y + 1 < self.width {
                Vec2(x, y + 1)
            } else {
                Vec2(x + 1, 0)
            })
        });
        idx_iter.zip(self.grid.iter().copied())
    }

    fn sibling_pos(&self, pos: Vec2, dir: Direction) -> Option<Vec2> {
        const DIRECTIONS: [(i8, i8); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        let x = pos.0.checked_add_signed(DIRECTIONS[dir as usize].0)?;
        let y = pos.1.checked_add_signed(DIRECTIONS[dir as usize].1)?;
        if self.height <= x || self.width <= y {
            return None;
        }
        Some(Vec2(x, y))
    }

    fn inner_sibling_pos(&self, push_dir: Direction) -> Vec2 {
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
pub enum Direction {
    Right = 0,
    Down,
    Left,
    Up,
}

impl Direction {
    pub const ALL: [Self; 4] = [Self::Right, Self::Down, Self::Left, Self::Up];

    pub fn reversed(self) -> Self {
        match self {
            Direction::Right => Direction::Left,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Up => Direction::Down,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InnerSibling {
    Wall,
    NonWall(GlobalPos),
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

    fn get_board_box_pos(&self, target_board: BoardId) -> Option<GlobalPos> {
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

    fn sibling(&self, mut gpos: GlobalPos, dir: Direction) -> Option<GlobalPos> {
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
