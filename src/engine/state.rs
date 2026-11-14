use std::sync::Arc;

use super::action::Direction;
use super::arena::Arena;
use super::grid::Grid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct PlayerState {
    x: usize,
    y: usize,
    direction: Direction,
    alive: bool,
    eliminated_at: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct GameState {
    arena: Arc<Arena>,
    players: Vec<PlayerState>,
    trails: Grid<u8>,
    tick: usize,
    alive_count: usize,
    max_ticks: usize,
}

impl GameState {
    pub fn new() -> GameState {
        todo!()
    }

    pub fn legal_actions() -> Direction {
        todo!()
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
}
