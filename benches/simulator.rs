use std::sync::Arc;

use divan::Bencher;
use rand::{SeedableRng, rngs::ChaCha8Rng};
use tron_mcts::{
    agents::{
        interface::{Agent, AgentContext, SearchBudget},
        simple::SimpleAgent,
    },
    engine::{action::Direction, grid::Grid, state::Spawn},
    runner::match_runner::{MatchRunner, RunConfig},
};

fn new_runner(player_count: usize, board_size: usize, config: RunConfig) -> MatchRunner {
    let arena = Arc::new(Grid::filled(board_size, board_size, false));

    // Distribute spawn locations evenly over the board.
    let columns = (player_count as f64).sqrt().ceil() as usize;
    let rows = player_count.div_ceil(columns);
    let spawns = (0..player_count)
        .map(|i| Spawn {
            x: (i % columns + 1) * board_size / (columns + 1),
            y: (i / columns + 1) * board_size / (rows + 1),
            direction: Direction::VALUES[i % 4],
        })
        .collect();

    let agents: Vec<Box<dyn Agent>> = (0..player_count)
        .map(|_| Box::new(SimpleAgent) as Box<dyn Agent>)
        .collect();

    let contexts = (0..player_count)
        .map(|i| AgentContext {
            rng: ChaCha8Rng::seed_from_u64(i as u64 + 1),
            budget: SearchBudget::Simulations(0),
        })
        .collect();

    MatchRunner::new(arena, spawns, agents, contexts, config).unwrap()
}

#[divan::bench(args = [(2, 40), (4, 56), (8, 80), (12, 100)])]
fn simple_match(bencher: divan::Bencher, case: (usize, usize)) {
    let (player_count, board_size) = case;
    bencher
        .with_inputs(|| {
            new_runner(
                player_count,
                board_size,
                RunConfig {
                    max_ticks: board_size * board_size,
                    record_replay: false,
                },
            )
        })
        .bench_local_values(|mut runner| {
            let result = runner.run().unwrap();
            divan::black_box(result.ticks);
        });
}

#[divan::bench(args = [false, true])]
fn replay_overhead(bencher: Bencher, record_replay: bool) {
    bencher
        .with_inputs(|| {
            new_runner(
                8,
                80,
                RunConfig {
                    max_ticks: 80 * 80,
                    record_replay,
                },
            )
        })
        .bench_local_values(|mut runner| {
            let result = runner.run().unwrap();
            divan::black_box(result.ticks);
        });
}

fn main() {
    divan::main();
}
