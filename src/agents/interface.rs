use std::time::Duration;

use rand::rngs::ChaCha8Rng;

use crate::engine::{action::Action, state::GameState};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SearchBudget {
    Simulations(usize),
    Time(Duration),
}

#[derive(Debug, Eq, PartialEq)]
pub struct AgentContext {
    pub rng: ChaCha8Rng,
    pub budget: SearchBudget,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct AgentDecision {
    pub action: Option<Action>,
    pub simulation_count: usize,
}

pub trait Agent {
    fn select_action(
        &mut self,
        state: &GameState,
        player_id: usize,
        context: &mut AgentContext,
    ) -> AgentDecision;
}
