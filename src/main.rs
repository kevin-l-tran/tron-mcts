use std::sync::Arc;

use rand::{SeedableRng, rngs::ChaCha8Rng};
use tron_mcts::{
    agents::{
        interface::{Agent, AgentContext, SearchBudget},
        random::SafeRandomAgent,
    },
    engine::generate::{ArenaConfig, SpawnConfig, generate_arena, generate_spawns},
    runner::{
        match_runner::{MatchRunner, RunConfig},
        replay::Replay,
    },
    visualizer::run,
};

fn demo_replay() -> Replay {
    let arena = generate_arena(
        &ArenaConfig {
            width: 80,
            height: 80,
            obstacle_density: 0.01,
        },
        &mut ChaCha8Rng::from_rng(&mut rand::rng()),
    )
    .expect("arena generation should succeed");

    let spawns = generate_spawns(
        &arena,
        &SpawnConfig {
            player_count: 12,
            min_spawn_distance: 10,
            min_boundary_distance: 5,
            min_reachable_region_proportion: 0.8,
            max_attempts: None,
        },
        &mut ChaCha8Rng::from_rng(&mut rand::rng()),
    )
    .expect("spawn generation should succeed");

    let agents: Vec<Box<dyn Agent>> = (0..spawns.len())
        .map(|_| Box::new(SafeRandomAgent) as Box<dyn Agent>)
        .collect();

    let contexts = (0..spawns.len())
        .map(|id| AgentContext {
            rng: ChaCha8Rng::seed_from_u64(1234 + id as u64),
            budget: SearchBudget::Simulations(0),
        })
        .collect();

    let arena = Arc::new(arena);
    let mut match_runner = MatchRunner::new(
        Arc::clone(&arena),
        spawns.clone(),
        agents,
        contexts,
        RunConfig {
            max_ticks: 1500,
            record_replay: true,
        },
    )
    .expect("demo scenario must be valid");

    let result = match_runner.run().expect("demo match should finish");
    Replay {
        arena,
        spawns,
        frames: result.replay.expect("replay recording is enabled"),
    }
}

fn main() -> eframe::Result {
    run(demo_replay)
}
