use rand::rngs::ChaCha8Rng;

use crate::mcts::node::{ActionStats, JointAction};

pub struct PlayerPreferences {
    pub ucb_scores: [f64; 3],
    pub probabilities: [f64; 3],
}

pub type Preferences = Vec<Option<PlayerPreferences>>;

pub trait SelectionPolicy {
    fn get_preferences(
        &self,
        action_stats: &[Option<[ActionStats; 3]>],
        node_visits: u32,
    ) -> Preferences;

    fn select_joint_action(&self, preferences: &Preferences, rng: &mut ChaCha8Rng) -> JointAction;

    fn select_existing_child(
        &self,
        preferences: &Preferences,
        expanded_actions: &[&JointAction],
        rng: &mut ChaCha8Rng,
    ) -> usize; // index of chosen action from expanded_actions
}
