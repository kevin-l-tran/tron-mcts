use std::collections::{HashSet, VecDeque};

use rand::{Rng, RngExt, seq::SliceRandom};

use crate::engine::{
    action::Direction::{East, North, South, West},
    state::{Arena, Spawn},
};

#[derive(Clone, Copy, Debug)]
pub struct ArenaConfig {
    pub width: usize,
    pub height: usize,
    pub obstacle_density: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ArenaGenerationError {
    InvalidDimensions,
    ArenaTooLarge,
    InvalidObstacleDensity,
}

pub fn generate_arena(
    config: &ArenaConfig,
    rng: &mut impl Rng,
) -> Result<Arena, ArenaGenerationError> {
    if config.width == 0 || config.height == 0 {
        return Err(ArenaGenerationError::InvalidDimensions);
    }

    let Some(cell_count) = config.width.checked_mul(config.height) else {
        return Err(ArenaGenerationError::ArenaTooLarge);
    };

    if cell_count > isize::MAX as usize {
        return Err(ArenaGenerationError::ArenaTooLarge);
    }

    if !config.obstacle_density.is_finite() || !(0.0..=1.0).contains(&config.obstacle_density) {
        return Err(ArenaGenerationError::InvalidObstacleDensity);
    }

    let mut arena = Arena::filled(config.width, config.height, false);

    for cell in &mut arena.cells {
        if rng.random::<f64>() < config.obstacle_density {
            *cell = true;
        }
    }

    Ok(arena)
}

#[derive(Clone, Copy, Debug)]
pub struct SpawnConfig {
    pub player_count: u8,
    pub min_spawn_distance: usize,
    pub min_boundary_distance: usize,
    pub min_reachable_region_proportion: f64,
    pub max_attempts: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SpawnGenerationError {
    Timeout,
    PlayerCountTooHigh,
    InvalidRegionProportion,
    InvalidArenaDimensions,
    ImpossibleBoundaryDistance,
    ImpossibleSpawnDistance,
}

pub fn generate_spawns(
    arena: &Arena,
    config: &SpawnConfig,
    rng: &mut impl Rng,
) -> Result<Vec<Spawn>, SpawnGenerationError> {
    if arena.width == 0 || arena.height == 0 {
        return Err(SpawnGenerationError::InvalidArenaDimensions);
    }

    let Some(cell_count) = arena.width.checked_mul(arena.height) else {
        return Err(SpawnGenerationError::InvalidArenaDimensions);
    };

    if cell_count > isize::MAX as usize {
        return Err(SpawnGenerationError::InvalidArenaDimensions);
    }

    if config.player_count >= u8::MAX {
        return Err(SpawnGenerationError::PlayerCountTooHigh);
    }

    if !config.min_reachable_region_proportion.is_finite()
        || !(0.0..=1.0).contains(&config.min_reachable_region_proportion)
    {
        return Err(SpawnGenerationError::InvalidRegionProportion);
    }

    if config.player_count > 0 {
        let distance = config.min_boundary_distance;
        if distance > (arena.width - 1) / 2 || distance > (arena.height - 1) / 2 {
            return Err(SpawnGenerationError::ImpossibleBoundaryDistance);
        }

        let max_spawn_distance = (arena.width - 1 - 2 * distance)
            .saturating_add(arena.height - 1 - 2 * distance);
        if config.player_count > 1 && config.min_spawn_distance > max_spawn_distance {
            return Err(SpawnGenerationError::ImpossibleSpawnDistance);
        }
    }

    let mut modified_arena = arena.clone();
    let mut spawns: Vec<Spawn> = vec![];
    let mut iteration_count = 0;
    let max_attempts = config.max_attempts.unwrap_or(cell_count.saturating_mul(20).max(100));

    fn reachable_regions(arena: &Arena, spawn_x: usize, spawn_y: usize) -> usize {
        let mut queue = VecDeque::from([(spawn_x, spawn_y)]);
        let mut visited = HashSet::from([(spawn_x, spawn_y)]);

        // bfs search
        while !queue.is_empty() {
            let (x, y) = queue.pop_front().unwrap();
            let deltas = [North, South, East, West].map(|d| d.delta());
            let neighbors =
                deltas.map(|(dx, dy)| (x.checked_add_signed(dx), y.checked_add_signed(dy)));

            for (x, y) in neighbors {
                let Some(x) = x else {
                    continue;
                };
                let Some(y) = y else {
                    continue;
                };

                if arena.in_bounds(x, y) && !arena[(x, y)] && !visited.contains(&(x, y)) {
                    visited.insert((x, y));
                    queue.push_back((x, y));
                }
            }
        }

        return visited.len();
    }

    while spawns.len() < config.player_count as usize && iteration_count < max_attempts {
        iteration_count += 1;

        // 1. sample random position
        let x = rng.random_range(0..modified_arena.width);
        let y = rng.random_range(0..modified_arena.height);

        // 2. reject if it is an obstacle
        if modified_arena[(x, y)] {
            continue;
        }

        // 3. reject if it is too close to the boundary
        let left_bd_dist = x;
        let right_bd_dist = modified_arena.width - x - 1;
        let top_bd_dist = y;
        let bottom_db_dist = modified_arena.height - y - 1;

        if [left_bd_dist, right_bd_dist, top_bd_dist, bottom_db_dist]
            .iter()
            .min()
            .unwrap()
            < &config.min_boundary_distance
        {
            continue;
        }

        // 4. reject if it is too close to other spawns
        let mut is_too_close = false;
        for spawn in &spawns {
            let distance = spawn.x.abs_diff(x) + spawn.y.abs_diff(y);
            if distance < config.min_spawn_distance {
                is_too_close = true;
                break;
            }
        }

        if is_too_close {
            continue;
        }

        // 5. check if position has enough reachable space
        let reachable_region_proportion = reachable_regions(&modified_arena, x, y) as f64
            / (modified_arena.width * modified_arena.height) as f64;
        if reachable_region_proportion < config.min_reachable_region_proportion {
            continue;
        }

        // 6. randomly select direction that points to open cell
        let mut shuffled_directions = [North, South, East, West];
        shuffled_directions.shuffle(rng);
        let mut selected_direction = None;

        for direction in shuffled_directions {
            let (dx, dy) = direction.delta();
            let Some(proj_x) = x.checked_add_signed(dx) else {
                continue;
            };
            let Some(proj_y) = y.checked_add_signed(dy) else {
                continue;
            };

            let projected_cell = modified_arena.get(proj_x, proj_y).unwrap_or(&true);
            if !projected_cell {
                selected_direction = Some(direction);
                break;
            }
        }

        let Some(direction) = selected_direction else {
            continue;
        };
        let spawn = Spawn { x, y, direction };

        // 7. add spawn to spawns
        spawns.push(spawn);

        // 8. add spawn position as an obstacle to modified arena
        modified_arena[(x, y)] = true;
    }

    if spawns.len() < config.player_count as usize {
        return Err(SpawnGenerationError::Timeout);
    }

    Ok(spawns)
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    fn spawn_config(player_count: u8) -> SpawnConfig {
        SpawnConfig {
            player_count,
            min_spawn_distance: 3,
            min_boundary_distance: 2,
            min_reachable_region_proportion: 0.0,
            max_attempts: Some(100),
        }
    }

    mod generate_arena {
        use super::*;

        #[test]
        fn generates_arena_with_correct_dimensions() {
            let config = ArenaConfig {
                width: 12,
                height: 8,
                obstacle_density: 0.2,
            };
            let mut rng = StdRng::seed_from_u64(42);

            let arena = super::super::generate_arena(&config, &mut rng).unwrap();

            assert_eq!(arena.width, 12);
            assert_eq!(arena.height, 8);
            assert_eq!(arena.cells.len(), 96);
        }

        #[test]
        fn generates_open_arena_with_zero_density() {
            let config = ArenaConfig {
                width: 12,
                height: 8,
                obstacle_density: 0.0,
            };
            let mut rng = StdRng::seed_from_u64(42);

            let arena = super::super::generate_arena(&config, &mut rng).unwrap();

            assert!(arena.cells.iter().all(|cell| !cell));
        }

        #[test]
        fn generates_fully_blocked_arena_with_full_density() {
            let config = ArenaConfig {
                width: 12,
                height: 8,
                obstacle_density: 1.0,
            };
            let mut rng = StdRng::seed_from_u64(42);

            let arena = super::super::generate_arena(&config, &mut rng).unwrap();

            assert!(arena.cells.iter().all(|cell| *cell));
        }

        #[test]
        fn generates_same_arena_with_same_seed() {
            let config = ArenaConfig {
                width: 12,
                height: 8,
                obstacle_density: 0.3,
            };
            let mut rng_1 = StdRng::seed_from_u64(42);
            let mut rng_2 = StdRng::seed_from_u64(42);

            let arena_1 = super::super::generate_arena(&config, &mut rng_1).unwrap();
            let arena_2 = super::super::generate_arena(&config, &mut rng_2).unwrap();

            assert_eq!(arena_1, arena_2);
        }

        #[test]
        fn rejects_zero_sized_arena() {
            let mut rng = StdRng::seed_from_u64(42);

            for (width, height) in [(0, 4), (4, 0), (0, 0)] {
                let config = ArenaConfig {
                    width,
                    height,
                    obstacle_density: 0.2,
                };

                assert_eq!(
                    super::super::generate_arena(&config, &mut rng),
                    Err(ArenaGenerationError::InvalidDimensions)
                );
            }
        }

        #[test]
        fn rejects_arena_dimensions_that_overflow() {
            let config = ArenaConfig {
                width: usize::MAX,
                height: 2,
                obstacle_density: 0.2,
            };
            let mut rng = StdRng::seed_from_u64(42);

            assert_eq!(
                super::super::generate_arena(&config, &mut rng),
                Err(ArenaGenerationError::ArenaTooLarge)
            );
        }

        #[test]
        fn rejects_invalid_obstacle_density() {
            let mut rng = StdRng::seed_from_u64(42);

            for obstacle_density in [-0.1, 1.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                let config = ArenaConfig {
                    width: 4,
                    height: 4,
                    obstacle_density,
                };

                assert_eq!(
                    super::super::generate_arena(&config, &mut rng),
                    Err(ArenaGenerationError::InvalidObstacleDensity)
                );
            }
        }
    }

    mod generate_spawns {
        use super::*;

        #[test]
        fn rejects_zero_sized_arena() {
            let mut rng = StdRng::seed_from_u64(42);

            for (width, height) in [(0, 4), (4, 0), (0, 0)] {
                let arena = Arena::filled(width, height, false);
                let config = spawn_config(1);

                assert_eq!(
                    super::super::generate_spawns(&arena, &config, &mut rng),
                    Err(SpawnGenerationError::InvalidArenaDimensions)
                );
            }
        }

        #[test]
        fn rejects_too_many_players() {
            let arena = Arena::filled(16, 16, false);
            let config = spawn_config(255);
            let mut rng = StdRng::seed_from_u64(42);

            assert_eq!(
                super::super::generate_spawns(&arena, &config, &mut rng),
                Err(SpawnGenerationError::PlayerCountTooHigh)
            );
        }

        #[test]
        fn rejects_invalid_region_proportion() {
            let arena = Arena::filled(12, 12, false);
            let mut rng = StdRng::seed_from_u64(42);

            for proportion in [-0.1, 1.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                let mut config = spawn_config(1);
                config.min_reachable_region_proportion = proportion;

                assert_eq!(
                    super::super::generate_spawns(&arena, &config, &mut rng),
                    Err(SpawnGenerationError::InvalidRegionProportion)
                );
            }
        }

        #[test]
        fn accepts_minimum_reachable_area_below_four_cells() {
            let arena = Arena::filled(4, 4, false);
            let mut config = spawn_config(1);
            config.min_boundary_distance = 0;
            config.min_reachable_region_proportion = 0.125;
            let mut rng = StdRng::seed_from_u64(42);

            assert_eq!(
                super::super::generate_spawns(&arena, &config, &mut rng)
                    .unwrap()
                    .len(),
                1
            );
        }

        #[test]
        fn rejects_impossible_boundary_distance() {
            let arena = Arena::filled(4, 4, false);
            let mut config = spawn_config(1);
            config.min_boundary_distance = 2;
            let mut rng = StdRng::seed_from_u64(42);

            assert_eq!(
                super::super::generate_spawns(&arena, &config, &mut rng),
                Err(SpawnGenerationError::ImpossibleBoundaryDistance)
            );
        }

        #[test]
        fn rejects_impossible_spawn_distance() {
            let arena = Arena::filled(4, 4, false);
            let mut config = spawn_config(2);
            config.min_boundary_distance = 0;
            config.min_spawn_distance = 7;
            let mut rng = StdRng::seed_from_u64(42);

            assert_eq!(
                super::super::generate_spawns(&arena, &config, &mut rng),
                Err(SpawnGenerationError::ImpossibleSpawnDistance)
            );
        }

        #[test]
        fn times_out_with_default_attempt_limit() {
            let arena = Arena::filled(4, 4, true);
            let mut config = spawn_config(1);
            config.min_boundary_distance = 0;
            config.max_attempts = None;
            let mut rng = StdRng::seed_from_u64(42);

            assert_eq!(
                super::super::generate_spawns(&arena, &config, &mut rng),
                Err(SpawnGenerationError::Timeout)
            );
        }

        #[test]
        fn rejects_spawn_with_no_open_directions() {
            let mut arena = Arena::filled(3, 3, true);
            arena[(1, 1)] = false;
            let mut config = spawn_config(1);
            config.min_boundary_distance = 0;
            config.max_attempts = Some(100);
            let mut rng = StdRng::seed_from_u64(42);

            assert_eq!(
                super::super::generate_spawns(&arena, &config, &mut rng),
                Err(SpawnGenerationError::Timeout)
            );
        }

        #[test]
        fn generates_requested_number_of_spawns() {
            let arena = Arena::filled(12, 12, false);
            let config = spawn_config(4);
            let mut rng = StdRng::seed_from_u64(42);

            let spawns = super::super::generate_spawns(&arena, &config, &mut rng).unwrap();

            assert_eq!(spawns.len(), 4);
        }

        #[test]
        fn generates_spawns_on_open_cells() {
            let mut arena = Arena::filled(12, 12, false);
            arena[(3, 3)] = true;
            arena[(4, 5)] = true;
            arena[(7, 8)] = true;

            let config = spawn_config(4);
            let mut rng = StdRng::seed_from_u64(42);

            let spawns = super::super::generate_spawns(&arena, &config, &mut rng).unwrap();

            for spawn in spawns {
                assert!(arena.in_bounds(spawn.x, spawn.y));
                assert!(!arena[(spawn.x, spawn.y)]);
            }
        }

        #[test]
        fn does_not_generate_duplicate_spawn_locations() {
            let arena = Arena::filled(12, 12, false);
            let config = spawn_config(4);
            let mut rng = StdRng::seed_from_u64(42);

            let spawns = super::super::generate_spawns(&arena, &config, &mut rng).unwrap();
            let locations = spawns.iter().map(|s| (s.x, s.y)).collect::<HashSet<_>>();

            assert_eq!(locations.len(), spawns.len());
        }

        #[test]
        fn respects_minimum_boundary_distance() {
            let arena = Arena::filled(12, 12, false);
            let config = spawn_config(4);
            let mut rng = StdRng::seed_from_u64(42);

            let spawns = super::super::generate_spawns(&arena, &config, &mut rng).unwrap();

            for spawn in spawns {
                assert!(spawn.x >= config.min_boundary_distance);
                assert!(spawn.y >= config.min_boundary_distance);
                assert!(arena.width - 1 - spawn.x >= config.min_boundary_distance);
                assert!(arena.height - 1 - spawn.y >= config.min_boundary_distance);
            }
        }

        #[test]
        fn respects_minimum_spawn_distance() {
            let arena = Arena::filled(12, 12, false);
            let config = spawn_config(4);
            let mut rng = StdRng::seed_from_u64(42);

            let spawns = super::super::generate_spawns(&arena, &config, &mut rng).unwrap();

            for (i, spawn) in spawns.iter().enumerate() {
                for other in &spawns[i + 1..] {
                    let distance = spawn.x.abs_diff(other.x) + spawn.y.abs_diff(other.y);
                    assert!(distance >= config.min_spawn_distance);
                }
            }
        }

        #[test]
        fn selects_direction_pointing_to_open_cell() {
            let mut arena = Arena::filled(12, 12, false);
            arena[(3, 3)] = true;
            arena[(4, 5)] = true;
            arena[(7, 8)] = true;

            let config = spawn_config(4);
            let mut rng = StdRng::seed_from_u64(42);

            let spawns = super::super::generate_spawns(&arena, &config, &mut rng).unwrap();

            for spawn in spawns {
                let (dx, dy) = spawn.direction.delta();
                let x = spawn.x.checked_add_signed(dx).unwrap();
                let y = spawn.y.checked_add_signed(dy).unwrap();

                assert!(arena.in_bounds(x, y));
                assert!(!arena[(x, y)]);
            }
        }

        #[test]
        fn generates_same_spawns_with_same_seed() {
            let arena = Arena::filled(12, 12, false);
            let config = spawn_config(4);
            let mut rng_1 = StdRng::seed_from_u64(42);
            let mut rng_2 = StdRng::seed_from_u64(42);

            let spawns_1 = super::super::generate_spawns(&arena, &config, &mut rng_1).unwrap();
            let spawns_2 = super::super::generate_spawns(&arena, &config, &mut rng_2).unwrap();

            assert_eq!(spawns_1, spawns_2);
        }

        #[test]
        fn does_not_modify_original_arena() {
            let arena = Arena::filled(12, 12, false);
            let original_arena = arena.clone();
            let config = spawn_config(4);
            let mut rng = StdRng::seed_from_u64(42);

            let _ = super::super::generate_spawns(&arena, &config, &mut rng).unwrap();

            assert_eq!(arena, original_arena);
        }

        #[test]
        fn returns_empty_spawns_when_no_players_requested() {
            let arena = Arena::filled(4, 4, false);
            let config = SpawnConfig {
                player_count: 0,
                min_spawn_distance: 0,
                min_boundary_distance: 0,
                min_reachable_region_proportion: 0.0,
                max_attempts: Some(0),
            };
            let mut rng = StdRng::seed_from_u64(42);

            assert_eq!(
                super::super::generate_spawns(&arena, &config, &mut rng),
                Ok(vec![])
            );
        }

        #[test]
        fn times_out_when_no_valid_spawns_exist() {
            let arena = Arena::filled(4, 4, true);
            let config = SpawnConfig {
                player_count: 1,
                min_spawn_distance: 0,
                min_boundary_distance: 0,
                min_reachable_region_proportion: 0.0,
                max_attempts: Some(10),
            };
            let mut rng = StdRng::seed_from_u64(42);

            assert_eq!(
                super::super::generate_spawns(&arena, &config, &mut rng),
                Err(SpawnGenerationError::Timeout)
            );
        }

        #[test]
        fn times_out_when_max_attempts_is_zero() {
            let arena = Arena::filled(4, 4, false);
            let config = SpawnConfig {
                player_count: 1,
                min_spawn_distance: 0,
                min_boundary_distance: 0,
                min_reachable_region_proportion: 0.0,
                max_attempts: Some(0),
            };
            let mut rng = StdRng::seed_from_u64(42);

            assert_eq!(
                super::super::generate_spawns(&arena, &config, &mut rng),
                Err(SpawnGenerationError::Timeout)
            );
        }

        #[test]
        fn rejects_spawns_in_regions_that_are_too_small() {
            let mut arena = Arena::filled(10, 10, true);
            for y in 2..6 {
                for x in 2..6 {
                    arena[(x, y)] = false;
                }
            }
            arena[(8, 8)] = false;

            let config = SpawnConfig {
                player_count: 1,
                min_spawn_distance: 0,
                min_boundary_distance: 0,
                min_reachable_region_proportion: 0.1,
                max_attempts: Some(500),
            };
            let mut rng = StdRng::seed_from_u64(42);

            let spawns = super::super::generate_spawns(&arena, &config, &mut rng).unwrap();

            assert_eq!(spawns.len(), 1);
            assert!((2..6).contains(&spawns[0].x));
            assert!((2..6).contains(&spawns[0].y));
        }
    }
}
