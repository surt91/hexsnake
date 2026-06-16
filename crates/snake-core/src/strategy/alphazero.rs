//! AlphaZero-light: a Monte-Carlo *tree* search guided by a policy/value
//! network, replacing the random rollouts of the plain `MonteCarlo` strategy.
//!
//! The network is one MLP with 7 outputs: six policy logits (heading-relative
//! directions, like the other net strategies) and one value. PUCT selection
//! uses the policy as priors; leaves are evaluated by the value head instead
//! of rolling out random games. Edge values also fold in a dense per-step
//! reward (food-approach shaping + eat bonus), so the search prefers
//! food-ward moves even with an untrained value head — without it the policy
//! collapses to safe circling.
//!
//! Training: the **gradient** trainer (`python/train_alphazero.py`) runs
//! self-play with [`self_play`] (this exact search), then fits the net by
//! backprop — policy loss against the MCTS visit distribution, value loss
//! against the discounted return. A gradient-free GA fallback also exists
//! (`snake-train az`, no Python needed). Inference stays pure Rust/WASM.

use crate::coords::Direction;
use crate::game::{GameState, Status};
use crate::nn::{az_features, Mlp};
use crate::strategy::{doomed_move, safe_moves, Strategy, StrategyDebug};

/// Output dimension of the policy/value net (6 policy logits + 1 value).
pub const AZ_OUTPUTS: usize = 7;
/// Relative-direction index of the (ignored) 180° reverse move; excluded
/// from the search since the game would override it with "straight".
const REVERSE: usize = 3;
const C_PUCT: f32 = 1.4;

pub struct AlphaZeroLite {
    mlp: Mlp,
    sims: u32,
    /// MCTS edge reward for eating food (default 0.3).
    eat_bonus: f32,
    debug: StrategyDebug,
}

struct Node {
    state: GameState,
    terminal: bool,
    expanded: bool,
    priors: [f32; 6],
    n: [u32; 6],
    w: [f32; 6],
    child: [i32; 6],
}

impl Node {
    fn leaf(state: GameState, terminal: bool) -> Self {
        Self {
            state,
            terminal,
            expanded: false,
            priors: [0.0; 6],
            n: [0; 6],
            w: [0.0; 6],
            child: [-1; 6],
        }
    }
}

impl AlphaZeroLite {
    pub fn new(mlp: Mlp, sims: u32) -> Self {
        Self::with_eat_bonus(mlp, sims, 0.3)
    }

    pub fn with_eat_bonus(mlp: Mlp, sims: u32, eat_bonus: f32) -> Self {
        Self {
            mlp,
            sims,
            eat_bonus,
            debug: StrategyDebug::default(),
        }
    }

    /// The checked-in embedded policy/value net (see
    /// `docs/training/alphazero/guide.md`). The search budget matches the one
    /// the embedded net was trained with — the value head is calibrated for
    /// that depth.
    pub fn embedded() -> Self {
        let mlp = Mlp::from_text(EMBEDDED_AZ).expect("embedded AlphaZero net must parse");
        Self::new(mlp, 24)
    }

    /// Policy priors (softmax over the five non-reverse actions) and a value
    /// in [-1, 1] from the net.
    fn evaluate(&self, state: &GameState) -> ([f32; 6], f32) {
        let out = self.mlp.forward(&az_features(state));
        let mut max = f32::NEG_INFINITY;
        for (a, &o) in out.iter().enumerate().take(6) {
            if a != REVERSE && o > max {
                max = o;
            }
        }
        let mut priors = [0.0f32; 6];
        let mut sum = 0.0;
        for a in 0..6 {
            if a != REVERSE {
                let e = (out[a] - max).exp();
                priors[a] = e;
                sum += e;
            }
        }
        if sum > 0.0 {
            for p in &mut priors {
                *p /= sum;
            }
        }
        (priors, out[AZ_OUTPUTS - 1].tanh())
    }

    /// PUCT action selection at an expanded node.
    fn select(node: &Node) -> usize {
        let total: u32 = node.n.iter().sum();
        let sqrt_total = ((total as f32) + 1.0).sqrt();
        let mut best = (f32::NEG_INFINITY, 0usize);
        for a in 0..6 {
            if a == REVERSE {
                continue;
            }
            let q = if node.n[a] > 0 {
                node.w[a] / node.n[a] as f32
            } else {
                0.0
            };
            let u = C_PUCT * node.priors[a] * sqrt_total / (1.0 + node.n[a] as f32);
            let score = q + u;
            if score > best.0 {
                best = (score, a);
            }
        }
        best.1
    }

