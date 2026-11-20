use std::{
    sync::{
        Arc,
        mpsc::{Receiver, channel},
    },
    thread,
    time::Duration,
};

use rand::{SeedableRng, rngs::ChaCha8Rng};
use tron_mcts::{
    agents::{
        interface::{Agent, AgentContext, SearchBudget},
        voronoi::VoronoiAgent,
    },
    engine::generate::{ArenaConfig, SpawnConfig, generate_arena, generate_spawns},
    runner::{
        match_runner::{MatchRunner, RunConfig},
        replay::Replay,
    },
    visualizer::{app::MatchEvent, run},
};

fn start_demo_match() -> (Replay, Receiver<MatchEvent>) {
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

    let contexts = (0..spawns.len())
        .map(|id| AgentContext {
            rng: ChaCha8Rng::seed_from_u64(1234 + id as u64),
            budget: SearchBudget::Time(Duration::new(0, 100_000_000)),
        })
        .collect();

    let arena = Arc::new(arena);

    let t_arena = arena.clone();
    let t_spawns = spawns.clone();
    let (tx, rx) = channel();

    thread::spawn(move || {
        let agents: Vec<Box<dyn Agent>> = (0..t_spawns.len())
            .map(|_| Box::new(VoronoiAgent) as Box<dyn Agent>)
            .collect();

        let mut match_runner = MatchRunner::new(
            Arc::clone(&t_arena),
            t_spawns,
            agents,
            contexts,
            RunConfig {
                max_ticks: 1500,
                record_replay: true,
            },
        )
        .expect("demo scenario must be valid");

        let result = match_runner.run_with_callback(|frame| {
            tx.send(MatchEvent::Frame(frame))
                .expect("receiver should exist")
        });

        let event = match result {
            Ok(_) => MatchEvent::Finished,
            Err(err) => MatchEvent::Error(err),
        };

        let _ = tx.send(event);
    });

    (
        Replay {
            arena,
            spawns,
            frames: vec![],
        },
        rx,
    )
}

fn main() -> eframe::Result {
    run(start_demo_match)
}
