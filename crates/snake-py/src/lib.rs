//! `hexsnake_env`: a Python extension module exposing the HexSnake game as a
//! Gym-like environment, so DQN/PPO (stable-baselines3) can train against the
//! exact same deterministic `snake-core` the game and server use.
//!
//! The pure-Rust [`env::Env`] holds all logic and is unit-tested without
//! Python; the PyO3 wrapper below is built into a wheel by maturin (enable
//! the `python` feature). See `python/README.md`.

pub mod env;

#[cfg(feature = "python")]
mod bindings {
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    use crate::env::{Env, ObsKind, ACTIONS};
    use snake_core::BoundaryMode;

    /// One self-play row returned to Python: (features, policy_target, value).
    type AzSampleRow = (Vec<f32>, Vec<f32>, f32);

    /// Gym-like environment. Actions are the six heading-relative directions.
    #[pyclass(name = "HexSnakeEnv")]
    struct PyEnv {
        inner: Env,
    }

    #[pymethods]
    impl PyEnv {
        #[new]
        #[pyo3(signature = (width=16, height=12, boundary="walls", max_ticks=2000, observation="rays"))]
        fn new(
            width: i32,
            height: i32,
            boundary: &str,
            max_ticks: u64,
            observation: &str,
        ) -> PyResult<Self> {
            let boundary = match boundary {
                "walls" => BoundaryMode::Walls,
                "torus" | "periodic" => BoundaryMode::Periodic,
                other => return Err(PyValueError::new_err(format!("bad boundary: {other:?}"))),
            };
            let obs = match observation {
                "rays" => ObsKind::Rays,
                "grid" => ObsKind::Grid,
                other => return Err(PyValueError::new_err(format!("bad observation: {other:?}"))),
            };
            Ok(Self {
                inner: Env::new(width, height, boundary, max_ticks, obs),
            })
        }

        /// Reset to a fresh episode; returns the first observation.
        #[pyo3(signature = (seed=0))]
        fn reset(&mut self, seed: u64) -> Vec<f32> {
            self.inner.reset(seed)
        }

        /// Apply a relative action; returns
        /// `(observation, reward, terminated, truncated, score)`.
        fn step(&mut self, action: usize) -> (Vec<f32>, f32, bool, bool, u32) {
            let r = self.inner.step(action);
            (r.observation, r.reward, r.terminated, r.truncated, r.score)
        }

        #[getter]
        fn observation_size(&self) -> usize {
            self.inner.observation_size()
        }

        #[getter]
        fn num_actions(&self) -> usize {
            ACTIONS
        }

        #[getter]
        fn score(&self) -> u32 {
            self.inner.score()
        }

        #[getter]
        fn ticks(&self) -> u64 {
            self.inner.ticks()
        }
    }

    /// Run `snake-core`'s MLP forward pass on a `.mlp` weight text and an
    /// input vector. Lets the Python exporter verify that its weight layout
    /// matches the Rust inference exactly (guards transposition/layout bugs).
    #[pyfunction]
    fn mlp_forward(text: &str, input: Vec<f32>) -> PyResult<Vec<f32>> {
        let mlp = snake_core::nn::Mlp::from_text(text).map_err(PyValueError::new_err)?;
        Ok(mlp.forward(&input))
    }

    /// Run `snake-core`'s hex-conv forward pass on a `.cnn` text over explicit
    /// input planes (`in_channels × height × width`, channel-major then
    /// row-major). `head_idx` is `row*width + col` of the read-out cell. Lets
    /// the Python roundtrip check the exact conv/readout/pool layout against the
    /// real Rust inference, independent of torch.
    #[pyfunction]
    #[pyo3(signature = (text, planes, width, height, head_idx, boundary="walls"))]
    fn cnn_forward(
        text: &str,
        planes: Vec<f32>,
        width: usize,
        height: usize,
        head_idx: usize,
        boundary: &str,
    ) -> PyResult<Vec<f32>> {
        let boundary = parse_boundary(boundary)?;
        let net = snake_core::nn::HexConv::from_text(text).map_err(PyValueError::new_err)?;
        Ok(net.forward_planes(&planes, width, height, boundary, head_idx))
    }

