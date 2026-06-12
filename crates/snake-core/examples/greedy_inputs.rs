//! Print a per-tick key script (w/e/d/s/a/q) that greedily eats food and
//! then crashes north — for reproducing browser states via the
//! `?seed=…&inputs=…` URL parameters (see docs in `snake-app`).
//!
//! Usage: cargo run -p snake-core --example greedy_inputs [seed] [target_score]

use snake_core::{BoundaryMode, Config, Direction, GameState, Status};

fn key(dir: Direction) -> char {
    match dir {
        Direction::North => 'w',
        Direction::NorthEast => 'e',
        Direction::SouthEast => 'd',
        Direction::South => 's',
        Direction::SouthWest => 'a',
        Direction::NorthWest => 'q',
    }
}

fn step_toward(state: &GameState) -> Option<Direction> {
    let board = *state.board();
    let head = state.head();
    Direction::ALL
        .into_iter()
        .filter(|d| *d != state.direction().opposite())
        .filter_map(|d| board.neighbor(head, d).map(|n| (d, n)))
        .filter(|(_, n)| !state.occupies(*n) || *n == state.tail())
        .min_by_key(|(_, n)| board.distance(*n, state.food()))
        .map(|(d, _)| d)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(42);
    let target: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1);

    // Matches the app's default settings: medium field, walls, normal speed.
    let mut state = GameState::new(Config {
        width: 24,
        height: 18,
        boundary: BoundaryMode::Walls,
        seed,
    });

    let mut script = String::new();
    while state.status() == Status::Running && script.len() < 2_000 {
        let dir = if state.score() < target {
            step_toward(&state)
        } else {
            Some(Direction::North) // done eating: run into the top wall
        };
        if let Some(d) = dir {
            script.push(key(d));
        }
        state.tick(dir);
    }
    println!("{script}");
    eprintln!(
        "seed={seed} score={} status={:?} ticks={}",
        state.score(),
        state.status(),
        state.ticks()
    );
}
