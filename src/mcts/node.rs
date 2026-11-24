use std::num;

use crate::engine::action::Action;

#[derive(Clone, Copy, Debug)]
pub struct ActionStats {
    pub visits: u32,
    pub reward_sum: f64,
}

impl ActionStats {
    pub fn mean_reward(&self) -> f64 {
        if self.visits == 0 {
            0.0
        } else {
            self.reward_sum / self.visits as f64
        }
    }

    pub fn observe(&mut self, reward: f64) {
        self.visits += 1;
        self.reward_sum += reward;
    }
}

impl Default for ActionStats {
    fn default() -> Self {
        Self {
            visits: 0,
            reward_sum: 0.0,
        }
    }
}

pub type NodeId = u32;

pub struct SearchNode {
    pub visits: u32,
    pub action_stats: Vec<Option<[ActionStats; 3]>>, // one action stat per movement option
    pub outgoing_edges: Vec<EdgeId>,
    pub parent_edge: Option<EdgeId>,
}

impl SearchNode {
    pub fn new(
        num_players: usize,
        dead_player_ids: &[usize],
        parent_edge: Option<EdgeId>,
    ) -> SearchNode {
        let empty_stats = [ActionStats::default(); 3];
        let mut action_stats = vec![Some(empty_stats); num_players];

        for id in dead_player_ids {
            action_stats[*id] = None;
        }

        SearchNode {
            visits: 0,
            action_stats,
            outgoing_edges: vec![],
            parent_edge,
        }
    }

    pub fn visit(&mut self) {
        self.visits += 1;
    }

    pub fn update_stats(&mut self, joint_action: &JointAction, reward_vector: &[f64]) {
        for (id, action) in joint_action.iter().enumerate() {
            // player is dead if action stats is None
            let Some(action_stats) = &mut self.action_stats[id] else {
                continue;
            };

            // None actions default to Action::Straight
            let action = action.unwrap_or(Action::Straight);

            let action_id = action.value_index();
            let action_stat = &mut action_stats[action_id];

            action_stat.observe(reward_vector[id]);
        }
    }
}

pub type EdgeId = u32;

pub type JointAction = Vec<Option<Action>>;

pub struct SearchEdge {
    action: JointAction,
    child: NodeId,
}

pub struct SearchTree {
    root: NodeId,
    nodes: Vec<SearchNode>,
    edges: Vec<SearchEdge>,
}