    /// Run the tree search from `root`; returns per-action visit counts.
    fn search(&self, root: &GameState) -> [u32; 6] {
        let mut arena: Vec<Node> = vec![Node::leaf(root.clone(), false)];
        for _ in 0..self.sims {
            let mut path: Vec<(usize, usize)> = Vec::new();
            let mut idx = 0usize;
            let value;
            loop {
                if arena[idx].terminal {
                    value = -1.0;
                    break;
                }
                if !arena[idx].expanded {
                    let (p, v) = self.evaluate(&arena[idx].state);
                    arena[idx].priors = p;
                    arena[idx].expanded = true;
                    value = v;
                    break;
                }
                let a = Self::select(&arena[idx]);
                path.push((idx, a));
                let child = arena[idx].child[a];
                if child < 0 {
                    // Expand: apply the action to a cloned state. Distance to
                    // the food before the move drives the approach shaping.
                    let dist_before = {
                        let p = &arena[idx].state;
                        p.board().distance(p.head(), p.food())
                    };
                    let mut s = arena[idx].state.clone();
                    let dir = s.direction().rotated_cw(a as u8);
                    let score_before = s.score();
                    s.tick(Some(dir));
                    let terminal = s.status() != Status::Running;
                    let new_idx = arena.len();
                    let ate = s.score() > score_before;
                    let won = s.status() == Status::Won;
                    // Dense per-step reward folded into the edge value, so the
                    // search prefers food-ward moves even with an untrained
                    // value head (otherwise the policy collapses to circling).
                    let shaped = if ate {
                        self.eat_bonus
                    } else if !terminal {
                        let d = s.board().distance(s.head(), s.food());
                        0.1 * (dist_before as f32 - d as f32)
                    } else {
                        0.0
                    };
                    arena.push(Node::leaf(s, terminal));
                    arena[idx].child[a] = new_idx as i32;
                    value = if terminal {
                        if won {
                            1.0
                        } else {
                            -1.0
                        }
                    } else {
                        let (p, v) = self.evaluate(&arena[new_idx].state);
                        arena[new_idx].priors = p;
                        arena[new_idx].expanded = true;
                        (v + shaped).clamp(-1.0, 1.0)
                    };
                    break;
                }
                idx = child as usize;
            }
            for (nidx, a) in path {
                arena[nidx].n[a] += 1;
                arena[nidx].w[a] += value;
            }
        }
        arena[0].n
    }
}

impl Strategy for AlphaZeroLite {
    fn next_move(&mut self, state: &GameState) -> Direction {
        let visits = self.search(state);
        let heading = state.direction();

        self.debug.move_scores.clear();
        for (a, &v) in visits.iter().enumerate() {
            if a != REVERSE {
                self.debug
                    .move_scores
                    .push((heading.rotated_cw(a as u8), f64::from(v)));
            }
        }

        // Pick the most-visited move among the immediately safe ones.
        let safe: Vec<Direction> = safe_moves(state).into_iter().map(|(d, _)| d).collect();
        let mut best: Option<(Direction, u32)> = None;
        for (a, &v) in visits.iter().enumerate() {
            if a == REVERSE {
                continue;
            }
            let dir = heading.rotated_cw(a as u8);
            if !safe.contains(&dir) {
                continue;
            }
            if best.is_none_or(|(_, bn)| v > bn) {
                best = Some((dir, v));
            }
        }
        best.map(|(d, _)| d).unwrap_or_else(|| doomed_move(state))
    }

    fn debug(&self) -> Option<&StrategyDebug> {
        Some(&self.debug)
    }
}

/// Embedded AlphaZero-light policy/value net (`.mlp`, 7 outputs).
pub const EMBEDDED_AZ: &str = include_str!("../../assets/alphazero/best.mlp");

/// One self-play training sample for gradient training (Python side).
#[derive(Debug, Clone)]
pub struct AzSample {
    /// Sensor features at the state.
    pub features: Vec<f32>,
    /// MCTS policy target: normalized visit counts over the six relative
    /// actions (the reverse action stays 0).
    pub policy: [f32; 6],
    /// Value target: tanh of the discounted return from this state.
    pub value: f32,
}

/// Outcome of one self-play game: the training samples plus the game-level
/// stats the trainer needs to pick checkpoints by the *real* objective
/// (food eaten), not by survival time.
#[derive(Debug, Clone)]
pub struct SelfPlayResult {
    /// Per-step training samples.
    pub samples: Vec<AzSample>,
    /// Final score (food eaten) of the game.
    pub score: u32,
    /// Number of ticks the game lasted.
    pub ticks: u64,
}

/// Discount for the self-play value target.
const SELFPLAY_GAMMA: f32 = 0.99;

/// Play one self-play game with `mlp` driving the MCTS and return training
/// samples. Moves are sampled from the visit counts with `temperature`
/// (0 ⇒ greedy) for exploration. Uses the *same* search as inference, so the
/// policy targets match what `AlphaZeroLite` actually does — no second MCTS to
/// keep in sync.
pub fn self_play(
    mlp: &Mlp,
    config: crate::game::Config,
    sims: u32,
    temperature: f32,
    seed: u64,
    max_ticks: u64,
) -> Vec<AzSample> {
    self_play_with_rewards(mlp, config, sims, temperature, seed, max_ticks, 0.3, 1.0).samples
}

