use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::{cell_index, doomed_move, safe_moves, Strategy, StrategyDebug};
use crate::coords::{Direction, Offset};
use crate::game::{GameState, Status};

/// A*-based planner: shortest safe path to the food, but only if the snake
/// can still reach its own tail afterwards (then it can never be fully
/// trapped). Otherwise it chases its tail and waits for a better chance.
#[derive(Default)]
pub struct PathPlanner {
    debug: StrategyDebug,
}

impl PathPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    fn remember_path(&mut self, state: &GameState, path: &[Direction]) {
        self.debug.path.clear();
        let board = state.board();
        let mut at = state.head();
        for &dir in path {
            let Some(next) = board.neighbor(at, dir) else {
                break;
            };
            self.debug.path.push(next);
            at = next;
        }
    }
}

impl Strategy for PathPlanner {
    fn next_move(&mut self, state: &GameState) -> Direction {
        let vacate = vacate_times(state);

        if let Some(path) = astar(state, state.food(), &vacate) {
            if !path.is_empty() && survives_after(state, &path) {
                self.remember_path(state, &path);
                return path[0];
            }
        }

        // No (safe) path to the food: follow the own tail to stall.
        if let Some(path) = astar(state, state.tail(), &vacate) {
            if let Some(&first) = path.first() {
                self.remember_path(state, &path);
                return first;
            }
        }

        self.debug.path.clear();
        safe_moves(state)
            .first()
            .map(|(dir, _)| *dir)
            .unwrap_or_else(|| doomed_move(state))
    }

    fn debug(&self) -> Option<&StrategyDebug> {
        Some(&self.debug)
    }
}

/// Tick at which each cell becomes free, assuming the snake keeps moving
/// without growing: the tail cell vacates at tick 1, the head cell last.
/// Free cells are 0. This makes A* "time-aware": it may plan through cells
/// the snake will have left by the time it arrives.
fn vacate_times(state: &GameState) -> Vec<u32> {
    let board = state.board();
    let mut vacate = vec![0u32; board.num_cells()];
    let len = state.snake_len() as u32;
    for (i, cell) in state.snake().enumerate() {
        vacate[cell_index(board, cell)] = len - i as u32;
    }
    vacate
}

/// Deterministic A* from the head to `goal` on the (possibly periodic)
/// board. Returns the per-tick directions of a shortest path. The first
/// step never reverses the current direction (the game would ignore it).
fn astar(state: &GameState, goal: Offset, vacate: &[u32]) -> Option<Vec<Direction>> {
    let board = state.board();
    let start = state.head();
    let start_idx = cell_index(board, start);
    let goal_idx = cell_index(board, goal);
    let n = board.num_cells();

    let mut g_cost = vec![u32::MAX; n];
    let mut came_from: Vec<Option<(usize, Direction)>> = vec![None; n];
    // Reverse min-heap over (f, g, cell index): deterministic tie-breaking.
    let mut frontier = BinaryHeap::new();
    g_cost[start_idx] = 0;
    frontier.push(Reverse((board.distance(start, goal), 0u32, start_idx)));

    while let Some(Reverse((_, g, idx))) = frontier.pop() {
        if g > g_cost[idx] {
            continue; // stale heap entry
        }
        if idx == goal_idx {
            let mut path = Vec::with_capacity(g as usize);
            let mut at = idx;
            while let Some((prev, dir)) = came_from[at] {
                path.push(dir);
                at = prev;
            }
            path.reverse();
            return Some(path);
        }
        let cell = Offset::new((idx as i32) % board.width, (idx as i32) / board.width);
        for dir in Direction::ALL {
            if g == 0 && dir == state.direction().opposite() {
                continue;
            }
            let Some(next) = board.neighbor(cell, dir) else {
                continue;
            };
            let next_idx = cell_index(board, next);
            let arrival = g + 1;
            if arrival < vacate[next_idx] && next_idx != goal_idx {
                continue; // still occupied when we would arrive
            }
            if arrival < g_cost[next_idx] {
                g_cost[next_idx] = arrival;
                came_from[next_idx] = Some((idx, dir));
                frontier.push(Reverse((
                    arrival + board.distance(next, goal),
                    arrival,
                    next_idx,
                )));
            }
        }
    }
    None
}

/// Simulate following `path` on a clone of the real state (deterministic,
/// including the food respawn) and check that the snake can still reach its
/// own tail afterwards.
fn survives_after(state: &GameState, path: &[Direction]) -> bool {
    let mut sim = state.clone();
    sim.clear_input_queue();
    for &dir in path {
        sim.tick(Some(dir));
        if sim.status() != Status::Running {
            return false;
        }
    }
    tail_reachable(&sim)
}

/// BFS: can the head reach the tail cell over free cells?
fn tail_reachable(state: &GameState) -> bool {
    let board = state.board();
    let target = state.tail();
    let mut visited = vec![false; board.num_cells()];
    let mut queue = std::collections::VecDeque::new();
    visited[cell_index(board, state.head())] = true;
    queue.push_back(state.head());
    while let Some(cell) = queue.pop_front() {
        for dir in Direction::ALL {
            let Some(next) = board.neighbor(cell, dir) else {
                continue;
            };
            if next == target {
                return true;
            }
            let idx = cell_index(board, next);
            if !visited[idx] && !state.occupies(next) {
                visited[idx] = true;
                queue.push_back(next);
            }
        }
    }
    false
}
