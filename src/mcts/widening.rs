pub trait WideningPolicy {
    fn can_expand(&self, node_visits: u32, child_count: usize, joint_action_count: usize) -> bool;
}
