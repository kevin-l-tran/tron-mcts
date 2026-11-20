use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    agents::interface::{Agent, AgentContext},
    engine::{
        action::Action,
        state::{Arena, GameState, GameStateError, Spawn},
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct RunConfig {
    pub max_ticks: usize,
    pub record_replay: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TerminationReason {
    GameOver,
    MaxTickTimeout,
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

    pub fn step(&mut self) -> Result<Vec<Option<Action>>, MatchRunnerError> {
        if self.termination_reason().is_some() {
            return Err(MatchRunnerError::GameTerminated);
        }

        let mut actions = Vec::with_capacity(self.agents.len());

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
        }

        // execute actions
        let _ = match self.state.step(&actions) {
            Ok(val) => val,
            Err(err) => return Err(MatchRunnerError::RuntimeGameStateError(err)),
        };

        // update replay frames
        if let Some(replay) = &mut self.replay {
            replay.push(actions.clone());
        }

        Ok(actions)
    }

    pub fn run_with_callback(
        &mut self,
        mut callback: impl FnMut(ReplayFrame),
    ) -> Result<MatchResult, MatchRunnerError> {
        while self.termination_reason().is_none() {
            let frame = self.step()?;
            callback(frame);
        }

        let termination_reason = self.termination_reason().unwrap();

        Ok(MatchResult {
            final_state: self.state.clone(),
            ticks: self.state.tick,
            ranks: self.state.outcome(),
            termination: termination_reason,
            agent_stats: self.agent_stats.clone(),
            replay: self.replay.clone(),
        })
    }

    pub fn run(&mut self) -> Result<MatchResult, MatchRunnerError> {
        while self.termination_reason().is_none() {
            self.step()?;
        }

        let termination_reason = self.termination_reason().unwrap();

        Ok(MatchResult {
            final_state: self.state.clone(),
            ticks: self.state.tick,
            ranks: self.state.outcome(),
            termination: termination_reason,
            agent_stats: self.agent_stats.clone(),
            replay: self.replay.clone(),
        })
    }

    fn termination_reason(&self) -> Option<TerminationReason> {
        if self.state.is_terminal() {
            Some(TerminationReason::GameOver)
        } else if self.state.tick >= self.config.max_ticks {
            Some(TerminationReason::MaxTickTimeout)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::ChaCha8Rng};

    use crate::{
        agents::interface::{AgentDecision, SearchBudget},
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

            assert!(runner.step().is_ok());

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
        fn accumulates_agent_simulation_statistics() {
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
        fn returns_error_when_tick_limit_has_been_reached() {
            let mut runner = test_runner(false);
            runner.config.max_ticks = 0;
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
            assert_eq!(
                runner.termination_reason(),
                Some(TerminationReason::MaxTickTimeout)
            );
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
        fn simulation_counts_do_not_terminate_match() {
            let mut runner = runner_with_agents(
                test_spawns(),
                fixed_agents(&[Some(Action::Straight), Some(Action::Straight)], 3),
            );
            for context in &mut runner.agent_contexts {
                context.budget = SearchBudget::Simulations(1);
            }

            runner.step().unwrap();
            runner.step().unwrap();

            assert_eq!(runner.state.tick, 2);
            assert_eq!(runner.agent_stats[0].total_simulation_counts, 6);
            assert_eq!(runner.agent_stats[1].total_simulation_counts, 6);
            assert_eq!(runner.termination_reason(), None);
        }

        #[test]
        fn decision_time_does_not_terminate_match() {
            let mut runner = test_runner(false);
            for context in &mut runner.agent_contexts {
                context.budget = SearchBudget::Time(Duration::ZERO);
            }

            runner.step().unwrap();
            runner.step().unwrap();

            assert_eq!(runner.state.tick, 2);
            assert_eq!(runner.termination_reason(), None);
        }

        #[test]
        fn accumulates_simulation_counts_across_ticks() {
            let mut runner = runner_with_agents(
                test_spawns(),
                fixed_agents(&[Some(Action::Straight), Some(Action::Straight)], 2),
            );

            runner.step().unwrap();
            assert_eq!(runner.agent_stats[0].total_simulation_counts, 2);
            assert_eq!(runner.agent_stats[1].total_simulation_counts, 2);

            runner.step().unwrap();
            assert_eq!(runner.agent_stats[0].total_simulation_counts, 4);
            assert_eq!(runner.agent_stats[1].total_simulation_counts, 4);
        }
    }

    mod run {
        use super::*;

        struct SimulationAgent {
            simulation_count: usize,
        }

        impl Agent for SimulationAgent {
            fn select_action(
                &mut self,
                _state: &GameState,
                _player_id: usize,
                _context: &mut AgentContext,
            ) -> AgentDecision {
                AgentDecision {
                    action: Some(Action::Straight),
                    simulation_count: self.simulation_count,
                }
            }
        }

        fn simulation_runner(simulation_count: usize, record_replay: bool) -> MatchRunner {
            MatchRunner::new(
                Arc::new(Grid::filled(4, 4, false)),
                test_spawns(),
                (0..2)
                    .map(|_| Box::new(SimulationAgent { simulation_count }) as Box<dyn Agent>)
                    .collect(),
                test_contexts(2),
                test_config(record_replay),
            )
            .unwrap()
        }

        fn collision_runner(max_ticks: usize, record_replay: bool) -> MatchRunner {
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
            let mut config = test_config(record_replay);
            config.max_ticks = max_ticks;

            MatchRunner::new(
                Arc::new(Grid::filled(5, 5, false)),
                spawns,
                test_agents(2),
                test_contexts(2),
                config,
            )
            .unwrap()
        }

        #[test]
        fn runs_until_all_players_collide() {
            let mut runner = collision_runner(10, true);

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 1);
            assert_eq!(result.final_state.tick, 1);
            assert_eq!(result.final_state.alive_count, 0);
            assert_eq!(result.termination, TerminationReason::GameOver);
            assert_eq!(result.ranks, Some(vec![1.5, 1.5]));
            assert_eq!(result.replay, Some(vec![vec![Some(Action::Straight); 2]]));
        }

        #[test]
        fn returns_ranks_when_one_player_survives() {
            let spawns = vec![
                Spawn {
                    x: 0,
                    y: 0,
                    direction: Direction::North,
                },
                Spawn {
                    x: 3,
                    y: 3,
                    direction: Direction::West,
                },
            ];
            let mut runner = MatchRunner::new(
                Arc::new(Grid::filled(4, 4, false)),
                spawns,
                test_agents(2),
                test_contexts(2),
                test_config(false),
            )
            .unwrap();

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 1);
            assert_eq!(result.termination, TerminationReason::GameOver);
            assert_eq!(result.final_state.alive_count, 1);
            assert_eq!(result.ranks, Some(vec![2.0, 1.0]));
        }

        #[test]
        fn returns_immediately_when_game_is_initially_terminal() {
            let spawns = vec![Spawn {
                x: 1,
                y: 1,
                direction: Direction::East,
            }];
            let mut runner = MatchRunner::new(
                Arc::new(Grid::filled(4, 4, false)),
                spawns,
                test_agents(1),
                test_contexts(1),
                test_config(true),
            )
            .unwrap();

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 0);
            assert_eq!(result.termination, TerminationReason::GameOver);
            assert_eq!(result.ranks, Some(vec![1.0]));
            assert_eq!(result.replay, Some(vec![]));
            assert_eq!(result.agent_stats[0].total_simulation_counts, 0);
        }

        #[test]
        fn stops_at_tick_limit_without_finishing_game() {
            let mut runner = test_runner(false);
            runner.config.max_ticks = 2;

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 2);
            assert_eq!(result.final_state.tick, 2);
            assert_eq!(result.final_state.alive_count, 2);
            assert_eq!(result.termination, TerminationReason::MaxTickTimeout);
            assert_eq!(result.ranks, None);
        }

        #[test]
        fn does_not_step_when_max_ticks_is_zero() {
            let mut runner = test_runner(true);
            runner.config.max_ticks = 0;
            let initial_state = runner.state.clone();

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 0);
            assert_eq!(result.termination, TerminationReason::MaxTickTimeout);
            assert_eq!(result.final_state, initial_state);
            assert_eq!(result.replay, Some(vec![]));
            assert!(
                result
                    .agent_stats
                    .iter()
                    .all(|stats| stats.total_simulation_counts == 0)
            );
        }

        #[test]
        fn prefers_game_over_when_game_ends_on_last_allowed_tick() {
            let mut runner = collision_runner(1, false);

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 1);
            assert_eq!(result.termination, TerminationReason::GameOver);
        }

        #[test]
        fn records_every_action_frame_for_a_complete_match() {
            let mut runner = test_runner(true);

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 4);
            assert_eq!(result.termination, TerminationReason::GameOver);
            assert_eq!(
                result.replay,
                Some(vec![vec![Some(Action::Straight); 2]; 4])
            );
        }

        #[test]
        fn omits_replay_when_recording_is_disabled() {
            let mut runner = test_runner(false);

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 4);
            assert_eq!(result.replay, None);
        }

        #[test]
        fn continues_after_manual_steps_without_resetting_statistics_or_replay() {
            let mut runner = simulation_runner(3, true);
            runner.step().unwrap();
            assert_eq!(runner.state.tick, 1);

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 4);
            assert_eq!(result.termination, TerminationReason::GameOver);
            assert_eq!(result.replay.as_ref().unwrap().len(), 4);
            assert_eq!(result.agent_stats.len(), 2);
            assert_eq!(result.agent_stats[0].total_simulation_counts, 12);
            assert_eq!(result.agent_stats[1].total_simulation_counts, 12);
            assert_eq!(result.agent_stats, runner.agent_stats);
            assert_eq!(result.replay, runner.replay);
        }

        #[test]
        fn runs_to_game_over_despite_simulation_counts() {
            let mut runner = simulation_runner(3, false);
            for context in &mut runner.agent_contexts {
                context.budget = SearchBudget::Simulations(1);
            }

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 4);
            assert_eq!(result.termination, TerminationReason::GameOver);
            assert_eq!(result.agent_stats[0].total_simulation_counts, 12);
            assert_eq!(result.agent_stats[1].total_simulation_counts, 12);
        }

        #[test]
        fn runs_to_game_over_despite_zero_decision_time_budget() {
            let mut runner = test_runner(false);
            for context in &mut runner.agent_contexts {
                context.budget = SearchBudget::Time(Duration::ZERO);
            }

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 4);
            assert_eq!(result.termination, TerminationReason::GameOver);
        }

        #[test]
        fn running_an_already_finished_match_does_not_change_its_result() {
            let mut runner = collision_runner(10, true);
            let first_result = runner.run().unwrap();

            let second_result = runner.run().unwrap();

            assert_eq!(second_result, first_result);
        }

        #[test]
        fn reports_game_over_when_agents_report_simulations() {
            let mut runner = collision_runner(10, false);
            runner.agents = (0..2)
                .map(|_| {
                    Box::new(SimulationAgent {
                        simulation_count: 3,
                    }) as Box<dyn Agent>
                })
                .collect();
            for context in &mut runner.agent_contexts {
                context.budget = SearchBudget::Simulations(1);
            }

            let result = runner.run().unwrap();

            assert_eq!(result.ticks, 1);
            assert_eq!(result.final_state.alive_count, 0);
            assert_eq!(result.termination, TerminationReason::GameOver);
        }
    }
}
