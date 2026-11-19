use std::sync::Arc;

use crate::{
    engine::state::{Arena, GameState, GameStateError, Spawn, UndoRecord},
    runner::match_runner::ReplayFrame,
};

#[derive(Clone, Debug)]
pub struct Replay {
    pub arena: Arc<Arena>,
    pub spawns: Vec<Spawn>,
    pub frames: Vec<ReplayFrame>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayError {
    InvalidTick(usize),
    GameState(GameStateError),
}

impl From<GameStateError> for ReplayError {
    fn from(error: GameStateError) -> Self {
        Self::GameState(error)
    }
}

pub struct ReplayPlayer {
    replay: Replay,
    state: GameState,
    undo_records: Vec<UndoRecord>,
}

impl ReplayPlayer {
    pub fn new(replay: Replay) -> Result<Self, ReplayError> {
        let state = GameState::new(Arc::clone(&replay.arena), replay.spawns.clone())?;
        Ok(Self {
            replay,
            state,
            undo_records: Vec::new(),
        })
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn tick(&self) -> usize {
        self.undo_records.len()
    }

    pub fn total_ticks(&self) -> usize {
        self.replay.frames.len()
    }

    pub fn step_forward(&mut self) -> Result<bool, ReplayError> {
        let Some(frame) = self.replay.frames.get(self.tick()) else {
            return Ok(false);
        };
        let undo_record = self.state.step(frame)?;
        self.undo_records.push(undo_record);
        Ok(true)
    }

    pub fn step_backward(&mut self) -> Result<bool, ReplayError> {
        let Some(record) = self.undo_records.last().cloned() else {
            return Ok(false);
        };
        self.state.undo_step(record)?;
        self.undo_records.pop();
        Ok(true)
    }

    pub fn seek(&mut self, tick: usize) -> Result<(), ReplayError> {
        if tick > self.total_ticks() {
            return Err(ReplayError::InvalidTick(tick));
        }

        while self.tick() < tick {
            self.step_forward()?;
        }
        while self.tick() > tick {
            self.step_backward()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{
        action::{Action, Direction},
        grid::Grid,
    };

    fn test_replay() -> Replay {
        Replay {
            arena: Arc::new(Grid::filled(5, 5, false)),
            spawns: vec![
                Spawn {
                    x: 1,
                    y: 2,
                    direction: Direction::East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: Direction::West,
                },
            ],
            frames: vec![vec![Some(Action::Straight); 2]],
        }
    }

    #[test]
    fn plays_forward_and_backward() {
        let mut player = ReplayPlayer::new(test_replay()).unwrap();
        let original = player.state().clone();

        assert_eq!(player.step_forward(), Ok(true));
        assert_eq!(player.state().alive_count, 0);
        assert_eq!(player.state().trails[(2, 2)], u8::MAX);
        assert_eq!(player.step_forward(), Ok(false));

        assert_eq!(player.step_backward(), Ok(true));
        assert_eq!(player.state(), &original);
        assert_eq!(player.step_backward(), Ok(false));
    }

    #[test]
    fn seeking_matches_sequential_playback() {
        let replay = test_replay();
        let mut sequential = ReplayPlayer::new(replay.clone()).unwrap();
        let mut seeking = ReplayPlayer::new(replay).unwrap();

        sequential.step_forward().unwrap();
        seeking.seek(1).unwrap();
        assert_eq!(seeking.state(), sequential.state());

        seeking.seek(0).unwrap();
        sequential.step_backward().unwrap();
        assert_eq!(seeking.state(), sequential.state());
    }

    #[test]
    fn rejects_seek_past_end() {
        let mut player = ReplayPlayer::new(test_replay()).unwrap();
        let original = player.state().clone();
        assert_eq!(player.seek(2), Err(ReplayError::InvalidTick(2)));
        assert_eq!(player.state(), &original);
    }
}