    /// For each cell (row-major) return its six hex-neighbor indices in
    /// `Direction::ALL` order, with `-1` for off-board neighbors (walls). The
    /// Python hex-conv model uses this table so its neighbor geometry matches
    /// the Rust convolution exactly instead of reimplementing the hex math.
    #[pyfunction]
    #[pyo3(signature = (width, height, boundary="walls"))]
    fn neighbor_table(width: i32, height: i32, boundary: &str) -> PyResult<Vec<i64>> {
        use snake_core::Direction;
        let boundary = parse_boundary(boundary)?;
        let board = snake_core::Board::new(width, height, boundary);
        let mut out = Vec::with_capacity((width * height) as usize * 6);
        for row in 0..height {
            for col in 0..width {
                let cell = snake_core::Offset::new(col, row);
                for dir in Direction::ALL {
                    out.push(
                        board
                            .neighbor(cell, dir)
                            .map(|n| (n.row * width + n.col) as i64)
                            .unwrap_or(-1),
                    );
                }
            }
        }
        Ok(out)
    }

    /// Play `games` games with the A* `PathPlanner` (the strongest classical
    /// AI) and return behavior-cloning rows: per running step a 3-channel grid
    /// (body/head/food, channel-major then row-major — the same layout as the
    /// env's `grid` observation), the planner's chosen **absolute** move as an
    /// index into `Direction::ALL`, and a `toward` flag (1.0 if the move
    /// strictly reduced the (torus-aware) distance to the food, else 0.0). The
    /// flag lets the trainer upweight food-seeking moves so the clone does not
    /// collapse into safe circling. Python adds the constant topology plane and
    /// reads the head index off the grid. The GIL is released for the games.
    #[pyfunction]
    #[pyo3(signature = (boundary="walls", width=16, height=12, max_ticks=2000, games=200, seed=0))]
    fn expert_rollout(
        py: Python<'_>,
        boundary: &str,
        width: i32,
        height: i32,
        max_ticks: u64,
        games: u64,
        seed: u64,
    ) -> PyResult<(Vec<Vec<f32>>, Vec<usize>, Vec<f32>)> {
        use snake_core::strategy::{PathPlanner, Strategy};
        use snake_core::{Config, Direction, GameState, Status};

        let boundary = parse_boundary(boundary)?;
        let (w, h) = (width as usize, height as usize);
        let plane = w * h;

        let result = py.allow_threads(|| {
            let mut grids: Vec<Vec<f32>> = Vec::new();
            let mut labels: Vec<usize> = Vec::new();
            let mut toward: Vec<f32> = Vec::new();
            for g in 0..games {
                let mut state = GameState::new(Config {
                    width,
                    height,
                    boundary,
                    seed: seed.wrapping_add(g),
                });
                let mut planner = PathPlanner::new();
                while state.status() == Status::Running && state.ticks() < max_ticks {
                    let dir = planner.next_move(&state);
                    let label = Direction::ALL
                        .iter()
                        .position(|d| *d == dir)
                        .expect("direction is in ALL");

                    let head = state.head();
                    let food = state.food();
                    // Did this move advance toward the food (torus-aware)?
                    let dist_before = state.board().distance(head, food);
                    let toward_food = state
                        .board()
                        .neighbor(head, dir)
                        .map(|n| state.board().distance(n, food) < dist_before)
                        .unwrap_or(false);

                    // 3-channel grid: body (excl. head), head, food.
                    let mut grid = vec![0.0f32; 3 * plane];
                    for cell in state.snake() {
                        let idx = (cell.row as usize) * w + cell.col as usize;
                        let ch = if cell == head { 1 } else { 0 };
                        grid[ch * plane + idx] = 1.0;
                    }
                    grid[2 * plane + (food.row as usize) * w + food.col as usize] = 1.0;

                    grids.push(grid);
                    labels.push(label);
                    toward.push(if toward_food { 1.0 } else { 0.0 });
                    state.tick(Some(dir));
                }
            }
            (grids, labels, toward)
        });
        Ok(result)
    }

