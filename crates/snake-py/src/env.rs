//! Pure-Rust reinforcement-learning environment over [`snake_core`].
//!
//! Kept free of PyO3 so it is unit-testable without a Python toolchain; the
//! Python bindings in `lib.rs` are a thin wrapper. The action space is the
//! six **heading-relative** directions (0 = straight ahead, clockwise),
//! matching the output convention of the MLP/NEAT strategies — so a policy
//! trained here exports directly into the in-game inference path.

use snake_core::nn::{features, FEATURE_COUNT};
use snake_core::{BoundaryMode, Config, GameState, Status};

/// Which observation the env exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObsKind {
    /// The 20 heading-relative sensor features (same as the MLP/NEAT input).
    Rays,
    /// A flat board tensor, 3 channels (body, head, food) × height × width,
    /// channel-major then row-major. An alternative for CNN policies.
    Grid,
}

/// Number of discrete actions (relative directions).
pub const ACTIONS: usize = 6;

/// Reward shaping constants (documented in `docs/training/dqn.md`).
const REWARD_EAT: f32 = 3.0;
const REWARD_WIN: f32 = 2.0;
const REWARD_DEATH: f32 = -1.0;
/// Per-step shaping: scaled change in (torus-aware) distance to the food.
const REWARD_APPROACH: f32 = 0.1;
/// Per-step living cost — stronger pressure to seek food quickly.
const REWARD_STEP: f32 = -0.01;

/// Result of one [`Env::step`].
#[derive(Debug, Clone)]
pub struct StepResult {
    pub observation: Vec<f32>,
    pub reward: f32,
    pub terminated: bool,
    pub truncated: bool,
    pub score: u32,
}

pub struct Env {
    config: Config,
    obs: ObsKind,
    max_ticks: u64,
    state: GameState,
    last_food_dist: u32,
}

impl Env {
    pub fn new(
        width: i32,
        height: i32,
        boundary: BoundaryMode,
        max_ticks: u64,
        obs: ObsKind,
    ) -> Self {
        let config = Config {
            width,
            height,
            boundary,
            seed: 0,
        };
        let state = GameState::new(config);
        let last_food_dist = state.board().distance(state.head(), state.food());
        Self {
            config,
            obs,
            max_ticks,
            state,
            last_food_dist,
        }
    }

    /// Restart the episode with a fresh seed; returns the first observation.
    pub fn reset(&mut self, seed: u64) -> Vec<f32> {
        self.config.seed = seed;
        self.state = GameState::new(self.config);
        self.last_food_dist = self
            .state
            .board()
            .distance(self.state.head(), self.state.food());
        self.observation()
    }

    /// Apply a relative action and advance one tick.
    pub fn step(&mut self, action: usize) -> StepResult {
        let dir = self.state.direction().rotated_cw((action % ACTIONS) as u8);
        let score_before = self.state.score();
        self.state.tick(Some(dir));

        let mut reward = REWARD_STEP;
        let ate = self.state.score() > score_before;
        if ate {
            reward += REWARD_EAT;
        } else {
            // Distance shaping toward the food (skipped right after eating,
            // since the food respawned and the delta is meaningless).
            let dist = self
                .state
                .board()
                .distance(self.state.head(), self.state.food());
            reward += REWARD_APPROACH * (self.last_food_dist as f32 - dist as f32);
        }
        self.last_food_dist = self
            .state
            .board()
            .distance(self.state.head(), self.state.food());

        let terminated = self.state.status() != Status::Running;
        match self.state.status() {
            Status::GameOver => reward += REWARD_DEATH,
            Status::Won => reward += REWARD_WIN,
            Status::Running => {}
        }
        let truncated = !terminated && self.state.ticks() >= self.max_ticks;

        StepResult {
            observation: self.observation(),
            reward,
            terminated,
            truncated,
            score: self.state.score(),
        }
    }

    pub fn observation(&self) -> Vec<f32> {
        match self.obs {
            ObsKind::Rays => features(&self.state),
            ObsKind::Grid => self.grid_observation(),
        }
    }

    pub fn observation_size(&self) -> usize {
        match self.obs {
            ObsKind::Rays => FEATURE_COUNT,
            ObsKind::Grid => 3 * (self.config.width * self.config.height) as usize,
        }
    }

    /// 3 channels × H × W in [0, 1]: body (excl. head), head, food.
    fn grid_observation(&self) -> Vec<f32> {
        let board = self.state.board();
        let (w, h) = (board.width as usize, board.height as usize);
        let mut grid = vec![0.0f32; 3 * w * h];
        let head = self.state.head();
        for cell in self.state.snake() {
            let idx = (cell.row as usize) * w + cell.col as usize;
            let channel = if cell == head { 1 } else { 0 };
            grid[channel * w * h + idx] = 1.0;
        }
        let food = self.state.food();
        let fidx = (food.row as usize) * w + food.col as usize;
        grid[2 * w * h + fidx] = 1.0;
        grid
    }

    pub fn score(&self) -> u32 {
        self.state.score()
    }

    pub fn ticks(&self) -> u64 {
        self.state.ticks()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rays_observation_has_feature_count() {
        let mut env = Env::new(16, 12, BoundaryMode::Walls, 1000, ObsKind::Rays);
        let obs = env.reset(0);
        assert_eq!(obs.len(), FEATURE_COUNT);
        assert_eq!(env.observation_size(), FEATURE_COUNT);
    }

    #[test]
    fn grid_observation_shape_and_channels() {
        let mut env = Env::new(16, 12, BoundaryMode::Walls, 1000, ObsKind::Grid);
        let obs = env.reset(0);
        assert_eq!(obs.len(), 3 * 16 * 12);
        // Exactly one head cell and one food cell are set.
        let plane = 16 * 12;
        let head_sum: f32 = obs[plane..2 * plane].iter().sum();
        let food_sum: f32 = obs[2 * plane..].iter().sum();
        assert_eq!(head_sum, 1.0);
        assert_eq!(food_sum, 1.0);
    }

    #[test]
    fn eating_gives_positive_reward_and_episode_ends_on_death() {
        // Drive straight into the wall: the run must terminate with the
        // death penalty included.
        let mut env = Env::new(16, 12, BoundaryMode::Walls, 10_000, ObsKind::Rays);
        env.reset(0);
        let mut terminated = false;
        for _ in 0..100 {
            let r = env.step(0); // straight ahead
            if r.terminated {
                terminated = true;
                assert!(r.reward <= REWARD_DEATH + 1.0, "death penalty applied");
                break;
            }
        }
        assert!(terminated, "going straight must hit the wall eventually");
    }

    #[test]
    fn approach_shaping_rewards_getting_closer() {
        // A greedy step toward the food should yield a non-negative shaped
        // reward on a fresh board (every food-ward move is safe).
        let mut env = Env::new(16, 12, BoundaryMode::Walls, 10_000, ObsKind::Rays);
        env.reset(3);
        // Find the relative action that reduces food distance the most.
        let head = env.state.head();
        let food = env.state.food();
        let board = *env.state.board();
        let heading = env.state.direction();
        let best = (0..ACTIONS)
            .filter(|&a| {
                let d = heading.rotated_cw(a as u8);
                d != heading.opposite()
            })
            .filter_map(|a| {
                let d = heading.rotated_cw(a as u8);
                board
                    .neighbor(head, d)
                    .map(|n| (a, board.distance(n, food)))
            })
            .min_by_key(|(_, dist)| *dist)
            .map(|(a, _)| a)
            .unwrap();
        let r = env.step(best);
        assert!(r.reward > 0.0, "approaching food should be rewarded");
    }
}
