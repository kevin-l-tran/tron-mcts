use std::sync::Arc;

use super::action::Direction;
use super::arena::Arena;

pub struct PlayerState {
    x: u8,
    y: u8,
    direction: Direction,
    alive: bool,
    eliminated_at: Option<bool>,
}

pub struct GameState {
    arena: Arc<Arena>,
    players: Vec<PlayerState>,
    trails: Vec<u8>,
    tick: u16,
    alive_count: u8,
    max_ticks: u16,
}
