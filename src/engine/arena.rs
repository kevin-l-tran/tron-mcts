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
    InvalidRocksArray,
    SpawnOutOfArena(Spawn),
}

#[derive(Clone, Debug)]
pub struct Arena {
    grid: Grid<u8>,
    spawns: Vec<Spawn>,
}

impl Arena {
    pub fn new(grid: Grid<u8>, spawns: Vec<Spawn>) -> Result<Arena, ArenaError> {
        for spawn in &spawns {
            if !grid.in_bounds(spawn.x, spawn.y) {
                return Err(ArenaError::SpawnOutOfArena(*spawn));
            }
        }

        Ok(Arena { grid, spawns })
    }
}