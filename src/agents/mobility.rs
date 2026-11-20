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

            if project_safe_actions.len() >= max_safety.0 {
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rand::{SeedableRng, rngs::ChaCha8Rng};

    use crate::{
        agents::interface::SearchBudget,
        engine::{
            action::{Action, Direction},
            grid::Grid,
            state::Spawn,
        },
    };

    use super::*;

    fn setup_game_state(arena: Grid<bool>, spawn: Spawn) -> GameState {
        GameState::new(Arc::new(arena), vec![spawn]).unwrap()
    }

    fn context() -> AgentContext {
        AgentContext {
            rng: ChaCha8Rng::seed_from_u64(1234),
            budget: SearchBudget::Simulations(0),
        }
    }

    mod select_action {
        use super::*;

        #[test]
        fn returns_none_when_no_safe_actions_exist() {
            let mut arena = Grid::filled(3, 3, false);
            arena[(1, 0)] = true;
            arena[(0, 1)] = true;
            arena[(2, 1)] = true;
            let state = setup_game_state(
                arena,
                Spawn { x: 1, y: 1, direction: Direction::North },
            );

            let decision = MobilityAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, None);
            assert_eq!(decision.simulation_count, 0);
        }

        #[test]
        fn prefers_action_with_most_follow_up_moves() {
            let mut arena = Grid::filled(5, 5, false);
            for (x, y) in [(0, 2), (1, 3), (4, 2), (3, 1), (3, 3)] {
                arena[(x, y)] = true;
            }
            let state = setup_game_state(
                arena,
                Spawn { x: 2, y: 2, direction: Direction::North },
            );

            // Straight has 2 safe follow-ups; left has 1 and right has 0.
            let decision = MobilityAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, Some(Action::Straight));
            assert_eq!(decision.simulation_count, 3);
        }

        #[test]
        fn turns_when_straight_leads_to_dead_end() {
            let mut arena = Grid::filled(5, 5, false);
            for (x, y) in [(2, 0), (1, 1), (3, 1), (0, 2), (1, 3)] {
                arena[(x, y)] = true;
            }
            let state = setup_game_state(
                arena,
                Spawn { x: 2, y: 2, direction: Direction::North },
            );

            // Straight and left have 0 safe follow-ups; right has 2.
            let decision = MobilityAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, Some(Action::Right));
            assert_eq!(decision.simulation_count, 3);
        }

        #[test]
        fn chooses_only_safe_action_even_if_it_has_no_follow_up_moves() {
            let mut arena = Grid::filled(5, 5, false);
            for (x, y) in [(2, 1), (1, 2), (4, 2), (3, 1), (3, 3)] {
                arena[(x, y)] = true;
            }
            let state = setup_game_state(
                arena,
                Spawn { x: 2, y: 2, direction: Direction::North },
            );

            let decision = MobilityAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, Some(Action::Right));
            assert_eq!(decision.simulation_count, 1);
        }

        #[test]
        fn returns_a_safe_action_when_all_scores_are_zero() {
            let mut arena = Grid::filled(5, 5, false);
            for (x, y) in [(2, 0), (1, 1), (3, 1), (0, 2), (4, 2), (1, 3), (3, 3)] {
                arena[(x, y)] = true;
            }
            let state = setup_game_state(
                arena,
                Spawn { x: 2, y: 2, direction: Direction::North },
            );
            let safe_actions = state.safe_actions(0).unwrap();

            let decision = MobilityAgent.select_action(&state, 0, &mut context());

            assert!(safe_actions.contains(&decision.action.unwrap()));
            assert_eq!(decision.simulation_count, 3);
        }

        #[test]
        fn returns_none_for_eliminated_player() {
            let arena = Grid::filled(3, 3, false);
            let mut state = setup_game_state(
                arena,
                Spawn { x: 1, y: 1, direction: Direction::North },
            );
            state.players[0].eliminated_at = Some(1);
            state.alive_count = 0;

            let decision = MobilityAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, None);
            assert_eq!(decision.simulation_count, 0);
        }

        #[test]
        fn does_not_modify_original_game_state() {
            let state = setup_game_state(
                Grid::filled(5, 5, false),
                Spawn { x: 2, y: 2, direction: Direction::North },
            );
            let original = state.clone();

            let _ = MobilityAgent.select_action(&state, 0, &mut context());

            assert_eq!(state, original);
        }
    }
}
