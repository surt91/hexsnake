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

    /// Play one AlphaZero-light self-play game in Rust and return training
    /// samples `(features, policy_target, value_target)`.
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
    ) -> PyResult<Vec<AzSampleRow>> {
        use snake_core::nn::{Mlp, FEATURE_COUNT};
        use snake_core::strategy::{self_play_with_rewards, AZ_OUTPUTS};

        let boundary = match boundary {
            "walls" => BoundaryMode::Walls,
            "torus" | "periodic" => BoundaryMode::Periodic,
            other => return Err(PyValueError::new_err(format!("bad boundary: {other:?}"))),
        };
        let dims = dims.unwrap_or_else(|| vec![FEATURE_COUNT, 32, 24, AZ_OUTPUTS]);
        let mlp = Mlp::from_params(&dims, params).map_err(PyValueError::new_err)?;
        let config = snake_core::Config {
            width,
            height,
            boundary,
            seed: 0,
        };
        // The game is pure Rust and touches no Python state → release the GIL
        // so threaded callers fan out across cores.
        let samples = py.allow_threads(|| {
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
        Ok(samples
            .into_iter()
            .map(|s| (s.features, s.policy.to_vec(), s.value))
            .collect())
    }

    #[pymodule]
    fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<PyEnv>()?;
        m.add_function(wrap_pyfunction!(mlp_forward, m)?)?;
        m.add_function(wrap_pyfunction!(az_selfplay, m)?)?;
        m.add("NUM_ACTIONS", ACTIONS)?;
        Ok(())
    }
}
