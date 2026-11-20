use crate::{
    agents::interface::{Agent, AgentContext, AgentDecision},
    engine::{action::Action, state::GameState},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SimpleAgent;

impl Agent for SimpleAgent {
    fn select_action(
        &mut self,
        state: &GameState,
        player_id: usize,
        _context: &mut AgentContext,
    ) -> AgentDecision {
        let player = state.players[player_id];
        let safe_actions = state
            .safe_actions(player_id)
            .expect("expected player_id to be valid");

        if safe_actions.is_empty() {
            AgentDecision {
                action: None,
                simulation_count: 0,
            }
        } else if safe_actions.contains(&Action::Straight) {
            AgentDecision {
                action: Some(Action::Straight),
                simulation_count: 0,
            }
        } else {
            if !safe_actions.contains(&Action::Left) {
                return AgentDecision {
                    action: Some(Action::Right),
                    simulation_count: 0,
                };
            } else if !safe_actions.contains(&Action::Right) {
                return AgentDecision {
                    action: Some(Action::Left),
                    simulation_count: 0,
                };
            }

            let (l_dx, l_dy) = Action::Left.apply_to_direction(player.direction).delta();
            let (r_dx, r_dy) = Action::Right.apply_to_direction(player.direction).delta();

            let (x, y) = (player.x, player.y);

            // actions are safe so conversions should be safe
            let (mut l_x, mut l_y) = ((x as isize + l_dx) as usize, (y as isize + l_dy) as usize);
            let (mut r_x, mut r_y) = ((x as isize + r_dx) as usize, (y as isize + r_dy) as usize);

            let mut decision = AgentDecision {
                action: None,
                simulation_count: 0,
            };

            while !state.is_occupied(l_x, l_y) && !state.is_occupied(r_x, r_y) {
                let Some(new_l_x) = l_x.checked_add_signed(l_dx) else {
                    decision.action = Some(Action::Right);
                    return decision;
                };
                let Some(new_l_y) = l_y.checked_add_signed(l_dy) else {
                    decision.action = Some(Action::Right);
                    return decision;
                };

                let Some(new_r_x) = r_x.checked_add_signed(r_dx) else {
                    decision.action = Some(Action::Left);
                    return decision;
                };
                let Some(new_r_y) = r_y.checked_add_signed(r_dy) else {
                    decision.action = Some(Action::Left);
                    return decision;
                };

                l_x = new_l_x;
                l_y = new_l_y;
                r_x = new_r_x;
                r_y = new_r_y;
            }

            if state.is_occupied(l_x, l_y) && !state.is_occupied(r_x, r_y) {
                decision.action = Some(Action::Right);
            } else if !state.is_occupied(l_x, l_y) && state.is_occupied(r_x, r_y) {
                decision.action = Some(Action::Left);
            } else {
                decision.action = Some(Action::Right);
            }

            decision
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
        fn prefers_straight_when_safe() {
            let state = setup_game_state(
                Grid::filled(5, 5, false),
                Spawn { x: 2, y: 2, direction: Direction::North },
            );

            let decision = SimpleAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, Some(Action::Straight));
            assert_eq!(decision.simulation_count, 0);
        }

        #[test]
        fn returns_none_when_no_safe_actions_exist() {
            let mut arena = Grid::filled(3, 3, false);
            for (x, y) in [(1, 0), (0, 1), (2, 1)] {
                arena[(x, y)] = true;
            }
            let state = setup_game_state(
                arena,
                Spawn { x: 1, y: 1, direction: Direction::North },
            );

            let decision = SimpleAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, None);
            assert_eq!(decision.simulation_count, 0);
        }

        #[test]
        fn turns_right_when_right_is_only_safe_action() {
            let mut arena = Grid::filled(5, 5, false);
            arena[(2, 1)] = true;
            arena[(1, 2)] = true;
            let state = setup_game_state(
                arena,
                Spawn { x: 2, y: 2, direction: Direction::North },
            );

            let decision = SimpleAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, Some(Action::Right));
            assert_eq!(decision.simulation_count, 0);
        }

        #[test]
        fn turns_left_when_left_is_only_safe_action() {
            let mut arena = Grid::filled(5, 5, false);
            arena[(2, 1)] = true;
            arena[(3, 2)] = true;
            let state = setup_game_state(
                arena,
                Spawn { x: 2, y: 2, direction: Direction::North },
            );

            let decision = SimpleAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, Some(Action::Left));
            assert_eq!(decision.simulation_count, 0);
        }

        #[test]
        fn chooses_left_when_left_has_more_straight_line_clearance() {
            let mut arena = Grid::filled(7, 5, false);
            arena[(4, 1)] = true;
            let state = setup_game_state(
                arena,
                Spawn { x: 4, y: 2, direction: Direction::North },
            );

            let decision = SimpleAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, Some(Action::Left));
            assert_eq!(decision.simulation_count, 0);
        }

        #[test]
        fn chooses_right_when_right_has_more_straight_line_clearance() {
            let mut arena = Grid::filled(7, 5, false);
            arena[(2, 1)] = true;
            let state = setup_game_state(
                arena,
                Spawn { x: 2, y: 2, direction: Direction::North },
            );

            let decision = SimpleAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, Some(Action::Right));
            assert_eq!(decision.simulation_count, 0);
        }

        #[test]
        fn chooses_right_when_left_and_right_have_equal_clearance() {
            let mut arena = Grid::filled(5, 5, false);
            arena[(2, 1)] = true;
            let state = setup_game_state(
                arena,
                Spawn { x: 2, y: 2, direction: Direction::North },
            );

            let decision = SimpleAgent.select_action(&state, 0, &mut context());

            assert_eq!(decision.action, Some(Action::Right));
            assert_eq!(decision.simulation_count, 0);
        }

        #[test]
        fn does_not_modify_original_game_state() {
            let mut arena = Grid::filled(5, 5, false);
            arena[(2, 1)] = true;
            let state = setup_game_state(
                arena,
                Spawn { x: 2, y: 2, direction: Direction::North },
            );
            let original = state.clone();

            let _ = SimpleAgent.select_action(&state, 0, &mut context());

            assert_eq!(state, original);
        }
    }
}
