use std::collections::HashSet;

use super::action::Direction;
use super::grid::Grid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Spawn {
    pub x: usize,
    pub y: usize,
    pub direction: Direction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ArenaError {
    SpawnOutOfArena(Spawn),
    SpawnOnObstacle(Spawn),
    DuplicateSpawnLocation(Spawn),
}

#[derive(Clone, Debug)]
pub struct Arena {
    grid: Grid<bool>,
    spawns: Vec<Spawn>,
}

impl Arena {
    pub fn new(grid: Grid<bool>, spawns: Vec<Spawn>) -> Result<Arena, ArenaError> {
        let mut spawn_locations: HashSet<(usize, usize)> = HashSet::new();

        for spawn in &spawns {
            let is_obstacle = grid.get(spawn.x, spawn.y);

            match is_obstacle {
                None => return Err(ArenaError::SpawnOutOfArena(*spawn)),
                Some(true) => return Err(ArenaError::SpawnOnObstacle(*spawn)),
                Some(false) => {}
            }

            if !spawn_locations.insert((spawn.x, spawn.y)) {
                return Err(ArenaError::DuplicateSpawnLocation(*spawn));
            }
        }

        Ok(Arena { grid, spawns })
    }
}
