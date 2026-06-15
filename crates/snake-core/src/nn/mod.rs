//! Minimal neural-net stack: sensor features, a pure-Rust MLP forward
//! pass and a text weight format shared with the later Python export.

mod features;
mod mlp;
mod neat;

pub use features::{az_features, features, AZ_FEATURE_COUNT, FEATURE_COUNT};
pub use mlp::Mlp;
pub use neat::{ConnGene, Genome, Net, NodeGene, NodeKind};

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
/// training run — see `docs/training/mlp-ga/guide.md`.
pub const EMBEDDED_WEIGHTS: &str = include_str!("../../assets/mlp-ga/best.mlp");

/// Embedded DQN policy weights (stable-baselines3 export, `.mlp` format).
pub const EMBEDDED_DQN: &str = include_str!("../../assets/dqn/policy.mlp");
/// Embedded PPO policy weights (stable-baselines3 export, `.mlp` format).
pub const EMBEDDED_PPO: &str = include_str!("../../assets/ppo/policy.mlp");

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

    /// The checked-in embedded network (GA/ES).
    pub fn embedded() -> Self {
        Self::new(Mlp::from_text(EMBEDDED_WEIGHTS).expect("embedded weights must parse"))
    }

    /// The embedded DQN policy exported from stable-baselines3 (a short
    /// mixed-boundary smoke run — see `docs/training/dqn/guide.md`).
    pub fn embedded_dqn() -> Self {
        Self::new(Mlp::from_text(EMBEDDED_DQN).expect("embedded DQN weights must parse"))
    }

    /// The embedded PPO policy exported from stable-baselines3 (a short
    /// mixed-boundary smoke run — see `docs/training/ppo/guide.md`).
    pub fn embedded_ppo() -> Self {
        Self::new(Mlp::from_text(EMBEDDED_PPO).expect("embedded PPO weights must parse"))
    }
}

impl Strategy for NeuralNet {
    fn next_move(&mut self, state: &GameState) -> Direction {
        let scores = self.mlp.forward(&features(state));
        pick_relative_move(state, &scores, &mut self.debug)
    }

    fn debug(&self) -> Option<&StrategyDebug> {
        Some(&self.debug)
    }
}

/// Pick a move from six heading-relative scores: index `i` = the current
/// heading rotated clockwise by `i` steps. Immediately fatal moves are
/// masked; the highest-scoring safe move wins, falling back to a doomed
/// straight move when none is safe. Shared by the MLP and NEAT strategies.
fn pick_relative_move(state: &GameState, scores: &[f32], debug: &mut StrategyDebug) -> Direction {
    let safe: Vec<Direction> = safe_moves(state).into_iter().map(|(d, _)| d).collect();
    let heading = state.direction();
    debug.move_scores.clear();

    let mut best: Option<(Direction, f32)> = None;
    for (i, &score) in scores.iter().enumerate() {
        let dir = heading.rotated_cw(i as u8);
        debug.move_scores.push((dir, f64::from(score)));
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

/// NEAT-driven strategy: same sensor features and move masking as
/// [`NeuralNet`], but the network has an evolved topology.
pub struct NeatNet {
    net: Net,
    debug: StrategyDebug,
}

impl NeatNet {
    /// Compile a genome into a ready-to-play strategy.
    pub fn new(genome: &Genome) -> Self {
        Self {
            net: genome.compile(),
            debug: StrategyDebug::default(),
        }
    }

    /// The checked-in embedded NEAT genome (smoke-run artifact until the user
    /// runs the real training — see `docs/training/neat.md`).
    pub fn embedded() -> Self {
        let genome = Genome::from_text(EMBEDDED_NEAT).expect("embedded NEAT genome must parse");
        Self::new(&genome)
    }
}

impl Strategy for NeatNet {
    fn next_move(&mut self, state: &GameState) -> Direction {
        let scores = self.net.forward(&features(state));
        pick_relative_move(state, &scores, &mut self.debug)
    }

    fn debug(&self) -> Option<&StrategyDebug> {
        Some(&self.debug)
    }
}

/// Embedded NEAT genome. Built with [`FEATURE_COUNT`] inputs and six outputs.
pub const EMBEDDED_NEAT: &str = include_str!("../../assets/neat/best.neat");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Config, Status};

    #[test]
    fn embedded_weights_parse_and_match_architecture() {
        for net in [
            NeuralNet::embedded(),
            NeuralNet::embedded_dqn(),
            NeuralNet::embedded_ppo(),
        ] {
            assert_eq!(net.mlp.dims(), default_dims());
        }
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

    #[test]
    fn embedded_neat_parses_and_plays_legally() {
        let mut net = NeatNet::embedded();
        let mut state = GameState::new(Config::default());
        for _ in 0..200 {
            if state.status() != Status::Running {
                break;
            }
            let had_safe = !safe_moves(&state).is_empty();
            let dir = net.next_move(&state);
            state.tick(Some(dir));
            if had_safe {
                assert_eq!(
                    state.status(),
                    Status::Running,
                    "neat must not pick a fatal move while a safe one exists"
                );
            }
        }
    }
}
