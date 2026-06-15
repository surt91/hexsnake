//! Deterministic game state and tick logic.

use std::collections::VecDeque;

use rand::{Rng as _, SeedableRng as _};
use rand_pcg::Pcg32;

use crate::board::{Board, BoundaryMode};
use crate::coords::{Direction, Offset};

/// Maximum number of buffered direction inputs.
const INPUT_QUEUE_CAPACITY: usize = 3;
/// Snake length at game start.
const START_LENGTH: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Running,
    GameOver,
    /// The snake fills the whole board — there is no cell left for food.
    Won,
}

/// Immutable game parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub width: i32,
    pub height: i32,
    pub boundary: BoundaryMode,
    pub seed: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 16,
            height: 12,
            boundary: BoundaryMode::Walls,
            seed: 0,
        }
    }
}

/// Full game state. Fully deterministic: the same config and the same input
/// sequence always produce the same sequence of states.
#[derive(Debug, Clone, PartialEq)]
pub struct GameState {
    board: Board,
    /// Snake cells, head at the front, tail at the back.
    snake: VecDeque<Offset>,
    /// Flat occupancy map (row-major) for O(1) `occupies()` checks.
    occupied: Vec<bool>,
    direction: Direction,
    input_queue: VecDeque<Direction>,
    food: Offset,
    score: u32,
    ticks: u64,
    /// Ticks since the snake last ate (0 right after eating). Drives the
    /// AlphaZero hunger feature so the value head can tell a freshly-fed state
    /// from one that has been circling without eating.
    ticks_since_food: u64,
    status: Status,
    rng: Pcg32,
}

impl GameState {
    pub fn new(config: Config) -> Self {
        let board = Board::new(config.width, config.height, config.boundary);
        assert!(
            board.num_cells() > START_LENGTH + 1,
            "board too small for the snake"
        );

        // Head near the center, body extending south, moving north.
        let head = Offset::new(config.width / 2, config.height / 2);
        let snake: VecDeque<Offset> = (0..START_LENGTH as i32)
            .map(|i| Offset::new(head.col, head.row + i))
            .collect();
        debug_assert!(snake.iter().all(|c| board.contains(*c)));

        let mut occupied = vec![false; board.num_cells()];
        for cell in &snake {
            occupied[Self::cell_index_for(&board, *cell)] = true;
        }

        let mut state = Self {
            board,
            snake,
            occupied,
            direction: Direction::North,
            input_queue: VecDeque::new(),
            food: Offset::new(0, 0), // placeholder, respawned below
            score: 0,
            ticks: 0,
            ticks_since_food: 0,
            status: Status::Running,
            rng: Pcg32::seed_from_u64(config.seed),
        };
        state.respawn_food();
        state
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    /// Snake cells, head first.
    pub fn snake(&self) -> impl DoubleEndedIterator<Item = Offset> + ExactSizeIterator + '_ {
        self.snake.iter().copied()
    }

    pub fn head(&self) -> Offset {
        *self.snake.front().expect("snake is never empty")
    }

    pub fn tail(&self) -> Offset {
        *self.snake.back().expect("snake is never empty")
    }

