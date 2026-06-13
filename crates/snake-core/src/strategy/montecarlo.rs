use rand::{Rng as _, SeedableRng as _};
use rand_pcg::Pcg32;

use super::{doomed_move, flood_fill, safe_moves, Strategy, StrategyDebug};
use crate::coords::Direction;
use crate::game::{GameState, Status};

/// Default rollouts per candidate move. Together with the horizon this is
/// the per-tick budget — keep it small enough for WASM at high tick rates.
const DEFAULT_ROLLOUTS: usize = 12;
/// Default rollout length in ticks.
const DEFAULT_HORIZON: u32 = 40;

/// Reward weights: eating food dominates, surviving matters, and free
/// space at the end of the rollout breaks the camping habit.
const FOOD_REWARD: f64 = 30.0;
const SURVIVAL_REWARD_PER_TICK: f64 = 1.0;
const SPACE_REWARD: f64 = 10.0;

/// Monte-Carlo lookahead: for every legal first move, play random (safe)
/// rollouts and average their reward. Plays "humanly risky" and differs
/// from run to run — but deterministically per seed.
pub struct MonteCarlo {
    rng: Pcg32,
    rollouts: usize,
    horizon: u32,
    debug: StrategyDebug,
    /// Reusable buffer for flood_fill to avoid per-rollout heap allocation.
    fill_buf: Vec<super::Offset>,
}

impl MonteCarlo {
    pub fn new(seed: u64) -> Self {
        Self::with_params(seed, DEFAULT_ROLLOUTS, DEFAULT_HORIZON)
    }

    /// Custom budget, mainly for fast-running tests.
    pub fn with_params(seed: u64, rollouts: usize, horizon: u32) -> Self {
        Self {
            rng: Pcg32::seed_from_u64(seed),
            rollouts,
            horizon,
            debug: StrategyDebug::default(),
            fill_buf: Vec::new(),
        }
    }

    fn rollout(&mut self, state: &GameState, first: Direction) -> f64 {
        let mut sim = state.clone();
        sim.clear_input_queue();
        sim.tick(Some(first));
        let mut survived = 1u32;
        while survived < self.horizon && sim.status() == Status::Running {
            let moves = safe_moves(&sim);
            let dir = if moves.is_empty() {
                doomed_move(&sim)
            } else {
                moves[self.rng.random_range(0..moves.len())].0
            };
            sim.tick(Some(dir));
            survived += 1;
        }

        let food = f64::from(sim.score() - state.score());
        let mut reward = food * FOOD_REWARD + f64::from(survived) * SURVIVAL_REWARD_PER_TICK;
        if sim.status() == Status::Running {
            flood_fill(&sim, sim.head(), &mut self.fill_buf);
            reward += SPACE_REWARD * self.fill_buf.len() as f64 / sim.board().num_cells() as f64;
        }
        reward
    }
}

impl Strategy for MonteCarlo {
    fn next_move(&mut self, state: &GameState) -> Direction {
        self.debug.move_scores.clear();
        self.debug.path.clear();
        self.debug.heatmap.clear();

        let mut best: Option<(Direction, f64)> = None;
        for (dir, _) in safe_moves(state) {
            let mut total = 0.0;
            for _ in 0..self.rollouts {
                total += self.rollout(state, dir);
            }
            let avg = total / self.rollouts.max(1) as f64;
            self.debug.move_scores.push((dir, avg));
            // Strictly-greater keeps Direction::ALL order on exact ties.
            if best.is_none_or(|(_, b)| avg > b) {
                best = Some((dir, avg));
            }
        }
        best.map(|(dir, _)| dir)
            .unwrap_or_else(|| doomed_move(state))
    }

    fn debug(&self) -> Option<&StrategyDebug> {
        Some(&self.debug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmark::run_series;
    use crate::board::BoundaryMode;
    use crate::game::Config;
    use crate::strategy::ChaosWalker;

    #[test]
    fn is_deterministic_per_seed() {
        let state = GameState::new(Config::default());
        let mut a = MonteCarlo::with_params(9, 4, 12);
        let mut b = MonteCarlo::with_params(9, 4, 12);
        for _ in 0..20 {
            assert_eq!(a.next_move(&state), b.next_move(&state));
        }
    }

    #[test]
    fn beats_chaos_clearly() {
        let base = Config {
            width: 12,
            height: 10,
            boundary: BoundaryMode::Walls,
            seed: 0,
        };
        let games = 4;
        let max_ticks = 1_200;
        let chaos = run_series(
            &mut |seed| Box::new(ChaosWalker::new(seed)),
            base,
            games,
            max_ticks,
        );
        // Reduced budget keeps the debug-profile test fast.
        let mc = run_series(
            &mut |seed| Box::new(MonteCarlo::with_params(seed, 6, 25)),
            base,
            games,
            max_ticks,
        );
        assert!(
            mc.avg_score > 2.0 * chaos.avg_score,
            "monte carlo ({:.2}) should clearly beat chaos ({:.2})",
            mc.avg_score,
            chaos.avg_score
        );
    }
}
