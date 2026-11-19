use rand::RngExt;

use crate::{
    agents::interface::{Agent, AgentContext, AgentDecision},
    engine::{action::Action, state::GameState},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct RandomAgent;

impl Agent for RandomAgent {
    fn select_action(
        &mut self,
        _state: &GameState,
        _player_id: usize,
        context: &mut AgentContext,
    ) -> AgentDecision {
        let action =
            Some([Action::Left, Action::Right, Action::Straight][context.rng.random_range(0..3)]);

        AgentDecision {
            action,
            simulation_count: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SafeRandomAgent;

impl Agent for SafeRandomAgent {
    fn select_action(
        &mut self,
        state: &GameState,
        player_id: usize,
        context: &mut AgentContext,
    ) -> AgentDecision {
        let safe_actions = state.safe_actions(player_id).unwrap();
        let num_safe_actions = safe_actions.len();

        let action = if num_safe_actions > 0 {
            Some(safe_actions[context.rng.random_range(0..num_safe_actions)])
        } else {
            None
        };

        AgentDecision {
            action,
            simulation_count: 0,
        }
    }
}