    pub fn snake_len(&self) -> usize {
        self.snake.len()
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn food(&self) -> Offset {
        self.food
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Ticks since the snake last ate (0 right after eating).
    pub fn ticks_since_food(&self) -> u64 {
        self.ticks_since_food
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn occupies(&self, cell: Offset) -> bool {
        self.occupied[Self::cell_index_for(&self.board, cell)]
    }

    fn cell_index_for(board: &Board, cell: Offset) -> usize {
        (cell.row * board.width + cell.col) as usize
    }

    /// Buffer a direction change for the next ticks. Inputs beyond the queue
    /// capacity and duplicates of the last queued direction are dropped.
    pub fn push_input(&mut self, dir: Direction) {
        if self.input_queue.back() == Some(&dir) {
            return;
        }
        if self.input_queue.len() < INPUT_QUEUE_CAPACITY {
            self.input_queue.push_back(dir);
        }
    }

    /// Drop all buffered inputs, e.g. when the autopilot takes over or
    /// before simulating a planned path.
    pub fn clear_input_queue(&mut self) {
        self.input_queue.clear();
    }

    /// Advance the game by one tick. `input` (if any) is pushed onto the
    /// input queue first, equivalent to calling [`Self::push_input`].
    ///
    /// A queued 180° turn (the opposite of the current direction) is
    /// discarded, as in classic snake.
    pub fn tick(&mut self, input: Option<Direction>) {
        if let Some(dir) = input {
            self.push_input(dir);
        }
        if self.status != Status::Running {
            return;
        }

        while let Some(dir) = self.input_queue.pop_front() {
            if dir != self.direction.opposite() && dir != self.direction {
                self.direction = dir;
                break;
            }
        }

        let Some(next) = self.board.neighbor(self.head(), self.direction) else {
            self.status = Status::GameOver;
            return;
        };

        let grows = next == self.food;
        // The tail cell vacates this tick unless the snake grows, so moving
        // onto it is only fatal when growing.
        let tail = self.tail();
        let hits_body = self.occupies(next) && (grows || next != tail);
        if hits_body {
            self.status = Status::GameOver;
            return;
        }

        self.occupied[Self::cell_index_for(&self.board, next)] = true;
        self.snake.push_front(next);
        if grows {
            self.score += 1;
            self.ticks_since_food = 0;
            self.respawn_food();
        } else {
            let removed = self.snake.pop_back().expect("snake is never empty");
            self.occupied[Self::cell_index_for(&self.board, removed)] = false;
            self.ticks_since_food += 1;
        }
        self.ticks += 1;
    }

    /// Place food on a uniformly random free cell, deterministically derived
    /// from the seeded RNG. Sets [`Status::Won`] if the board is full.
    fn respawn_food(&mut self) {
        // Row-major iteration keeps this independent of hash-map order (determinism).
        // Reservoir sampling avoids collecting a Vec: each free cell wins with
        // probability 1/k, yielding a uniform draw in one pass.
        let mut chosen: Option<Offset> = None;
        let mut count = 0u32;
        for cell in self.board.cells() {
            if !self.occupied[Self::cell_index_for(&self.board, cell)] {
                count += 1;
                if self.rng.random_range(0..count) == 0 {
                    chosen = Some(cell);
                }
            }
        }
        match chosen {
            None => self.status = Status::Won,
            Some(cell) => self.food = cell,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config::default()
    }

    /// Direct the snake one step toward a target cell, avoiding death.
    fn step_toward(state: &GameState, target: Offset) -> Option<Direction> {
        let board = *state.board();
        let head = state.head();
        Direction::ALL
            .into_iter()
            .filter(|d| *d != state.direction().opposite())
            .filter_map(|d| board.neighbor(head, d).map(|n| (d, n)))
            .filter(|(_, n)| !state.occupies(*n) || *n == state.tail())
            .min_by_key(|(_, n)| board.distance(*n, target))
            .map(|(d, _)| d)
    }

    #[test]
    fn snake_starts_with_three_segments_moving_north() {
        let state = GameState::new(config());
        assert_eq!(state.snake_len(), 3);
        assert_eq!(state.direction(), Direction::North);
        assert_eq!(state.status(), Status::Running);
        assert!(
            !state.occupies(state.food()),
            "food must spawn on free cell"
        );
    }

    #[test]
    fn moving_eats_food_and_grows() {
        let mut state = GameState::new(config());
        let mut eaten = 0;
        for _ in 0..2_000 {
            if state.status() != Status::Running {
                break;
            }
            let food = state.food();
            let before = state.snake_len();
            state.tick(step_toward(&state, food));
            if state.score() as usize > eaten {
                eaten = state.score() as usize;
                assert_eq!(state.snake_len(), before + 1, "snake grows exactly by one");
                assert_ne!(state.food(), state.head(), "food respawned elsewhere");
            }
        }
        assert!(
            eaten >= 3,
            "greedy walker should eat a few times, ate {eaten}"
        );
    }

    #[test]
    fn running_into_wall_is_game_over() {
        let mut state = GameState::new(config());
        // Head starts mid-board heading north; the wall is at row 0.
        for _ in 0..config().height {
            state.tick(None);
        }
        assert_eq!(state.status(), Status::GameOver);
    }

    #[test]
    fn periodic_board_wraps_instead_of_dying() {
        let mut state = GameState::new(Config {
            boundary: BoundaryMode::Periodic,
            ..config()
        });
        for _ in 0..3 * config().height {
            state.tick(None);
            // Eating food can happen by chance; only the boundary matters here.
            if state.status() != Status::Running {
                panic!("snake died on a torus while going straight");
            }
        }
    }

    #[test]
    fn reverse_input_is_ignored() {
        let mut state = GameState::new(config());
        let head_before = state.head();
        state.tick(Some(Direction::South)); // 180° turn, must be discarded
        assert_eq!(state.status(), Status::Running);
        assert_eq!(state.direction(), Direction::North);
        assert_eq!(
            state.head(),
            Offset::new(head_before.col, head_before.row - 1)
        );
    }

    #[test]
    fn self_bite_is_game_over() {
        // Grow the snake first so a tight turn can actually bite the body.
        let mut state = GameState::new(Config {
            width: 24,
            height: 18,
            ..config()
        });
        while state.score() < 4 && state.status() == Status::Running {
            let food = state.food();
            let dir = step_toward(&state, food);
            state.tick(dir);
        }
        assert_eq!(state.status(), Status::Running, "setup phase died early");

        // Tight clockwise turn: with length >= 7 the head must hit the body.
        let mut survived = 0;
        'outer: for _ in 0..4 {
            for dir in [
                Direction::NorthEast,
                Direction::SouthEast,
                Direction::South,
                Direction::SouthWest,
                Direction::NorthWest,
                Direction::North,
            ] {
                if state.status() != Status::Running {
                    break 'outer;
                }
                state.tick(Some(dir));
                survived += 1;
            }
        }
        assert_eq!(
            state.status(),
            Status::GameOver,
            "circling in place must end in a self bite (survived {survived} ticks)"
        );
    }

    #[test]
    fn moving_onto_vacating_tail_is_allowed() {
        // A length-3 snake turning sharply may step onto the cell its tail
        // is just leaving — that must not be fatal. Verified implicitly by
        // the rule, explicitly here: follow your own tail in a tight loop.
        let mut state = GameState::new(config());
        let tail = state.tail();
        // Step that targets the current tail cell, if one exists.
        let dir = Direction::ALL.into_iter().find(|d| {
            state.board().neighbor(state.head(), *d) == Some(tail)
                && *d != state.direction().opposite()
        });
        if let Some(dir) = dir {
            state.tick(Some(dir));
            assert_eq!(state.status(), Status::Running);
        }
    }

    #[test]
    fn input_queue_buffers_and_deduplicates() {
        let mut state = GameState::new(config());
        state.push_input(Direction::NorthEast);
        state.push_input(Direction::NorthEast); // dropped duplicate
        state.push_input(Direction::SouthEast);
        state.push_input(Direction::South);
        state.push_input(Direction::SouthWest); // dropped, queue full
        state.tick(None);
        assert_eq!(state.direction(), Direction::NorthEast);
        state.tick(None);
        assert_eq!(state.direction(), Direction::SouthEast);
        state.tick(None);
        assert_eq!(state.direction(), Direction::South);
        state.tick(None);
        assert_eq!(state.direction(), Direction::South, "queue exhausted");
    }

    #[test]
    fn same_seed_and_inputs_give_identical_run() {
        let cfg = Config {
            boundary: BoundaryMode::Periodic,
            seed: 42,
            ..config()
        };
        let mut a = GameState::new(cfg);
        let mut b = GameState::new(cfg);
        assert_eq!(a, b);
        for i in 0..500 {
            // Deterministic pseudo-input pattern, including some invalid turns.
            let input = match i % 7 {
                0 => Some(Direction::NorthEast),
                3 => Some(Direction::SouthEast),
                5 => Some(Direction::South),
                _ => None,
            };
            a.tick(input);
            b.tick(input);
            assert_eq!(a, b, "states diverged at tick {i}");
        }
    }

    #[test]
    fn different_seeds_give_different_food() {
        let a = GameState::new(Config {
            seed: 1,
            ..config()
        });
        let b = GameState::new(Config {
            seed: 2,
            ..config()
        });
        // Not guaranteed in general, but stable for these fixed seeds.
        assert_ne!(a.food(), b.food());
    }

    #[test]
    fn full_game_from_start_to_game_over() {
        // Headless complete match: a simple greedy walker plays until it
        // dies. This exercises eating, growing, respawning and collision in
        // one run.
        let mut state = GameState::new(Config {
            seed: 7,
            ..config()
        });
        let mut ticks = 0u32;
        while state.status() == Status::Running && ticks < 50_000 {
            let food = state.food();
            let dir = step_toward(&state, food);
            state.tick(dir);
            ticks += 1;
        }
        assert_eq!(
            state.status(),
            Status::GameOver,
            "greedy walker should eventually trap itself (score {})",
            state.score()
        );
        assert!(state.score() > 0, "should have eaten before dying");
    }
}
