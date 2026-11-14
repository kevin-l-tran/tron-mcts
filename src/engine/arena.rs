use super::action::Direction;

pub struct Spawn {
    x: u8,
    y: u8,
    direction: Direction,
}

pub struct Arena {
    width: u8,
    height: u8,
    rocks: Vec<u8>,
    spawns: Vec<Spawn>,
}
