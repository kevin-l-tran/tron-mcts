#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;

    use rand::{RngExt, SeedableRng, rngs::ChaCha8Rng};
    use tron_mcts::{
        agents::{
            interface::{Agent, AgentContext, SearchBudget},
            random::{RandomAgent, SafeRandomAgent},
        },
        engine::{
            action::Direction,
            grid::Grid,
            state::{GameState, Spawn},
        },
        runner::match_runner::{MatchRunner, RunConfig, TerminationReason},
    };

    fn contexts(count: usize, seed: u64) -> Vec<AgentContext> {
        (0..count)
            .map(|id| AgentContext {
                rng: ChaCha8Rng::seed_from_u64(seed + id as u64 + 1),
                budget: SearchBudget::Simulations(100),
            })
            .collect()
    }

    fn seeded_setup(count: usize, seed: u64) -> (Arc<Grid<bool>>, Vec<Spawn>) {
        let spawns = (0..count)
            .map(|id| Spawn {
                x: 2 + (id % 4) * 5,
                y: 2 + (id / 4) * 5,
                direction: Direction::East,
            })
            .collect::<Vec<_>>();

        let mut arena = Grid::filled(20, 20, false);
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        for cell in &mut arena.cells {
            *cell = rng.random_bool(0.08);
        }
        for spawn in &spawns {
            arena[(spawn.x, spawn.y)] = false;
        }

        (Arc::new(arena), spawns)
    }

    fn seeded_runner(count: usize, seed: u64, record_replay: bool, safe: bool) -> MatchRunner {
        let (arena, spawns) = seeded_setup(count, seed);
        runner_from_setup(arena, spawns, seed, record_replay, safe)
    }

    fn runner_from_setup(
        arena: Arc<Grid<bool>>,
        spawns: Vec<Spawn>,
        seed: u64,
        record_replay: bool,
        safe: bool,
    ) -> MatchRunner {
        let count = spawns.len();
        let agents = (0..count)
            .map(|_| {
                if safe {
                    Box::new(SafeRandomAgent) as Box<dyn Agent>
                } else {
                    Box::new(RandomAgent) as Box<dyn Agent>
                }
            })
            .collect();

        MatchRunner::new(
            arena,
            spawns,
            agents,
            contexts(count, seed),
            RunConfig {
                max_ticks: 1000,
                record_replay,
            },
        )
        .unwrap()
    }

    #[test]
    fn random_agents_complete_match() {
        let result = seeded_runner(4, 1, false, false).run().unwrap();

        assert_eq!(result.termination, TerminationReason::GameOver);
        assert!(result.ticks > 0);
        assert!(result.final_state.alive_count <= 1);
    }

    #[test]
    fn same_seed_produces_same_game() {
        let first = seeded_runner(8, 42, true, true).run().unwrap();
        let second = seeded_runner(8, 42, true, true).run().unwrap();

        assert_eq!(first.final_state, second.final_state);
        assert_eq!(first.ticks, second.ticks);
        assert_eq!(first.ranks, second.ranks);
        assert_eq!(first.replay, second.replay);
    }

    #[test]
    fn replay_reproduces_final_state() {
        let (arena, spawns) = seeded_setup(6, 7);
        let mut replayed_state = GameState::new(Arc::clone(&arena), spawns.clone()).unwrap();

        let result = runner_from_setup(arena, spawns, 7, true, true)
            .run()
            .unwrap();
        for actions in result.replay.as_ref().unwrap() {
            replayed_state.step(actions).unwrap();
        }

        assert_eq!(replayed_state, result.final_state);
    }

    #[test]
    fn handles_twelve_players() {
        let result = seeded_runner(12, 10, false, true).run().unwrap();

        assert_eq!(result.termination, TerminationReason::GameOver);
        assert_eq!(result.final_state.players.len(), 12);
        assert_eq!(result.agent_stats.len(), 12);
        assert_eq!(result.ranks.unwrap().len(), 12);
    }

    #[test]
    fn handles_no_safe_actions() {
        let mut arena = Grid::filled(5, 5, false);
        for (x, y) in [(2, 1), (2, 3), (1, 2), (3, 2)] {
            arena[(x, y)] = true;
        }

        let spawns = vec![
            Spawn {
                x: 2,
                y: 2,
                direction: Direction::North,
            },
            Spawn {
                x: 4,
                y: 4,
                direction: Direction::West,
            },
        ];
        let agents: Vec<Box<dyn Agent>> =
            vec![Box::new(SafeRandomAgent), Box::new(SafeRandomAgent)];

        let mut runner = MatchRunner::new(
            Arc::new(arena),
            spawns,
            agents,
            contexts(2, 10),
            RunConfig {
                max_ticks: 10,
                record_replay: true,
            },
        )
        .unwrap();

        let result = runner.run().unwrap();

        assert_eq!(result.termination, TerminationReason::GameOver);
        assert_eq!(result.final_state.players[0].eliminated_at, Some(1));
        assert_eq!(result.replay.unwrap()[0][0], None);
    }

    #[test]
    fn completes_many_random_games() {
        for seed in 0..200 {
            let count = [2, 4, 8, 12][seed as usize % 4];
            let result = seeded_runner(count, seed, false, seed % 2 == 0)
                .run()
                .unwrap();

            assert_eq!(
                result.termination,
                TerminationReason::GameOver,
                "seed {seed}"
            );
            assert_eq!(result.final_state.players.len(), count);
        }
    }

    #[test]
    fn replay_does_not_change_outcome() {
        let with_replay = seeded_runner(8, 77, true, true).run().unwrap();
        let without_replay = seeded_runner(8, 77, false, true).run().unwrap();

        assert_eq!(with_replay.final_state, without_replay.final_state);
        assert_eq!(with_replay.ticks, without_replay.ticks);
        assert_eq!(with_replay.ranks, without_replay.ranks);
        assert_eq!(with_replay.termination, without_replay.termination);
        assert!(with_replay.replay.is_some());
        assert!(without_replay.replay.is_none());
    }
}
