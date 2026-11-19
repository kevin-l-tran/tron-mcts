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
