use rand::seq::SliceRandom;

use crate::{
    agents::interface::{Agent, AgentContext, AgentDecision},
    algorithms::voronoi::voronoi_territory,
    engine::state::GameState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct VoronoiAgent;

impl Agent for VoronoiAgent {
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

        let mut max_territory = (-f64::INFINITY, None);
        let mut simulation_count = 0;

        for action in safe_actions {
            let mut actions = vec![None; temp_state.players.len()];
            actions[player_id] = Some(action);

            let undo = temp_state
                .step(&actions)
                .expect("simulated step should succeed");

            simulation_count += 1;

            if temp_state.players[player_id].eliminated_at.is_some() {
                // if player collided, give it a very low score
                if max_territory.0 == -f64::INFINITY {
                    max_territory.1 = Some(action);
                }

                temp_state
                    .undo_step(undo)
                    .expect("undo step should succeed")
            } else {
                let territory_scores = voronoi_territory(&temp_state);

                if territory_scores[player_id] > max_territory.0 {
                    max_territory = (territory_scores[player_id], Some(action));
                }

                temp_state
                    .undo_step(undo)
                    .expect("undo step should succeed")
            }
        }

        AgentDecision {
            action: max_territory.1,
            simulation_count,
        }
    }
}
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rand::{rngs::ChaCha8Rng, SeedableRng};

    use crate::{
        agents::interface::{Agent, AgentContext, SearchBudget},
        engine::{
            action::{Action, Direction},
            grid::Grid,
            state::{GameState, Spawn},
        },
    };

    use super::*;

    fn setup_game_state(arena: Grid<bool>, spawns: &[Spawn]) -> GameState {
        GameState::new(Arc::new(arena), spawns.to_vec()).unwrap()
    }

    fn select_action(state: &GameState, player_id: usize) -> AgentDecision {
        let mut agent = VoronoiAgent;
        let mut context = AgentContext {
            rng: ChaCha8Rng::seed_from_u64(1234),
            budget: SearchBudget::Simulations(0),
        };
        agent.select_action(state, player_id, &mut context)
    }

    // blocked column with a single opening occupied by the player
    fn setup_split_arena(barrier_x: usize) -> GameState {
        let mut arena = Grid::filled(9, 5, false);
        for y in 0..5 {
            if y != 2 {
                arena[(barrier_x, y)] = true;
            }
        }
        let spawns = [Spawn {
            x: barrier_x,
            y: 2,
            direction: Direction::North,
        }];
        setup_game_state(arena, &spawns)
    }

    mod select_action {
        use super::*;

        #[test]
        fn prefers_right_when_right_region_is_larger() {
            // left region: 3 x 5 cells; right region: 5 x 5 cells
            let state = setup_split_arena(3);

            let decision = select_action(&state, 0);

            assert_eq!(decision.action, Some(Action::Right));
            assert_eq!(decision.simulation_count, 2);
        }

        #[test]
        fn prefers_left_when_left_region_is_larger() {
            // left region: 5 x 5 cells; right region: 3 x 5 cells
            let state = setup_split_arena(5);

            let decision = select_action(&state, 0);

            assert_eq!(decision.action, Some(Action::Left));
            assert_eq!(decision.simulation_count, 2);
        }

        #[test]
        fn avoids_head_on_collision_with_straight_moving_opponent() {
            let arena = Grid::filled(5, 5, false);
            let spawns = [
                Spawn { x: 1, y: 2, direction: Direction::East },
                Spawn { x: 3, y: 2, direction: Direction::West },
            ];
            let state = setup_game_state(arena, &spawns);

            // straight would send both players into (2, 2)
            let decision = select_action(&state, 0);

            assert!(matches!(decision.action, Some(Action::Left | Action::Right)));
            assert_eq!(decision.simulation_count, 3);
        }

        #[test]
        fn returns_safe_action_when_all_territory_scores_are_zero() {
            let mut arena = Grid::filled(3, 3, true);
            for position in [(1, 1), (1, 0), (0, 1), (2, 1)] {
                arena[position] = false;
            }
            let spawns = [Spawn { x: 1, y: 1, direction: Direction::North }];
            let state = setup_game_state(arena, &spawns);

            // each available destination is a dead end with zero empty cells
            let decision = select_action(&state, 0);

            assert!(state.safe_actions(0).unwrap().contains(&decision.action.unwrap()));
            assert_eq!(decision.simulation_count, 3);
        }

        #[test]
        fn returns_none_when_no_safe_actions_exist() {
            let mut arena = Grid::filled(3, 3, true);
            arena[(1, 1)] = false;
            let spawns = [Spawn { x: 1, y: 1, direction: Direction::East }];
            let state = setup_game_state(arena, &spawns);

            let decision = select_action(&state, 0);

            assert_eq!(decision.action, None);
            assert_eq!(decision.simulation_count, 0);
        }

        #[test]
        fn does_not_modify_original_game_state() {
            let arena = Grid::filled(5, 5, false);
            let spawns = [
                Spawn { x: 1, y: 2, direction: Direction::East },
                Spawn { x: 3, y: 2, direction: Direction::West },
            ];
            let state = setup_game_state(arena, &spawns);
            let original = state.clone();

            let _ = select_action(&state, 0);

            assert_eq!(state, original);
        }
    }
}
