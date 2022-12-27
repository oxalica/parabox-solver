use crate::{Direction, Game, GlobalPos, State};

type IndexMap<K, V> = indexmap::IndexMap<K, V, fxhash::FxBuildHasher>;

pub fn bfs(game: Game, on_step: impl FnMut()) -> Option<Vec<Direction>> {
    let states = bfs_big_step(game, on_step)?;

    // Resolve intermediate steps.
    let mut solution = Vec::new();
    let mut state_parent = IndexMap::default();
    for w in states.windows(2) {
        let substeps = bfs_small_step(&w[0], &w[1], &mut state_parent).expect("Must be reachable");
        solution.extend(substeps);
    }
    Some(solution)
}

fn bfs_big_step(game: Game, mut on_step: impl FnMut()) -> Option<Vec<State>> {
    let mut state_parent = IndexMap::default();
    state_parent.insert(game.state, !0usize); // Sentinel.

    // Non-pushing states reachable from the current state.
    let mut trivial_visited = BucketIndexSet::<GlobalPos, { GlobalPos::TO_USIZE_LIMIT }>::new();

    let mut big_cursor = 0;
    let final_state = 'bfs: loop {
        #[cfg(feature = "coz")]
        coz::scope!("Big step");

        if big_cursor >= state_parent.len() {
            return None;
        }

        let get_init_state = |state_parent: &IndexMap<State, _>| {
            state_parent.get_index(big_cursor).unwrap().0.clone()
        };

        let mut state = get_init_state(&state_parent);
        trivial_visited.clear();
        trivial_visited.try_insert(state.player);

        let mut small_cursor = 0;
        while small_cursor < trivial_visited.len() {
            let gpos = trivial_visited[small_cursor];

            for dir in Direction::ALL {
                on_step();

                #[cfg(feature = "coz")]
                coz::progress!("Step");

                state.set_player(gpos);

                let Ok(do_pushed) = state.go(dir) else { continue };

                // Success.
                if state.is_success_on(&game.config) {
                    break 'bfs state;
                }

                // Trivial move.
                if !do_pushed {
                    trivial_visited.try_insert(state.player);
                    continue;
                }

                // Non-trivial push. The state now cannot be reused.
                state_parent.entry(state).or_insert(big_cursor);
                state = get_init_state(&state_parent);
            }
            small_cursor += 1;
        }
        big_cursor += 1;
    };

    let mut states = std::iter::successors(Some((&final_state, &big_cursor)), |(_, &i)| {
        state_parent.get_index(i)
    })
    .map(|(state, _)| state.clone())
    .collect::<Vec<_>>();
    states.reverse();
    Some(states)
}

fn bfs_small_step(
    before: &State,
    after: &State,
    state_parent: &mut IndexMap<State, (usize, Direction)>,
) -> Option<Vec<Direction>> {
    state_parent.insert(before.clone(), (!0usize, Direction::Right)); // Sentinel.
    let mut cursor = 0;
    let final_dir = 'bfs: loop {
        if cursor >= state_parent.len() {
            return None;
        }

        for dir in Direction::ALL {
            let mut state = state_parent.get_index(cursor).unwrap().0.clone();
            let Ok(do_pushed) = state.go(dir) else { continue };
            // NB. The last state transition may not push anything.
            if state == *after {
                break 'bfs dir;
            }
            if !do_pushed {
                state_parent.entry(state).or_insert((cursor, dir));
            }
        }
        cursor += 1;
    };

    let mut steps = std::iter::successors(Some((cursor, final_dir)), |&(i, _)| {
        let (parent, dir) = state_parent[i];
        (parent != !0usize).then_some((parent, dir))
    })
    .map(|(_, dir)| dir)
    .collect::<Vec<_>>();
    steps.reverse();
    Some(steps)
}

struct BucketIndexSet<T, const N: usize> {
    len: usize,
    elems: [T; N],
    set: [u8; N],
    /// The current "true" value, for fast clearing.
    set_marker: u8,
}

impl<T, const N: usize> BucketIndexSet<T, N>
where
    T: Default + Copy + Into<usize>,
{
    fn new() -> Self {
        Self {
            len: 0,
            elems: [T::default(); N],
            set: [0; N],
            set_marker: 1,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn clear(&mut self) {
        self.len = 0;
        self.set_marker = self.set_marker.wrapping_add(1);
    }

    fn try_insert(&mut self, value: T) {
        let i = value.into();
        if self.set[i] == self.set_marker {
            return;
        }
        self.set[i] = self.set_marker;
        self.elems[self.len] = value;
        self.len += 1;
    }
}

impl<T, const N: usize> std::ops::Index<usize> for BucketIndexSet<T, N> {
    type Output = T;
    fn index(&self, i: usize) -> &Self::Output {
        assert!(i < self.len);
        &self.elems[i]
    }
}
