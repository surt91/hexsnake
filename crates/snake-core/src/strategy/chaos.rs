use rand::{Rng as _, SeedableRng as _};
use rand_pcg::Pcg32;

use super::{doomed_move, safe_moves, Strategy};
use crate::coords::Direction;
use crate::game::GameState;

/// Random baseline: picks uniformly among the moves that are not
/// immediately fatal.
pub struct ChaosWalker {
    rng: Pcg32,
}

impl ChaosWalker {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Pcg32::seed_from_u64(seed),
        }
    }
}

impl Strategy for ChaosWalker {
    fn next_move(&mut self, state: &GameState) -> Direction {
        let moves = safe_moves(state);
        if moves.is_empty() {
            return doomed_move(state);
        }
        moves[self.rng.random_range(0..moves.len())].0
    }
}
