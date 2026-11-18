use std::time::Duration;

use crate::{
    agents::interface::{Agent, AgentContext, SearchBudget}, engine::{action::Action, state::GameState},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct RunConfig {
    pub max_ticks: usize,
    pub decision_budget: SearchBudget,
    pub record_replay: bool,
    pub seed: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TerminationReason {
    GameOver,
    TickLimit,
}

pub type ReplayFrame = Vec<Option<Action>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RunError {}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchResult {
    pub final_state: GameState,
    pub ticks: usize,
    pub ranks: Option<Vec<f64>>,
    pub termination: TerminationReason,
    pub agent_stats: Vec<AgentStats>,
    pub replay: Option<Vec<ReplayFrame>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct AgentStats {
    pub total_decision_time: Duration,
    pub total_simulation_counts: usize,
}

pub struct MatchRunner {
    pub agents: Vec<Box<dyn Agent>>,
    pub agent_contexts: Vec<AgentContext>,
    pub agent_stats: Vec<AgentStats>,
    pub config: RunConfig,
    pub replay: Option<Vec<ReplayFrame>>,

    state: GameState,
}

impl MatchRunner {
    pub fn step(&mut self) -> Result<(), RunError> {
        todo!()
    }

    pub fn run(&mut self) -> Result<MatchResult, RunError> {
        todo!()
    }
}
