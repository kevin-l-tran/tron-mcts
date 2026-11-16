use std::sync::Arc;

use crate::engine::{action::Direction, grid::Grid};

pub type Arena = Grid<bool>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Spawn {
    pub x: usize,
    pub y: usize,
    pub direction: Direction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct PlayerState {
    x: usize,
    y: usize,
    direction: Direction,
    eliminated_at: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum GameStateError {
    SpawnOutOfArena(Spawn),
    SpawnOnObstacle(Spawn),
    DuplicateSpawnLocation(Spawn, Spawn),
    TooManyPlayers(usize),
    InvalidPlayerIndex(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameState {
    arena: Arc<Arena>,
    players: Vec<PlayerState>,
    trails: Grid<u8>,
    tick: usize,
    alive_count: usize,
}

impl GameState {
    pub fn new(arena: Arc<Arena>, spawns: Vec<Spawn>) -> Result<GameState, GameStateError> {
        if u8::try_from(spawns.len()).is_err() {
            return Err(GameStateError::TooManyPlayers(spawns.len()));
        }

        let mut trails = Grid::filled(arena.width, arena.height, 0u8);

        for (index, spawn) in spawns.iter().enumerate() {
            let is_obstacle = arena.get(spawn.x, spawn.y);

            match is_obstacle {
                None => return Err(GameStateError::SpawnOutOfArena(*spawn)),
                Some(true) => return Err(GameStateError::SpawnOnObstacle(*spawn)),
                Some(false) => {}
            }

            let existing_trail = trails[(spawn.x, spawn.y)];
            if existing_trail > 0 {
                let duplicate_spawn = spawns[existing_trail as usize - 1];
                return Err(GameStateError::DuplicateSpawnLocation(
                    duplicate_spawn,
                    *spawn,
                ));
            }

            trails[(spawn.x, spawn.y)] = (index + 1) as u8;
        }

        let players = spawns
            .iter()
            .map(|spawn| PlayerState {
                x: spawn.x,
                y: spawn.y,
                direction: spawn.direction,
                eliminated_at: None,
            })
            .collect::<Vec<PlayerState>>();

        Ok(GameState {
            alive_count: players.len(),
            arena,
            players,
            trails,
            tick: 0,
        })
    }

    pub fn legal_actions(&self, id: usize) -> Result<Vec<Direction>, GameStateError> {
        let player = match self.players.get(id) {
            Some(player) => player,
            None => return Err(GameStateError::InvalidPlayerIndex(id)),
        };

        if player.eliminated_at.is_some() {
            return Ok(vec![]);
        }

        let mut legal_directions = vec![];

        for direction in Direction::VALUES {
            if direction == -player.direction {
                continue;
            }

            let (dx, dy) = direction.delta();
            let Some(player_x) = player.x.checked_add_signed(dx) else {
                continue;
            };
            let Some(player_y) = player.y.checked_add_signed(dy) else {
                continue;
            };

            if !self.arena.in_bounds(player_x, player_y) || self.is_occupied(player_x, player_y) {
                continue;
            }

            legal_directions.push(direction);
        }

        Ok(legal_directions)
    }

    pub fn alive_players() -> Vec<PlayerState> {
        todo!()
    }

    pub fn step() {
        todo!()
    }

    pub fn undo_step() {
        todo!()
    }

    pub fn is_terminal() -> bool {
        todo!()
    }

    pub fn outcome() -> Vec<(PlayerState, usize)> {
        todo!()
    }

    fn trail_owner(&self, x: usize, y: usize) -> Option<usize> {
        if self.trails[(x, y)] == 0 {
            None
        } else {
            Some(self.trails[(x, y)] as usize - 1)
        }
    }

    fn set_trail(&mut self, x: usize, y: usize, id: usize) {
        debug_assert!(id < self.players.len());
        self.trails[(x, y)] = (id + 1) as u8;
    }

    fn is_occupied(&self, x: usize, y: usize) -> bool {
        self.trails[(x, y)] != 0 || self.arena[(x, y)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod new {
        use crate::engine::action::Direction::{East, South};

        use super::*;

        #[test]
        fn accepts_valid_arena_and_spans() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![Spawn {
                x: 0,
                y: 0,
                direction: South,
            }];

            assert!(GameState::new(Arc::new(arena), spawns).is_ok());
        }

        #[test]
        fn initializes_players_correctly() {
            let num_players = 12 as u8;
            let arena = Grid::filled(12, 12, false);

            let spawns = (0..num_players)
                .map(|i| Spawn {
                    x: i as usize,
                    y: i as usize,
                    direction: South,
                })
                .collect();

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            for index in 0..num_players as usize {
                assert_eq!(
                    game_state.players[index],
                    PlayerState {
                        x: index,
                        y: index,
                        direction: South,
                        eliminated_at: None,
                    }
                );
            }
        }

        #[test]
        fn initializes_alive_count_correctly() {
            let num_players = 12 as u8;
            let arena = Grid::filled(12, 12, false);

            let spawns = (0..num_players)
                .map(|i| Spawn {
                    x: i as usize,
                    y: i as usize,
                    direction: South,
                })
                .collect();

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert_eq!(game_state.alive_count, num_players as usize);
        }

        #[test]
        fn initializes_spawns_correctly() {
            let num_players = 12 as u8;
            let arena = Grid::filled(12, 12, false);

            let spawns = (0..num_players)
                .map(|i| Spawn {
                    x: i as usize,
                    y: i as usize,
                    direction: South,
                })
                .collect();

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            for index in 0..num_players as usize {
                assert_eq!(game_state.trails[(index, index)], index as u8 + 1);
            }
        }

        #[test]
        fn accepts_at_most_255_players() {
            let num_players = u8::MAX as usize; // 255 players
            let arena = Grid::filled(16, 16, false);

            let spawns = (0..num_players)
                .map(|i| Spawn {
                    x: i % 16,
                    y: i / 16,
                    direction: South,
                })
                .collect();

            assert!(GameState::new(Arc::new(arena), spawns).is_ok());
        }

        #[test]
        fn rejects_too_many_players() {
            let num_players = u8::MAX as usize + 1; // 256 players
            let arena = Grid::filled(16, 16, false);

            let spawns = (0..num_players)
                .map(|i| Spawn {
                    x: i % 16,
                    y: i / 16,
                    direction: South,
                })
                .collect();

            assert_eq!(
                GameState::new(Arc::new(arena), spawns),
                Err(GameStateError::TooManyPlayers(num_players))
            );
        }

        #[test]
        fn rejects_out_of_arena_spawns() {
            let outside_locations = [(4, 0), (0, 4), (4, 4)];

            let arena = Arc::new(Grid::filled(4, 4, false));

            for outside_location in outside_locations {
                let outside_spawn = Spawn {
                    x: outside_location.0,
                    y: outside_location.1,
                    direction: South,
                };

                let spawns = vec![outside_spawn];

                assert_eq!(
                    GameState::new(Arc::clone(&arena), spawns),
                    Err(GameStateError::SpawnOutOfArena(outside_spawn)),
                );
            }
        }

        #[test]
        fn rejects_spawns_on_obstacles() {
            let mut arena = Grid::filled(4, 4, false);
            arena[(0, 0)] = true;

            let obstacle_spawn = Spawn {
                x: 0,
                y: 0,
                direction: South,
            };

            let spawns = vec![obstacle_spawn];

            assert_eq!(
                GameState::new(Arc::new(arena), spawns),
                Err(GameStateError::SpawnOnObstacle(obstacle_spawn)),
            );
        }

        #[test]
        fn rejects_duplicate_spawn_locations() {
            let arena = Grid::filled(4, 4, false);

            let spawn_1 = Spawn {
                x: 0,
                y: 0,
                direction: South,
            };

            let spawn_2 = Spawn {
                x: 0,
                y: 0,
                direction: East,
            };

            let spawns = vec![spawn_1, spawn_2];

            assert_eq!(
                GameState::new(Arc::new(arena), spawns),
                Err(GameStateError::DuplicateSpawnLocation(spawn_1, spawn_2)),
            );
        }
    }

    mod legal_actions {
        use crate::engine::action::Direction::{East, North, South, West};

        use super::*;

        #[test]
        fn returns_all_valid_directions() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![Spawn {
                x: 1,
                y: 1,
                direction: South,
            }];

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert_eq!(game_state.legal_actions(0).unwrap(), [South, East, West]);
        }

        #[test]
        fn does_not_include_backwards_direction() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![Spawn {
                x: 1,
                y: 1,
                direction: South,
            }];

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert!(!game_state.legal_actions(0).unwrap().contains(&North));
        }

        #[test]
        fn does_not_include_directions_going_out_of_arena() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![Spawn {
                x: 1,
                y: 0,
                direction: East,
            }];

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert!(!game_state.legal_actions(0).unwrap().contains(&North));
        }

        #[test]
        fn does_not_include_directions_going_into_arena_obstacle() {
            let mut arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![Spawn {
                x: 1,
                y: 1,
                direction: South,
            }];

            arena[(2, 1)] = true;

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert!(!game_state.legal_actions(0).unwrap().contains(&East));
        }

        #[test]
        fn does_not_include_directions_going_into_trails() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![
                Spawn {
                    x: 1,
                    y: 1,
                    direction: South,
                },
                Spawn {
                    x: 2,
                    y: 2,
                    direction: South,
                },
            ];

            let mut game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            game_state.trails[(0, 1)] = 1;
            game_state.trails[(2, 1)] = 2;

            assert!(!game_state.legal_actions(0).unwrap().contains(&East));
            assert!(!game_state.legal_actions(0).unwrap().contains(&West));
        }

        #[test]
        fn returns_empty_array_for_trapped_player() {
            let mut arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![Spawn {
                x: 1,
                y: 1,
                direction: South,
            }];

            arena[(0, 1)] = true;
            arena[(1, 0)] = true;
            arena[(2, 1)] = true;
            arena[(1, 2)] = true;

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert_eq!(game_state.legal_actions(0).unwrap(), []);
        }

        #[test]
        fn returns_empty_array_for_dead_player() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![Spawn {
                x: 0,
                y: 0,
                direction: South,
            }];

            let mut game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            game_state.players[0].eliminated_at = Some(0);

            assert_eq!(game_state.legal_actions(0), Ok(vec![]));
        }

        #[test]
        fn rejects_invalid_player_id() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![];

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert_eq!(
                game_state.legal_actions(0),
                Err(GameStateError::InvalidPlayerIndex(0))
            );
        }
    }
}
