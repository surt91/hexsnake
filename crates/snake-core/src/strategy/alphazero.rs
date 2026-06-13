//! AlphaZero-light: a Monte-Carlo *tree* search guided by a policy/value
//! network, replacing the random rollouts of the plain `MonteCarlo` strategy.
//!
//! The network is one MLP with 7 outputs: six policy logits (heading-relative
//! directions, like the other net strategies) and one value. PUCT selection
//! uses the policy as priors; leaves are evaluated by the value head instead
//! of rolling out random games. The net is trained gradient-free by the GA
//! trainer (`snake-train az`) — "self-play" here means the net guides its own
//! lookahead and we evolve it on the resulting game scores.

use crate::coords::Direction;
use crate::game::{GameState, Status};
use crate::nn::{features, Mlp};
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
        Self {
            mlp,
            sims,
            debug: StrategyDebug::default(),
        }
    }

    /// The checked-in embedded policy/value net (smoke artifact until the
    /// real training — see `docs/training/alphazero.md`). The search budget
    /// matches the one the embedded net was trained with: the value head is
    /// only calibrated for that depth, and more sims would over-trust it.
    pub fn embedded() -> Self {
        let mlp = Mlp::from_text(EMBEDDED_AZ).expect("embedded AlphaZero net must parse");
        Self::new(mlp, 16)
    }

    /// Policy priors (softmax over the five non-reverse actions) and a value
    /// in [-1, 1] from the net.
    fn evaluate(&self, state: &GameState) -> ([f32; 6], f32) {
        let out = self.mlp.forward(&features(state));
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
                    // Expand: apply the action to a cloned state.
                    let mut s = arena[idx].state.clone();
                    let dir = s.direction().rotated_cw(a as u8);
                    let score_before = s.score();
                    s.tick(Some(dir));
                    let terminal = s.status() != Status::Running;
                    let new_idx = arena.len();
                    let ate = s.score() > score_before;
                    let won = s.status() == Status::Won;
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
                        // A small bonus for eating keeps the search hungry.
                        (v + if ate { 0.3 } else { 0.0 }).clamp(-1.0, 1.0)
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
