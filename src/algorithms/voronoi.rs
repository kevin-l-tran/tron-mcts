use std::thread;

use crate::{
    algorithms::bfs::bfs_distances,
    engine::{grid::Grid, state::GameState},
};

pub fn voronoi_territory(state: &GameState) -> Vec<f64> {
    let mut scores = vec![0.0; state.players.len()];

    let worker_count = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(state.players.len());

    let chunk_size = state.players.len().div_ceil(worker_count);

    let territory_grids: Vec<Grid<usize>> = thread::scope(|scope| {
        let handles: Vec<_> = state
            .players
            .chunks(chunk_size)
            .map(|players| {
                scope.spawn(move || {
                    players
                        .iter()
                        .map(|player| {
                            if player.eliminated_at.is_some() {
                                Grid::filled(state.arena.width, state.arena.height, usize::MAX)
                            } else {
                                bfs_distances(state, (player.x, player.y))
                            }
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("BFS worker panicked"))
            .collect()
    });

    for i in 0..state.arena.size() {
        let grid_i = state.arena.index(i);

        if state.is_occupied(grid_i.0, grid_i.1) {
            continue;
        }

        let mut lowest_score = usize::MAX;
        let mut tie_count = 0;

        for grid in &territory_grids {
            if grid[grid_i] < lowest_score {
                lowest_score = grid[grid_i];
                tie_count = 1;
            } else if grid[grid_i] == lowest_score {
                tie_count += 1;
            }
        }

        // continue when the square is reachable by no one
        if lowest_score == usize::MAX {
            continue;
        }

        let territory_score = 1.0 / tie_count as f64;

        for (j, score) in scores.iter_mut().enumerate() {
            if territory_grids[j][grid_i] == lowest_score {
                *score += territory_score;
            }
        }
    }

    scores
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::engine::{
        action::Direction::East,
        grid::Grid,
        state::{GameState, Spawn},
    };

    use super::*;

    fn setup_game_state(arena: Grid<bool>, spawns: &[Spawn]) -> GameState {
        GameState::new(Arc::new(arena), spawns.to_vec()).unwrap()
    }

    mod voronoi_territory {
        use super::*;

        #[test]
        fn assigns_all_reachable_empty_cells_to_single_player() {
            let arena = Grid::filled(3, 3, false);
            let spawns = [Spawn {
                x: 1,
                y: 1,
                direction: East,
            }];
            let state = setup_game_state(arena, &spawns);

            // eight empty cells; the player's occupied starting cell is excluded.
            assert_eq!(voronoi_territory(&state), vec![8.0]);
        }

        #[test]
        fn splits_tied_cells_evenly_between_players() {
            let arena = Grid::filled(3, 3, false);
            let spawns = [
                Spawn {
                    x: 0,
                    y: 1,
                    direction: East,
                },
                Spawn {
                    x: 2,
                    y: 1,
                    direction: East,
                },
            ];
            let state = setup_game_state(arena, &spawns);

            // each player owns two cells and half of the three middle-column cells.
            assert_eq!(voronoi_territory(&state), vec![3.5, 3.5]);
        }

        #[test]
        fn respects_walls_separating_player_territories() {
            let mut arena = Grid::filled(5, 3, false);
            for y in 0..3 {
                arena[(2, y)] = true;
            }
            let spawns = [
                Spawn {
                    x: 0,
                    y: 1,
                    direction: East,
                },
                Spawn {
                    x: 4,
                    y: 1,
                    direction: East,
                },
            ];
            let state = setup_game_state(arena, &spawns);

            assert_eq!(voronoi_territory(&state), vec![5.0, 5.0]);
        }

        #[test]
        fn excludes_obstacles_and_existing_trails() {
            let mut arena = Grid::filled(3, 3, false);
            arena[(0, 0)] = true;
            let spawns = [Spawn {
                x: 1,
                y: 1,
                direction: East,
            }];
            let mut state = setup_game_state(arena, &spawns);
            state.trails[(2, 2)] = 1;

            assert_eq!(voronoi_territory(&state), vec![6.0]);
        }

        #[test]
        fn ignores_eliminated_players() {
            let arena = Grid::filled(3, 3, false);
            let spawns = [
                Spawn {
                    x: 0,
                    y: 1,
                    direction: East,
                },
                Spawn {
                    x: 2,
                    y: 1,
                    direction: East,
                },
            ];
            let mut state = setup_game_state(arena, &spawns);
            state.players[1].eliminated_at = Some(1);
            state.alive_count = 1;

            // eliminated player's last occupied cell remains blocked.
            assert_eq!(voronoi_territory(&state), vec![7.0, 0.0]);
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

            let _ = voronoi_territory(&state);

            assert_eq!(state, original);
        }
    }
}
