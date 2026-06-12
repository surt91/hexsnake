use super::{doomed_move, flood_fill, safe_moves, Strategy, StrategyDebug};
use crate::coords::Direction;
use crate::game::GameState;

/// Flood-fill survivalist ("Raumgreifer"): picks the move that keeps the
/// most free cells reachable; food distance only breaks ties. Plays very
/// defensively — survives long, scores slowly.
pub struct SpaceKeeper {
    debug: StrategyDebug,
}

impl SpaceKeeper {
    pub fn new() -> Self {
        Self {
            debug: StrategyDebug::default(),
        }
    }
}

impl Default for SpaceKeeper {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for SpaceKeeper {
    fn next_move(&mut self, state: &GameState) -> Direction {
        let board = *state.board();
        let mut best: Option<(Direction, usize, u32)> = None;
        let mut reachable = Vec::new();
        let mut best_reachable = Vec::new();
        self.debug.move_scores.clear();

        for (dir, _) in safe_moves(state) {
            // Exact one-step simulation (handles tail vacating and growth).
            let mut sim = state.clone();
            sim.clear_input_queue();
            sim.tick(Some(dir));
            flood_fill(&sim, sim.head(), &mut reachable);
            let space = reachable.len();
            let food_dist = board.distance(sim.head(), sim.food());
            self.debug.move_scores.push((dir, space as f64));

            let better = match best {
                None => true,
                // Maximize space; on equal space prefer smaller food
                // distance; final tie falls to Direction::ALL order.
                Some((_, s, d)) => space > s || (space == s && food_dist < d),
            };
            if better {
                best = Some((dir, space, food_dist));
                std::mem::swap(&mut best_reachable, &mut reachable);
            }
        }

        self.debug.heatmap = best_reachable;
        self.debug.path.clear();
        best.map(|(dir, _, _)| dir)
            .unwrap_or_else(|| doomed_move(state))
    }

    fn debug(&self) -> Option<&StrategyDebug> {
        Some(&self.debug)
    }
}
