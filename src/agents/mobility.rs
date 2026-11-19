use rand::seq::SliceRandom;

use crate::{
    agents::interface::{Agent, AgentContext, AgentDecision},
    engine::state::GameState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct MobilityAgent;

impl Agent for MobilityAgent {
    fn select_action(
        &mut self,
        state: &GameState,
        player_id: usize,
        context: &mut AgentContext,
    ) -> AgentDecision {
        let mut temp_state = state.clone();

        let mut safe_actions = temp_state
            .safe_actions(player_id)
            .expect("expected player_id to be valid");

        // shuffle actions so agent doesn't always choose same direction
        safe_actions.shuffle(&mut context.rng);

        let mut max_safety = (0, None);
        let mut simulation_count = 0;

        for action in safe_actions {
            let mut actions = vec![None; state.players.len()];
            actions[player_id] = Some(action);

            let undo = temp_state
                .step(&actions)
                .expect("simulated step should succeed");

            simulation_count += 1;

            let project_safe_actions = temp_state
                .safe_actions(player_id)
                .expect("expected player_id to be valid");

            if project_safe_actions.len() > max_safety.0 {
                max_safety = (project_safe_actions.len(), Some(action));
            }

            temp_state
                .undo_step(undo)
                .expect("undo step should succeed")
        }

        AgentDecision {
            action: max_safety.1,
            simulation_count,
        }
    }
}