#[allow(clippy::too_many_arguments)]
pub fn self_play_with_rewards(
    mlp: &Mlp,
    config: crate::game::Config,
    sims: u32,
    temperature: f32,
    seed: u64,
    max_ticks: u64,
    eat_bonus: f32,
    sp_eat: f32,
) -> SelfPlayResult {
    use rand::SeedableRng as _;

    let az = AlphaZeroLite::with_eat_bonus(mlp.clone(), sims, eat_bonus);
    let mut rng = rand_pcg::Pcg64::seed_from_u64(seed);
    let mut state = GameState::new(config);

    let mut steps: Vec<(Vec<f32>, [f32; 6])> = Vec::new();
    let mut rewards: Vec<f32> = Vec::new();

    while state.status() == Status::Running && state.ticks() < max_ticks {
        let visits = az.search(&state);
        let total: u32 = visits.iter().sum();
        let mut policy = [0.0f32; 6];
        if total > 0 {
            for (a, &v) in visits.iter().enumerate() {
                policy[a] = v as f32 / total as f32;
            }
        }
        steps.push((az_features(&state), policy));

        let action = sample_action(&visits, temperature, &mut rng);
        let score_before = state.score();
        // Food-approach shaping needs the distance before the move (mirrors
        // the RL env reward, so the value target carries a dense navigation
        // signal — without it the search never discovers food and the policy
        // collapses to safe circling).
        let dist_before = state.board().distance(state.head(), state.food());
        let dir = state.direction().rotated_cw(action as u8);
        state.tick(Some(dir));

        let mut reward = -0.005;
        if state.score() > score_before {
            reward += sp_eat;
        } else {
            let dist = state.board().distance(state.head(), state.food());
            reward += 0.1 * (dist_before as f32 - dist as f32);
        }
        match state.status() {
            Status::GameOver => reward -= 1.0,
            Status::Won => reward += 2.0,
            Status::Running => {}
        }
        rewards.push(reward);
    }

    // Discounted returns → tanh value targets (computed back-to-front).
    let mut g = 0.0f32;
    let mut values = vec![0.0f32; rewards.len()];
    for t in (0..rewards.len()).rev() {
        g = rewards[t] + SELFPLAY_GAMMA * g;
        values[t] = g.tanh();
    }

    let samples = steps
        .into_iter()
        .zip(values)
        .map(|((features, policy), value)| AzSample {
            features,
            policy,
            value,
        })
        .collect();

    SelfPlayResult {
        samples,
        score: state.score(),
        ticks: state.ticks(),
    }
}

/// Sample an action from visit counts weighted by `count^(1/temperature)`.
/// `temperature <= 0` selects the most-visited action (greedy).
fn sample_action(visits: &[u32; 6], temperature: f32, rng: &mut rand_pcg::Pcg64) -> usize {
    use rand::Rng as _;
    if temperature <= 1e-3 {
        return visits
            .iter()
            .enumerate()
            .max_by_key(|(_, &v)| v)
            .map(|(i, _)| i)
            .unwrap_or(0);
    }
    let inv_t = 1.0 / temperature as f64;
    let mut weights = [0.0f64; 6];
    let mut sum = 0.0;
    for (a, &v) in visits.iter().enumerate() {
        if v > 0 {
            let w = (v as f64).powf(inv_t);
            weights[a] = w;
            sum += w;
        }
    }
    if sum <= 0.0 {
        return 0;
    }
    let mut pick = rng.random::<f64>() * sum;
    for (a, &w) in weights.iter().enumerate() {
        pick -= w;
        if pick <= 0.0 {
            return a;
        }
    }
    5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Config, Status};

    #[test]
    fn embedded_net_has_value_head() {
        let az = AlphaZeroLite::embedded();
        assert_eq!(*az.mlp.dims().last().unwrap(), AZ_OUTPUTS);
    }

    #[test]
    fn self_play_yields_well_formed_samples() {
        use crate::nn::AZ_FEATURE_COUNT;
        let mlp = AlphaZeroLite::embedded().mlp;
        let config = Config {
            seed: 5,
            ..Config::default()
        };
        let a = self_play(&mlp, config, 12, 1.0, 99, 500);
        let b = self_play(&mlp, config, 12, 1.0, 99, 500);
        assert!(!a.is_empty());
        assert_eq!(a.len(), b.len(), "same seed ⇒ identical self-play");
        for s in &a {
            assert_eq!(s.features.len(), AZ_FEATURE_COUNT);
            assert!((-1.0..=1.0).contains(&s.value));
            assert_eq!(s.policy[REVERSE], 0.0, "reverse never gets visits");
            let sum: f32 = s.policy.iter().sum();
            assert!((sum - 1.0).abs() < 1e-4 || sum == 0.0);
        }
    }

    #[test]
    fn plays_legal_moves() {
        let mut az = AlphaZeroLite::embedded();
        let mut state = GameState::new(Config::default());
        for _ in 0..150 {
            if state.status() != Status::Running {
                break;
            }
            let had_safe = !safe_moves(&state).is_empty();
            let dir = az.next_move(&state);
            state.tick(Some(dir));
            if had_safe {
                assert_eq!(
                    state.status(),
                    Status::Running,
                    "AlphaZero-light must not step into death while a safe move exists"
                );
            }
        }
    }
}
