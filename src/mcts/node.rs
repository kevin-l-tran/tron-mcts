use crate::engine::{action::Action, state::GameState};

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
    pub action: JointAction,
    pub child: NodeId,
}

pub struct SearchTree {
    pub root: NodeId,
    pub nodes: Vec<SearchNode>,
    pub edges: Vec<SearchEdge>,
}

impl SearchTree {
    pub fn new(game_state: &GameState) -> SearchTree {
        let root = SearchNode::new(
            game_state.players.len(),
            &game_state
                .dead_players()
                .map(|(id, _)| id)
                .collect::<Vec<usize>>(),
            None,
        );
        let nodes = vec![root];
        let edges = vec![];

        SearchTree {
            root: 0,
            nodes,
            edges,
        }
    }

    pub fn find_edge(&self, parent_node_id: NodeId, action: &JointAction) -> Option<EdgeId> {
        let node = &self.nodes[parent_node_id as usize];

        node.outgoing_edges
            .iter()
            .find(|&&id| &self.edges[id as usize].action == action)
            .copied()
    }

    pub fn add_child(
        &mut self,
        parent_node_id: NodeId,
        action: &JointAction,
        child_game_state: &GameState,
    ) {
        debug_assert!(
            self.find_edge(parent_node_id, action).is_none(),
            "duplicate joint action"
        );

        let new_node_id = self.nodes.len();
        let new_edge_id = self.edges.len();

        let new_edge = SearchEdge {
            action: action.clone(),
            child: new_node_id as u32,
        };

        let new_node = SearchNode::new(
            child_game_state.players.len(),
            &child_game_state
                .dead_players()
                .map(|(id, _)| id)
                .collect::<Vec<usize>>(),
            Some(parent_node_id),
        );

        self.edges.push(new_edge);
        self.nodes.push(new_node);

        let node = &mut self.nodes[parent_node_id as usize];
        node.outgoing_edges.push(new_edge_id as u32);
    }

    pub fn record_visit(&mut self, node_id: NodeId) {
        let node = &mut self.nodes[node_id as usize];

        node.visit();
    }

    pub fn record_action(
        &mut self,
        parent_node_id: NodeId,
        joint_action: &JointAction,
        reward_vector: &[f64],
    ) {
        let node = &mut self.nodes[parent_node_id as usize];

        node.update_stats(joint_action, reward_vector);
    }
}
