//! Autopilot strategies. All strategies are deterministic, work on the pure
//! [`GameState`] (no UI access) and compile to WASM.

mod alphazero;
mod chaos;
mod cycle_surgeon;
mod greedy;
mod hamilton;
mod montecarlo;
mod planner;
mod space;

pub use alphazero::{
    self_play, self_play_conv_with_rewards, self_play_with_rewards, AlphaZeroConv, AlphaZeroLite,
    AzSample, ConvAzSample, ConvSelfPlayResult, SelfPlayResult, AZ_OUTPUTS,
};
pub use chaos::ChaosWalker;
pub use cycle_surgeon::CycleSurgeon;
pub use greedy::Greedy;
pub use hamilton::{serpentine_cycle, HamiltonRider};
pub use montecarlo::MonteCarlo;
pub use planner::PathPlanner;
pub use space::SpaceKeeper;

use crate::board::Board;
use crate::coords::{Direction, Offset};
use crate::game::GameState;

/// What a strategy "thought" during its last decision — input for the
/// debug overlay. Strategies fill in whatever applies to them.
#[derive(Debug, Clone, Default)]
pub struct StrategyDebug {
    /// Planned path cells in order, starting with the cell after the head.
    pub path: Vec<Offset>,
    /// Cells highlighted by the strategy (e.g. flood-fill reachable area).
    pub heatmap: Vec<Offset>,
    /// Evaluation per candidate direction (higher = better).
    pub move_scores: Vec<(Direction, f64)>,
}

pub trait Strategy {
    /// Decide the next move for the current state. Called once per tick.
    fn next_move(&mut self, state: &GameState) -> Direction;

    /// Visualization of the last decision, if the strategy provides one.
    fn debug(&self) -> Option<&StrategyDebug> {
        None
    }
}

pub(crate) fn cell_index(board: &Board, cell: Offset) -> usize {
    (cell.row * board.width + cell.col) as usize
}

/// Collect all free cells reachable from `from` (BFS in deterministic
/// direction order). `from` itself may be occupied (a hypothetical head)
/// and is not counted.
pub(crate) fn flood_fill(state: &GameState, from: Offset, out: &mut Vec<Offset>) {
    let board = state.board();
    out.clear();
    let mut visited = vec![false; board.num_cells()];
    visited[cell_index(board, from)] = true;
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(from);
    while let Some(cell) = queue.pop_front() {
        for dir in Direction::ALL {
            let Some(next) = board.neighbor(cell, dir) else {
                continue;
            };
            let idx = cell_index(board, next);
            if !visited[idx] && !state.occupies(next) {
                visited[idx] = true;
                out.push(next);
                queue.push_back(next);
            }
        }
    }
}

/// Moves that do not kill the snake this tick, in `Direction::ALL` order
/// (deterministic). The 180° turn is excluded — the game would ignore it
/// and keep going straight, which is rarely what the strategy intended.
pub(crate) fn safe_moves(state: &GameState) -> Vec<(Direction, Offset)> {
    let board = state.board();
    let head = state.head();
    Direction::ALL
        .into_iter()
        .filter(|dir| *dir != state.direction().opposite())
        .filter_map(|dir| board.neighbor(head, dir).map(|n| (dir, n)))
        // Moving onto the tail cell is safe: the tail vacates this tick
        // (food is never on the snake, so this move cannot grow).
        .filter(|(_, n)| !state.occupies(*n) || *n == state.tail())
        .collect()
}

/// Last-resort move when no safe move exists: keep going straight.
pub(crate) fn doomed_move(state: &GameState) -> Direction {
    state.direction()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::BoundaryMode;
    use crate::game::{Config, Status};

    fn config(boundary: BoundaryMode) -> Config {
        Config {
            width: 16,
            height: 12,
            boundary,
            seed: 3,
        }
    }

    #[test]
    fn chaos_never_dies_while_a_safe_move_exists() {
        for seed in 0..5 {
            let mut state = GameState::new(Config {
                seed,
                ..config(BoundaryMode::Walls)
            });
            let mut chaos = ChaosWalker::new(seed);
            for _ in 0..1_000 {
                if state.status() != Status::Running {
                    break;
                }
                let had_safe_move = !safe_moves(&state).is_empty();
                let dir = chaos.next_move(&state);
                state.tick(Some(dir));
                if had_safe_move {
                    assert_eq!(
                        state.status(),
                        Status::Running,
                        "chaos walker died although a safe move existed (seed {seed})"
                    );
                }
            }
        }
    }

    #[test]
    fn chaos_is_deterministic_per_seed() {
        let state = GameState::new(config(BoundaryMode::Periodic));
        let mut a = ChaosWalker::new(7);
        let mut b = ChaosWalker::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_move(&state), b.next_move(&state));
        }
    }

    #[test]
    fn greedy_moves_toward_food() {
        // On a fresh board every food-ward move is safe, so the very first
        // greedy move must strictly reduce the distance to the food.
        for boundary in [BoundaryMode::Walls, BoundaryMode::Periodic] {
            for seed in 0..10 {
                let mut state = GameState::new(Config {
                    seed,
                    ..config(boundary)
                });
                let before = state.board().distance(state.head(), state.food());
                let mut greedy = Greedy;
                let dir = greedy.next_move(&state);
                state.tick(Some(dir));
                let after = state.board().distance(state.head(), state.food());
                assert!(
                    after < before,
                    "greedy did not approach food (seed {seed}, {boundary:?}): {before} -> {after}"
                );
            }
        }
    }

    #[test]
    fn planner_eats_first_food_on_shortest_path() {
        for boundary in [BoundaryMode::Walls, BoundaryMode::Periodic] {
            let mut state = GameState::new(config(boundary));
            let expected = state.board().distance(state.head(), state.food());
            let mut planner = PathPlanner::new();
            let mut steps = 0;
            while state.score() == 0 && steps < 500 {
                let dir = planner.next_move(&state);
                state.tick(Some(dir));
                steps += 1;
                assert_eq!(state.status(), Status::Running);
            }
            assert_eq!(
                steps, expected,
                "planner should reach the first food on a shortest path ({boundary:?})"
            );
        }
    }

    #[test]
    fn safe_moves_excludes_reverse_and_walls() {
        let state = GameState::new(config(BoundaryMode::Walls));
        let moves = safe_moves(&state);
        assert!(moves
            .iter()
            .all(|(d, _)| *d != state.direction().opposite()));
        assert!(!moves.is_empty());
    }
}
