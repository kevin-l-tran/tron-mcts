use std::{sync::Arc, time::Duration};

use crate::{
    agents::interface::{Agent, AgentContext, SearchBudget},
    engine::{
        action::Action,
        state::{Arena, GameState, GameStateError, Spawn},
    },
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
pub enum MatchRunnerError {
    InvalidGameStateSetup(GameStateError),
    InvalidNumberOfAgents(usize),
    InvalidNumberOfAgentContexts(usize),
}

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
    pub fn new(
        arena: Arc<Arena>,
        spawns: Vec<Spawn>,
        agents: Vec<Box<dyn Agent>>,
        agent_contexts: Vec<AgentContext>,
        config: RunConfig,
    ) -> Result<MatchRunner, MatchRunnerError> {
        if agents.len() != spawns.len() {
            return Err(MatchRunnerError::InvalidNumberOfAgents(agents.len()));
        }

        if agent_contexts.len() != agents.len() {
            return Err(MatchRunnerError::InvalidNumberOfAgentContexts(
                agent_contexts.len(),
            ));
        }

        let state = match GameState::new(arena, spawns) {
            Ok(val) => val,
            Err(err) => return Err(MatchRunnerError::InvalidGameStateSetup(err)),
        };

        let agent_stats = vec![
            AgentStats {
                total_decision_time: Duration::new(0, 0),
                total_simulation_counts: 0,
            };
            agents.len()
        ];

        let replay = if config.record_replay {
            Some(vec![])
        } else {
            None
        };

        Ok(MatchRunner {
            agents,
            agent_contexts,
            agent_stats,
            config,
            replay,
            state,
        })
    }

    pub fn step(&mut self) -> Result<(), MatchRunnerError> {
        todo!()
    }

    pub fn run(&mut self) -> Result<MatchResult, MatchRunnerError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::ChaCha8Rng};

    use crate::{
        agents::interface::AgentDecision,
        engine::{action::Direction, grid::Grid},
    };

    use super::*;

    struct TestAgent;

    impl Agent for TestAgent {
        fn select_action(
            &mut self,
            _state: &GameState,
            _player_id: usize,
            _context: &mut AgentContext,
        ) -> AgentDecision {
            AgentDecision {
                action: Action::Straight,
                simulation_count: 0,
            }
        }
    }

    fn test_agents(count: usize) -> Vec<Box<dyn Agent>> {
        (0..count)
            .map(|_| Box::new(TestAgent) as Box<dyn Agent>)
            .collect()
    }

    fn test_contexts(count: usize) -> Vec<AgentContext> {
        (0..count)
            .map(|id| AgentContext {
                rng: ChaCha8Rng::seed_from_u64(id as u64 + 1),
                budget: SearchBudget::Simulations(100),
            })
            .collect()
    }

    fn test_spawns() -> Vec<Spawn> {
        vec![
            Spawn {
                x: 0,
                y: 0,
                direction: Direction::East,
            },
            Spawn {
                x: 3,
                y: 3,
                direction: Direction::West,
            },
        ]
    }

    fn test_config(record_replay: bool) -> RunConfig {
        RunConfig {
            max_ticks: 100,
            decision_budget: SearchBudget::Simulations(100),
            record_replay,
            seed: 42,
        }
    }

    fn test_runner(record_replay: bool) -> MatchRunner {
        MatchRunner::new(
            Arc::new(Grid::filled(4, 4, false)),
            test_spawns(),
            test_agents(2),
            test_contexts(2),
            test_config(record_replay),
        )
        .unwrap()
    }

    #[test]
    fn initializes_game_state_from_spawns() {
        let arena = Arc::new(Grid::filled(4, 4, false));
        let spawns = test_spawns();
        let runner = MatchRunner::new(
            Arc::clone(&arena),
            spawns.clone(),
            test_agents(2),
            test_contexts(2),
            test_config(false),
        )
        .unwrap();

        assert!(Arc::ptr_eq(&runner.state.arena, &arena));
        assert_eq!(runner.state.tick, 0);
        assert_eq!(runner.state.alive_count, 2);
        assert_eq!(runner.state.players.len(), 2);
        assert_eq!(runner.state.trails[(0, 0)], 1);
        assert_eq!(runner.state.trails[(3, 3)], 2);

        for (player, spawn) in runner.state.players.iter().zip(spawns.iter()) {
            assert_eq!((player.x, player.y), (spawn.x, spawn.y));
            assert_eq!(player.direction, spawn.direction);
            assert_eq!(player.eliminated_at, None);
        }
    }

    #[test]
    fn preserves_config_agents_and_agent_contexts() {
        let config = test_config(false);
        let contexts = test_contexts(2);
        let expected_contexts = test_contexts(2);
        let runner = MatchRunner::new(
            Arc::new(Grid::filled(4, 4, false)),
            test_spawns(),
            test_agents(2),
            contexts,
            config,
        )
        .unwrap();

        assert_eq!(runner.config, config);
        assert_eq!(runner.agents.len(), 2);
        assert_eq!(runner.agent_contexts, expected_contexts);
    }

    #[test]
    fn initializes_zero_agent_statistics() {
        let runner = test_runner(false);

        assert_eq!(runner.agent_stats.len(), 2);
        for stats in &runner.agent_stats {
            assert_eq!(stats.total_decision_time, Duration::ZERO);
            assert_eq!(stats.total_simulation_counts, 0);
        }
    }

    #[test]
    fn initializes_empty_replay_when_enabled() {
        let runner = test_runner(true);

        assert_eq!(runner.replay, Some(vec![]));
    }

    #[test]
    fn disables_replay_when_not_requested() {
        let runner = test_runner(false);

        assert_eq!(runner.replay, None);
    }

    #[test]
    fn rejects_too_few_agents() {
        let result = MatchRunner::new(
            Arc::new(Grid::filled(4, 4, false)),
            test_spawns(),
            test_agents(1),
            test_contexts(1),
            test_config(false),
        );

        assert!(matches!(
            result,
            Err(MatchRunnerError::InvalidNumberOfAgents(1))
        ));
    }

    #[test]
    fn rejects_too_many_agents() {
        let result = MatchRunner::new(
            Arc::new(Grid::filled(4, 4, false)),
            test_spawns(),
            test_agents(3),
            test_contexts(3),
            test_config(false),
        );

        assert!(matches!(
            result,
            Err(MatchRunnerError::InvalidNumberOfAgents(3))
        ));
    }

    #[test]
    fn rejects_too_few_agent_contexts() {
        let result = MatchRunner::new(
            Arc::new(Grid::filled(4, 4, false)),
            test_spawns(),
            test_agents(2),
            test_contexts(1),
            test_config(false),
        );

        assert!(matches!(
            result,
            Err(MatchRunnerError::InvalidNumberOfAgentContexts(1))
        ));
    }

    #[test]
    fn rejects_too_many_agent_contexts() {
        let result = MatchRunner::new(
            Arc::new(Grid::filled(4, 4, false)),
            test_spawns(),
            test_agents(2),
            test_contexts(3),
            test_config(false),
        );

        assert!(matches!(
            result,
            Err(MatchRunnerError::InvalidNumberOfAgentContexts(3))
        ));
    }

    #[test]
    fn propagates_out_of_bounds_spawn_error() {
        let mut spawns = test_spawns();
        spawns[0].x = 4;
        let bad_spawn = spawns[0];

        let result = MatchRunner::new(
            Arc::new(Grid::filled(4, 4, false)),
            spawns,
            test_agents(2),
            test_contexts(2),
            test_config(false),
        );

        assert!(matches!(
            result,
            Err(MatchRunnerError::InvalidGameStateSetup(
                GameStateError::SpawnOutOfArena(spawn)
            )) if spawn == bad_spawn
        ));
    }

    #[test]
    fn propagates_spawn_on_obstacle_error() {
        let mut arena = Grid::filled(4, 4, false);
        arena[(0, 0)] = true;
        let bad_spawn = test_spawns()[0];

        let result = MatchRunner::new(
            Arc::new(arena),
            test_spawns(),
            test_agents(2),
            test_contexts(2),
            test_config(false),
        );

        assert!(matches!(
            result,
            Err(MatchRunnerError::InvalidGameStateSetup(
                GameStateError::SpawnOnObstacle(spawn)
            )) if spawn == bad_spawn
        ));
    }

    #[test]
    fn propagates_duplicate_spawn_error() {
        let mut spawns = test_spawns();
        spawns[1].x = spawns[0].x;
        spawns[1].y = spawns[0].y;
        let first_spawn = spawns[0];
        let second_spawn = spawns[1];

        let result = MatchRunner::new(
            Arc::new(Grid::filled(4, 4, false)),
            spawns,
            test_agents(2),
            test_contexts(2),
            test_config(false),
        );

        assert!(matches!(
            result,
            Err(MatchRunnerError::InvalidGameStateSetup(
                GameStateError::DuplicateSpawnLocation(first, second)
            )) if first == first_spawn && second == second_spawn
        ));
    }
}
