use std::collections::{VecDeque};

use crate::engine::{
    action::Direction::{East, North, South, West},
    grid::Grid,
    state::GameState,
};

pub fn bfs_distances(state: &GameState, start: (usize, usize)) -> Grid<usize> {
    let mut distance_grid = Grid::filled(state.arena.width, state.arena.height, usize::MAX);
    distance_grid[start] = 0; // player is 0 distance from themselves

    let mut queue = VecDeque::from([(start, 0)]);

    while !queue.is_empty() {
        let ((x, y), distance) = queue.pop_front().unwrap();
        let deltas = [North, South, East, West].map(|d| d.delta());
        let neighbors = deltas.map(|(dx, dy)| (x.checked_add_signed(dx), y.checked_add_signed(dy)));

        for (x, y) in neighbors {
            let Some(x) = x else {
                continue;
            };
            let Some(y) = y else {
                continue;
            };

            if !state.is_occupied(x, y) && distance_grid[(x,y)] == usize::MAX {
                queue.push_back(((x, y), distance + 1));
                distance_grid[(x, y)] = distance + 1;
            }
        }
    }

    distance_grid
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::engine::{action::Direction::East, state::Spawn};

    use super::*;

    fn setup_game_state(arena: Grid<bool>, spawns: &[Spawn]) -> GameState {
        GameState::new(Arc::new(arena), spawns.to_vec()).unwrap()
    }

    mod bfs_distances {
        use super::*;

        #[test]
        fn returns_shortest_distances_on_open_grid() {
            let arena = Grid::filled(3, 3, false);
            let spawns = [Spawn {
                x: 1,
                y: 1,
                direction: East,
            }];
            let state = setup_game_state(arena, &spawns);

            let distances = bfs_distances(&state, (1, 1));

            assert_eq!(distances, Grid::new(3, 3, vec![2, 1, 2, 1, 0, 1, 2, 1, 2]));
        }

        #[test]
        fn finds_detours_around_obstacles() {
            let mut arena = Grid::filled(5, 3, false);
            arena[(2, 0)] = true;
            arena[(2, 1)] = true;
            let spawns = [Spawn {
                x: 0,
                y: 1,
                direction: East,
            }];
            let state = setup_game_state(arena, &spawns);

            let distances = bfs_distances(&state, (0, 1));

            assert_eq!(distances[(2, 1)], usize::MAX);
            assert_eq!(distances[(2, 2)], 3);
            assert_eq!(distances[(3, 1)], 5);
        }

        #[test]
        fn marks_disconnected_cells_as_unreachable() {
            let mut arena = Grid::filled(5, 3, false);
            for y in 0..3 {
                arena[(2, y)] = true;
            }
            let spawns = [Spawn {
                x: 0,
                y: 1,
                direction: East,
            }];
            let state = setup_game_state(arena, &spawns);

            let distances = bfs_distances(&state, (0, 1));

            assert_eq!(distances[(1, 1)], 1);
            assert_eq!(distances[(2, 1)], usize::MAX);
            assert_eq!(distances[(3, 1)], usize::MAX);
        }

        #[test]
        fn treats_other_player_positions_as_blocked() {
            let arena = Grid::filled(4, 3, false);
            let spawns = [
                Spawn {
                    x: 0,
                    y: 1,
                    direction: East,
                },
                Spawn {
                    x: 1,
                    y: 1,
                    direction: East,
                },
            ];
            let state = setup_game_state(arena, &spawns);

            let distances = bfs_distances(&state, (0, 1));

            assert_eq!(distances[(0, 1)], 0);
            assert_eq!(distances[(1, 1)], usize::MAX);
            assert_eq!(distances[(2, 1)], 4);
        }

        #[test]
        fn treats_existing_trails_as_blocked() {
            let arena = Grid::filled(4, 3, false);
            let spawns = [Spawn {
                x: 0,
                y: 1,
                direction: East,
            }];
            let mut state = setup_game_state(arena, &spawns);
            state.trails[(1, 1)] = 1;

            let distances = bfs_distances(&state, (0, 1));

            assert_eq!(distances[(1, 1)], usize::MAX);
            assert_eq!(distances[(2, 1)], 4);
        }

        #[test]
        fn does_not_modify_game_state() {
            let arena = Grid::filled(3, 3, false);
            let spawns = [Spawn {
                x: 1,
                y: 1,
                direction: East,
            }];
            let state = setup_game_state(arena, &spawns);
            let original = state.clone();

            let _ = bfs_distances(&state, (1, 1));

            assert_eq!(state, original);
        }
    }
}
