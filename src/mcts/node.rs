use crate::engine::action::Action;

pub struct ActionStats {
    visits: u32,
    reward_sum: f64,
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
