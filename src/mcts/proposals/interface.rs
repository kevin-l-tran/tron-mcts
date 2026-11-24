use rand::rngs::ChaCha8Rng;

use crate::{
    engine::state::GameState,
    mcts::{node::JointAction, selection::Preferences},
};

pub trait JointActionProposal {
    fn initialize(&mut self, root_state: &GameState, player_id: usize);

    fn propose(&mut self, preferences: &Preferences, rng: &mut ChaCha8Rng) -> JointAction;

    fn observe(&mut self, joint_action: &JointAction, reward_vector: &[f64]);
}
