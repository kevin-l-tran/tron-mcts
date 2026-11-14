use std::sync::Arc;

use super::action::Direction;
use super::arena::Arena;

pub struct PlayerState {
    x: usize,
    y: usize,
    direction: Direction,
    alive: bool,
    eliminated_at: Option<bool>,
}

pub struct GameState {
    arena: Arc<Arena>,
    players: Vec<PlayerState>,
    trails: Vec<usize>,
    tick: usize,
    alive_count: usize,
    max_ticks: usize,
}
