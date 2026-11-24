use crate::engine::action::Action;

pub struct ActionStats {
    pub visits: u32,
    pub reward_sum: f64,
}

impl ActionStats {
    pub fn empty() -> ActionStats {
        ActionStats {
            visits: 0,
            reward_sum: 0.0,
        }
    }

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

pub type NodeId = u32;

pub struct SearchNode {
    visits: u32,
    action_stats: Vec<Option<[ActionStats; 3]>>, // one action stat per movement option
    outgoing_edges: Vec<EdgeId>,
    parent_edge: Option<EdgeId>,
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