    fn parse_boundary(s: &str) -> PyResult<BoundaryMode> {
        match s {
            "walls" => Ok(BoundaryMode::Walls),
            "torus" | "periodic" => Ok(BoundaryMode::Periodic),
            other => Err(PyValueError::new_err(format!("bad boundary: {other:?}"))),
        }
    }

    /// Play one AlphaZero-light self-play game in Rust and return
    /// `(rows, score, ticks)`, where `rows` are training samples
    /// `(features, policy_target, value_target)` and `score`/`ticks` are the
    /// game-level outcome the trainer uses to pick checkpoints by food eaten
    /// rather than survival time.
    ///
    /// `params` are the flat weights of a 7-output MLP (same layout as the
    /// `.mlp` format). `dims` specifies the layer sizes including input and
    /// output (default `[20, 32, 24, 7]`). The MCTS is the exact same search
    /// `AlphaZeroLite` uses at inference, so the policy targets need no second
    /// implementation. The GIL is released during the (pure-Rust) game, so
    /// many self-play games can run in parallel from Python threads.
    #[pyfunction]
    #[pyo3(signature = (params, boundary="walls", width=16, height=12, sims=24, temperature=1.0, seed=0, max_ticks=2000, eat_bonus=0.3, sp_eat=1.0, dims=None))]
    #[allow(clippy::too_many_arguments)]
    fn az_selfplay(
        py: Python<'_>,
        params: Vec<f32>,
        boundary: &str,
        width: i32,
        height: i32,
        sims: u32,
        temperature: f32,
        seed: u64,
        max_ticks: u64,
        eat_bonus: f32,
        sp_eat: f32,
        dims: Option<Vec<usize>>,
    ) -> PyResult<(Vec<AzSampleRow>, u32, u64)> {
        use snake_core::nn::{Mlp, AZ_FEATURE_COUNT};
        use snake_core::strategy::{self_play_with_rewards, AZ_OUTPUTS};

        let boundary = match boundary {
            "walls" => BoundaryMode::Walls,
            "torus" | "periodic" => BoundaryMode::Periodic,
            other => return Err(PyValueError::new_err(format!("bad boundary: {other:?}"))),
        };
        let dims = dims.unwrap_or_else(|| vec![AZ_FEATURE_COUNT, 32, 24, AZ_OUTPUTS]);
        let mlp = Mlp::from_params(&dims, params).map_err(PyValueError::new_err)?;
        // Use `seed` for the *board* RNG too (not just action sampling), so
        // each self-play game runs on a different board layout / food sequence.
        // Previously the board was hardcoded to seed 0, so every game trained
        // on a single board — the policy overfit it and circled on the unseen
        // boards the benchmark uses (seeds 0..N).
        let config = snake_core::Config {
            width,
            height,
            boundary,
            seed,
        };
        // The game is pure Rust and touches no Python state → release the GIL
        // so threaded callers fan out across cores.
        let result = py.allow_threads(|| {
            self_play_with_rewards(
                &mlp,
                config,
                sims,
                temperature,
                seed,
                max_ticks,
                eat_bonus,
                sp_eat,
            )
        });
        let rows = result
            .samples
            .into_iter()
            .map(|s| (s.features, s.policy.to_vec(), s.value))
            .collect();
        Ok((rows, result.score, result.ticks))
    }

    #[pymodule]
    fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<PyEnv>()?;
        m.add_function(wrap_pyfunction!(mlp_forward, m)?)?;
        m.add_function(wrap_pyfunction!(cnn_forward, m)?)?;
        m.add_function(wrap_pyfunction!(neighbor_table, m)?)?;
        m.add_function(wrap_pyfunction!(expert_rollout, m)?)?;
        m.add_function(wrap_pyfunction!(az_selfplay, m)?)?;
        m.add("NUM_ACTIONS", ACTIONS)?;
        Ok(())
    }
}
