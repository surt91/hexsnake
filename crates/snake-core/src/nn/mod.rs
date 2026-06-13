//! Minimal neural-net stack: sensor features, a pure-Rust MLP forward
//! pass and a text weight format shared with the later Python export.

mod features;
mod mlp;

pub use features::{features, FEATURE_COUNT};
pub use mlp::Mlp;

use crate::coords::Direction;
use crate::game::GameState;
use crate::strategy::{safe_moves, Strategy, StrategyDebug};

/// Hidden-layer sizes of the default architecture. Input/output dims are
/// fixed by the feature vector and the six directions.
pub const HIDDEN: [usize; 2] = [32, 24];

/// The full layer dimensions of the default architecture.
pub fn default_dims() -> Vec<usize> {
    let mut dims = vec![FEATURE_COUNT];
    dims.extend_from_slice(&HIDDEN);
    dims.push(6);
    dims
}

/// Weights trained by `snake-train` (GA/ES). The checked-in file starts
/// out as a smoke-run artifact and is replaced by the user's real
/// training run — see `docs/training/neural-net-ga.md`.
pub const EMBEDDED_WEIGHTS: &str = include_str!("../../assets/neural-net-ga/best.mlp");

/// MLP-driven strategy: features in, one score per (relative) direction
/// out, fatal moves masked.
pub struct NeuralNet {
    mlp: Mlp,
    debug: StrategyDebug,
}

impl NeuralNet {
    pub fn new(mlp: Mlp) -> Self {
        Self {
            mlp,
            debug: StrategyDebug::default(),
        }
    }

    /// The checked-in embedded network.
    pub fn embedded() -> Self {
        Self::new(Mlp::from_text(EMBEDDED_WEIGHTS).expect("embedded weights must parse"))
    }
}

impl Strategy for NeuralNet {
    fn next_move(&mut self, state: &GameState) -> Direction {
        let input = features(state);
        let scores = self.mlp.forward(&input);

        // Outputs are relative to the current heading: index i = heading
        // rotated clockwise by i steps. Mask everything immediately fatal.
        let safe: Vec<Direction> = safe_moves(state).into_iter().map(|(d, _)| d).collect();
        let heading = state.direction();
        self.debug.move_scores.clear();

        let mut best: Option<(Direction, f32)> = None;
        for (i, &score) in scores.iter().enumerate() {
            let dir = heading.rotated_cw(i as u8);
            self.debug.move_scores.push((dir, f64::from(score)));
            if !safe.contains(&dir) {
                continue;
            }
            if best.is_none_or(|(_, b)| score > b) {
                best = Some((dir, score));
            }
        }
        best.map(|(d, _)| d)
            .unwrap_or_else(|| crate::strategy::doomed_move(state))
    }

    fn debug(&self) -> Option<&StrategyDebug> {
        Some(&self.debug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Config, Status};

    #[test]
    fn embedded_weights_parse_and_match_architecture() {
        let net = NeuralNet::embedded();
        assert_eq!(net.mlp.dims(), default_dims());
    }

    #[test]
    fn plays_legal_moves_deterministically() {
        let mut a = NeuralNet::embedded();
        let mut b = NeuralNet::embedded();
        let mut state = GameState::new(Config::default());
        for _ in 0..200 {
            if state.status() != Status::Running {
                break;
            }
            let da = a.next_move(&state);
            assert_eq!(da, b.next_move(&state), "deterministic");
            let had_safe = !safe_moves(&state).is_empty();
            state.tick(Some(da));
            if had_safe {
                assert_eq!(
                    state.status(),
                    Status::Running,
                    "nn must not pick a fatal move while a safe one exists"
                );
            }
        }
    }
}
