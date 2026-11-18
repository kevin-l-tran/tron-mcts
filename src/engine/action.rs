use std::ops::Neg;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    pub const VALUES: [Self; 4] = [Self::North, Self::South, Self::East, Self::West];

    pub fn delta(self) -> (isize, isize) {
        match self {
            Self::North => (0, -1),
            Self::South => (0, 1),
            Self::East => (1, 0),
            Self::West => (-1, 0),
        }
    }

    pub fn action_from_change(self, current_direction: Direction) -> Option<Action> {
        match current_direction {
            Direction::North => match self {
                Direction::North => Some(Action::Straight),
                Direction::South => None,
                Direction::East => Some(Action::Right),
                Direction::West => Some(Action::Left),
            },
            Direction::South => match self {
                Direction::North => None,
                Direction::South => Some(Action::Straight),
                Direction::East => Some(Action::Left),
                Direction::West => Some(Action::Right),
            },
            Direction::East => match self {
                Direction::North => Some(Action::Left),
                Direction::South => Some(Action::Right),
                Direction::East => Some(Action::Straight),
                Direction::West => None,
            },
            Direction::West => match self {
                Direction::North => Some(Action::Right),
                Direction::South => Some(Action::Left),
                Direction::East => None,
                Direction::West => Some(Action::Straight),
            },
        }
    }
}

impl Neg for Direction {
    type Output = Direction;

    fn neg(self) -> Self::Output {
        match self {
            Self::North => Self::South,
            Self::South => Self::North,
            Self::East => Self::West,
            Self::West => Self::East,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Action {
    Straight,
    Left,
    Right,
}

impl Action {
    pub const VALUES: [Self; 3] = [Self::Straight, Self::Left, Self::Right];

    pub fn apply_to_direction(self, current_direction: Direction) -> Direction {
        match current_direction {
            Direction::North => match self {
                Self::Left => Direction::West,
                Self::Right => Direction::East,
                Self::Straight => Direction::North,
            },
            Direction::South => match self {
                Self::Left => Direction::East,
                Self::Right => Direction::West,
                Self::Straight => Direction::South,
            },
            Direction::East => match self {
                Self::Left => Direction::North,
                Self::Right => Direction::South,
                Self::Straight => Direction::East,
            },
            Direction::West => match self {
                Self::Left => Direction::South,
                Self::Right => Direction::North,
                Self::Straight => Direction::West,
            },
        }
    }
}
