use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::{Direction, State};

#[derive(Clone)]
struct Node {
    state: State,
    steps: u32,
    parent: usize,
    dir: Direction,
}

pub fn bfs(init_state: State, mut on_step: impl FnMut(usize)) -> Option<Vec<Direction>> {
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
