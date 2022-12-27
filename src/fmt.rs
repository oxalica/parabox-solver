use std::fmt;

use crate::{BoardId, Cell, Game, GlobalPos, State};

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // FIXME
        self.state.fmt(f)
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (board, id) in self.boards.iter().zip(0..) {
            id.fmt(f)?;
            for (pos, cell) in board.cells() {
                if pos.1 == 0 {
                    "\n".fmt(f)?;
                }
                let gpos = GlobalPos {
                    board_id: BoardId(id),
                    pos,
                };
                if gpos == self.player {
                    "p".fmt(f)?;
                } else {
                    cell.fmt(f)?;
                }
            }
            "\n\n".fmt(f)?;
        }
        Ok(())
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cell::Empty => ".".fmt(f),
            Cell::Wall => "#".fmt(f),
            Cell::Box => "b".fmt(f),
            Cell::Board(BoardId(id)) => id.fmt(f),
        }
    }
}
