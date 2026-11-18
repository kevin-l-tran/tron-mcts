use std::{
    sync::Arc,
    time::{Duration, Instant},
};

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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TimeoutReason {
    MaxTickTimeout,
    MaxSimulationCountTimeout,
    MaxDurationTimeout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TerminationReason {
    GameOver,
    MaxTickTimeout,
    MaxSimulationCountTimeout,
    MaxDurationTimeout,
}

pub type ReplayFrame = Vec<Option<Action>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MatchRunnerError {
    InvalidGameStateSetup(GameStateError),
    InvalidNumberOfAgents(usize),
    InvalidNumberOfAgentContexts(usize),
    GameTerminated,
    RuntimeGameStateError(GameStateError),
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
    timeout_reason: Option<TimeoutReason>,
    total_decision_time: Duration,
    total_simulation_counts: usize,
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
            timeout_reason: None,
            total_decision_time: Duration::new(0, 0),
            total_simulation_counts: 0,
        })
    }

    pub fn step(&mut self) -> Result<(), MatchRunnerError> {
        if self.state.is_terminal() || self.timeout_reason.is_some() {
            return Err(MatchRunnerError::GameTerminated);
        }

        if self.state.tick >= self.config.max_ticks {
            self.timeout_reason = Some(TimeoutReason::MaxTickTimeout);
            return Err(MatchRunnerError::GameTerminated);
        }

        let mut actions = vec![];

        for (id, agent) in self.agents.iter_mut().enumerate() {
            // skip dead agents
            if self.state.players[id].eliminated_at.is_some() {
                actions.push(None);
                continue;
            }

            let now = Instant::now();

            // ask for decision
            let decision = agent.select_action(&self.state, id, &mut self.agent_contexts[id]);

            let elapsed = now.elapsed();

            // add agent action
            actions.push(decision.action);

            // update agent stats
            self.agent_stats[id].total_decision_time += elapsed;
            self.agent_stats[id].total_simulation_counts += decision.simulation_count;

            // update total caches
            self.total_decision_time += elapsed;
            self.total_simulation_counts += decision.simulation_count;
        }

        // execute actions
        let _ = match self.state.step(&actions) {
            Ok(val) => val,
            Err(err) => return Err(MatchRunnerError::RuntimeGameStateError(err)),
        };

        // update replay frames
        if let Some(replay) = &mut self.replay {
            replay.push(actions);
        }

        // update timeout status
        if self.state.tick >= self.config.max_ticks && !self.state.is_terminal() {
            self.timeout_reason = Some(TimeoutReason::MaxTickTimeout);
        }

        match self.config.decision_budget {
            SearchBudget::Simulations(max_count) => {
                if self.total_simulation_counts > max_count {
                    self.timeout_reason = Some(TimeoutReason::MaxSimulationCountTimeout);
                }
            }
            SearchBudget::Time(max_duration) => {
                if self.total_decision_time > max_duration {
                    self.timeout_reason = Some(TimeoutReason::MaxDurationTimeout);
                }
            }
        }

        Ok(())
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
                action: Some(Action::Straight),
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

    mod new {
        use super::*;

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

    mod step {
        use std::{
            cell::{Cell, RefCell},
            rc::Rc,
        };

        use super::*;

        struct FixedAgent {
            action: Option<Action>,
            simulation_count: usize,
        }

        impl Agent for FixedAgent {
            fn select_action(
                &mut self,
                _state: &GameState,
                _player_id: usize,
                _context: &mut AgentContext,
            ) -> AgentDecision {
                AgentDecision {
                    action: self.action,
                    simulation_count: self.simulation_count,
                }
            }
        }

        fn fixed_agents(
            actions: &[Option<Action>],
            simulation_count: usize,
        ) -> Vec<Box<dyn Agent>> {
            actions
                .iter()
                .map(|&action| {
                    Box::new(FixedAgent {
                        action,
                        simulation_count,
                    }) as Box<dyn Agent>
                })
                .collect()
        }

        fn runner_with_agents(spawns: Vec<Spawn>, agents: Vec<Box<dyn Agent>>) -> MatchRunner {
            let count = agents.len();
            MatchRunner::new(
                Arc::new(Grid::filled(6, 6, false)),
                spawns,
                agents,
                test_contexts(count),
                test_config(false),
            )
            .unwrap()
        }

        #[test]
        fn advances_game_by_one_tick() {
            let mut runner = test_runner(false);

            assert_eq!(runner.step(), Ok(()));

            assert_eq!(runner.state.tick, 1);
            assert_eq!(
                (runner.state.players[0].x, runner.state.players[0].y),
                (1, 0)
            );
            assert_eq!(
                (runner.state.players[1].x, runner.state.players[1].y),
                (2, 3)
            );
            assert_eq!(runner.state.alive_count, 2);
        }

        #[test]
        fn applies_each_agents_selected_action() {
            let spawns = vec![
                Spawn {
                    x: 1,
                    y: 1,
                    direction: Direction::East,
                },
                Spawn {
                    x: 4,
                    y: 3,
                    direction: Direction::West,
                },
            ];
            let mut runner = runner_with_agents(
                spawns,
                fixed_agents(&[Some(Action::Right), Some(Action::Left)], 0),
            );

            runner.step().unwrap();

            assert_eq!(
                (runner.state.players[0].x, runner.state.players[0].y),
                (1, 2)
            );
            assert_eq!(
                (runner.state.players[1].x, runner.state.players[1].y),
                (4, 4)
            );
            assert_eq!(runner.state.players[0].direction, Direction::South);
            assert_eq!(runner.state.players[1].direction, Direction::South);
        }

        struct ObservingAgent {
            observations: Rc<RefCell<Vec<GameState>>>,
        }

        impl Agent for ObservingAgent {
            fn select_action(
                &mut self,
                state: &GameState,
                _player_id: usize,
                _context: &mut AgentContext,
            ) -> AgentDecision {
                self.observations.borrow_mut().push(state.clone());
                AgentDecision {
                    action: Some(Action::Straight),
                    simulation_count: 0,
                }
            }
        }

        #[test]
        fn all_agents_observe_the_same_pre_step_state() {
            let observations = Rc::new(RefCell::new(Vec::new()));
            let agents = (0..2)
                .map(|_| {
                    Box::new(ObservingAgent {
                        observations: Rc::clone(&observations),
                    }) as Box<dyn Agent>
                })
                .collect();
            let mut runner = runner_with_agents(test_spawns(), agents);
            let initial_state = runner.state.clone();

            runner.step().unwrap();

            let observed = observations.borrow();
            assert_eq!(observed.len(), 2);
            assert_eq!(observed[0], initial_state);
            assert_eq!(observed[1], initial_state);
        }

        #[test]
        fn accumulates_agent_and_match_simulation_statistics() {
            let mut runner = runner_with_agents(
                test_spawns(),
                vec![
                    Box::new(FixedAgent {
                        action: Some(Action::Straight),
                        simulation_count: 3,
                    }),
                    Box::new(FixedAgent {
                        action: Some(Action::Straight),
                        simulation_count: 5,
                    }),
                ],
            );

            runner.step().unwrap();
            runner.step().unwrap();

            assert_eq!(runner.agent_stats[0].total_simulation_counts, 6);
            assert_eq!(runner.agent_stats[1].total_simulation_counts, 10);
            assert_eq!(runner.total_simulation_counts, 16);
            assert_eq!(
                runner.total_decision_time,
                runner
                    .agent_stats
                    .iter()
                    .map(|stats| stats.total_decision_time)
                    .sum(),
            );
        }

        struct RandomConsumingAgent;

        impl Agent for RandomConsumingAgent {
            fn select_action(
                &mut self,
                _state: &GameState,
                _player_id: usize,
                context: &mut AgentContext,
            ) -> AgentDecision {
                use rand::RngExt;
                let _: u64 = context.rng.random();
                AgentDecision {
                    action: Some(Action::Straight),
                    simulation_count: 0,
                }
            }
        }

        #[test]
        fn preserves_agent_rng_state_across_ticks() {
            use rand::RngExt;

            let mut runner = runner_with_agents(
                test_spawns(),
                vec![
                    Box::new(RandomConsumingAgent),
                    Box::new(RandomConsumingAgent),
                ],
            );
            let mut expected_rng = ChaCha8Rng::seed_from_u64(1);
            let _: u64 = expected_rng.random();
            let _: u64 = expected_rng.random();

            runner.step().unwrap();
            runner.step().unwrap();

            assert_eq!(runner.agent_contexts[0].rng, expected_rng);
        }

        #[test]
        fn records_one_replay_frame_per_tick_when_enabled() {
            let mut runner = test_runner(true);

            runner.step().unwrap();
            runner.step().unwrap();

            assert_eq!(
                runner.replay,
                Some(vec![
                    vec![Some(Action::Straight), Some(Action::Straight)],
                    vec![Some(Action::Straight), Some(Action::Straight)],
                ]),
            );
        }

        #[test]
        fn does_not_record_replay_when_disabled() {
            let mut runner = test_runner(false);

            runner.step().unwrap();

            assert_eq!(runner.replay, None);
        }

        #[test]
        fn returns_error_when_game_is_initially_terminal() {
            let spawns = vec![Spawn {
                x: 1,
                y: 1,
                direction: Direction::East,
            }];
            let mut runner = runner_with_agents(spawns, test_agents(1));
            let initial_state = runner.state.clone();

            assert_eq!(runner.step(), Err(MatchRunnerError::GameTerminated));
            assert_eq!(runner.state, initial_state);
            assert_eq!(runner.agent_stats[0].total_simulation_counts, 0);
        }

        #[test]
        fn returns_error_when_game_terminated() {
            let spawns = vec![
                Spawn {
                    x: 1,
                    y: 2,
                    direction: Direction::East,
                },
                Spawn {
                    x: 3,
                    y: 2,
                    direction: Direction::West,
                },
            ];
            let mut runner = runner_with_agents(spawns, test_agents(2));

            runner.step().unwrap();
            assert_eq!(runner.state.alive_count, 0);
            let terminal_state = runner.state.clone();

            assert_eq!(runner.step(), Err(MatchRunnerError::GameTerminated));
            assert_eq!(runner.state, terminal_state);
        }

        #[test]
        fn returns_error_when_timeout_has_already_occurred() {
            let mut runner = test_runner(false);
            runner.timeout_reason = Some(TimeoutReason::MaxTickTimeout);
            let initial_state = runner.state.clone();

            assert_eq!(runner.step(), Err(MatchRunnerError::GameTerminated));
            assert_eq!(runner.state, initial_state);
        }

        #[test]
        fn enforces_maximum_tick_count_without_extra_tick() {
            let mut runner = test_runner(false);
            runner.config.max_ticks = 1;

            runner.step().unwrap();

            assert_eq!(runner.state.tick, 1);
            assert_eq!(runner.timeout_reason, Some(TimeoutReason::MaxTickTimeout));
            assert_eq!(runner.step(), Err(MatchRunnerError::GameTerminated));
            assert_eq!(runner.state.tick, 1);
        }

        #[test]
        fn does_not_advance_when_maximum_ticks_is_zero() {
            let mut runner = test_runner(false);
            runner.config.max_ticks = 0;
            let initial_state = runner.state.clone();

            assert_eq!(runner.step(), Err(MatchRunnerError::GameTerminated));
            assert_eq!(runner.state, initial_state);
        }

        struct CountingAgent {
            calls: Rc<Cell<usize>>,
            simulation_count: usize,
        }

        impl Agent for CountingAgent {
            fn select_action(
                &mut self,
                _state: &GameState,
                _player_id: usize,
                _context: &mut AgentContext,
            ) -> AgentDecision {
                self.calls.set(self.calls.get() + 1);
                AgentDecision {
                    action: Some(Action::Straight),
                    simulation_count: self.simulation_count,
                }
            }
        }

        #[test]
        fn skips_agents_eliminated_on_previous_ticks() {
            let spawns = vec![
                Spawn {
                    x: 0,
                    y: 0,
                    direction: Direction::North,
                },
                Spawn {
                    x: 0,
                    y: 5,
                    direction: Direction::East,
                },
                Spawn {
                    x: 5,
                    y: 5,
                    direction: Direction::West,
                },
            ];
            let calls = Rc::new(Cell::new(0));
            let agents: Vec<Box<dyn Agent>> = vec![
                Box::new(CountingAgent {
                    calls: Rc::clone(&calls),
                    simulation_count: 7,
                }),
                Box::new(TestAgent),
                Box::new(TestAgent),
            ];
            let mut runner = runner_with_agents(spawns, agents);

            runner.step().unwrap();
            assert_eq!(runner.state.players[0].eliminated_at, Some(1));
            assert_eq!(runner.state.alive_count, 2);
            runner.step().unwrap();

            assert_eq!(calls.get(), 1);
            assert_eq!(runner.agent_stats[0].total_simulation_counts, 7);
        }

        #[test]
        fn records_simulation_count_timeout_after_exceeding_budget() {
            let mut runner = runner_with_agents(
                test_spawns(),
                fixed_agents(&[Some(Action::Straight), Some(Action::Straight)], 3),
            );
            runner.config.decision_budget = SearchBudget::Simulations(5);

            runner.step().unwrap();

            assert_eq!(runner.total_simulation_counts, 6);
            assert_eq!(
                runner.timeout_reason,
                Some(TimeoutReason::MaxSimulationCountTimeout),
            );
            assert_eq!(runner.step(), Err(MatchRunnerError::GameTerminated));
        }

        #[test]
        fn records_time_timeout_when_cumulative_decision_time_exceeds_budget() {
            let mut runner = test_runner(false);
            runner.config.decision_budget = SearchBudget::Time(Duration::from_secs(1));
            runner.total_decision_time = Duration::from_secs(2);

            runner.step().unwrap();

            assert_eq!(
                runner.timeout_reason,
                Some(TimeoutReason::MaxDurationTimeout)
            );
            assert_eq!(runner.step(), Err(MatchRunnerError::GameTerminated));
        }

        #[test]
        fn accumulates_simulation_counts_before_timeout() {
            let mut runner = runner_with_agents(
                test_spawns(),
                fixed_agents(&[Some(Action::Straight), Some(Action::Straight)], 2),
            );
            runner.config.decision_budget = SearchBudget::Simulations(5);

            runner.step().unwrap();
            assert_eq!(runner.total_simulation_counts, 4);
            assert_eq!(runner.timeout_reason, None);

            runner.step().unwrap();
            assert_eq!(runner.total_simulation_counts, 8);
            assert_eq!(
                runner.timeout_reason,
                Some(TimeoutReason::MaxSimulationCountTimeout),
            );
        }
    }
}
