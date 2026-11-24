use rand::rngs::ChaCha8Rng;

use crate::{
    engine::state::GameState,
    mcts::node::{EdgeId, JointAction, NodeId, SearchTree},
};

pub enum TreeDecision {
    Traverse { edge_id: EdgeId },
    Expand { joint_action: JointAction },
}

pub trait TreePolicy {
    fn initialize(&mut self, root_state: &GameState, player_id: usize);

    fn choose(&mut self, tree: &SearchTree, node_id: NodeId, rng: &mut ChaCha8Rng) -> TreeDecision;

    fn observe_root(&mut self, joint_action: &JointAction, reward_vector: &[f64]);
}
