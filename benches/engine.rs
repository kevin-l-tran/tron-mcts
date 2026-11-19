use std::sync::Arc;

use tron_mcts::engine::{
    action::{Action, Direction},
    grid::Grid,
    state::{GameState, Spawn},
};

const BOARD_WIDTH: usize = 256;

fn new_state(player_count: usize) -> GameState {
    let arena = Arc::new(Grid::filled(BOARD_WIDTH, player_count * 2 + 1, false));
    let spawns = (0..player_count)
        .map(|id| Spawn {
            x: 0,
            y: id * 2 + 1,
            direction: Direction::East,
        })
        .collect();

    GameState::new(arena, spawns).unwrap()
}

fn straight_actions(player_count: usize) -> Vec<Option<Action>> {
    vec![Some(Action::Straight); player_count]
}

fn play_and_rewind(state: &mut GameState, actions: &[Option<Action>]) {
    let mut history = Vec::with_capacity(BOARD_WIDTH);

    // every player travels east for 255 ticks, then hits the boundary on tick 256.
    for _ in 0..BOARD_WIDTH {
        history.push(state.step(actions).unwrap());
    }
    assert!(state.is_terminal());

    while let Some(record) = history.pop() {
        state.undo_step(record).unwrap();
    }
}

#[divan::bench(args = [2, 4, 8, 12])]
fn step(bencher: divan::Bencher, player_count: usize) {
    let actions = straight_actions(player_count);
    bencher
        .with_inputs(|| new_state(player_count))
        .bench_local_values(|mut state| {
            divan::black_box(state.step(&actions).unwrap());
            divan::black_box(state);
        });
}

#[divan::bench(args = [2, 4, 8, 12])]
fn step_undo(bencher: divan::Bencher, player_count: usize) {
    let actions = straight_actions(player_count);
    bencher
        .with_inputs(|| new_state(player_count))
        .bench_local_values(|mut state| {
            let record = state.step(&actions).unwrap();
            state.undo_step(record).unwrap();
            divan::black_box(state);
        });
}

#[divan::bench(args = [2, 4, 8, 12])]
fn rollout(bencher: divan::Bencher, player_count: usize) {
    let actions = straight_actions(player_count);

    bencher
        .with_inputs(|| new_state(player_count))
        .bench_local_values(|mut state| {
            play_and_rewind(&mut state, &actions);
            divan::black_box(state);
        });
}

#[divan::bench(args = [2, 4, 8, 12])]
fn safe_actions(bencher: divan::Bencher, player_count: usize) {
    bencher
        .with_inputs(|| new_state(player_count))
        .bench_local_refs(|state| {
            for id in 0..player_count {
                divan::black_box(state.safe_actions(id).unwrap());
            }
        });
}

#[divan::bench(args = [2, 4, 8, 12])]
fn state_clone(bencher: divan::Bencher, player_count: usize) {
    bencher
        .with_inputs(|| new_state(player_count))
        .bench_local_refs(|state| {
            divan::black_box(state.clone());
        });
}

fn main() {
    divan::main();
}
