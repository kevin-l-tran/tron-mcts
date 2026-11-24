use rand::rngs::ChaCha8Rng;

use crate::engine::state::GameState;

pub trait RolloutEvaluator {
    fn evaluate(&self, state: &GameState, rng: &mut ChaCha8Rng) -> Vec<f64>;
}
