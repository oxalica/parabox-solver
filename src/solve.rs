use crate::{Direction, State};

type IndexMap<K, V> = indexmap::IndexMap<K, V, fxhash::FxBuildHasher>;

#[derive(Clone)]
struct Node {
    parent: usize,
    prev_direction: Direction,
}

pub fn bfs(init_state: State, mut on_step: impl FnMut(usize)) -> Option<Vec<Direction>> {
    let mut visited = IndexMap::default();
    visited.insert(
        init_state,
        // Unused.
        Node {
            parent: 0,
            prev_direction: Direction::Right,
        },
    );

    let mut cur = 0;
    let mut success_dir = None;
    'bfs: while cur != visited.len() {
        on_step(visited.len());
        for dir in Direction::ALL {
            let mut state = visited.get_index(cur).unwrap().0.clone();
            if state.go(dir).is_err() {
                continue;
            }
            if state.is_success() {
                success_dir = Some(dir);
                break 'bfs;
            }
            visited.entry(state).or_insert(Node {
                parent: cur,
                prev_direction: dir,
            });
        }
        cur += 1;
    }

    let success_dir = success_dir?;
    let mut ret = std::iter::successors(Some(cur), |&i| Some(visited[i].parent))
        .take_while(|&i| i != 0)
        .map(|i| visited[i].prev_direction)
        .collect::<Vec<_>>();
    ret.reverse();
    ret.push(success_dir);
    Some(ret)
}
