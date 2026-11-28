use rand::rngs::ChaCha8Rng;

use crate::mcts::node::{ActionStats, JointAction};

pub struct PlayerPreferences {
    pub ucb_scores: [f64; 3],
    pub probabilities: [f64; 3],
}

pub type Preferences = Vec<Option<PlayerPreferences>>;

pub trait SelectionPolicy {
    fn get_preferences(
        &self,
        action_stats: &[Option<[ActionStats; 3]>],
        node_visits: u32,
    ) -> Preferences;

    fn select_joint_action(&self, preferences: &Preferences, rng: &mut ChaCha8Rng) -> JointAction;

    fn select_existing_child(
        &self,
        preferences: &Preferences,
        expanded_actions: &[&JointAction],
        rng: &mut ChaCha8Rng,
    ) -> usize; // index of chosen action from expanded_actions
}

pub struct DecoupledUcb {
    pub epsilon: f64,
}

impl Default for DecoupledUcb {
    fn default() -> Self {
        Self { epsilon: 0.1 }
    }
}

impl SelectionPolicy for DecoupledUcb {
    fn get_preferences(
        &self,
        action_stats: &[Option<[ActionStats; 3]>],
        node_visits: u32,
    ) -> Preferences {
        let mut preferences = Vec::with_capacity(action_stats.len());

        for action_stat in action_stats {
            let Some(action_stat) = action_stat else {
                preferences.push(None);
                continue;
            };

            // UCB-1 tuned
            // https://resolver.tudelft.nl/uuid:814a4ba7-9508-410e-ba12-79de980cb67b
            let ucb_scores = action_stat.each_ref().map(|stat| {
                if stat.visits == 0 {
                    f64::INFINITY
                } else {
                    let empirical_variance = (stat.reward_squared_sum / stat.visits as f64
                        - f64::powi(stat.mean_reward(), 2))
                    .max(0.0);

                    let variance_uncertainty =
                        f64::sqrt(2.0 * f64::ln(node_visits as f64) / stat.visits as f64);

                    stat.mean_reward()
                        + f64::sqrt(
                            f64::ln(node_visits as f64)
                                * f64::min(empirical_variance + variance_uncertainty, 0.25)
                                / stat.visits as f64,
                        )
                }
            });

            let max_ucb_score = ucb_scores.into_iter().reduce(f64::max).unwrap_or(0.);
            let num_max_scores = ucb_scores.iter().filter(|s| **s >= max_ucb_score).count();

            // epsilon-greedy method
            // https://en.wikipedia.org/wiki/Multi-armed_bandit#Semi-uniform_strategies
            let mut probabilities = [self.epsilon / 3.0; 3];
            for (id, score) in ucb_scores.iter().enumerate() {
                if *score >= max_ucb_score {
                    probabilities[id] += (1.0 - self.epsilon) / num_max_scores as f64;
                }
            }

            preferences.push(Some(PlayerPreferences {
                ucb_scores,
                probabilities,
            }));
        }

        preferences
    }

    fn select_joint_action(&self, preferences: &Preferences, rng: &mut ChaCha8Rng) -> JointAction {
        todo!()
    }

    fn select_existing_child(
        &self,
        preferences: &Preferences,
        expanded_actions: &[&JointAction],
        rng: &mut ChaCha8Rng,
    ) -> usize {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-10,
            "expected {expected}, got {actual}"
        );
    }

    mod decoupled_ucb {
        use super::*;

        mod get_preferences {
            use super::*;

            fn preferences(stats: [ActionStats; 3], node_visits: u32) -> PlayerPreferences {
                DecoupledUcb { epsilon: 0.1 }
                    .get_preferences(&[Some(stats)], node_visits)
                    .remove(0)
                    .unwrap()
            }

            #[test]
            fn unvisited_actions_have_infinite_scores_and_uniform_probabilities() {
                let p = preferences(
                    [
                        ActionStats::new(0, 0.0, 0.0),
                        ActionStats::new(0, 0.0, 0.0),
                        ActionStats::new(0, 0.0, 0.0),
                    ],
                    0,
                );
                assert!(p.ucb_scores.iter().all(|score| score.is_infinite()));
                for probability in p.probabilities {
                    assert_close(probability, 1.0 / 3.0);
                }
            }

            #[test]
            fn epsilon_greedy_distribution_has_a_single_winner() {
                let p = preferences(
                    [
                        ActionStats::new(10, 10.0, 10.0),
                        ActionStats::new(10, 0.0, 0.0),
                        ActionStats::new(10, 0.0, 0.0),
                    ],
                    30,
                );
                assert!(p.ucb_scores[0] > p.ucb_scores[1]);
                assert_close(p.probabilities[0], 0.9 + 0.1 / 3.0);
                assert_close(p.probabilities[1], 0.1 / 3.0);
                assert_close(p.probabilities[2], 0.1 / 3.0);
                assert_close(p.probabilities.iter().sum::<f64>(), 1.0);
            }

            #[test]
            fn epsilon_greedy_splits_exploitation_among_tied_winners() {
                let p = preferences(
                    [
                        ActionStats::new(10, 8.0, 6.4),
                        ActionStats::new(10, 8.0, 6.4),
                        ActionStats::new(10, 1.0, 0.1),
                    ],
                    30,
                );
                assert_close(p.probabilities[0], 0.9 / 2.0 + 0.1 / 3.0);
                assert_close(p.probabilities[1], 0.9 / 2.0 + 0.1 / 3.0);
                assert_close(p.probabilities[2], 0.1 / 3.0);
            }

            #[test]
            fn tuned_score_matches_variance_formula() {
                let p = preferences(
                    [
                        ActionStats::new(1000, 600.0, 400.0),
                        ActionStats::new(1000, 500.0, 250.0),
                        ActionStats::new(1000, 500.0, 250.0),
                    ],
                    3000,
                );
                let log_n = 3000.0_f64.ln();
                let variance = 0.4 - 0.6_f64.powi(2);
                let uncertainty = (2.0 * log_n / 1000.0).sqrt();
                let expected = 0.6 + (log_n / 1000.0 * (variance + uncertainty).min(0.25)).sqrt();
                assert_close(p.ucb_scores[0], expected);
            }

            #[test]
            fn tuned_bonus_is_larger_for_higher_variance_at_large_sample_counts() {
                let p = preferences(
                    [
                        ActionStats::new(1000, 500.0, 250.0),
                        ActionStats::new(1000, 500.0, 500.0),
                        ActionStats::new(1000, 500.0, 250.0),
                    ],
                    3000,
                );
                assert!(p.ucb_scores[1] > p.ucb_scores[0]);
            }

            #[test]
            fn eliminated_players_have_no_preferences() {
                let policy = DecoupledUcb::default();
                let stats = [
                    None,
                    Some([
                        ActionStats::new(0, 0.0, 0.0),
                        ActionStats::new(0, 0.0, 0.0),
                        ActionStats::new(0, 0.0, 0.0),
                    ]),
                    None,
                ];
                let p = policy.get_preferences(&stats, 0);
                assert!(p[0].is_none());
                assert!(p[1].is_some());
                assert!(p[2].is_none());
            }
        }
    }
}
