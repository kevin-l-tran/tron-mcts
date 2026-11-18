use std::sync::Arc;

use crate::engine::{
    action::{Action, Direction},
    grid::Grid,
};

pub type Arena = Grid<bool>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Spawn {
    pub x: usize,
    pub y: usize,
    pub direction: Direction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct PlayerState {
    pub x: usize,
    pub y: usize,
    pub direction: Direction,
    pub eliminated_at: Option<usize>,
}

pub type StepInput<'a> = &'a [Option<Action>];

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct UndoRecord {
    prev_directions: Vec<Direction>,
    created_collisions: Vec<(usize, usize)>,
    tick: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum GameStateError {
    SpawnOutOfArena(Spawn),
    SpawnOnObstacle(Spawn),
    DuplicateSpawnLocation(Spawn, Spawn),
    TooManyPlayers(usize),
    InvalidPlayerIndex(usize),
    ExpectedNActions(usize),
    ArenaTooLarge(usize, usize),
    GameJustStarted,
    GameTerminated,
    InvalidUndoRecord,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameState {
    pub arena: Arc<Arena>,
    pub players: Vec<PlayerState>,
    pub trails: Grid<u8>,
    pub tick: usize,
    pub alive_count: usize,
}

impl GameState {
    pub fn new(arena: Arc<Arena>, spawns: Vec<Spawn>) -> Result<GameState, GameStateError> {
        if spawns.len() >= u8::MAX as usize {
            return Err(GameStateError::TooManyPlayers(spawns.len()));
        }

        // should be impossible to construct an arena this large since the vector will overflow
        if isize::try_from(arena.width).is_err() || isize::try_from(arena.height).is_err() {
            return Err(GameStateError::ArenaTooLarge(arena.width, arena.height));
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

    pub fn safe_actions(&self, id: usize) -> Result<Vec<Action>, GameStateError> {
        let player = match self.players.get(id) {
            Some(player) => player,
            None => return Err(GameStateError::InvalidPlayerIndex(id)),
        };

        if player.eliminated_at.is_some() {
            return Ok(vec![]);
        }

        let mut safe_actions = vec![];

        for action in Action::VALUES {
            let direction = action.apply_to_direction(player.direction);
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

            safe_actions.push(action);
        }

        Ok(safe_actions)
    }

    pub fn alive_players(&self) -> impl Iterator<Item = (usize, &PlayerState)> {
        self.players
            .iter()
            .enumerate()
            .filter(|p| p.1.eliminated_at.is_none())
    }

    pub fn step(&mut self, actions: StepInput) -> Result<UndoRecord, GameStateError> {
        if actions.len() != self.players.len() {
            return Err(GameStateError::ExpectedNActions(self.players.len()));
        }

        // 1. update game tick
        self.tick += 1;

        let mut record = UndoRecord {
            prev_directions: self.players.iter().map(|p| p.direction).collect(),
            created_collisions: vec![],
            tick: self.tick,
        };

        // None if position is out of bounds, Some((usize, usize)) otherwise
        let mut final_positions = vec![];

        for (id, action) in actions.iter().enumerate() {
            let player = &mut self.players[id];
            let resolved_action = action.unwrap_or(Action::Straight);

            if player.eliminated_at.is_some() {
                final_positions.push(Some((player.x, player.y)));
                continue;
            }

            // 2. set player direction
            let resolved_direction = resolved_action.apply_to_direction(player.direction);
            player.direction = resolved_direction;

            // 3. push None if out of bounds, else Some((player_x, player_y))
            let (dx, dy) = resolved_direction.delta();

            // if addition results in signed errors, player is out of arena
            let Some(player_x) = player.x.checked_add_signed(dx) else {
                final_positions.push(None);
                continue;
            };
            let Some(player_y) = player.y.checked_add_signed(dy) else {
                final_positions.push(None);
                continue;
            };

            if !self.arena.in_bounds(player_x, player_y) {
                final_positions.push(None);
                continue;
            }

            final_positions.push(Some((player_x, player_y)));
        }

        // 4. add collisions to trails and record if players overlap positions
        for i in 1..final_positions.len() {
            let Some((x, y)) = final_positions[i - 1] else {
                continue;
            };

            if final_positions[i..].contains(&Some((x, y))) && !self.is_occupied(x, y) {
                self.trails[(x, y)] = u8::MAX;
                record.created_collisions.push((x, y));
            }
        }

        for id in 0..final_positions.len() {
            // skip already eliminated players
            if self.players[id].eliminated_at.is_some() {
                continue;
            }

            // 5. for each player, eliminate when out of bounds or colliding with an object
            let Some((x, y)) = final_positions[id] else {
                self.players[id].eliminated_at = Some(self.tick);
                continue;
            };

            if self.is_occupied(x, y) {
                self.players[id].eliminated_at = Some(self.tick);
                continue;
            }

            // 6. update the player's current position
            self.players[id].x = x;
            self.players[id].y = y;

            // 7. add a trail at the player's current position
            self.set_trail(x, y, id);
        }

        // 8. update alive count
        self.alive_count = self.alive_players().count();

        Ok(record)
    }

    pub fn undo_step(&mut self, record: UndoRecord) -> Result<(), GameStateError> {
        if self.tick == 0 {
            return Err(GameStateError::GameJustStarted);
        }

        if record.tick != self.tick {
            return Err(GameStateError::InvalidUndoRecord);
        }

        // 1. clear created collision trails
        for (x, y) in record.created_collisions {
            self.trails[(x, y)] = 0;
        }

        for player in &mut self.players {
            let elimination_status = player.eliminated_at;

            // if player was eliminated
            if elimination_status.is_some() {
                // ...was just eliminated
                if elimination_status.unwrap() == self.tick {
                    // 2. set eliminated_at to None
                    player.eliminated_at = None;
                }
            // if player is alive
            } else {
                // 2. remove trail at current position
                self.trails[(player.x, player.y)] = 0;

                let (dx, dy) = player.direction.delta();
                let past_x = player
                    .x
                    .checked_sub_signed(dx)
                    .expect("player should have valid prior x position");
                let past_y = player
                    .y
                    .checked_sub_signed(dy)
                    .expect("player should have valid prior y position");

                // 3. set current position to previous position
                player.x = past_x;
                player.y = past_y;
            }
        }

        for (i, player) in self.players.iter_mut().enumerate() {
            // 4. set direction to previous direction in record
            player.direction = record.prev_directions[i];
        }

        // 5. decrement tick
        self.tick -= 1;

        // 6. update alive count
        self.alive_count = self.alive_players().count();

        Ok(())
    }

    pub fn is_terminal(&self) -> bool {
        self.alive_count <= 1
    }

    pub fn outcome(&self) -> Option<Vec<usize>> {
        if !self.is_terminal() {
            return None;
        }

        let ranks = self
            .players
            .iter()
            .map(|player| {
                let elimination_at = player.eliminated_at.unwrap_or(usize::MAX);

                1 + self
                    .players
                    .iter()
                    .filter(|other| other.eliminated_at.unwrap_or(usize::MAX) > elimination_at)
                    .count()
            })
            .collect();

        Some(ranks)
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
        self.arena.in_bounds(x, y) && (self.trails[(x, y)] != 0 || self.arena[(x, y)])
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
        fn accepts_at_most_254_players() {
            let num_players = u8::MAX as usize - 1; // 254 players
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
            let num_players = u8::MAX as usize; // 255 players
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

    mod safe_actions {
        use crate::engine::action::{
            Action::{Left, Right, Straight},
            Direction::{East, South},
        };

        use super::*;

        #[test]
        fn returns_all_safe_actions() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![Spawn {
                x: 1,
                y: 1,
                direction: South,
            }];

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert_eq!(game_state.safe_actions(0).unwrap(), [Straight, Left, Right]);
        }

        #[test]
        fn does_not_include_actions_going_out_of_arena() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![Spawn {
                x: 1,
                y: 0,
                direction: East,
            }];

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert!(!game_state.safe_actions(0).unwrap().contains(&Left));
        }

        #[test]
        fn does_not_include_actions_going_into_arena_obstacle() {
            let mut arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![Spawn {
                x: 1,
                y: 1,
                direction: South,
            }];

            arena[(2, 1)] = true;

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert!(!game_state.safe_actions(0).unwrap().contains(&Left));
        }

        #[test]
        fn does_not_include_actions_going_into_trails() {
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

            assert!(!game_state.safe_actions(0).unwrap().contains(&Left));
            assert!(!game_state.safe_actions(0).unwrap().contains(&Right));
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

            assert_eq!(game_state.safe_actions(0).unwrap(), []);
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

            assert_eq!(game_state.safe_actions(0), Ok(vec![]));
        }

        #[test]
        fn rejects_invalid_player_id() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![];

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert_eq!(
                game_state.safe_actions(0),
                Err(GameStateError::InvalidPlayerIndex(0))
            );
        }
    }

    mod alive_players {
        use crate::engine::action::Direction::South;

        use super::*;

        #[test]
        fn returns_all_when_none_eliminated() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![
                Spawn {
                    x: 0,
                    y: 0,
                    direction: South,
                },
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
                Spawn {
                    x: 3,
                    y: 3,
                    direction: South,
                },
            ];

            let game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            assert_eq!(
                game_state
                    .alive_players()
                    .map(|(i, _)| i)
                    .collect::<Vec<usize>>(),
                vec![0, 1, 2, 3]
            )
        }

        #[test]
        fn returns_only_alive_players() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![
                Spawn {
                    x: 0,
                    y: 0,
                    direction: South,
                },
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
                Spawn {
                    x: 3,
                    y: 3,
                    direction: South,
                },
            ];

            let mut game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            game_state.players[2].eliminated_at = Some(0);

            assert_eq!(
                game_state
                    .alive_players()
                    .map(|(i, _)| i)
                    .collect::<Vec<usize>>(),
                vec![0, 1, 3]
            )
        }

        #[test]
        fn returns_empty_when_all_eliminated() {
            let arena = Grid::new(4, 4, vec![false; 16]);
            let spawns = vec![
                Spawn {
                    x: 0,
                    y: 0,
                    direction: South,
                },
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
                Spawn {
                    x: 3,
                    y: 3,
                    direction: South,
                },
            ];

            let mut game_state = GameState::new(Arc::new(arena), spawns).unwrap();

            game_state.players[0].eliminated_at = Some(0);
            game_state.players[1].eliminated_at = Some(0);
            game_state.players[2].eliminated_at = Some(0);
            game_state.players[3].eliminated_at = Some(0);

            assert_eq!(
                game_state
                    .alive_players()
                    .map(|(i, _)| i)
                    .collect::<Vec<usize>>(),
                vec![]
            )
        }
    }

    mod step {
        use crate::engine::action::{
            Action::{Left, Right, Straight},
            Direction::{East, North, South, West},
        };

        use super::*;

        static DEFAULT_ARENA_CELLS: [bool; 16] = [false; 16];

        static DEFAULT_SPAWNS: [Spawn; 4] = [
            Spawn {
                x: 0,
                y: 0,
                direction: South,
            },
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
            Spawn {
                x: 3,
                y: 3,
                direction: South,
            },
        ];

        static SURVIVAL_SPAWNS: [Spawn; 4] = [
            Spawn {
                x: 0,
                y: 0,
                direction: South,
            },
            Spawn {
                x: 1,
                y: 1,
                direction: East,
            },
            Spawn {
                x: 2,
                y: 2,
                direction: West,
            },
            Spawn {
                x: 3,
                y: 3,
                direction: North,
            },
        ];

        static CORNER_SPAWNS: [Spawn; 4] = [
            Spawn {
                x: 0,
                y: 0,
                direction: North,
            },
            Spawn {
                x: 3,
                y: 0,
                direction: East,
            },
            Spawn {
                x: 3,
                y: 3,
                direction: South,
            },
            Spawn {
                x: 0,
                y: 3,
                direction: West,
            },
        ];

        fn setup_game_state(spawns: &[Spawn], arena_cells: &[bool]) -> GameState {
            let arena = Grid::new(4, 4, Vec::from(arena_cells));
            GameState::new(Arc::new(arena), Vec::from(spawns)).unwrap()
        }

        #[test]
        fn moves_players_in_given_actions() {
            let game_state = &mut setup_game_state(&DEFAULT_SPAWNS, &DEFAULT_ARENA_CELLS);
            let actions = [Some(Straight), Some(Left), Some(Right), Some(Right)];

            let final_positions = vec![(0, 1), (2, 1), (1, 2), (2, 3)];
            let _ = game_state.step(&actions);

            assert_eq!(
                game_state
                    .players
                    .iter()
                    .map(|p| (p.x, p.y))
                    .collect::<Vec<(usize, usize)>>(),
                final_positions
            )
        }

        #[test]
        fn moves_players_forwards_when_no_movement_is_provided() {
            let game_state = &mut setup_game_state(&SURVIVAL_SPAWNS, &DEFAULT_ARENA_CELLS);
            let actions = [None; 4];

            let final_positions = vec![(0, 1), (2, 1), (1, 2), (3, 2)];
            let _ = game_state.step(&actions);

            assert_eq!(
                game_state
                    .players
                    .iter()
                    .map(|p| (p.x, p.y))
                    .collect::<Vec<(usize, usize)>>(),
                final_positions
            )
        }

        #[test]
        fn increments_tick() {
            let game_state = &mut setup_game_state(&DEFAULT_SPAWNS, &DEFAULT_ARENA_CELLS);
            let actions = [None; 4];

            let _ = game_state.step(&actions);

            assert_eq!(game_state.tick, 1);
        }

        #[test]
        fn updates_player_directions() {
            let game_state = &mut setup_game_state(&DEFAULT_SPAWNS, &DEFAULT_ARENA_CELLS);
            let actions = [Some(Straight), Some(Left), Some(Right), Some(Right)];

            let _ = game_state.step(&actions);

            assert_eq!(
                game_state
                    .players
                    .iter()
                    .map(|p| p.direction)
                    .collect::<Vec<Direction>>(),
                [South, East, West, West]
            )
        }

        #[test]
        fn add_trails_to_new_position() {
            let game_state = &mut setup_game_state(&DEFAULT_SPAWNS, &DEFAULT_ARENA_CELLS);
            let actions = [Some(Straight), Some(Left), Some(Right), Some(Right)];

            let trails = &mut game_state.trails.clone();
            let new_trail_positions = vec![(0, 1), (2, 1), (1, 2), (2, 3)];

            for (i, (x, y)) in new_trail_positions.iter().enumerate() {
                trails[(*x, *y)] = (i + 1) as u8;
            }

            let _ = game_state.step(&actions);

            assert_eq!(game_state.trails, *trails)
        }

        #[test]
        fn returns_undo_record_containing_previous_directions_and_current_tick() {
            let game_state = &mut setup_game_state(&DEFAULT_SPAWNS, &DEFAULT_ARENA_CELLS);
            let actions = [Some(Straight), Some(Left), Some(Right), Some(Right)];

            assert_eq!(
                game_state.step(&actions),
                Ok(UndoRecord {
                    prev_directions: vec![South, South, South, South],
                    created_collisions: vec![],
                    tick: 1,
                })
            )
        }

        #[test]
        fn eliminates_players_at_arena_boundary() {
            let game_state = &mut setup_game_state(&CORNER_SPAWNS, &DEFAULT_ARENA_CELLS);
            let actions = [None; 4];

            let _ = game_state.step(&actions);

            assert_eq!(
                game_state
                    .players
                    .iter()
                    .map(|p| p.eliminated_at)
                    .collect::<Vec<Option<usize>>>(),
                vec![Some(1), Some(1), Some(1), Some(1)]
            )
        }

        #[test]
        fn eliminates_players_at_obstacles() {
            let cells = [
                false, false, false, false, true, false, true, false, false, true, false, true,
                false, false, false, false,
            ];

            let game_state = &mut setup_game_state(&SURVIVAL_SPAWNS, &cells);
            let actions = [None; 4];

            let _ = game_state.step(&actions);

            assert_eq!(
                game_state
                    .players
                    .iter()
                    .map(|p| p.eliminated_at)
                    .collect::<Vec<Option<usize>>>(),
                vec![Some(1), Some(1), Some(1), Some(1)]
            )
        }

        #[test]
        fn eliminates_players_at_other_trails() {
            let game_state = &mut setup_game_state(&CORNER_SPAWNS, &DEFAULT_ARENA_CELLS);
            let actions = [Some(Right); 4];

            let _ = game_state.step(&actions);
            let _ = game_state.step(&[None; 4]);
            let _ = game_state.step(&[None; 4]);

            assert_eq!(
                game_state
                    .players
                    .iter()
                    .map(|p| p.eliminated_at)
                    .collect::<Vec<Option<usize>>>(),
                vec![Some(3), Some(3), Some(3), Some(3)]
            )
        }

        #[test]
        fn eliminates_players_at_own_trails() {
            let spawns = [Spawn {
                x: 1,
                y: 1,
                direction: East,
            }];
            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);

            let _ = game_state.step(&[None]);
            let _ = game_state.step(&[Some(Right)]);
            let _ = game_state.step(&[Some(Right)]);
            let _ = game_state.step(&[Some(Right)]);

            assert_eq!(game_state.players[0].eliminated_at, Some(4));
        }

        #[test]
        fn preserves_previous_trails() {
            let game_state = &mut setup_game_state(&DEFAULT_SPAWNS, &DEFAULT_ARENA_CELLS);
            let actions = [Some(Straight), Some(Left), Some(Right), Some(Right)];

            let _ = game_state.step(&actions);

            for (i, spawn) in DEFAULT_SPAWNS.iter().enumerate() {
                assert_eq!(game_state.trails[(spawn.x, spawn.y)], (i + 1) as u8);
            }
        }

        #[test]
        fn eliminates_two_players_on_head_on_collision() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
            ];

            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let _ = game_state.step(&[None; 2]);

            assert_eq!(game_state.players[0].eliminated_at, Some(1));
            assert_eq!(game_state.players[1].eliminated_at, Some(1));
            assert_eq!(game_state.trails[(2, 2)], u8::MAX);
            assert_eq!(game_state.alive_count, 0);
        }

        #[test]
        fn eliminates_three_players_on_head_on_collision() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
                Spawn {
                    x: 2,
                    y: 1,
                    direction: South,
                },
            ];

            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let _ = game_state.step(&[None; 3]);

            assert_eq!(
                game_state
                    .players
                    .iter()
                    .map(|p| p.eliminated_at)
                    .collect::<Vec<Option<usize>>>(),
                vec![Some(1), Some(1), Some(1)]
            );
            assert_eq!(game_state.trails[(2, 2)], u8::MAX);
            assert_eq!(game_state.alive_count, 0);
        }

        #[test]
        fn preserves_existing_trail_on_head_on_collision() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
                Spawn {
                    x: 2,
                    y: 2,
                    direction: North,
                },
            ];

            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let _ = game_state.step(&[None; 3]);

            assert_eq!(game_state.players[0].eliminated_at, Some(1));
            assert_eq!(game_state.players[1].eliminated_at, Some(1));
            assert_eq!(game_state.players[2].eliminated_at, None);
            assert_eq!(game_state.trails[(2, 2)], 3);
            assert_eq!(game_state.trails[(2, 1)], 3);
            assert_eq!(game_state.alive_count, 1);
        }

        #[test]
        fn saves_created_collision_squares_in_undo_record() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
            ];

            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);

            assert_eq!(
                game_state.step(&[None; 2]),
                Ok(UndoRecord {
                    prev_directions: vec![East, West],
                    created_collisions: vec![(2, 2)],
                    tick: 1,
                })
            );
        }

        #[test]
        fn does_not_create_trail_on_head_on_collision_with_obstacle() {
            let mut cells = DEFAULT_ARENA_CELLS;
            cells[2 + 2 * 4] = true;

            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
            ];

            let game_state = &mut setup_game_state(&spawns, &cells);
            let _ = game_state.step(&[None; 2]);

            assert_eq!(game_state.players[0].eliminated_at, Some(1));
            assert_eq!(game_state.players[1].eliminated_at, Some(1));
            assert_eq!(game_state.trails[(2, 2)], 0);
            assert_eq!(game_state.arena[(2, 2)], true);
        }

        #[test]
        fn eliminates_players_swapping_positions() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 1,
                    direction: East,
                },
                Spawn {
                    x: 2,
                    y: 1,
                    direction: West,
                },
            ];

            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_trails = game_state.trails.clone();
            let _ = game_state.step(&[None; 2]);

            assert_eq!(game_state.players[0].eliminated_at, Some(1));
            assert_eq!(game_state.players[1].eliminated_at, Some(1));
            assert_eq!((game_state.players[0].x, game_state.players[0].y), (1, 1));
            assert_eq!((game_state.players[1].x, game_state.players[1].y), (2, 1));
            assert_eq!(game_state.trails, old_trails);
        }

        #[test]
        fn does_not_move_eliminated_players() {
            let spawns = [Spawn {
                x: 1,
                y: 0,
                direction: North,
            }];

            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let _ = game_state.step(&[None]);
            let eliminated_player = game_state.players[0];

            let _ = game_state.step(&[Some(Left)]);
            let _ = game_state.step(&[Some(Right)]);

            assert_eq!(game_state.players[0], eliminated_player);
            assert_eq!(game_state.tick, 3);
        }

        #[test]
        fn ignores_actions_for_eliminated_players() {
            let spawns = [Spawn {
                x: 1,
                y: 0,
                direction: North,
            }];

            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let _ = game_state.step(&[None]);
            let _ = game_state.step(&[Some(Left)]);

            assert_eq!(game_state.players[0].direction, North);
            assert_eq!(game_state.players[0].eliminated_at, Some(1));
        }

        #[test]
        fn preserves_elimination_tick() {
            let spawns = [Spawn {
                x: 1,
                y: 0,
                direction: North,
            }];

            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let _ = game_state.step(&[None]);
            let _ = game_state.step(&[None]);
            let _ = game_state.step(&[None]);

            assert_eq!(game_state.players[0].eliminated_at, Some(1));
            assert_eq!(game_state.alive_count, 0);
        }

        #[test]
        fn does_not_add_trails_for_eliminated_players() {
            let spawns = [Spawn {
                x: 1,
                y: 0,
                direction: North,
            }];

            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_trails = game_state.trails.clone();
            let _ = game_state.step(&[None]);
            let _ = game_state.step(&[Some(Left)]);

            assert_eq!(game_state.trails, old_trails);
        }

        #[test]
        fn preserves_eliminated_player_trail_on_collision() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 0,
                    direction: North,
                },
                Spawn {
                    x: 3,
                    y: 1,
                    direction: West,
                },
            ];

            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let _ = game_state.step(&[None; 2]);
            let _ = game_state.step(&[None, Some(Right)]);
            let _ = game_state.step(&[None, Some(Left)]);

            assert_eq!(game_state.players[0].eliminated_at, Some(1));
            assert_eq!(game_state.players[1].eliminated_at, Some(3));
            assert_eq!(game_state.trails[(1, 0)], 1);
            assert_eq!(game_state.alive_count, 0);
        }

        #[test]
        fn updates_alive_count_after_eliminations() {
            let game_state = &mut setup_game_state(&SURVIVAL_SPAWNS, &DEFAULT_ARENA_CELLS);

            assert_eq!(game_state.alive_count, 4);

            let _ = game_state.step(&[Some(Right), None, None, None]);
            assert_eq!(game_state.alive_count, 3);

            let _ = game_state.step(&[None, Some(Right), None, None]);
            assert_eq!(game_state.alive_count, 2);

            assert_eq!(
                game_state
                    .players
                    .iter()
                    .map(|p| p.eliminated_at)
                    .collect::<Vec<Option<usize>>>(),
                vec![Some(1), Some(2), None, None]
            );
        }

        #[test]
        fn does_not_eliminate_uninvolved_players_on_head_on_collision() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
                Spawn {
                    x: 0,
                    y: 0,
                    direction: South,
                },
            ];

            let game_state = &mut setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let _ = game_state.step(&[None; 3]);

            assert_eq!(game_state.players[0].eliminated_at, Some(1));
            assert_eq!(game_state.players[1].eliminated_at, Some(1));
            assert_eq!(game_state.players[2].eliminated_at, None);
            assert_eq!((game_state.players[2].x, game_state.players[2].y), (0, 1));
            assert_eq!(game_state.alive_count, 1);
        }

        #[test]
        fn errors_if_actions_is_not_length_of_players() {
            let mut game_state = setup_game_state(&DEFAULT_SPAWNS, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            assert_eq!(
                game_state.step(&[]),
                Err(GameStateError::ExpectedNActions(4))
            );
            assert_eq!(
                game_state.step(&[
                    Some(Straight),
                    Some(Straight),
                    Some(Straight),
                    Some(Straight),
                    Some(Straight)
                ]),
                Err(GameStateError::ExpectedNActions(4))
            );
            assert_eq!(
                game_state.step(&[
                    None,
                    Some(Straight),
                    Some(Straight),
                    Some(Straight),
                    Some(Straight)
                ]),
                Err(GameStateError::ExpectedNActions(4))
            );

            assert_eq!(game_state, old_state);
        }
    }

    mod undo_step {
        use crate::engine::action::{
            Action::{Left, Right},
            Direction::{East, North, South, West},
        };

        use super::*;

        static DEFAULT_ARENA_CELLS: [bool; 16] = [false; 16];

        fn setup_game_state(spawns: &[Spawn], arena_cells: &[bool]) -> GameState {
            let arena = Grid::new(4, 4, Vec::from(arena_cells));
            GameState::new(Arc::new(arena), Vec::from(spawns)).unwrap()
        }

        #[test]
        fn restores_state_after_movement() {
            let spawns = [
                Spawn {
                    x: 0,
                    y: 0,
                    direction: South,
                },
                Spawn {
                    x: 3,
                    y: 3,
                    direction: North,
                },
                Spawn {
                    x: 1,
                    y: 2,
                    direction: West,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 3]).unwrap();
            assert_eq!(game_state.alive_count, 3);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state, old_state);
        }

        #[test]
        fn restores_previous_directions() {
            let spawns = [
                Spawn {
                    x: 0,
                    y: 0,
                    direction: South,
                },
                Spawn {
                    x: 3,
                    y: 3,
                    direction: North,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[Some(Left), Some(Left)]).unwrap();
            assert_eq!(game_state.players[0].direction, East);
            assert_eq!(game_state.players[1].direction, West);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state, old_state);
        }

        #[test]
        fn restores_state_after_none_actions() {
            let spawns = [
                Spawn {
                    x: 0,
                    y: 0,
                    direction: South,
                },
                Spawn {
                    x: 1,
                    y: 1,
                    direction: East,
                },
                Spawn {
                    x: 2,
                    y: 2,
                    direction: West,
                },
                Spawn {
                    x: 3,
                    y: 3,
                    direction: North,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 4]).unwrap();
            let _ = game_state.undo_step(record);

            assert_eq!(game_state, old_state);
        }

        #[test]
        fn restores_state_after_boundary_elimination() {
            let spawns = [
                Spawn {
                    x: 0,
                    y: 0,
                    direction: North,
                },
                Spawn {
                    x: 3,
                    y: 0,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 3,
                    direction: South,
                },
                Spawn {
                    x: 0,
                    y: 3,
                    direction: West,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 4]).unwrap();
            assert_eq!(game_state.alive_count, 0);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state, old_state);
        }

        #[test]
        fn restores_state_after_obstacle_elimination() {
            let cells = [
                false, false, false, false, true, false, true, false, false, true, false, true,
                false, false, false, false,
            ];
            let spawns = [
                Spawn {
                    x: 0,
                    y: 0,
                    direction: South,
                },
                Spawn {
                    x: 1,
                    y: 1,
                    direction: East,
                },
                Spawn {
                    x: 2,
                    y: 2,
                    direction: West,
                },
                Spawn {
                    x: 3,
                    y: 3,
                    direction: North,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &cells);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 4]).unwrap();
            assert_eq!(game_state.alive_count, 0);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state, old_state);
        }

        #[test]
        fn restores_state_after_own_trail_elimination() {
            let spawns = [Spawn {
                x: 1,
                y: 1,
                direction: East,
            }];
            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);

            let _ = game_state.step(&[None]).unwrap();
            let _ = game_state.step(&[Some(Right)]).unwrap();
            let _ = game_state.step(&[Some(Right)]).unwrap();
            let old_state = game_state.clone();

            let record = game_state.step(&[Some(Right)]).unwrap();
            assert_eq!(game_state.players[0].eliminated_at, Some(4));

            let _ = game_state.undo_step(record);

            assert_eq!(game_state, old_state);
        }

        #[test]
        fn restores_state_after_other_trail_elimination() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 2,
                    y: 2,
                    direction: North,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 2]).unwrap();
            assert_eq!(game_state.players[0].eliminated_at, Some(1));
            assert_eq!(game_state.players[1].eliminated_at, None);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state, old_state);
        }

        #[test]
        fn removes_new_head_on_collision_marker() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 2]).unwrap();
            assert_eq!(game_state.trails[(2, 2)], u8::MAX);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state.trails[(2, 2)], 0);
            assert_eq!(game_state, old_state);
        }

        #[test]
        fn removes_three_player_collision_marker() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
                Spawn {
                    x: 2,
                    y: 1,
                    direction: South,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 3]).unwrap();
            assert_eq!(game_state.trails[(2, 2)], u8::MAX);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state.trails[(2, 2)], 0);
            assert_eq!(game_state, old_state);
        }

        #[test]
        fn preserves_existing_collision_marker() {
            let arena = Grid::filled(6, 6, false);
            let spawns = vec![
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
                Spawn {
                    x: 2,
                    y: 0,
                    direction: South,
                },
                Spawn {
                    x: 2,
                    y: 4,
                    direction: North,
                },
            ];

            let mut game_state = GameState::new(Arc::new(arena), spawns).unwrap();
            let _ = game_state.step(&[None; 4]).unwrap();
            assert_eq!(game_state.trails[(2, 2)], u8::MAX);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 4]).unwrap();
            assert_eq!(game_state.alive_count, 0);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state.trails[(2, 2)], u8::MAX);
            assert_eq!(game_state, old_state);
        }

        #[test]
        fn preserves_existing_trails() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
                Spawn {
                    x: 2,
                    y: 2,
                    direction: North,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 3]).unwrap();
            assert_eq!(game_state.trails[(2, 2)], 3);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state.trails[(2, 2)], 3);
            assert_eq!(game_state, old_state);
        }

        #[test]
        fn restores_state_after_position_swap() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 1,
                    direction: East,
                },
                Spawn {
                    x: 2,
                    y: 1,
                    direction: West,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 2]).unwrap();
            assert_eq!(game_state.alive_count, 0);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state, old_state);
        }

        #[test]
        fn restores_mixed_survival_and_elimination() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
                Spawn {
                    x: 0,
                    y: 0,
                    direction: South,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 3]).unwrap();
            assert_eq!(game_state.alive_count, 1);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state, old_state);
        }

        #[test]
        fn does_not_revive_previously_eliminated_players() {
            let spawns = [
                Spawn {
                    x: 0,
                    y: 0,
                    direction: North,
                },
                Spawn {
                    x: 3,
                    y: 3,
                    direction: North,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let _ = game_state.step(&[None; 2]).unwrap();
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 2]).unwrap();
            let _ = game_state.undo_step(record);

            assert_eq!(game_state.players[0].eliminated_at, Some(1));
            assert_eq!(game_state, old_state);
        }

        #[test]
        fn revives_players_eliminated_on_current_tick() {
            let spawns = [Spawn {
                x: 1,
                y: 0,
                direction: East,
            }];
            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);

            let _ = game_state.step(&[None]).unwrap();
            let _ = game_state.step(&[None]).unwrap();
            let old_state = game_state.clone();

            let record = game_state.step(&[None]).unwrap();
            assert_eq!(game_state.players[0].eliminated_at, Some(3));

            let _ = game_state.undo_step(record);

            assert_eq!(game_state.players[0].eliminated_at, None);
            assert_eq!(game_state, old_state);
        }

        #[test]
        fn restores_direction_after_boundary_underflow() {
            let spawns = [
                Spawn {
                    x: 0,
                    y: 0,
                    direction: East,
                },
                Spawn {
                    x: 2,
                    y: 0,
                    direction: East,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[Some(Left), Some(Left)]).unwrap();
            assert_eq!(game_state.alive_count, 0);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state.players[0].direction, East);
            assert_eq!(game_state.players[1].direction, East);
            assert_eq!(game_state, old_state);
        }

        #[test]
        fn undoes_multiple_steps_in_reverse_order() {
            let spawns = [
                Spawn {
                    x: 0,
                    y: 0,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 3,
                    direction: West,
                },
                Spawn {
                    x: 0,
                    y: 3,
                    direction: North,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let state_0 = game_state.clone();

            let record_1 = game_state.step(&[None; 3]).unwrap();
            let state_1 = game_state.clone();

            let record_2 = game_state.step(&[Some(Right); 3]).unwrap();
            let state_2 = game_state.clone();

            let record_3 = game_state.step(&[Some(Left); 3]).unwrap();
            assert_eq!(game_state.alive_count, 1);

            let _ = game_state.undo_step(record_3);
            assert_eq!(game_state, state_2);

            let _ = game_state.undo_step(record_2);
            assert_eq!(game_state, state_1);

            let _ = game_state.undo_step(record_1);
            assert_eq!(game_state, state_0);
        }

        #[test]
        fn restores_alive_count() {
            let spawns = [
                Spawn {
                    x: 1,
                    y: 2,
                    direction: East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: West,
                },
                Spawn {
                    x: 0,
                    y: 0,
                    direction: South,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record = game_state.step(&[None; 3]).unwrap();
            assert_eq!(game_state.alive_count, 1);

            let _ = game_state.undo_step(record);

            assert_eq!(game_state.alive_count, 3);
            assert_eq!(game_state, old_state);
        }

        #[test]
        fn restores_state_when_all_players_are_dead() {
            let spawns = [
                Spawn {
                    x: 0,
                    y: 0,
                    direction: North,
                },
                Spawn {
                    x: 3,
                    y: 3,
                    direction: South,
                },
            ];

            let mut game_state = setup_game_state(&spawns, &DEFAULT_ARENA_CELLS);
            let old_state = game_state.clone();

            let record_1 = game_state.step(&[None; 2]).unwrap();
            assert_eq!(game_state.alive_count, 0);
            let dead_state = game_state.clone();

            let record_2 = game_state.step(&[None; 2]).unwrap();
            let _ = game_state.undo_step(record_2);

            assert_eq!(game_state, dead_state);

            let _ = game_state.undo_step(record_1);

            assert_eq!(game_state, old_state);
        }

        #[test]
        fn rejects_undo_when_game_just_started() {
            let arena = Grid::filled(4, 4, false);
            let spawns = vec![Spawn {
                x: 1,
                y: 1,
                direction: South,
            }];

            let mut game_state = GameState::new(Arc::new(arena), spawns).unwrap();
            let old_state = game_state.clone();

            let mut advanced_state = game_state.clone();
            let record = advanced_state.step(&[None]).unwrap();

            assert_eq!(
                game_state.undo_step(record),
                Err(GameStateError::GameJustStarted)
            );
            assert_eq!(game_state, old_state);
        }

        #[test]
        fn rejects_undo_record_with_wrong_tick() {
            let arena = Grid::filled(4, 4, false);
            let spawns = vec![Spawn {
                x: 1,
                y: 1,
                direction: South,
            }];

            let mut game_state = GameState::new(Arc::new(arena), spawns).unwrap();
            let mut record = game_state.step(&[None]).unwrap();
            let old_state = game_state.clone();

            record.tick += 1;

            assert_eq!(
                game_state.undo_step(record),
                Err(GameStateError::InvalidUndoRecord)
            );
            assert_eq!(game_state, old_state);
        }
    }

    mod outcome {
        use crate::engine::action::Direction::South;

        use super::*;

        fn setup_game_state() -> GameState {
            let arena = Grid::filled(4, 4, false);
            let spawns = (0..4)
                .map(|i| Spawn {
                    x: i,
                    y: i,
                    direction: South,
                })
                .collect();

            GameState::new(Arc::new(arena), spawns).unwrap()
        }

        #[test]
        fn returns_none_for_ongoing_game() {
            let mut game_state = setup_game_state();

            game_state.players[0].eliminated_at = Some(1);
            game_state.players[1].eliminated_at = Some(2);
            game_state.alive_count = 2;

            assert_eq!(game_state.outcome(), None);
        }

        #[test]
        fn ranks_last_surviving_player_first() {
            let mut game_state = setup_game_state();

            game_state.players[0].eliminated_at = Some(1);
            game_state.players[1].eliminated_at = Some(2);
            game_state.players[2].eliminated_at = Some(3);
            game_state.alive_count = 1;

            assert_eq!(game_state.outcome(), Some(vec![4, 3, 2, 1]));
        }

        #[test]
        fn assigns_same_rank_to_simultaneous_eliminations() {
            let mut game_state = setup_game_state();

            game_state.players[0].eliminated_at = Some(1);
            game_state.players[1].eliminated_at = Some(2);
            game_state.players[2].eliminated_at = Some(2);
            game_state.alive_count = 1;

            assert_eq!(game_state.outcome(), Some(vec![4, 2, 2, 1]));
        }

        #[test]
        fn ranks_players_when_everyone_is_eliminated() {
            let mut game_state = setup_game_state();

            game_state.players[0].eliminated_at = Some(1);
            game_state.players[1].eliminated_at = Some(1);
            game_state.players[2].eliminated_at = Some(3);
            game_state.players[3].eliminated_at = Some(3);
            game_state.alive_count = 0;

            assert_eq!(game_state.outcome(), Some(vec![3, 3, 1, 1]));
        }
    }
}
